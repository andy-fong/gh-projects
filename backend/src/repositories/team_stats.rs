//! Read side of Team Stats: the metric SQL.
//!
//! Every query filters through the `human_reviews` / `pr_stats` views defined in
//! `migrations/013_team_stats.sql`, which join `team_members` live. That is
//! deliberate: marking a login as a bot in the Team dialog retroactively
//! corrects all history with no re-sync, which is also why there is no rollup
//! table and no stored `first_review_at` column.

use async_trait::async_trait;
use sqlx::{query::QueryAs, sqlite::SqliteArguments, QueryBuilder, Sqlite, SqlitePool};

use crate::{
    error::AppError,
    models::team_stats::{
        MemberWeekPoint, PersonStat, RepoHealth, StatsTotals, WeekPoint, WorklistRow,
    },
};

/// The set of repos a query covers: those opted in to stats, optionally
/// narrowed to an explicit caller-supplied list.
#[derive(Debug, Clone)]
pub struct RepoScope(Vec<String>);

impl RepoScope {
    pub fn new(requested: Vec<String>) -> Self {
        Self(requested)
    }
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

/// `repo IN (…)` restricted to tracked repos, with the caller's filter applied
/// when they gave one. Written as a fragment so every query scopes identically.
fn scope_sql(scope: &RepoScope) -> String {
    if scope.0.is_empty() {
        "SELECT lower(owner_repo) FROM repos WHERE track_stats = 1".to_string()
    } else {
        let ph = vec!["?"; scope.0.len()].join(",");
        format!(
            "SELECT lower(owner_repo) FROM repos \
             WHERE track_stats = 1 AND lower(owner_repo) IN ({ph})"
        )
    }
}

/// Build a query and bind the repo scope in one step.
///
/// The scope is a CTE that appears exactly ONCE in every query here, however
/// many times the body references `tracked`. Binding it at construction makes
/// double-binding structurally impossible — an earlier version bound it again
/// per reference site, which silently shifted every following parameter and
/// made `submitted_at >= '<a repo name>'` match nothing.
///
/// So: bind the scope here, and bind only the query's own parameters after.
fn scoped_query<'q, T>(
    sql: &'q str,
    scope: &'q RepoScope,
) -> QueryAs<'q, Sqlite, T, SqliteArguments<'q>>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
{
    let mut q = sqlx::query_as::<_, T>(sql);
    for r in &scope.0 {
        q = q.bind(r);
    }
    q
}

#[async_trait]
pub trait TeamStatsRepository: Send + Sync {
    async fn people(
        &self,
        scope: &RepoScope,
        groups: &[String],
        since: &str,
        until: &str,
    ) -> Result<Vec<PersonStat>, AppError>;

    async fn repo_health(
        &self,
        scope: &RepoScope,
        since: &str,
        until: &str,
    ) -> Result<Vec<RepoHealth>, AppError>;

    async fn trends(
        &self,
        scope: &RepoScope,
        since: &str,
        until: &str,
    ) -> Result<Vec<WeekPoint>, AppError>;

    /// Per-member weekly series on the same week spine as [`Self::trends`].
    async fn member_trends(
        &self,
        scope: &RepoScope,
        since: &str,
        until: &str,
    ) -> Result<Vec<MemberWeekPoint>, AppError>;

    async fn totals(
        &self,
        scope: &RepoScope,
        since: &str,
        until: &str,
    ) -> Result<StatsTotals, AppError>;

