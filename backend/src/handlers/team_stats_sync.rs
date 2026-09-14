//! Team Stats sync: pull PR + review facts from GitHub via `gh api graphql`.
//!
//! Deliberately synchronous-from-the-caller's-view and manual-only: there is no
//! cron and no background task. With the default 2026-08-01 backfill floor a
//! cold run across five repos is ~450 PRs ≈ 26 GraphQL calls ≈ 30s, so the
//! whole thing finishes inside one request and needs no chunking or resume.

use axum::{extract::State, Json};
use chrono::{Duration, Utc};
use serde_json::Value;
use std::collections::HashSet;

use crate::{
    error::AppError,
    handlers::gh::run_gh_args,
    models::team_stats::{
        IngestCounts, PrFactRow, PrIngest, RateLimitInfo, ReviewFactRow, ReviewRequestRow,
        SyncRepoResult, SyncRequest, SyncResult,
    },
    state::AppState,
};

/// `reviews(first: 50)` is comfortably above the observed distribution
/// (median 1, p95 5, max 7 over a 100-PR sample); `totalCount` plus
/// `pageInfo.hasNextPage` detect the rare overflow rather than letting it skew
/// the numbers silently.
const PR_SEARCH_QUERY: &str = r#"
query($q: String!, $cursor: String) {
  rateLimit { cost remaining resetAt }
  search(query: $q, type: ISSUE, first: 25, after: $cursor) {
    issueCount
    pageInfo { hasNextPage endCursor }
    nodes { ... on PullRequest {
      id number title url state isDraft createdAt updatedAt mergedAt closedAt
      additions deletions changedFiles
      author { login } mergedBy { login }
      repository { nameWithOwner }
      reviews(first: 50) {
        totalCount
        pageInfo { hasNextPage }
        nodes { id author { login } state submittedAt }
      }
      reviewRequests(first: 20) {
        totalCount
        nodes { requestedReviewer { ... on User { login } ... on Team { slug } } }
      }
      comments { totalCount }
    } }
  }
}
"#;

/// GitHub's search index is eventually consistent, so the cursor is rewound by
/// this much on every successful run.
const CURSOR_SLACK_MINUTES: i64 = 10;
/// Stop before exhausting the hourly budget rather than failing mid-page.
const MIN_RATE_LIMIT_REMAINING: i64 = 100;
/// GitHub's *secondary* limits punish bursts of graphql calls harder than the
/// point budget does.
const INTER_CALL_SLEEP_MS: u64 = 250;

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string())
}
fn i(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(|x| x.as_i64()).unwrap_or(0)
}
fn login(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.get("login")).and_then(|x| x.as_str()).map(|x| x.to_string())
}

/// One page of search results plus the rate-limit reading that came with it.
struct Page {
    nodes: Vec<Value>,
    next: Option<String>,
    cost: i64,
    remaining: i64,
    reset_at: Option<String>,
    issue_count: i64,
}

async fn fetch_page(q: &str, cursor: Option<&str>) -> Result<Page, AppError> {
    let mut args = vec![
        "api".to_string(),
        "graphql".to_string(),
        "-f".to_string(),
        format!("query={PR_SEARCH_QUERY}"),
        "-f".to_string(),
        format!("q={q}"),
    ];
    // `-F` coerces types, so it is only safe for the literal null on page one;
    // an opaque base64 cursor must go through `-f`.
    match cursor {
        Some(c) => args.extend(["-f".to_string(), format!("cursor={c}")]),
        None => args.extend(["-F".to_string(), "cursor=null".to_string()]),
    }

    let (parsed, _) = run_gh_args(&args).await?;
    let data = parsed
        .get("data")
        .ok_or_else(|| AppError::Command(format!("gh graphql returned no data: {parsed}")))?;
    let rl = data.get("rateLimit").cloned().unwrap_or(Value::Null);
    let search = data
        .get("search")
        .ok_or_else(|| AppError::Command("gh graphql response had no search field".into()))?;
    let page_info = search.get("pageInfo").cloned().unwrap_or(Value::Null);

    Ok(Page {
        nodes: search
            .get("nodes")
            .and_then(|n| n.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|n| n.is_object() && !n.as_object().unwrap().is_empty())
            .collect(),
        next: if page_info.get("hasNextPage").and_then(|b| b.as_bool()).unwrap_or(false) {
            s(&page_info, "endCursor")
        } else {
            None
        },
        cost: i(&rl, "cost"),
        remaining: i(&rl, "remaining"),
        reset_at: s(&rl, "resetAt"),
        issue_count: i(search, "issueCount"),
    })
}

