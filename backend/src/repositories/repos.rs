use async_trait::async_trait;
use sqlx::SqlitePool;
use crate::{error::AppError, models::repo::{CreateRepoInput, Repo, UpdateRepoInput}};

#[async_trait]
pub trait RepoRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<Repo>, AppError>;
    /// Only the repos opted in to the Team Stats sync.
    async fn list_tracked(&self) -> Result<Vec<Repo>, AppError>;
    async fn create(&self, input: CreateRepoInput) -> Result<Repo, AppError>;
    async fn update(&self, id: i64, input: UpdateRepoInput) -> Result<Option<Repo>, AppError>;
    async fn delete(&self, id: i64) -> Result<bool, AppError>;
}

pub struct SqliteRepoRepository {
    pool: SqlitePool,
}

impl SqliteRepoRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Repo>, AppError> {
        Ok(sqlx::query_as::<_, Repo>("SELECT * FROM repos WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }
}

#[async_trait]
impl RepoRepository for SqliteRepoRepository {
    async fn list(&self) -> Result<Vec<Repo>, AppError> {
        Ok(sqlx::query_as::<_, Repo>("SELECT * FROM repos ORDER BY position, name")
            .fetch_all(&self.pool)
            .await?)
    }

    async fn list_tracked(&self) -> Result<Vec<Repo>, AppError> {
        Ok(sqlx::query_as::<_, Repo>(
            "SELECT * FROM repos WHERE track_stats = 1 ORDER BY position, name",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    async fn create(&self, input: CreateRepoInput) -> Result<Repo, AppError> {
        Ok(sqlx::query_as::<_, Repo>(
            "INSERT INTO repos (name, owner_repo, track_stats, position)
             VALUES (?, ?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM repos))
             RETURNING *",
        )
        .bind(&input.name)
        .bind(&input.owner_repo)
        .bind(input.track_stats.unwrap_or(false))
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update(&self, id: i64, input: UpdateRepoInput) -> Result<Option<Repo>, AppError> {
        let existing = match self.find_by_id(id).await? {
            Some(r) => r,
            None => return Ok(None),
        };
        Ok(sqlx::query_as::<_, Repo>(
            "UPDATE repos SET name = ?, owner_repo = ?, position = ?, track_stats = ?,
                 updated_at = datetime('now')
             WHERE id = ? RETURNING *",
        )
        .bind(input.name.unwrap_or(existing.name))
        .bind(input.owner_repo.unwrap_or(existing.owner_repo))
        .bind(input.position.unwrap_or(existing.position))
        .bind(input.track_stats.unwrap_or(existing.track_stats))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM repos WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }
}