    /// Open PRs needing a reviewer.
    ///
    /// `sort` is "oldest" or "newest". It is applied in SQL rather than by
    /// reversing the response: with paging, reversing a page would return the
    /// oldest N in reverse order instead of the newest N.
    async fn worklist(
        &self,
        scope: &RepoScope,
        filter: &str,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorklistRow>, AppError>;
}

pub struct SqliteTeamStatsRepository {
    pool: SqlitePool,
}

impl SqliteTeamStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TeamStatsRepository for SqliteTeamStatsRepository {
    async fn people(
        &self,
        scope: &RepoScope,
        groups: &[String],
        since: &str,
        until: &str,
    ) -> Result<Vec<PersonStat>, AppError> {
        let tracked = scope_sql(scope);
        let group_filter = if groups.is_empty() {
            String::new()
        } else {
            let ph = vec!["?"; groups.len()].join(",");
            format!("AND lower(COALESCE(tm.member_group,'community')) IN ({ph})")
        };

        let sql = format!(
            "WITH tracked AS ({tracked}),
             authored AS (
               -- Opened / Merged / Received all describe the PRs that are asking
               -- for review, so a still-draft PR is excluded from every one of
               -- them and surfaced separately as prs_draft. (A merged PR is
               -- never a draft, so prs_merged is unaffected either way.)
               SELECT lower(author_login) AS login,
                      SUM(is_draft = 0) AS prs_opened,
                      SUM(is_draft = 1) AS prs_draft,
                      SUM(state = 'MERGED') AS prs_merged,
                      SUM(state = 'MERGED'
                          AND (first_human_review_at IS NULL
                               OR first_human_review_at > merged_at)) AS prs_merged_without_review,
                      SUM(CASE WHEN is_draft = 0 THEN human_review_events ELSE 0 END)
                        AS reviews_received
               FROM pr_stats
               WHERE repo IN (SELECT * FROM tracked) AND author_login IS NOT NULL
                 AND gh_created_at >= ? AND gh_created_at < ?
               GROUP BY 1
             ),
             reviewed AS (
               SELECT lower(reviewer_login) AS login,
                      COUNT(*) AS review_events,
                      COUNT(DISTINCT repo || '#' || number) AS prs_reviewed,
                      SUM(state = 'APPROVED') AS approvals,
                      SUM(state = 'CHANGES_REQUESTED') AS changes_requested,
                      MAX(submitted_at) AS last_review_at
               FROM human_reviews
               WHERE repo IN (SELECT * FROM tracked)
                 AND submitted_at >= ? AND submitted_at < ?
               GROUP BY 1
             ),
             pending AS (
               SELECT lower(rr.requested_login) AS login, COUNT(*) AS open_review_requests
               FROM pr_review_requests rr
               JOIN pr_facts p ON p.repo = rr.repo AND p.number = rr.number
               WHERE rr.requested_type = 'user' AND p.state = 'OPEN' AND p.is_draft = 0
                 AND p.repo IN (SELECT * FROM tracked)
                 AND NOT EXISTS (
                   SELECT 1 FROM human_reviews h
                   WHERE h.repo = rr.repo AND h.number = rr.number
                     AND h.reviewer_login = rr.requested_login COLLATE NOCASE)
               GROUP BY 1
             ),
             people AS (
               SELECT login FROM authored
               UNION SELECT login FROM reviewed
               UNION SELECT login FROM pending
               -- quiet roster members still get a row, so a zero is visible
               UNION SELECT lower(login) FROM team_members
                     WHERE member_group NOT IN ('bot')
             )
             SELECT pe.login AS login,
                    COALESCE(tm.member_group, 'community') AS member_group,
                    COALESCE(a.prs_opened, 0) AS prs_opened,
                    COALESCE(a.prs_draft, 0) AS prs_draft,
                    COALESCE(a.prs_merged, 0) AS prs_merged,
                    COALESCE(a.prs_merged_without_review, 0) AS prs_merged_without_review,
                    COALESCE(r.prs_reviewed, 0) AS prs_reviewed,
                    COALESCE(r.review_events, 0) AS review_events,
                    COALESCE(r.approvals, 0) AS approvals,
                    COALESCE(r.changes_requested, 0) AS changes_requested,
                    COALESCE(a.reviews_received, 0) AS reviews_received,
                    COALESCE(p.open_review_requests, 0) AS open_review_requests,
                    r.last_review_at AS last_review_at
             FROM people pe
             LEFT JOIN team_members tm ON tm.login = pe.login COLLATE NOCASE
             LEFT JOIN authored a ON a.login = pe.login
             LEFT JOIN reviewed r ON r.login = pe.login
             LEFT JOIN pending  p ON p.login = pe.login
             WHERE COALESCE(tm.member_group, '') <> 'bot'
               AND pe.login NOT LIKE '%[bot]'
               {group_filter}
               AND (COALESCE(a.prs_opened,0) + COALESCE(r.review_events,0)
                    + COALESCE(p.open_review_requests,0) > 0
                    OR tm.member_group IN ('team','pe'))
             ORDER BY prs_opened DESC, prs_reviewed DESC"
        );

        let mut q = scoped_query::<PersonStat>(&sql, scope);
        q = q.bind(since).bind(until); // authored
        q = q.bind(since).bind(until); // reviewed
        for g in groups {
            q = q.bind(g);
        }
        Ok(q.fetch_all(&self.pool).await?)
    }

