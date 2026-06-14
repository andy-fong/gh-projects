use serde::{Deserialize, Serialize};

// ── Dashboard ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CalendarDashboard {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCalendarDashboardInput {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCalendarDashboardInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub position: Option<i64>,
}

// ── Component ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

#[derive(Debug, Deserialize)]
pub struct CreateCalendarComponentInput {
    pub name: String,
    /// Optional abbreviation shown on calendar pills instead of full name.
    pub short_name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCalendarComponentInput {
    pub name: Option<String>,
    /// Replace-style: None clears the short name.
    pub short_name: Option<String>,
    pub color: Option<String>,
    pub position: Option<i64>,
}

// ── Event ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CalendarEvent {
    pub id: i64,
    pub calendar_id: i64,
    pub component_id: Option<i64>,
    pub version: String,
    pub status: String,                // "on_track" | "at_risk" | "delayed"
    pub release_date: String,          // YYYY-MM-DD planned
    pub actual_release_date: Option<String>, // YYYY-MM-DD actual (set after release)
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCalendarEventInput {
    pub component_id: Option<i64>,
    pub version: String,
    pub status: Option<String>,
    pub release_date: String,
    pub actual_release_date: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCalendarEventInput {
    /// Replace-style: None clears the component association.
    pub component_id: Option<i64>,
    pub version: Option<String>,
    pub status: Option<String>,
    pub release_date: Option<String>,
    /// Replace-style: None clears the actual release date.
    pub actual_release_date: Option<String>,
    pub note: Option<String>,
}

// ── Composite read model ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CalendarDashboardDetail {
    #[serde(flatten)]
    pub dashboard: CalendarDashboard,
    pub components: Vec<CalendarComponent>,
    pub events: Vec<CalendarEvent>,
}
