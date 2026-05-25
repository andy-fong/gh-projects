use async_trait::async_trait;
use sqlx::SqlitePool;
use crate::{error::AppError, models::dashboard::{CreateDashboardInput, Dashboard, UpdateDashboardInput}};

#[async_trait]
pub trait DashboardRepository: Send + Sync {
    async fn create(&self, input: CreateDashboardInput) -> Result<Dashboard, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Dashboard>, AppError>;
    async fn list(&self) -> Result<Vec<Dashboard>, AppError>;
    async fn update(&self, id: i64, input: UpdateDashboardInput) -> Result<Option<Dashboard>, AppError>;
    async fn delete(&self, id: i64) -> Result<bool, AppError>;
}

pub struct SqliteDashboardRepository {
    pool: SqlitePool,
}

impl SqliteDashboardRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DashboardRepository for SqliteDashboardRepository {
    async fn create(&self, input: CreateDashboardInput) -> Result<Dashboard, AppError> {
        let d = sqlx::query_as::<_, Dashboard>(
            "INSERT INTO dashboards (name, description) VALUES (?, ?) RETURNING *"
        )
        .bind(&input.name)
        .bind(input.description.unwrap_or_default())
        .fetch_one(&self.pool)
        .await?;
        Ok(d)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Dashboard>, AppError> {
        Ok(sqlx::query_as::<_, Dashboard>("SELECT * FROM dashboards WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }

    async fn list(&self) -> Result<Vec<Dashboard>, AppError> {
        Ok(sqlx::query_as::<_, Dashboard>("SELECT * FROM dashboards ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?)
    }

    async fn update(&self, id: i64, input: UpdateDashboardInput) -> Result<Option<Dashboard>, AppError> {
        let existing = match self.find_by_id(id).await? {
            Some(d) => d,
            None => return Ok(None),
        };
        Ok(sqlx::query_as::<_, Dashboard>(
            "UPDATE dashboards SET name = ?, description = ?, updated_at = datetime('now')
             WHERE id = ? RETURNING *"
        )
        .bind(input.name.unwrap_or(existing.name))
        .bind(input.description.unwrap_or(existing.description))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM dashboards WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected() > 0)
    }
}
