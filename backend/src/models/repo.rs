use serde::{Deserialize, Serialize};

/// A reusable, global registry entry: a friendly label paired with an
/// `owner/repo`. War room groups select one of these to auto-scope items.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Repo {
    pub id: i64,
    pub name: String,
    pub owner_repo: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateRepoInput {
    pub name: String,
    pub owner_repo: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRepoInput {
    pub name: Option<String>,
    pub owner_repo: Option<String>,
    pub position: Option<i64>,
}
