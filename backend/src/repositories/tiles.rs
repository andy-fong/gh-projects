use async_trait::async_trait;
use sqlx::SqlitePool;
use crate::{error::AppError, models::tile::{CreateTileInput, Tile, UpdateTileInput, TileLayout}};

#[async_trait]
pub trait TileRepository: Send + Sync {
    async fn create(&self, dashboard_id: i64, input: CreateTileInput) -> Result<Tile, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Tile>, AppError>;
    async fn list_by_dashboard(&self, dashboard_id: i64) -> Result<Vec<Tile>, AppError>;
    async fn update(&self, id: i64, input: UpdateTileInput) -> Result<Option<Tile>, AppError>;
    async fn delete(&self, id: i64) -> Result<bool, AppError>;
}

pub struct SqliteTileRepository {
    pool: SqlitePool,
}

impl SqliteTileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TileRepository for SqliteTileRepository {
    async fn create(&self, dashboard_id: i64, input: CreateTileInput) -> Result<Tile, AppError> {
        let default_layout = TileLayout { x: 0, y: 0, w: 6, h: 4 };
        let layout = serde_json::to_string(&input.layout.unwrap_or(default_layout))?;
        let config = serde_json::to_string(&input.config)?;
        Ok(sqlx::query_as::<_, Tile>(
            "INSERT INTO tiles (dashboard_id, title, tile_type, config, layout)
             VALUES (?, ?, ?, ?, ?) RETURNING *"
        )
        .bind(dashboard_id)
        .bind(&input.title)
        .bind(&input.tile_type)
        .bind(&config)
        .bind(&layout)
        .fetch_one(&self.pool)
        .await?)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Tile>, AppError> {
        Ok(sqlx::query_as::<_, Tile>("SELECT * FROM tiles WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }

    async fn list_by_dashboard(&self, dashboard_id: i64) -> Result<Vec<Tile>, AppError> {
        Ok(sqlx::query_as::<_, Tile>("SELECT * FROM tiles WHERE dashboard_id = ? ORDER BY id")
            .bind(dashboard_id)
            .fetch_all(&self.pool)
            .await?)
    }

    async fn update(&self, id: i64, input: UpdateTileInput) -> Result<Option<Tile>, AppError> {
        let existing = match self.find_by_id(id).await? {
            Some(t) => t,
            None => return Ok(None),
        };
        let config = match input.config {
            Some(c) => serde_json::to_string(&c)?,
            None => existing.config.clone(),
        };
        let layout = match input.layout {
            Some(l) => serde_json::to_string(&l)?,
            None => existing.layout.clone(),
        };
        Ok(sqlx::query_as::<_, Tile>(
            "UPDATE tiles SET title = ?, config = ?, layout = ?, updated_at = datetime('now')
             WHERE id = ? RETURNING *"
        )
        .bind(input.title.unwrap_or(existing.title))
        .bind(&config)
        .bind(&layout)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM tiles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected() > 0)
    }
}
