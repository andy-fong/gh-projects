use async_trait::async_trait;
use sqlx::SqlitePool;
use crate::error::AppError;

#[async_trait]
pub trait RowOrderRepository: Send + Sync {
    async fn get(&self, tile_id: i64) -> Result<Vec<String>, AppError>;
    async fn set(&self, tile_id: i64, keys: Vec<String>) -> Result<Vec<String>, AppError>;
}

pub struct SqliteRowOrderRepository {
    pool: SqlitePool,
}

impl SqliteRowOrderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RowOrderRepository for SqliteRowOrderRepository {
    async fn get(&self, tile_id: i64) -> Result<Vec<String>, AppError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT ordered_keys FROM row_order WHERE tile_id = ?"
        )
        .bind(tile_id)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some((json,)) => Ok(serde_json::from_str(&json).unwrap_or_default()),
            None => Ok(vec![]),
        }
    }

    async fn set(&self, tile_id: i64, keys: Vec<String>) -> Result<Vec<String>, AppError> {
        let json = serde_json::to_string(&keys)?;
        sqlx::query(
            "INSERT INTO row_order (tile_id, ordered_keys) VALUES (?, ?)
             ON CONFLICT(tile_id) DO UPDATE SET ordered_keys = excluded.ordered_keys"
        )
        .bind(tile_id)
        .bind(&json)
        .execute(&self.pool)
        .await?;
        Ok(keys)
    }
}
