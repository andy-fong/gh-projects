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

// ---- Repo registry (reusable across war rooms) ----

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct ReleaseWatch {
    pub id: i64,
    pub repo_id: i64,
    pub limit_count: i64,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateReleaseWatchInput {
    pub repo_id: i64,
    pub limit_count: i64,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct Repo {
    pub id: i64,
    pub name: String,
    pub owner_repo: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct CreateRepoInput {
    pub name: String,
    pub owner_repo: String,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct UpdateRepoInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
}

// ---- War rooms ----

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct WarRoom {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub status: String,
    pub config: String,
    #[serde(default)]
    pub notes: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct WarRoomGroup {
    pub id: i64,
    pub war_room_id: i64,
    pub repo_id: Option<i64>,
    pub name: String,
    pub repo: Option<String>,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize, Default)]
pub struct ChecklistItem {
    pub text: String,
    #[serde(default)]
    pub done: bool,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct WarRoomItem {
    pub id: i64,
    pub group_id: i64,
    pub label: String,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub note: String,
    pub stage: String,
    pub checklist: String, // JSON array string
    pub depends_on: Option<i64>,
    #[serde(default)]
    pub source_item_id: Option<i64>,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl WarRoomItem {
    /// Parse the stored checklist JSON; empty/invalid yields an empty list.
    pub fn checklist_items(&self) -> Vec<ChecklistItem> {
        serde_json::from_str(&self.checklist).unwrap_or_default()
    }
}

/// Composite read model returned by `GET /api/war-rooms/:id` (fields flattened).
#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct WarRoomDetail {
    #[serde(flatten)]
    pub war_room: WarRoom,
    pub groups: Vec<GroupWithItems>,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct GroupWithItems {
    #[serde(flatten)]
    pub group: WarRoomGroup,
    pub items: Vec<WarRoomItem>,
}

// ---- War room request payloads ----

#[derive(Clone, Debug, Serialize, Default)]
pub struct CreateWarRoomInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct UpdateWarRoomInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct CreateGroupInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
}

/// Group update is replace-style: send the full group (nullable repo fields
/// included) so omitting them clears them.
#[derive(Clone, Debug, Serialize, Default)]
pub struct UpdateGroupInput {
    pub name: Option<String>,
    pub repo_id: Option<i64>,
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct CreateItemInput {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checklist: Option<Vec<ChecklistItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_item_id: Option<i64>,
}

/// Item update is replace-style: send the full item (nullable ref/depends
/// fields included) so omitting them clears them.
#[derive(Clone, Debug, Serialize, Default)]
pub struct UpdateItemInput {
    pub label: Option<String>,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub note: Option<String>,
    pub stage: Option<String>,
    pub checklist: Option<Vec<ChecklistItem>>,
    pub depends_on: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
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

// ---- Calendar dashboards ----

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct CalendarDashboard {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct CalendarComponent {
    pub id: i64,
    pub calendar_id: i64,
    pub name: String,
    pub short_name: Option<String>,
    pub color: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub struct CalendarEvent {
    pub id: i64,
    pub calendar_id: i64,
    pub component_id: Option<i64>,
    pub version: String,
    pub status: String,                // "on_track" | "at_risk" | "delayed"
    pub release_date: String,          // YYYY-MM-DD planned
    pub actual_release_date: Option<String>, // YYYY-MM-DD actual
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct CalendarDashboardDetail {
    #[serde(flatten)]
    pub dashboard: CalendarDashboard,
    pub components: Vec<CalendarComponent>,
    pub events: Vec<CalendarEvent>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct CreateCalendarDashboardInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct UpdateCalendarDashboardInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct CreateCalendarComponentInput {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct UpdateCalendarComponentInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub short_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct CreateCalendarEventInput {
    pub component_id: Option<i64>,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    pub release_date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct UpdateCalendarEventInput {
    /// Replace-style: None clears the association
    pub component_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    pub actual_release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
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