    async fn repo_health(
        &self,
        scope: &RepoScope,
        since: &str,
        until: &str,
    ) -> Result<Vec<RepoHealth>, AppError> {
        let tracked = scope_sql(scope);
        let sql = format!(
            "WITH tracked AS ({tracked}),
             scoped AS (SELECT * FROM pr_stats WHERE repo IN (SELECT * FROM tracked)),
             ttfr AS (
               SELECT repo, (julianday(first_human_review_at) - julianday(gh_created_at)) * 24.0 AS h
               FROM scoped
               WHERE is_draft = 0 AND first_human_review_at IS NOT NULL
                 AND gh_created_at >= ? AND gh_created_at < ?
             ),
             ranked AS (
               SELECT repo, h,
                      ROW_NUMBER() OVER (PARTITION BY repo ORDER BY h) AS rn,
                      COUNT(*)    OVER (PARTITION BY repo)             AS n
               FROM ttfr
             ),
             med AS (SELECT repo, AVG(h) AS median_h FROM ranked
                     WHERE rn IN ((n + 1) / 2, (n + 2) / 2) GROUP BY repo),
             p90 AS (SELECT repo, MIN(h) AS p90_h FROM ranked
                     WHERE rn >= (n * 9 + 9) / 10 GROUP BY repo)
             SELECT s.repo AS repo,
               SUM(s.state = 'OPEN') AS open_total,
               SUM(s.state = 'OPEN' AND s.is_draft = 1) AS open_draft,
               SUM(s.state = 'OPEN' AND s.is_draft = 0 AND s.human_review_events = 0) AS open_unreviewed,
               SUM(s.state = 'OPEN' AND s.is_draft = 0 AND s.human_review_events = 0
                   AND julianday('now') - julianday(s.gh_created_at) > 7) AS open_unreviewed_7d,
               SUM(s.state = 'OPEN' AND s.is_draft = 0 AND s.human_review_events = 0
                   AND s.bot_review_events > 0) AS open_bot_reviewed_only,
               MAX(CASE WHEN s.state = 'OPEN' AND s.is_draft = 0 AND s.human_review_events = 0
                        THEN CAST(julianday('now') - julianday(s.gh_created_at) AS INTEGER) END)
                 AS oldest_open_unreviewed_days,
               SUM(s.merged_at >= ? AND s.merged_at < ?) AS merged_in_window,
               SUM(s.merged_at >= ? AND s.merged_at < ?
                   AND (s.first_human_review_at IS NULL
                        OR s.first_human_review_at > s.merged_at)) AS merged_without_human_review,
               SUM(s.merged_at >= ? AND s.merged_at < ?
                   AND s.human_approvals = 0) AS merged_without_approval,
               SUM(s.gh_created_at >= ? AND s.gh_created_at < ?) AS opened_in_window,
               m.median_h AS median_ttfr_hours,
               p.p90_h    AS p90_ttfr_hours,
               ROUND(100.0 * SUM(s.is_draft = 0 AND s.gh_created_at >= ? AND s.gh_created_at < ?
                                 AND s.first_human_review_at IS NULL)
                          / NULLIF(SUM(s.is_draft = 0 AND s.gh_created_at >= ?
                                       AND s.gh_created_at < ?), 0), 1) AS pct_never_reviewed
             FROM scoped s
             LEFT JOIN med m ON m.repo = s.repo
             LEFT JOIN p90 p ON p.repo = s.repo
             GROUP BY s.repo
             ORDER BY open_unreviewed DESC, s.repo"
        );

        let mut q = scoped_query::<RepoHealth>(&sql, scope);
        q = q.bind(since).bind(until); // ttfr
        // merged_in_window, merged_without_human_review, merged_without_approval,
        // opened_in_window, and the two halves of pct_never_reviewed.
        for _ in 0..6 {
            q = q.bind(since).bind(until);
        }
        Ok(q.fetch_all(&self.pool).await?)
    }

