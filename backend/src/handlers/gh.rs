use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::process::Command;
use crate::{error::AppError, state::AppState};

#[derive(Deserialize)]
pub struct GhExecuteRequest {
    pub command: String, // e.g. "issue list --repo owner/repo --json number,title,state"
}

#[derive(Serialize)]
pub struct GhExecuteResponse {
    pub output: serde_json::Value,
    pub raw: String,
}

pub async fn execute_gh(
    State(_state): State<AppState>,
    Json(req): Json<GhExecuteRequest>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    // Parse the command string into args, stripping leading "gh " if present
    let cmd_str = req.command.trim_start_matches("gh ").trim();
    let args: Vec<&str> = cmd_str.split_whitespace().collect();

    let output = Command::new("gh")
        .args(&args)
        .output()
        .map_err(|e| AppError::Command(format!("Failed to run gh: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(AppError::Command(format!("gh exited with error: {stderr}")));
    }

    // Try to parse as JSON, fall back to wrapping in a string
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| serde_json::Value::String(stdout.clone()));

    Ok(Json(GhExecuteResponse { output: parsed, raw: stdout }))
}
