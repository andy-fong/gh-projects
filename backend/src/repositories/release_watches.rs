use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    error::AppError,
    models::release_watch::{CreateReleaseWatchInput, ReleaseWatch},
};

#[async_trait]
pub trait ReleaseWatchRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<ReleaseWatch>, AppError>;
    async fn create(&self, input: CreateReleaseWatchInput) -> Result<ReleaseWatch, AppError>;
    async fn delete(&self, id: i64) -> Result<bool, AppError>;
    async fn reorder(&self, ids: &[i64]) -> Result<(), AppError>;
    async fn delete_all(&self) -> Result<(), AppError>;
}

pub struct SqliteReleaseWatchRepository {
    pool: SqlitePool,
}

impl SqliteReleaseWatchRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReleaseWatchRepository for SqliteReleaseWatchRepository {
    async fn list(&self) -> Result<Vec<ReleaseWatch>, AppError> {
        Ok(
            sqlx::query_as::<_, ReleaseWatch>("SELECT * FROM release_watches ORDER BY position")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    async fn create(&self, input: CreateReleaseWatchInput) -> Result<ReleaseWatch, AppError> {
        Ok(sqlx::query_as::<_, ReleaseWatch>(
            "INSERT INTO release_watches (repo_id, limit_count, position)
             VALUES (?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM release_watches))
             RETURNING *",
        )
        .bind(input.repo_id)
        .bind(input.limit_count)
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM release_watches WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }

    async fn reorder(&self, ids: &[i64]) -> Result<(), AppError> {
        for (pos, id) in ids.iter().enumerate() {
            sqlx::query(
                "UPDATE release_watches SET position = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(pos as i64)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn delete_all(&self) -> Result<(), AppError> {
        sqlx::query("DELETE FROM release_watches")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
