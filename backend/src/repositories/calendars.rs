use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    error::AppError,
    models::calendar::{
        CalendarComponent, CalendarDashboard, CalendarEvent, CreateCalendarComponentInput,
        CreateCalendarDashboardInput, CreateCalendarEventInput, UpdateCalendarComponentInput,
        UpdateCalendarDashboardInput, UpdateCalendarEventInput,
    },
};

#[async_trait]
pub trait CalendarRepository: Send + Sync {
    // Dashboards
    async fn list_dashboards(&self) -> Result<Vec<CalendarDashboard>, AppError>;
    async fn find_dashboard(&self, id: i64) -> Result<Option<CalendarDashboard>, AppError>;
    async fn create_dashboard(
        &self,
        input: CreateCalendarDashboardInput,
    ) -> Result<CalendarDashboard, AppError>;
    async fn update_dashboard(
        &self,
        id: i64,
        input: UpdateCalendarDashboardInput,
    ) -> Result<Option<CalendarDashboard>, AppError>;
    async fn delete_dashboard(&self, id: i64) -> Result<bool, AppError>;

    // Components
    async fn list_components(&self, calendar_id: i64) -> Result<Vec<CalendarComponent>, AppError>;
    async fn create_component(
        &self,
        calendar_id: i64,
        input: CreateCalendarComponentInput,
    ) -> Result<CalendarComponent, AppError>;
    async fn update_component(
        &self,
        id: i64,
        input: UpdateCalendarComponentInput,
    ) -> Result<Option<CalendarComponent>, AppError>;
    async fn delete_component(&self, id: i64) -> Result<bool, AppError>;

    // Events
    async fn list_events(&self, calendar_id: i64) -> Result<Vec<CalendarEvent>, AppError>;
    async fn create_event(
        &self,
        calendar_id: i64,
        input: CreateCalendarEventInput,
    ) -> Result<CalendarEvent, AppError>;
    async fn update_event(
        &self,
        id: i64,
        input: UpdateCalendarEventInput,
    ) -> Result<Option<CalendarEvent>, AppError>;
    async fn delete_event(&self, id: i64) -> Result<bool, AppError>;
}

pub struct SqliteCalendarRepository {
    pool: SqlitePool,
}

impl SqliteCalendarRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CalendarRepository for SqliteCalendarRepository {
    // ── Dashboards ────────────────────────────────────────────────────────────