    async fn trends(
        &self,
        scope: &RepoScope,
        since: &str,
        until: &str,
    ) -> Result<Vec<WeekPoint>, AppError> {
        let tracked = scope_sql(scope);
        // The recursive week spine is what makes a quiet week render as a zero
        // instead of vanishing from the series — without it the chart line lies.
        let sql = format!(
            "WITH RECURSIVE tracked AS ({tracked}),
             weeks(w) AS (
               SELECT date(?, 'weekday 0', '-6 days')
               UNION ALL
               SELECT date(w, '+7 days') FROM weeks WHERE date(w, '+7 days') < ?
             ),
             op AS (
               SELECT created_week AS w, COUNT(*) AS n
               FROM pr_facts WHERE repo IN (SELECT * FROM tracked)
                 AND created_week >= (SELECT MIN(w) FROM weeks) GROUP BY 1
             ),
             mg AS (
               SELECT merged_week AS w, COUNT(*) AS n,
                      SUM(first_human_review_at IS NULL
                          OR first_human_review_at > merged_at) AS unreviewed
               FROM pr_stats WHERE repo IN (SELECT * FROM tracked) AND merged_at IS NOT NULL
                 AND merged_week >= (SELECT MIN(w) FROM weeks) GROUP BY 1
             ),
             rv AS (
               SELECT submitted_week AS w, COUNT(*) AS events,
                      COUNT(DISTINCT repo || '#' || number) AS prs_reviewed,
                      COUNT(DISTINCT lower(reviewer_login)) AS active_reviewers
               FROM human_reviews WHERE repo IN (SELECT * FROM tracked)
                 AND submitted_week >= (SELECT MIN(w) FROM weeks) GROUP BY 1
             ),
             bt AS (
               SELECT r.submitted_week AS w, COUNT(*) AS n
               FROM pr_review_facts r
               WHERE r.repo IN (SELECT * FROM tracked) AND r.is_self_review = 0
                 AND r.id NOT IN (SELECT id FROM human_reviews)
                 AND r.submitted_week >= (SELECT MIN(w) FROM weeks) GROUP BY 1
             )
             SELECT weeks.w AS week_start,
                    COALESCE(op.n, 0) AS prs_opened,
                    COALESCE(mg.n, 0) AS prs_merged,
                    COALESCE(mg.unreviewed, 0) AS prs_merged_without_review,
                    COALESCE(rv.prs_reviewed, 0) AS prs_reviewed,
                    COALESCE(rv.events, 0) AS review_events,
                    COALESCE(bt.n, 0) AS bot_review_events,
                    COALESCE(rv.active_reviewers, 0) AS active_reviewers
             FROM weeks
             LEFT JOIN op ON op.w = weeks.w
             LEFT JOIN mg ON mg.w = weeks.w
             LEFT JOIN rv ON rv.w = weeks.w
             LEFT JOIN bt ON bt.w = weeks.w
             ORDER BY weeks.w"
        );

        let q = scoped_query::<WeekPoint>(&sql, scope).bind(since).bind(until);
        Ok(q.fetch_all(&self.pool).await?)
    }

    async fn member_trends(
        &self,
        scope: &RepoScope,
        since: &str,
        until: &str,
    ) -> Result<Vec<MemberWeekPoint>, AppError> {
        let tracked = scope_sql(scope);
        // CROSS JOIN against the same recursive spine `trends` uses, so every
        // member gets a dense series with real zeros — a sparkline built from
        // sparse rows would silently compress its own gaps.
        let sql = format!(
            "WITH RECURSIVE tracked AS ({tracked}),
             weeks(w) AS (
               SELECT date(?, 'weekday 0', '-6 days')
               UNION ALL
               SELECT date(w, '+7 days') FROM weeks WHERE date(w, '+7 days') < ?
             ),
             rv AS (
               SELECT lower(reviewer_login) AS login, submitted_week AS w,
                      COUNT(DISTINCT repo || '#' || number) AS n
               FROM human_reviews WHERE repo IN (SELECT * FROM tracked)
                 AND submitted_week >= (SELECT MIN(w) FROM weeks) GROUP BY 1, 2
             ),
             op AS (
               SELECT lower(author_login) AS login, created_week AS w, COUNT(*) AS n
               FROM pr_facts WHERE repo IN (SELECT * FROM tracked) AND author_login IS NOT NULL
                 AND created_week >= (SELECT MIN(w) FROM weeks) GROUP BY 1, 2
             ),
             people AS (SELECT login FROM rv UNION SELECT login FROM op)
             SELECT p.login AS login, weeks.w AS week_start,
                    COALESCE(op.n, 0) AS prs_opened,
                    COALESCE(rv.n, 0) AS prs_reviewed
             FROM people p
             CROSS JOIN weeks
             LEFT JOIN rv ON rv.login = p.login AND rv.w = weeks.w
             LEFT JOIN op ON op.login = p.login AND op.w = weeks.w
             ORDER BY p.login, weeks.w"
        );
        let q = scoped_query::<MemberWeekPoint>(&sql, scope).bind(since).bind(until);
        Ok(q.fetch_all(&self.pool).await?)
    }

