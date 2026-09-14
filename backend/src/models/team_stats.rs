//! Team Stats: locally-cached GitHub PR/review facts and the aggregates the
//! dashboard reads.
//!
//! The fact tables *are* the time series — weekly buckets are a `GROUP BY`, not
//! a stored rollup. See `migrations/013_team_stats.sql`.

use serde::{Deserialize, Serialize};

// ── Ingest ───────────────────────────────────────────────────────────────────

/// One PR as fetched from GitHub, ready to upsert.
#[derive(Debug, Clone)]
pub struct PrFactRow {
    pub repo: String, // lowercased "owner/name"
    pub number: i64,
    pub node_id: Option<String>,
    pub title: String,
    pub url: String,
    pub state: String,
    pub is_draft: bool,
    pub author_login: Option<String>,
    pub merged_by_login: Option<String>,
    pub gh_created_at: String,
    pub gh_updated_at: String,
    pub merged_at: Option<String>,
    pub closed_at: Option<String>,
    pub additions: i64,
    pub deletions: i64,
    pub changed_files: i64,
    pub comment_count: i64,
    pub review_total: i64,
    pub reviews_truncated: bool,
    pub requests_truncated: bool,
}

#[derive(Debug, Clone)]
pub struct ReviewFactRow {
    pub node_id: Option<String>,
    pub reviewer_login: String,
    pub author_login: String,
    pub state: String,
    pub submitted_at: String,
}

#[derive(Debug, Clone)]
pub struct ReviewRequestRow {
    pub requested_login: String,
    pub requested_type: String, // 'user' | 'team'
}

