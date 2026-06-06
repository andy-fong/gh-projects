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

/// Roll up a group's overall status from its items' stages:
/// - any `blocked` → `blocked` (an alert wins)
/// - every item in the completion cluster (merged/released/done) → the furthest
///   stage present (done > released > merged); so all-merged → merged,
///   merged+released → released, and any done present → done
/// - every item still `todo` → `todo`
/// - otherwise (work has started but isn't all complete) → `in_progress`,
///   e.g. `[todo, done, done]` → in progress
///
/// Returns `None` for an empty group.
pub fn group_rollup_stage(stages: &[String]) -> Option<String> {
    if stages.is_empty() {
        return None;
    }
    if stages.iter().any(|s| s == "blocked") {
        return Some("blocked".to_string());
    }
    let completion = ["merged", "released", "done"];
    if stages.iter().all(|s| completion.contains(&s.as_str())) {
        for s in ["done", "released", "merged"] {
            if stages.iter().any(|x| x == s) {
                return Some(s.to_string());
            }
        }
    }
    if stages.iter().all(|s| s == "todo") {
        return Some("todo".to_string());
    }
    Some("in_progress".to_string())
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
