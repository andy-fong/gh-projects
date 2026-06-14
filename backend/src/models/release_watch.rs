use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReleaseWatch {
    pub id: i64,
    pub repo_id: i64,
    pub limit_count: i64,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateReleaseWatchInput {
    pub repo_id: i64,
    #[serde(default = "default_limit")]
    pub limit_count: i64,
}

fn default_limit() -> i64 {
    4
}

#[derive(Debug, Deserialize)]
pub struct ReorderReleaseWatchesInput {
    pub ids: Vec<i64>,
}
