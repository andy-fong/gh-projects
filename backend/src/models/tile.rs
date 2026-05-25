use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tile {
    pub id: i64,
    pub dashboard_id: i64,
    pub title: String,
    pub tile_type: String,
    pub config: String,  // JSON
    pub layout: String,  // JSON {x, y, w, h}
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTileInput {
    pub title: String,
    pub tile_type: String,
    pub config: serde_json::Value,
    pub layout: Option<TileLayout>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTileInput {
    pub title: Option<String>,
    pub config: Option<serde_json::Value>,
    pub layout: Option<TileLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TileLayout {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