    async fn totals(
        &self,
        scope: &RepoScope,
        since: &str,
        until: &str,
    ) -> Result<StatsTotals, AppError> {
        let tracked = scope_sql(scope);
        let sql = format!(
            "WITH tracked AS ({tracked}),
             scoped AS (SELECT * FROM pr_stats WHERE repo IN (SELECT * FROM tracked)),
             ttfr AS (
               SELECT (julianday(first_human_review_at) - julianday(gh_created_at)) * 24.0 AS h
               FROM scoped WHERE is_draft = 0 AND first_human_review_at IS NOT NULL
                 AND gh_created_at >= ? AND gh_created_at < ?
             ),
             ranked AS (
               SELECT h, ROW_NUMBER() OVER (ORDER BY h) AS rn, COUNT(*) OVER () AS n FROM ttfr
             ),
             rev AS (
               SELECT lower(reviewer_login) AS login, COUNT(*) AS n
               FROM human_reviews WHERE repo IN (SELECT * FROM tracked)
                 AND submitted_at >= ? AND submitted_at < ? GROUP BY 1
             ),
             top2 AS (SELECT SUM(n) AS s FROM (SELECT n FROM rev ORDER BY n DESC LIMIT 2))
             SELECT
               (SELECT COUNT(*) FROM scoped WHERE gh_created_at >= ? AND gh_created_at < ?) AS prs_opened,
               (SELECT COUNT(*) FROM scoped WHERE merged_at >= ? AND merged_at < ?) AS prs_merged,
               (SELECT COUNT(*) FROM scoped WHERE merged_at >= ? AND merged_at < ?
                  AND (first_human_review_at IS NULL
                       OR first_human_review_at > merged_at)) AS prs_merged_without_review,
               (SELECT COUNT(*) FROM scoped WHERE merged_at >= ? AND merged_at < ?
                  AND human_approvals = 0) AS merged_without_approval,
               (SELECT COUNT(*) FROM scoped WHERE state = 'OPEN' AND is_draft = 0
                  AND human_review_events = 0) AS open_unreviewed,
               (SELECT COUNT(*) FROM scoped WHERE state = 'OPEN' AND is_draft = 0
                  AND human_review_events = 0
                  AND julianday('now') - julianday(gh_created_at) > 7) AS open_unreviewed_7d,
               (SELECT COALESCE(SUM(n), 0) FROM rev) AS review_events,
               (SELECT COUNT(*) FROM pr_review_facts r
                  WHERE r.repo IN (SELECT * FROM tracked) AND r.is_self_review = 0
                    AND r.id NOT IN (SELECT id FROM human_reviews)
                    AND r.submitted_at >= ? AND r.submitted_at < ?) AS bot_review_events,
               (SELECT COUNT(*) FROM pr_review_facts r
                  WHERE r.repo IN (SELECT * FROM tracked) AND r.is_self_review = 1
                    AND r.submitted_at >= ? AND r.submitted_at < ?) AS self_review_events,
               (SELECT COUNT(*) FROM rev) AS active_reviewers,
               (SELECT AVG(h) FROM ranked WHERE rn IN ((n + 1) / 2, (n + 2) / 2)) AS median_ttfr_hours,
               (SELECT MIN(h) FROM ranked WHERE rn >= (n * 9 + 9) / 10) AS p90_ttfr_hours,
               (SELECT ROUND(100.0 * top2.s / NULLIF((SELECT SUM(n) FROM rev), 0), 1)
                  FROM top2) AS top2_review_share"
        );

        let mut q = scoped_query::<StatsTotals>(&sql, scope);
        q = q.bind(since).bind(until); // ttfr
        q = q.bind(since).bind(until); // rev
        // prs_opened, prs_merged, prs_merged_without_review, merged_without_approval
        for _ in 0..4 {
            q = q.bind(since).bind(until);
        }
        q = q.bind(since).bind(until); // bot_review_events
        q = q.bind(since).bind(until); // self_review_events
        Ok(q.fetch_one(&self.pool).await?)
    }

