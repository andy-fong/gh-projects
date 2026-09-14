//! Ingest side of Team Stats: writing PR/review facts and the sync cursors.
//!
//! Read-side aggregates live in [`crate::repositories::team_stats`].

use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    error::AppError,
    models::team_stats::{IngestCounts, PrIngest, SyncState, TrackedRepoStatus},
};

#[async_trait]
pub trait PrFactsRepository: Send + Sync {
    /// Upsert a page of PRs with their reviews and review requests.
    ///
    /// Batch-shaped on purpose: the whole page commits in one transaction, and
    /// the `sqlx::Transaction` stays inside the impl rather than leaking
    /// SQLite through the trait.
    async fn ingest_page(&self, rows: Vec<PrIngest>) -> Result<IngestCounts, AppError>;

    async fn get_sync_state(&self, repo: &str) -> Result<Option<SyncState>, AppError>;

    /// Record a successful run: advance the cursor and clear any stored error.
    async fn mark_synced(
        &self,
        repo: &str,
        updated_cursor: &str,
        backfill_from: &str,
        backfilled: bool,
    ) -> Result<(), AppError>;

    /// Record a failure, deliberately leaving `updated_cursor` untouched so the
    /// next run re-covers the window that failed.
    async fn mark_sync_error(&self, repo: &str, error: &str) -> Result<(), AppError>;

    /// PR numbers this repo still has marked OPEN — reconciled against the open
    /// sweep to catch PRs that closed outside the cursor window.
    async fn open_numbers(&self, repo: &str) -> Result<Vec<i64>, AppError>;

    async fn repo_status(&self) -> Result<Vec<TrackedRepoStatus>, AppError>;

    async fn latest_sync_at(&self) -> Result<Option<String>, AppError>;
}

pub struct SqlitePrFactsRepository {
    pool: SqlitePool,
}

impl SqlitePrFactsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PrFactsRepository for SqlitePrFactsRepository {
    async fn ingest_page(&self, rows: Vec<PrIngest>) -> Result<IngestCounts, AppError> {
        let mut counts = IngestCounts::default();
        let mut tx = self.pool.begin().await?;

        for item in rows {
            let p = &item.pr;
            // Explicit column list: created_week / merged_week are generated
            // columns and cannot be inserted into.
            sqlx::query(
                "INSERT INTO pr_facts (
                     repo, number, node_id, title, url, state, is_draft,
                     author_login, merged_by_login, gh_created_at, gh_updated_at,
                     merged_at, closed_at, additions, deletions, changed_files,
                     comment_count, review_total, reviews_truncated, requests_truncated
                 ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
                 ON CONFLICT(repo, number) DO UPDATE SET
                     node_id            = excluded.node_id,
                     title              = excluded.title,
                     url                = excluded.url,
                     state              = excluded.state,
                     is_draft           = excluded.is_draft,
                     author_login       = excluded.author_login,
                     merged_by_login    = excluded.merged_by_login,
                     gh_updated_at      = excluded.gh_updated_at,
                     merged_at          = excluded.merged_at,
                     closed_at          = excluded.closed_at,
                     additions          = excluded.additions,
                     deletions          = excluded.deletions,
                     changed_files      = excluded.changed_files,
                     comment_count      = excluded.comment_count,
                     review_total       = excluded.review_total,
                     reviews_truncated  = excluded.reviews_truncated,
                     requests_truncated = excluded.requests_truncated,
                     synced_at          = datetime('now')",
            )
            .bind(&p.repo)
            .bind(p.number)
            .bind(&p.node_id)
            .bind(&p.title)
            .bind(&p.url)
            .bind(&p.state)
            .bind(p.is_draft)
            .bind(&p.author_login)
            .bind(&p.merged_by_login)
            .bind(&p.gh_created_at)
            .bind(&p.gh_updated_at)
            .bind(&p.merged_at)
            .bind(&p.closed_at)
            .bind(p.additions)
            .bind(p.deletions)
            .bind(p.changed_files)
            .bind(p.comment_count)
            .bind(p.review_total)
            .bind(p.reviews_truncated)
            .bind(p.requests_truncated)
            .execute(&mut *tx)
            .await?;
            counts.prs += 1;

            // Delete-and-reinsert is exactly idempotent and is the only thing
            // that handles a review *disappearing* (dismissed and removed).
            // Median is 1 review per PR, so it is cheaper than reconciling.
            // When the fetch was truncated we upsert instead, so rows we never
            // saw are not deleted.
            if item.reviews_complete {
                sqlx::query("DELETE FROM pr_review_facts WHERE repo = ? AND number = ?")
                    .bind(&p.repo)
                    .bind(p.number)
                    .execute(&mut *tx)
                    .await?;
            }
            for r in &item.reviews {
                sqlx::query(
                    "INSERT INTO pr_review_facts
                         (repo, number, node_id, reviewer_login, author_login, state, submitted_at)
                     VALUES (?,?,?,?,?,?,?)
                     ON CONFLICT(repo, number, reviewer_login, submitted_at) DO UPDATE SET
                         state     = excluded.state,
                         node_id   = excluded.node_id,
                         synced_at = datetime('now')",
                )
                .bind(&p.repo)
                .bind(p.number)
                .bind(&r.node_id)
                .bind(&r.reviewer_login)
                .bind(&r.author_login)
                .bind(&r.state)
                .bind(&r.submitted_at)
                .execute(&mut *tx)
                .await?;
                counts.reviews += 1;
            }

            // Requested reviewers are a point-in-time snapshot by definition,
            // so they are always replaced wholesale.
            sqlx::query("DELETE FROM pr_review_requests WHERE repo = ? AND number = ?")
                .bind(&p.repo)
                .bind(p.number)
                .execute(&mut *tx)
                .await?;
            for q in &item.requests {
                sqlx::query(
                    "INSERT OR IGNORE INTO pr_review_requests
                         (repo, number, requested_login, requested_type)
                     VALUES (?,?,?,?)",
                )
                .bind(&p.repo)
                .bind(p.number)
                .bind(&q.requested_login)
                .bind(&q.requested_type)
                .execute(&mut *tx)
                .await?;
                counts.requests += 1;
            }
        }

        tx.commit().await?;
        Ok(counts)
    }

