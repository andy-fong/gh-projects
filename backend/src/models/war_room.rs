use serde::{Deserialize, Serialize};

// ---- War room (root) ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WarRoom {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub status: String,
    pub config: String, // JSON
    pub links: String,  // Markdown "useful links" box
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWarRoomInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWarRoomInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub config: Option<serde_json::Value>,
    pub links: Option<String>,
    pub position: Option<i64>,
}

// ---- Group ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

#[derive(Debug, Deserialize)]
pub struct CreateGroupInput {
    pub name: String,
    pub repo_id: Option<i64>,
    pub repo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupInput {
    pub name: Option<String>,
    pub repo_id: Option<i64>,
    pub repo: Option<String>,
    pub position: Option<i64>,
}

// ---- Item ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    pub text: String,
    #[serde(default)]
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WarRoomItem {
    pub id: i64,
    pub group_id: i64,
    pub label: String,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub note: String,
    pub stage: String,
    pub checklist: String, // JSON [{text, done}]
    pub depends_on: Option<i64>,
    pub source_item_id: Option<i64>, // set => this row mirrors another item
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateItemInput {
    pub label: String,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub note: Option<String>,
    pub stage: Option<String>,
    pub checklist: Option<Vec<ChecklistItem>>,
    pub depends_on: Option<i64>,
    pub source_item_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateItemInput {
    pub label: Option<String>,
    pub ref_type: Option<String>,
    pub ref_number: Option<i64>,
    pub note: Option<String>,
    pub stage: Option<String>,
    pub checklist: Option<Vec<ChecklistItem>>,
    pub depends_on: Option<i64>,
    pub position: Option<i64>,
}

// ---- Composite read model (GET /api/war-rooms/:id) ----

#[derive(Debug, Serialize)]
pub struct WarRoomDetail {
    #[serde(flatten)]
    pub war_room: WarRoom,
    pub groups: Vec<GroupWithItems>,
}

#[derive(Debug, Serialize)]
pub struct GroupWithItems {
    #[serde(flatten)]
    pub group: WarRoomGroup,
    pub items: Vec<WarRoomItem>,
}
