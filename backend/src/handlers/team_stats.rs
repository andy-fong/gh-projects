//! Read endpoints for the Team Stats page.

use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{Duration, Utc};
use std::collections::BTreeMap;

use crate::{
    error::AppError,
    models::team_stats::{MemberTrend, StatsQuery, StatsSummary, WorklistQuery},
    repositories::RepoScope,
    state::AppState,
};

/// Reporting window: the last `weeks` **complete** Monday-start weeks.
///
/// The current, still-running week is deliberately excluded — a partial week
/// drags every rate and average down for no reason other than the clock, and on
/// a Monday morning it would be empty. So "4w" means four finished weeks, and
/// the window always ends on the most recent Monday.
///
/// Returns `(since, until, through)`: `since` and `until` are the half-open
/// `[since, until)` bounds the queries use, and `through` is the last day
/// actually included, for display.
fn window_from(today: chrono::NaiveDate, weeks: i64) -> (String, String, String) {
    // %u is 1=Mon..7=Sun, so this lands on the current week's Monday.
    let weekday = today.format("%u").to_string().parse::<i64>().unwrap_or(1);
    let this_monday = today - Duration::days(weekday - 1);
    let until = this_monday; // exclusive: drops the in-progress week
    let since = until - Duration::weeks(weeks);
    let through = until - Duration::days(1);
    (
        since.format("%Y-%m-%d").to_string(),
        until.format("%Y-%m-%d").to_string(),
        through.format("%Y-%m-%d").to_string(),
    )
}

fn window(weeks: i64) -> (String, String, String) {
    window_from(Utc::now().date_naive(), weeks)
}

pub async fn get_summary(
    State(state): State<AppState>,
    Query(q): Query<StatsQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let weeks = q.weeks_or_default();
    let (since, until, through) = window(weeks);
    let scope = RepoScope::new(q.repo_list());
    let groups = q.group_list();

    let totals = state.team_stats.totals(&scope, &since, &until).await?;
    let people = state.team_stats.people(&scope, &groups, &since, &until).await?;
    let repo_health = state.team_stats.repo_health(&scope, &since, &until).await?;
    let trends = state.team_stats.trends(&scope, &since, &until).await?;

    // Pivot the (member, week) cells into one series per member, aligned to the
    // `trends` spine *by week* rather than by row order. The query already
    // returns a dense, ordered result, but indexing positionally would mean a
    // single missing or reordered row silently shifted somebody's whole history
    // sideways — a wrong chart with no error anywhere.
    let cells = state.team_stats.member_trends(&scope, &since, &until).await?;
    let mut by_login: BTreeMap<String, BTreeMap<String, (i64, i64)>> = BTreeMap::new();
    for c in cells {
        by_login
            .entry(c.login)
            .or_default()
            .insert(c.week_start, (c.prs_opened, c.prs_reviewed));
    }
    let member_trends: Vec<MemberTrend> = by_login
        .into_iter()
        .map(|(login, weeks)| {
            let (opened, reviewed) = trends
                .iter()
                .map(|w| weeks.get(&w.week_start).copied().unwrap_or((0, 0)))
                .unzip();
            MemberTrend { login, opened, reviewed }
        })
        .collect();
    // Inlined rather than a second endpoint so the header can render freshness
    // on first paint without the two readings skewing.
    let last_synced_at = state.pr_facts.latest_sync_at().await?;

    Ok(Json(StatsSummary {
        since,
        until,
        through,
        weeks,
        repos: scope.as_slice().to_vec(),
        last_synced_at,
        totals,
        people,
        repo_health,
        trends,
        member_trends,
    }))
}

pub async fn get_worklist(
    State(state): State<AppState>,
    Query(q): Query<WorklistQuery>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    let rows = state
        .team_stats
        .worklist(
            &RepoScope::new(q.repo_list()),
            q.filter.as_deref().unwrap_or("unreviewed"),
            q.sort_or_default(),
            q.limit_or_default(),
            q.offset.unwrap_or(0).max(0),
        )
        .await?;
    Ok(Json(rows))
}

pub async fn list_stats_repos(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, AppError> {
    Ok(Json(state.pr_facts.repo_status().await?))
}

#[cfg(test)]
mod tests {
    use super::window_from;
    use chrono::NaiveDate;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    /// The in-progress week is excluded, whatever day it is when you look.
    /// 2026-09-14 is a Monday, so every day of that week must report the same
    /// window, ending the day before it.
    #[test]
    fn current_partial_week_is_excluded() {
        for today in [
            "2026-09-14", // Monday
            "2026-09-17", // Thursday
            "2026-09-20", // Sunday
        ] {
            let (since, until, through) = window_from(d(today), 1);
            assert_eq!(until, "2026-09-14", "{today}: window must end at this Monday");
            assert_eq!(since, "2026-09-07", "{today}: 1w is the previous full week");
            assert_eq!(through, "2026-09-13", "{today}: last included day is the Sunday");
        }
    }

    /// N weeks means exactly N full weeks, so the span is always a multiple of 7.
    #[test]
    fn window_spans_whole_weeks() {
        for weeks in [1i64, 4, 8, 12, 26] {
            let (since, until, _) = window_from(d("2026-09-17"), weeks);
            let days = (d(&until) - d(&since)).num_days();
            assert_eq!(days, weeks * 7, "{weeks}w must span {} days", weeks * 7);
        }
    }

    /// Windows are nested: a longer one starts earlier and ends at the same place.
    #[test]
    fn longer_windows_share_an_end() {
        let (s1, u1, t1) = window_from(d("2026-09-17"), 1);
        let (s8, u8, t8) = window_from(d("2026-09-17"), 8);
        assert_eq!((u1.as_str(), t1.as_str()), (u8.as_str(), t8.as_str()));
        assert!(s8 < s1, "8w must reach further back than 1w");
    }

    /// Year boundaries are plain date arithmetic, not week-number math.
    #[test]
    fn crosses_the_year_boundary() {
        // 2026-01-01 is a Thursday; its week began Monday 2025-12-29.
        let (since, until, through) = window_from(d("2026-01-01"), 2);
        assert_eq!(until, "2025-12-29");
        assert_eq!(since, "2025-12-15");
        assert_eq!(through, "2025-12-28");
    }
}
