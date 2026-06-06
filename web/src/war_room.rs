//! Shared helpers for the War Room feature: manual lifecycle stages and
//! GitHub link building. (Live CI/merge status lands in phase 2.)

/// Manual lifecycle stages: `(key, label)`. Intentionally a small fixed set;
/// the column is unconstrained in the DB so this can grow later.
pub const STAGES: &[(&str, &str)] = &[
    ("todo", "To do"),
    ("in_progress", "In progress"),
    ("blocked", "Blocked"),
    ("merged", "Merged"),
    ("released", "Released"),
    ("done", "Done"),
];

pub fn stage_label(stage: &str) -> String {
    STAGES
        .iter()
        .find(|(k, _)| *k == stage)
        .map(|(_, l)| l.to_string())
        .unwrap_or_else(|| stage.to_string())
}

/// CSS modifier class for a stage badge (see `.wr-badge-*` in main.css).
pub fn stage_badge_class(stage: &str) -> &'static str {
    match stage {
        "in_progress" => "wr-badge-progress",
        "blocked" => "wr-badge-blocked",
        "merged" => "wr-badge-merged",
        "released" | "done" => "wr-badge-done",
        _ => "wr-badge-todo",
    }
}

/// Build a GitHub URL for an item given its group's `owner/repo`.
pub fn github_url(repo: &str, ref_type: Option<&str>, number: i64) -> String {
    let segment = match ref_type {
        Some("issue") => "issues",
        _ => "pull",
    };
    format!("https://github.com/{repo}/{segment}/{number}")
}
