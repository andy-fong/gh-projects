use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub repo: Option<String>,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub tags: String, // JSON array stored as text
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNoteInput {
    pub title: String,
    pub body: Option<String>,
    pub repo: Option<String>,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNoteInput {
    pub title: Option<String>,
    pub body: Option<String>,
    pub repo: Option<String>,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct NoteFilter {
    pub repo: Option<String>,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
}
