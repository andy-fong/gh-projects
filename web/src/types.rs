//! Serde types mirroring the backend JSON API (`backend/src/models`) and the
//! old `frontend/src/types/index.ts`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Note {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub repo: Option<String>,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub tags: String, // JSON array string
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Dashboard {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Tile {
    pub id: i64,
    pub dashboard_id: i64,
    pub title: String,
    pub tile_type: String, // "gh_query" | "note"
    pub config: String,    // JSON string
    pub layout: String,    // JSON string
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct TileLayout {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Default for TileLayout {
    fn default() -> Self {
        // Matches the backend default tile size.
        Self { x: 0, y: 0, w: 4, h: 4 }
    }
}

/// A template variable value: either a single string or a list (a list value
/// runs the command once per element). Mirrors `string | string[]`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VarValue {
    One(String),
    Many(Vec<String>),
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Default)]
pub struct GhQueryConfig {
    #[serde(default)]
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commands: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub columns: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_extractors: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variables: Option<BTreeMap<String, VarValue>>,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NoteConfig {
    pub note_id: i64,
}

// ---- Request input payloads ----

#[derive(Clone, Debug, PartialEq, Serialize, Default)]
pub struct CreateNoteInput {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct CreateDashboardInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct UpdateDashboardInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct CreateTileInput {
    pub title: String,
    pub tile_type: String,
    pub config: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<TileLayout>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct UpdateTileInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<TileLayout>,
}

// ---- Response payloads ----

#[derive(Clone, Debug, Deserialize)]
pub struct GhExecuteResponse {
    pub output: serde_json::Value,
    #[allow(dead_code)]
    pub raw: String,
    #[allow(dead_code)]
    pub cached: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RowOrderResponse {
    pub order: Vec<String>,
}