    async fn worklist(
        &self,
        scope: &RepoScope,
        filter: &str,
        sort: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WorklistRow>, AppError> {
        let tracked = scope_sql(scope);
        let predicate = match filter {
            // An AI reviewed it and no human did — the row that proves the thesis.
            "bot_only" => "AND p.human_review_events = 0 AND p.bot_review_events > 0",
            "all" => "",
            _ => "AND p.human_review_events = 0",
        };
        let mut b = QueryBuilder::<Sqlite>::new(format!(
            "WITH tracked AS ({tracked})
             SELECT p.repo AS repo, p.number AS number, p.title AS title, p.url AS url,
                    p.author_login AS author_login, p.author_group AS author_group,
                    p.gh_created_at AS gh_created_at, p.gh_updated_at AS gh_updated_at,
                    CAST(julianday('now') - julianday(p.gh_created_at) AS INTEGER) AS age_days,
                    CAST(julianday('now') - julianday(p.gh_updated_at) AS INTEGER) AS idle_days,
                    p.additions + p.deletions AS size,
                    p.changed_files AS changed_files,
                    p.human_review_events AS human_review_events,
                    p.bot_review_events AS bot_review_events,
                    p.self_review_events AS self_review_events,
                    (SELECT group_concat(
                         CASE WHEN rr.requested_type = 'team'
                              THEN 'team:' || rr.requested_login ELSE rr.requested_login END, ', ')
                       FROM pr_review_requests rr
                      WHERE rr.repo = p.repo AND rr.number = p.number) AS requested_reviewers
             FROM pr_stats p
             WHERE p.repo IN (SELECT * FROM tracked)
               AND p.state = 'OPEN' AND p.is_draft = 0 "
        ));
        b.push(predicate);
        // Fixed strings from a match, never caller input.
        b.push(match sort {
            "newest" => " ORDER BY p.gh_created_at DESC LIMIT ",
            _ => " ORDER BY p.gh_created_at ASC LIMIT ",
        });
        b.push_bind(limit);
        b.push(" OFFSET ");
        b.push_bind(offset);

        let q = scoped_query::<WorklistRow>(b.sql(), scope)
            .bind(limit)
            .bind(offset);
        Ok(q.fetch_all(&self.pool).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    /// Two repos, both tracked, with enough shape to exercise every branch:
    /// a merged PR reviewed by someone else, a self-review, a bot review, and
    /// a duplicate approval.
    async fn seed() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        for (id, owner) in [(1, "acme/alpha"), (2, "acme/beta")] {
            sqlx::query("INSERT INTO repos (id, name, owner_repo, track_stats) VALUES (?,?,?,1)")
                .bind(id)
                .bind(owner)
                .bind(owner)
                .execute(&pool)
                .await
                .unwrap();
        }
        sqlx::query("INSERT INTO team_members (login, member_group) VALUES ('alice','team'),('bob','team'),('robo','bot')")
            .execute(&pool)
            .await
            .unwrap();

        // (repo, number, author, merged_by)
        let prs = [
            ("acme/alpha", 1, "alice", Some("bob")),
            ("acme/alpha", 2, "bob", Some("bob")),
            ("acme/beta", 3, "alice", Some("bob")),
        ];
        for (repo, num, author, merged_by) in prs {
            sqlx::query(
                "INSERT INTO pr_facts (repo, number, title, url, state, author_login,
                     merged_by_login, gh_created_at, gh_updated_at, merged_at)
                 VALUES (?,?,'t','u','MERGED',?,?, '2026-08-03T00:00:00Z','2026-08-04T00:00:00Z','2026-08-04T00:00:00Z')",
            )
            .bind(repo).bind(num).bind(author).bind(merged_by)
            .execute(&pool).await.unwrap();
        }

        // Two open, unreviewed PRs so the worklist has something to order,
        // plus one still in draft, which must stay out of Opened.
        for (num, created, draft) in [
            (10, "2026-08-01T00:00:00Z", 0),
            (11, "2026-08-20T00:00:00Z", 0),
            (12, "2026-08-10T00:00:00Z", 1),
        ] {
            sqlx::query(
                "INSERT INTO pr_facts (repo, number, title, url, state, is_draft,
                     author_login, gh_created_at, gh_updated_at)
                 VALUES ('acme/alpha', ?, 't', 'u', 'OPEN', ?, 'alice', ?, ?)",
            )
            .bind(num).bind(draft).bind(created).bind(created)
            .execute(&pool).await.unwrap();
        }

        // (repo, number, reviewer, author, state, at)
        let reviews = [
            ("acme/alpha", 1, "bob", "alice", "APPROVED", "2026-08-03T01:00:00Z"),
            ("acme/alpha", 1, "bob", "alice", "APPROVED", "2026-08-03T02:00:00Z"), // duplicate
            ("acme/alpha", 1, "alice", "alice", "COMMENTED", "2026-08-03T03:00:00Z"), // self
            ("acme/alpha", 1, "robo", "alice", "COMMENTED", "2026-08-03T04:00:00Z"), // bot
            // PR 3 gets a comment but no approval — the governance case.
            ("acme/beta", 3, "bob", "alice", "COMMENTED", "2026-08-03T05:00:00Z"),
        ];
        for (repo, num, reviewer, author, state, at) in reviews {
            sqlx::query(
                "INSERT INTO pr_review_facts (repo, number, reviewer_login, author_login, state, submitted_at)
                 VALUES (?,?,?,?,?,?)",
            )
            .bind(repo).bind(num).bind(reviewer).bind(author).bind(state).bind(at)
            .execute(&pool).await.unwrap();
        }
        pool
    }

