use axum::{extract::State, Json};
use chrono::{Duration, Local};
use serde::{Deserialize, Serialize};
use sha1::{Sha1, Digest};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Command;
use tokio::time::timeout;
use crate::{error::AppError, state::AppState};

#[derive(Deserialize)]
pub struct GhExecuteRequest {
    pub command: String,
}

#[derive(Serialize)]
pub struct GhExecuteResponse {
    pub output: serde_json::Value,
    pub raw: String,
    pub cached: bool,
}

#[derive(Serialize)]
pub struct CacheStatusResponse {
    pub cache_dir: String,
    pub ttl_secs: u64,
    pub invalidated_at: Option<u64>,
}

#[derive(Serialize)]
pub struct InvalidateCacheResponse {
    pub invalidated_at: u64,
}

// Replaces {{today}} and {{date:-Nd}} with ISO dates, e.g. {{date:-7d}} → 2026-05-20
fn substitute_date_vars(cmd: &str) -> String {
    let mut result = cmd.to_string();
    let today = Local::now().date_naive();

    result = result.replace("{{today}}", &today.format("%Y-%m-%d").to_string());

    while let Some(start) = result.find("{{date:-") {
        let end = match result[start..].find("}}") {
            Some(i) => start + i + 2,
            None => break,
        };
        let token = &result[start..end];
        let inner = &token[8..token.len() - 2]; // strip "{{date:-" and "}}"
        let replaced = if let Some(days_str) = inner.strip_suffix('d') {
            if let Ok(days) = days_str.parse::<i64>() {
                (today - Duration::days(days)).format("%Y-%m-%d").to_string()
            } else {
                token.to_string()
            }
        } else {
            token.to_string()
        };
        result = format!("{}{}{}", &result[..start], replaced, &result[end..]);
    }

    result
}

// Splits a command string into args, respecting single- and double-quoted spans.
fn parse_args(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = cmd.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' | '\'' => {
                let quote = c;
                for inner in chars.by_ref() {
                    if inner == quote { break; }
                    current.push(inner);
                }
            }
            c if c.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() { args.push(current); }
    args
}

fn sha1_hex(data: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn cache_path(cache_dir: &Path, cmd: &str) -> PathBuf {
    let hash = sha1_hex(cmd);
    let (prefix, rest) = hash.split_at(2);
    cache_dir.join(prefix).join(rest)
}

fn invalidated_at_path(cache_dir: &Path) -> PathBuf {
    cache_dir.join(".invalidated_at")
}

fn read_invalidated_at(cache_dir: &Path) -> Option<u64> {
    let content = std::fs::read_to_string(invalidated_at_path(cache_dir)).ok()?;
    content.trim().parse().ok()
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn file_mtime_secs(path: &Path) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    Some(mtime.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
}

pub async fn execute_gh(
    State(state): State<AppState>,
    Json(req): Json<GhExecuteRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let substituted = substitute_date_vars(req.command.trim_start_matches("gh ").trim());
    let cmd_str = substituted.as_str();
    let path = cache_path(&state.cache_dir, cmd_str);
    let now = unix_now();

    // Check cache
    if let Some(mtime) = file_mtime_secs(&path) {
        let age = now.saturating_sub(mtime);
        if age <= state.cache_ttl_secs {
            // Check lazy invalidation
            let invalidated = read_invalidated_at(&state.cache_dir).unwrap_or(0);
            if mtime > invalidated {
                // Cache hit — serve from disk
                if let Ok(raw) = std::fs::read_to_string(&path) {
                    let parsed: serde_json::Value = serde_json::from_str(&raw)
                        .unwrap_or_else(|_| serde_json::Value::String(raw.clone()));
                    return Ok(Json(GhExecuteResponse { output: parsed, raw, cached: true }));
                }
            }
        }
    }

    // Cache miss — run gh
    let (parsed, stdout) = run_gh(cmd_str).await?;

    // Best-effort cache write — never fail the request on write errors
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &stdout);

    Ok(Json(GhExecuteResponse { output: parsed, raw: stdout, cached: false }))
}

/// How long a single `gh` invocation may run before it is killed. A hung `gh`
/// must not wedge a multi-call sync.
const GH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Run `gh` with an explicit argv and parse stdout as JSON. Bypasses the disk
/// cache (which lives in `execute_gh`, not here).
///
/// Prefer this over [`run_gh`] whenever an argument may contain newlines,
/// spaces or quotes — a GraphQL document, for instance. [`parse_args`] would
/// shred such an argument into dozens of argv entries; passing the vector
/// directly is the only way to keep it intact.
pub(crate) async fn run_gh_args(args: &[String]) -> Result<(serde_json::Value, String), AppError> {
    let child = Command::new("gh")
        .args(args)
        .kill_on_drop(true)
        .output();

    let output = timeout(GH_TIMEOUT, child)
        .await
        .map_err(|_| AppError::Command(format!(
            "gh timed out after {}s",
            GH_TIMEOUT.as_secs()
        )))?
        .map_err(|e| AppError::Command(format!("Failed to run gh: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AppError::Command(format!("gh exited with error: {stderr}")));
    }

    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| serde_json::Value::String(stdout.clone()));

    Ok((parsed, stdout))
}

/// Run `gh <cmd>` and parse stdout as JSON, bypassing the disk cache. Returns
/// `(parsed, raw)`; non-JSON output comes back as a `Value::String`.
pub(crate) async fn run_gh(cmd: &str) -> Result<(serde_json::Value, String), AppError> {
    run_gh_args(&parse_args(cmd)).await
}

pub async fn invalidate_cache(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let ts = unix_now();
    let path = invalidated_at_path(&state.cache_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&path, ts.to_string())
        .map_err(|e| AppError::Command(format!("Failed to write invalidated_at: {e}")))?;
    Ok(Json(InvalidateCacheResponse { invalidated_at: ts }))
}

pub async fn cache_status(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let invalidated_at = read_invalidated_at(&state.cache_dir);
    Ok(Json(CacheStatusResponse {
        cache_dir: state.cache_dir.display().to_string(),
        ttl_secs: state.cache_ttl_secs,
        invalidated_at,
    }))
}