/// Map one GraphQL PR node onto the rows we store.
fn to_ingest(node: &Value) -> Option<(PrIngest, i64)> {
    let repo = node
        .get("repository")?
        .get("nameWithOwner")?
        .as_str()?
        .to_lowercase(); // GitHub repo names are case-insensitive
    let number = node.get("number")?.as_i64()?;
    let author = login(node, "author");
    let merged_at = s(node, "mergedAt");
    // GitHub reports a merged PR's `state` as MERGED already, but derive it
    // from mergedAt so the two can never disagree.
    let state = if merged_at.is_some() {
        "MERGED".to_string()
    } else {
        s(node, "state").unwrap_or_else(|| "OPEN".into())
    };

    let reviews_node = node.get("reviews").cloned().unwrap_or(Value::Null);
    let reviews_truncated = reviews_node
        .get("pageInfo")
        .and_then(|p| p.get("hasNextPage"))
        .and_then(|b| b.as_bool())
        .unwrap_or(false);

    let mut skipped = 0i64;
    let mut reviews = Vec::new();
    if let Some(author) = author.as_ref() {
        for rv in reviews_node.get("nodes").and_then(|n| n.as_array()).cloned().unwrap_or_default() {
            // A null author is a deleted account; a null submittedAt is a
            // still-PENDING review. Count them rather than dropping silently.
            let (Some(reviewer), Some(submitted_at)) = (login(&rv, "author"), s(&rv, "submittedAt"))
            else {
                skipped += 1;
                continue;
            };
            reviews.push(ReviewFactRow {
                node_id: s(&rv, "id"),
                reviewer_login: reviewer,
                author_login: author.clone(),
                state: s(&rv, "state").unwrap_or_else(|| "COMMENTED".into()),
                submitted_at,
            });
        }
    } else {
        skipped += reviews_node.get("nodes").and_then(|n| n.as_array()).map_or(0, |a| a.len() as i64);
    }

    let requests_node = node.get("reviewRequests").cloned().unwrap_or(Value::Null);
    let mut requests = Vec::new();
    for rq in requests_node.get("nodes").and_then(|n| n.as_array()).cloned().unwrap_or_default() {
        let Some(r) = rq.get("requestedReviewer") else { continue };
        if let Some(l) = r.get("login").and_then(|x| x.as_str()) {
            requests.push(ReviewRequestRow {
                requested_login: l.to_string(),
                requested_type: "user".into(),
            });
        } else if let Some(t) = r.get("slug").and_then(|x| x.as_str()) {
            requests.push(ReviewRequestRow {
                requested_login: t.to_string(),
                requested_type: "team".into(),
            });
        }
    }

    Some((
        PrIngest {
            pr: PrFactRow {
                repo,
                number,
                node_id: s(node, "id"),
                title: s(node, "title").unwrap_or_default(),
                url: s(node, "url").unwrap_or_default(),
                state,
                is_draft: node.get("isDraft").and_then(|b| b.as_bool()).unwrap_or(false),
                author_login: author,
                merged_by_login: login(node, "mergedBy"),
                gh_created_at: s(node, "createdAt").unwrap_or_default(),
                gh_updated_at: s(node, "updatedAt").unwrap_or_default(),
                merged_at,
                closed_at: s(node, "closedAt"),
                additions: i(node, "additions"),
                deletions: i(node, "deletions"),
                changed_files: i(node, "changedFiles"),
                comment_count: node.get("comments").map_or(0, |c| i(c, "totalCount")),
                review_total: i(&reviews_node, "totalCount"),
                reviews_truncated,
                requests_truncated: i(&requests_node, "totalCount") > 20,
            },
            reviews,
            requests,
            reviews_complete: !reviews_truncated,
        },
        skipped,
    ))
}

struct RunBudget {
    cost: i64,
    remaining: i64,
    reset_at: Option<String>,
}

/// Page through one search query, ingesting as we go.
async fn drain(
    state: &AppState,
    q: &str,
    budget: &mut RunBudget,
    seen: &mut HashSet<i64>,
) -> Result<(IngestCounts, i64), AppError> {
    let mut counts = IngestCounts::default();
    let mut pages = 0i64;
    let mut cursor: Option<String> = None;

    loop {
        let page = fetch_page(q, cursor.as_deref()).await?;
        pages += 1;
        budget.cost += page.cost;
        budget.remaining = page.remaining;
        budget.reset_at = page.reset_at.clone();

        // GitHub's search API hard-stops at 1000 results however large
        // issueCount is. With an August floor the busiest repo returns ~160, so
        // this is insurance, not a code path we expect to hit.
        if pages == 1 && page.issue_count >= 1000 {
            tracing::warn!(
                query = q,
                issue_count = page.issue_count,
                "search returned >=1000 results; only the first 1000 are reachable"
            );
        }

        let mut batch = Vec::new();
        for node in &page.nodes {
            if let Some((ingest, skipped)) = to_ingest(node) {
                seen.insert(ingest.pr.number);
                counts.skipped += skipped;
                batch.push(ingest);
            }
        }
        if !batch.is_empty() {
            let c = state.pr_facts.ingest_page(batch).await?;
            counts.merge(&c);
        }

        if budget.remaining > 0 && budget.remaining < MIN_RATE_LIMIT_REMAINING {
            return Err(AppError::Command(format!(
                "stopping: GraphQL rate limit nearly exhausted ({} remaining)",
                budget.remaining
            )));
        }
        match page.next {
            Some(c) => cursor = Some(c),
            None => break,
        }
        tokio::time::sleep(std::time::Duration::from_millis(INTER_CALL_SLEEP_MS)).await;
    }
    Ok((counts, pages))
}