    async fn get_sync_state(&self, repo: &str) -> Result<Option<SyncState>, AppError> {
        Ok(
            sqlx::query_as::<_, SyncState>("SELECT * FROM stats_sync_state WHERE repo = ?")
                .bind(repo)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    async fn mark_synced(
        &self,
        repo: &str,
        updated_cursor: &str,
        backfill_from: &str,
        backfilled: bool,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO stats_sync_state
                 (repo, updated_cursor, backfill_from, backfilled,
                  last_sync_at, last_sync_status, last_error)
             VALUES (?, ?, ?, ?, datetime('now'), 'ok', NULL)
             ON CONFLICT(repo) DO UPDATE SET
                 updated_cursor   = excluded.updated_cursor,
                 backfill_from    = excluded.backfill_from,
                 backfilled       = excluded.backfilled,
                 last_sync_at     = datetime('now'),
                 last_sync_status = 'ok',
                 last_error       = NULL,
                 updated_at       = datetime('now')",
        )
        .bind(repo)
        .bind(updated_cursor)
        .bind(backfill_from)
        .bind(backfilled)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_sync_error(&self, repo: &str, error: &str) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO stats_sync_state (repo, last_sync_at, last_sync_status, last_error)
             VALUES (?, datetime('now'), 'error', ?)
             ON CONFLICT(repo) DO UPDATE SET
                 last_sync_at     = datetime('now'),
                 last_sync_status = 'error',
                 last_error       = excluded.last_error,
                 updated_at       = datetime('now')",
        )
        .bind(repo)
        .bind(error)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn open_numbers(&self, repo: &str) -> Result<Vec<i64>, AppError> {
        let rows: Vec<(i64,)> =
            sqlx::query_as("SELECT number FROM pr_facts WHERE repo = ? AND state = 'OPEN'")
                .bind(repo)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    async fn repo_status(&self) -> Result<Vec<TrackedRepoStatus>, AppError> {
        let rows: Vec<(i64, String, String, bool, i64, i64, Option<String>, Option<String>, Option<String>, Option<bool>, Option<String>)> =
            sqlx::query_as(
                "SELECT r.id, r.name, r.owner_repo, r.track_stats,
                        (SELECT COUNT(*) FROM pr_facts f WHERE f.repo = lower(r.owner_repo)),
                        (SELECT COUNT(*) FROM pr_review_facts v WHERE v.repo = lower(r.owner_repo)),
                        s.last_sync_at, s.last_sync_status, s.last_error,
                        s.backfilled, s.updated_cursor
                 FROM repos r
                 LEFT JOIN stats_sync_state s ON s.repo = lower(r.owner_repo)
                 ORDER BY r.position, r.name",
            )
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| TrackedRepoStatus {
                repo_id: r.0,
                name: r.1,
                owner_repo: r.2,
                track_stats: r.3,
                pr_count: r.4,
                review_count: r.5,
                last_sync_at: r.6,
                last_sync_status: r.7,
                last_error: r.8,
                backfilled: r.9.unwrap_or(false),
                updated_cursor: r.10,
            })
            .collect())
    }

    async fn latest_sync_at(&self) -> Result<Option<String>, AppError> {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT MAX(last_sync_at) FROM stats_sync_state")
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.and_then(|r| r.0))
    }
}