    const SINCE: &str = "2026-07-27";
    const UNTIL: &str = "2026-09-14";

    fn find<'a>(rows: &'a [PersonStat], login: &str) -> &'a PersonStat {
        rows.iter().find(|p| p.login == login).expect("login present")
    }

    /// A scope with repos in it must narrow the results, not empty them.
    ///
    /// Regression: the scope is a CTE appearing once per query, but the binding
    /// code used to bind it again per reference site. That shifted every later
    /// parameter, so `people` and `totals` silently returned nothing whenever a
    /// repo filter was applied.
    #[tokio::test]
    async fn repo_scope_narrows_rather_than_empties() {
        let repo = SqliteTeamStatsRepository::new(seed().await);
        let all = RepoScope::new(vec![]);
        let alpha = RepoScope::new(vec!["acme/alpha".into()]);

        let p_all = repo.people(&all, &[], SINCE, UNTIL).await.unwrap();
        let p_alpha = repo.people(&alpha, &[], SINCE, UNTIL).await.unwrap();
        assert!(!p_all.is_empty(), "unscoped people must return rows");
        assert!(!p_alpha.is_empty(), "scoped people must return rows");

        // alice authored alpha#1, beta#3, alpha#10, alpha#11 and draft #12.
        // Drafts are excluded, so 4 in all and 3 once scoped to alpha.
        assert_eq!(find(&p_all, "alice").prs_opened, 4);
        assert_eq!(find(&p_alpha, "alice").prs_opened, 3);

        let t_all = repo.totals(&all, SINCE, UNTIL).await.unwrap();
        let t_alpha = repo.totals(&alpha, SINCE, UNTIL).await.unwrap();
        // totals.prs_opened is repo-wide and still counts drafts — only the
        // per-person Opened column excludes them.
        assert_eq!(t_all.prs_opened, 6);
        assert_eq!(t_alpha.prs_opened, 5);
        assert!(t_alpha.review_events > 0, "scoped totals lost its review window");

        // Every query must survive a scope, and stay internally consistent.
        let tr = repo.trends(&alpha, SINCE, UNTIL).await.unwrap();
        assert_eq!(
            tr.iter().map(|w| w.prs_opened).sum::<i64>(),
            t_alpha.prs_opened,
            "trend total must agree with the headline total"
        );
        assert_eq!(repo.repo_health(&alpha, SINCE, UNTIL).await.unwrap().len(), 1);
        assert!(!repo.member_trends(&alpha, SINCE, UNTIL).await.unwrap().is_empty());
        assert!(!repo.worklist(&alpha, "unreviewed", "oldest", 10, 0).await.unwrap().is_empty());
    }

    /// A still-draft PR asks nobody for review, so it must not land in Opened
    /// (the denominator of the reciprocity ratio) — but it is still reported.
    #[tokio::test]
    async fn drafts_are_excluded_from_opened_but_reported() {
        let repo = SqliteTeamStatsRepository::new(seed().await);
        let all = RepoScope::new(vec![]);
        let people = repo.people(&all, &[], SINCE, UNTIL).await.unwrap();
        let alice = find(&people, "alice");

        // alice authored 5: alpha#1, beta#3, alpha#10, alpha#11 and draft #12.
        assert_eq!(alice.prs_opened, 4, "the draft must not count as opened");
        assert_eq!(alice.prs_draft, 1);

        // The worklist excludes drafts too, so #12 never appears there.
        let work = repo.worklist(&all, "unreviewed", "oldest", 50, 0).await.unwrap();
        assert!(
            !work.iter().any(|r| r.number == 12),
            "a draft is not waiting on a reviewer"
        );
    }

    /// The worklist can be ordered either way, and the two are exact reverses.
    ///
    /// Ordering is done in SQL, so a limit returns the newest N rather than the
    /// oldest N reversed — the assertion on `limit 1` is what pins that.
    #[tokio::test]
    async fn worklist_sorts_both_ways() {
        let repo = SqliteTeamStatsRepository::new(seed().await);
        let all = RepoScope::new(vec![]);

        let oldest = repo.worklist(&all, "unreviewed", "oldest", 10, 0).await.unwrap();
        let newest = repo.worklist(&all, "unreviewed", "newest", 10, 0).await.unwrap();

        let o: Vec<i64> = oldest.iter().map(|r| r.number).collect();
        let n: Vec<i64> = newest.iter().map(|r| r.number).collect();
        assert_eq!(o, vec![10, 11], "oldest first");
        assert_eq!(n, vec![11, 10], "newest first");
        assert_eq!(o.iter().rev().copied().collect::<Vec<_>>(), n);

        // A capped newest-first query must return the newest row, not the
        // oldest one — the bug you get from reversing client-side.
        let capped = repo.worklist(&all, "unreviewed", "newest", 1, 0).await.unwrap();
        assert_eq!(capped.len(), 1);
        assert_eq!(capped[0].number, 11, "limit must apply after ordering");
    }

    /// Self-reviews and bots are excluded; duplicate approvals still count as
    /// events while the PR counts once.
    #[tokio::test]
    async fn excludes_self_and_bot_reviews() {
        let repo = SqliteTeamStatsRepository::new(seed().await);
        let all = RepoScope::new(vec![]);
        let people = repo.people(&all, &[], SINCE, UNTIL).await.unwrap();

        let bob = find(&people, "bob");
        assert_eq!(bob.prs_reviewed, 2, "two distinct PRs, despite three reviews");
        assert_eq!(bob.approvals, 2, "approval events, including the duplicate");

        let alice = find(&people, "alice");
        assert_eq!(alice.prs_reviewed, 0, "her own COMMENTED review must not count");

        let totals = repo.totals(&all, SINCE, UNTIL).await.unwrap();
        assert_eq!(totals.review_events, 3, "excludes the self and bot reviews");
        assert_eq!(totals.bot_review_events, 1);
        assert_eq!(totals.self_review_events, 1);
        // Of three merged PRs, two lack an approval: #2 has no reviews at all,
        // and #3 was only commented on.
        assert_eq!(
            totals.merged_without_approval, 2,
            "#2 has no reviews and #3 was only commented on"
        );
        let health = repo.repo_health(&all, SINCE, UNTIL).await.unwrap();
        let beta = health.iter().find(|r| r.repo == "acme/beta").unwrap();
        assert_eq!(beta.merged_without_approval, 1);
    }

    /// Reclassifying a login as a bot must change history with no re-ingest —
    /// the property that justifies having no rollup table.
    #[tokio::test]
    async fn roster_changes_apply_retroactively() {
        let pool = seed().await;
        let repo = SqliteTeamStatsRepository::new(pool.clone());
        let all = RepoScope::new(vec![]);

        let before = repo.totals(&all, SINCE, UNTIL).await.unwrap();
        sqlx::query("UPDATE team_members SET member_group = 'bot' WHERE login = 'bob'")
            .execute(&pool)
            .await
            .unwrap();
        let after = repo.totals(&all, SINCE, UNTIL).await.unwrap();

        assert_eq!(before.review_events, 3);
        assert_eq!(after.review_events, 0, "bob's reviews reclassified without re-ingest");
        assert_eq!(after.bot_review_events, 4);
        assert_eq!(
            after.merged_without_approval, 3,
            "with bob a bot, no PR has a human approval any more"
        );
    }
}