pub async fn sync_team_stats(
    State(state): State<AppState>,
    body: Option<Json<SyncRequest>>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    // A second concurrent run would race the same cursors.
    let _guard = state.stats_sync_lock.try_lock().map_err(|_| {
        AppError::BadRequest("A Team Stats sync is already running".into())
    })?;

    let req = body.map(|Json(b)| b).unwrap_or_default();
    let started = Utc::now();
    // Anchored to run start, NOT to max(updatedAt) — see mark_synced below.
    let cursor_value = (started - Duration::minutes(CURSOR_SLACK_MINUTES)).to_rfc3339();

    let only: HashSet<String> = req.repos.iter().map(|r| r.to_lowercase()).collect();
    let tracked = state.repos.list_tracked().await?;

    let mut budget = RunBudget { cost: 0, remaining: 0, reset_at: None };
    let mut results = Vec::new();
    let mut errors = Vec::new();
    let mut totals = IngestCounts::default();

    for repo in tracked {
        let name = repo.owner_repo.trim().to_lowercase();
        if !only.is_empty() && !only.contains(&name) {
            continue;
        }

        let st = state.pr_facts.get_sync_state(&name).await?;
        let backfilled = st.as_ref().map(|s| s.backfilled).unwrap_or(false);
        let backfill_from = st
            .as_ref()
            .and_then(|s| s.backfill_from.clone())
            .unwrap_or_else(|| req.backfill_from.clone());
        let cursor = st.as_ref().and_then(|s| s.updated_cursor.clone());

        let (mode, main_q) = match (backfilled, cursor.as_deref()) {
            (true, Some(c)) => (
                "incremental",
                format!("repo:{name} is:pr sort:created-asc updated:>={c}"),
            ),
            // `sort:created-asc` is mandatory: createdAt is immutable, so page
            // cursors stay stable. Sorting by updated would let a PR touched
            // mid-run jump pages and be skipped.
            _ => (
                "backfill",
                format!("repo:{name} is:pr sort:created-asc created:>={backfill_from}"),
            ),
        };

        let mut seen = HashSet::new();
        let outcome: Result<(IngestCounts, i64), AppError> = async {
            let (mut c, mut pages) = drain(&state, &main_q, &mut budget, &mut seen).await?;
            tokio::time::sleep(std::time::Duration::from_millis(INTER_CALL_SLEEP_MS)).await;

            // Open sweep, every run. Cheap (~190 open PRs across all repos) and
            // it keeps the worklist exact regardless of the cursor window.
            let open_q = format!("repo:{name} is:pr sort:created-asc is:open");
            let (c2, p2) = drain(&state, &open_q, &mut budget, &mut seen).await?;
            c.merge(&c2);
            pages += p2;

            // Anything we still hold as OPEN but the sweep didn't return has
            // closed outside the cursor window. Re-fetch just those.
            let stale: Vec<i64> = state
                .pr_facts
                .open_numbers(&name)
                .await?
                .into_iter()
                .filter(|n| !seen.contains(n))
                .collect();
            if !stale.is_empty() {
                for chunk in stale.chunks(20) {
                    let terms: Vec<String> = chunk.iter().map(|n| n.to_string()).collect();
                    let q = format!(
                        "repo:{name} is:pr sort:created-asc {}",
                        terms.iter().map(|n| format!("{n} in:number")).collect::<Vec<_>>().join(" OR ")
                    );
                    // Best-effort: a failure here only leaves a stale OPEN row,
                    // which the next run retries.
                    if let Ok((c3, p3)) = drain(&state, &q, &mut budget, &mut seen).await {
                        c.merge(&c3);
                        pages += p3;
                    }
                }
            }
            Ok((c, pages))
        }
        .await;

        match outcome {
            Ok((counts, pages)) => {
                state
                    .pr_facts
                    .mark_synced(&name, &cursor_value, &backfill_from, true)
                    .await?;
                totals.merge(&counts);
                results.push(SyncRepoResult {
                    repo: name,
                    mode: mode.to_string(),
                    pages,
                    counts,
                    status: "ok".into(),
                    error: None,
                    updated_cursor: Some(cursor_value.clone()),
                });
            }
            Err(e) => {
                // One repo failing never aborts the others, and its cursor is
                // left untouched so the next run re-covers the same window.
                let msg = e.to_string();
                let _ = state.pr_facts.mark_sync_error(&name, &msg).await;
                errors.push(format!("{name}: {msg}"));
                results.push(SyncRepoResult {
                    repo: name,
                    mode: mode.to_string(),
                    pages: 0,
                    counts: IngestCounts::default(),
                    status: "error".into(),
                    error: Some(msg),
                    updated_cursor: cursor,
                });
            }
        }
    }

    Ok(Json(SyncResult {
        started_at: started.to_rfc3339(),
        finished_at: Utc::now().to_rfc3339(),
        repos: results,
        errors,
        rate_limit: RateLimitInfo {
            cost_total: budget.cost,
            remaining: budget.remaining,
            reset_at: budget.reset_at,
        },
        counts: totals,
    }))
}