/// A PR plus everything hanging off it, ingested as one unit.
#[derive(Debug, Clone)]
pub struct PrIngest {
    pub pr: PrFactRow,
    pub reviews: Vec<ReviewFactRow>,
    pub requests: Vec<ReviewRequestRow>,
    /// False when GitHub had more reviews than we fetched. Drives upsert-only
    /// instead of delete-and-reinsert, so a truncated fetch can't delete rows
    /// it simply didn't see.
    pub reviews_complete: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct IngestCounts {
    pub prs: i64,
    pub reviews: i64,
    pub requests: i64,
    /// Review nodes dropped for a null author or null submittedAt (deleted
    /// accounts, PENDING reviews). Counted rather than silently discarded.
    pub skipped: i64,
}

impl IngestCounts {
    pub fn merge(&mut self, other: &IngestCounts) {
        self.prs += other.prs;
        self.reviews += other.reviews;
        self.requests += other.requests;
        self.skipped += other.skipped;
    }
}

// ── Sync state ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SyncState {
    pub id: i64,
    pub repo: String,
    pub updated_cursor: Option<String>,
    pub backfill_from: Option<String>,
    pub backfilled: bool,
    pub last_sync_at: Option<String>,
    pub last_sync_status: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Sync request / response ──────────────────────────────────────────────────

fn default_backfill_from() -> String {
    "2026-08-01".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncRequest {
    /// Restrict to these "owner/repo" values. Empty/absent = every tracked repo.
    #[serde(default)]
    pub repos: Vec<String>,
    /// Floor for the first backfill of a repo. Only used when that repo has
    /// never been backfilled; later runs are incremental from the cursor.
    #[serde(default = "default_backfill_from")]
    pub backfill_from: String,
}

impl Default for SyncRequest {
    fn default() -> Self {
        Self { repos: Vec::new(), backfill_from: default_backfill_from() }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncRepoResult {
    pub repo: String,
    pub mode: String, // "backfill" | "incremental"
    pub pages: i64,
    pub counts: IngestCounts,
    pub status: String, // "ok" | "error"
    pub error: Option<String>,
    pub updated_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateLimitInfo {
    pub cost_total: i64,
    pub remaining: i64,
    pub reset_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub started_at: String,
    pub finished_at: String,
    pub repos: Vec<SyncRepoResult>,
    /// Per-repo failures, formatted "owner/repo: message". One repo failing
    /// never aborts the others.
    pub errors: Vec<String>,
    pub rate_limit: RateLimitInfo,
    pub counts: IngestCounts,
}

// ── Query params ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct StatsQuery {
    /// Size of the reporting window, in weeks back from today.
    pub weeks: Option<i64>,
    /// Comma-separated "owner/repo" filter. Absent = all tracked repos.
    pub repos: Option<String>,
    /// Comma-separated roster groups (team, pe, solo, maintainer, community).
    pub groups: Option<String>,
}

impl StatsQuery {
    pub fn weeks_or_default(&self) -> i64 {
        self.weeks.unwrap_or(8).clamp(1, 104)
    }

    pub fn repo_list(&self) -> Vec<String> {
        split_csv(self.repos.as_deref())
    }

    pub fn group_list(&self) -> Vec<String> {
        split_csv(self.groups.as_deref())
    }
}

fn split_csv(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorklistQuery {
    pub repos: Option<String>,
    /// "unreviewed" (default) | "bot_only" | "all"
    pub filter: Option<String>,
    /// "oldest" (default) | "newest"
    pub sort: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl WorklistQuery {
    pub fn repo_list(&self) -> Vec<String> {
        split_csv(self.repos.as_deref())
    }
    pub fn limit_or_default(&self) -> i64 {
        self.limit.unwrap_or(100).clamp(1, 500)
    }
    pub fn sort_or_default(&self) -> &str {
        match self.sort.as_deref() {
            Some("newest") => "newest",
            _ => "oldest",
        }
    }
}

// ── Aggregates ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct PersonStat {
    pub login: String,
    pub member_group: String,
    /// PRs that are actually asking for review — still-draft PRs are excluded,
    /// because they request nothing and would otherwise inflate the denominator
    /// of the reciprocity ratio. Measured here, only 2.8% of open drafts ever
    /// received a human review, against 24.8% of ready ones.
    pub prs_opened: i64,
    /// Still in draft. Shown for context, never counted in `prs_opened`.
    pub prs_draft: i64,
    pub prs_merged: i64,
    pub prs_merged_without_review: i64,
    /// Distinct PRs (not their own) they left a human review on. Leads the UI —
    /// event counts are gameable and duplicate approvals occur in real data.
    pub prs_reviewed: i64,
    pub review_events: i64,
    pub approvals: i64,
    pub changes_requested: i64,
    /// Distinct human reviews received across their own PRs — the "get" side.
    pub reviews_received: i64,
    /// Open PRs where they are a requested reviewer and haven't reviewed yet.
    pub open_review_requests: i64,
    pub last_review_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RepoHealth {
    pub repo: String,
    pub open_total: i64,
    pub open_draft: i64,
    pub open_unreviewed: i64,
    pub open_unreviewed_7d: i64,
    pub open_bot_reviewed_only: i64,
    pub oldest_open_unreviewed_days: Option<i64>,
    pub merged_in_window: i64,
    pub merged_without_human_review: i64,
    /// Merged with nobody having approved it.
    pub merged_without_approval: i64,
    pub opened_in_window: i64,
    pub median_ttfr_hours: Option<f64>,
    pub p90_ttfr_hours: Option<f64>,
    /// The uncensored companion to the median: a TTFR computed over only the
    /// PRs that got a review looks healthy precisely when the problem is worst.
    pub pct_never_reviewed: Option<f64>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WeekPoint {
    pub week_start: String,
    pub prs_opened: i64,
    pub prs_merged: i64,
    pub prs_merged_without_review: i64,
    pub prs_reviewed: i64,
    pub review_events: i64,
    pub bot_review_events: i64,
    pub active_reviewers: i64,
}

/// One (member, week) cell — the raw shape the query returns, pivoted into
/// [`MemberTrend`] before it reaches the client.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MemberWeekPoint {
    pub login: String,
    pub week_start: String,
    pub prs_opened: i64,
    pub prs_reviewed: i64,
}

/// Per-member weekly series, for the scoreboard sparklines.
#[derive(Debug, Clone, Serialize)]
pub struct MemberTrend {
    pub login: String,
    pub opened: Vec<i64>,
    pub reviewed: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WorklistRow {
    pub repo: String,
    pub number: i64,
    pub title: String,
    pub url: String,
    pub author_login: Option<String>,
    pub author_group: String,
    pub gh_created_at: String,
    pub gh_updated_at: String,
    pub age_days: i64,
    pub idle_days: i64,
    pub size: i64,
    pub changed_files: i64,
    pub human_review_events: i64,
    pub bot_review_events: i64,
    pub self_review_events: i64,
    /// Comma-joined pending reviewers; a `team:` prefix marks a team request.
    pub requested_reviewers: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct StatsTotals {
    pub prs_opened: i64,
    pub prs_merged: i64,
    pub prs_merged_without_review: i64,
    /// See `RepoHealth::merged_without_approval`.
    pub merged_without_approval: i64,
    pub open_unreviewed: i64,
    pub open_unreviewed_7d: i64,
    pub review_events: i64,
    pub bot_review_events: i64,
    pub self_review_events: i64,
    pub active_reviewers: i64,
    pub median_ttfr_hours: Option<f64>,
    pub p90_ttfr_hours: Option<f64>,
    /// Share of human reviews contributed by the two busiest reviewers. The
    /// number that exposes concentration — aggregate counts look healthy while
    /// two people carry everything.
    pub top2_review_share: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsSummary {
    pub since: String,
    /// Exclusive upper bound used by the queries.
    pub until: String,
    /// Last day actually included — `until` minus a day. For display, so the
    /// header never shows an end date the data does not reach.
    pub through: String,
    pub weeks: i64,
    pub repos: Vec<String>,
    /// Inlined so the header can render freshness on first paint without a
    /// second round trip (which would also let the two readings skew).
    pub last_synced_at: Option<String>,
    pub totals: StatsTotals,
    pub people: Vec<PersonStat>,
    pub repo_health: Vec<RepoHealth>,
    pub trends: Vec<WeekPoint>,
    /// Aligned index-for-index with `trends`, so the client can plot a member's
    /// series against the same week spine.
    pub member_trends: Vec<MemberTrend>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackedRepoStatus {
    pub repo_id: i64,
    pub name: String,
    pub owner_repo: String,
    pub track_stats: bool,
    pub pr_count: i64,
    pub review_count: i64,
    pub last_sync_at: Option<String>,
    pub last_sync_status: Option<String>,
    pub last_error: Option<String>,
    pub backfilled: bool,
    pub updated_cursor: Option<String>,
}