    async fn list_dashboards(&self) -> Result<Vec<CalendarDashboard>, AppError> {
        Ok(sqlx::query_as::<_, CalendarDashboard>(
            "SELECT * FROM calendar_dashboards ORDER BY position, id",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    async fn find_dashboard(&self, id: i64) -> Result<Option<CalendarDashboard>, AppError> {
        Ok(
            sqlx::query_as::<_, CalendarDashboard>(
                "SELECT * FROM calendar_dashboards WHERE id = ?",
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?,
        )
    }

    async fn create_dashboard(
        &self,
        input: CreateCalendarDashboardInput,
    ) -> Result<CalendarDashboard, AppError> {
        Ok(sqlx::query_as::<_, CalendarDashboard>(
            "INSERT INTO calendar_dashboards (name, description, position)
             VALUES (?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM calendar_dashboards))
             RETURNING *",
        )
        .bind(&input.name)
        .bind(input.description.unwrap_or_default())
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_dashboard(
        &self,
        id: i64,
        input: UpdateCalendarDashboardInput,
    ) -> Result<Option<CalendarDashboard>, AppError> {
        let existing = match self.find_dashboard(id).await? {
            Some(d) => d,
            None => return Ok(None),
        };
        Ok(sqlx::query_as::<_, CalendarDashboard>(
            "UPDATE calendar_dashboards
             SET name = ?, description = ?, position = ?, updated_at = datetime('now')
             WHERE id = ? RETURNING *",
        )
        .bind(input.name.unwrap_or(existing.name))
        .bind(input.description.unwrap_or(existing.description))
        .bind(input.position.unwrap_or(existing.position))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete_dashboard(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM calendar_dashboards WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }

    // ── Components ────────────────────────────────────────────────────────────

    async fn list_components(&self, calendar_id: i64) -> Result<Vec<CalendarComponent>, AppError> {
        Ok(sqlx::query_as::<_, CalendarComponent>(
            "SELECT * FROM calendar_components WHERE calendar_id = ? ORDER BY position, id",
        )
        .bind(calendar_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn create_component(
        &self,
        calendar_id: i64,
        input: CreateCalendarComponentInput,
    ) -> Result<CalendarComponent, AppError> {
        Ok(sqlx::query_as::<_, CalendarComponent>(
            "INSERT INTO calendar_components (calendar_id, name, short_name, color, position)
             VALUES (?, ?, ?, ?,
               (SELECT COALESCE(MAX(position) + 1, 0) FROM calendar_components WHERE calendar_id = ?))
             RETURNING *",
        )
        .bind(calendar_id)
        .bind(&input.name)
        .bind(input.short_name.as_deref())
        .bind(input.color.unwrap_or_else(|| "#2f81f7".to_string()))
        .bind(calendar_id)
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_component(
        &self,
        id: i64,
        input: UpdateCalendarComponentInput,
    ) -> Result<Option<CalendarComponent>, AppError> {
        let existing = match sqlx::query_as::<_, CalendarComponent>(
            "SELECT * FROM calendar_components WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        {
            Some(c) => c,
            None => return Ok(None),
        };
        Ok(sqlx::query_as::<_, CalendarComponent>(
            "UPDATE calendar_components
             SET name = ?, short_name = ?, color = ?, position = ?, updated_at = datetime('now')
             WHERE id = ? RETURNING *",
        )
        .bind(input.name.unwrap_or(existing.name))
        .bind(input.short_name.as_deref())
        .bind(input.color.unwrap_or(existing.color))
        .bind(input.position.unwrap_or(existing.position))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete_component(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM calendar_components WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }

    // ── Events ────────────────────────────────────────────────────────────────

    async fn list_events(&self, calendar_id: i64) -> Result<Vec<CalendarEvent>, AppError> {
        Ok(sqlx::query_as::<_, CalendarEvent>(
            "SELECT * FROM calendar_events WHERE calendar_id = ? ORDER BY release_date, id",
        )
        .bind(calendar_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn create_event(
        &self,
        calendar_id: i64,
        input: CreateCalendarEventInput,
    ) -> Result<CalendarEvent, AppError> {
        Ok(sqlx::query_as::<_, CalendarEvent>(
            "INSERT INTO calendar_events (calendar_id, component_id, version, status, release_date, actual_release_date, note)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             RETURNING *",
        )
        .bind(calendar_id)
        .bind(input.component_id)
        .bind(&input.version)
        .bind(input.status.unwrap_or_else(|| "on_track".to_string()))
        .bind(&input.release_date)
        .bind(input.actual_release_date.as_deref())
        .bind(input.note.unwrap_or_default())
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_event(
        &self,
        id: i64,
        input: UpdateCalendarEventInput,
    ) -> Result<Option<CalendarEvent>, AppError> {
        let existing = match sqlx::query_as::<_, CalendarEvent>(
            "SELECT * FROM calendar_events WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        {
            Some(e) => e,
            None => return Ok(None),
        };
        // actual_release_date is replace-style: None clears the field
        Ok(sqlx::query_as::<_, CalendarEvent>(
            "UPDATE calendar_events
             SET component_id = ?, version = ?, status = ?, release_date = ?,
                 actual_release_date = ?, note = ?, updated_at = datetime('now')
             WHERE id = ? RETURNING *",
        )
        // component_id is replace-style: None clears the association
        .bind(input.component_id)
        .bind(input.version.unwrap_or(existing.version))
        .bind(input.status.unwrap_or(existing.status))
        .bind(input.release_date.unwrap_or(existing.release_date))
        .bind(input.actual_release_date)
        .bind(input.note.unwrap_or(existing.note))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete_event(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM calendar_events WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }
}
