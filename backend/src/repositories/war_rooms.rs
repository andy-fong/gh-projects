use async_trait::async_trait;
use sqlx::SqlitePool;
use crate::{
    error::AppError,
    models::war_room::{
        CreateGroupInput, CreateItemInput, CreateWarRoomInput, UpdateGroupInput, UpdateItemInput,
        UpdateWarRoomInput, WarRoom, WarRoomGroup, WarRoomItem,
    },
};

#[async_trait]
pub trait WarRoomRepository: Send + Sync {
    // Rooms
    async fn list_rooms(&self) -> Result<Vec<WarRoom>, AppError>;
    async fn find_room(&self, id: i64) -> Result<Option<WarRoom>, AppError>;
    async fn create_room(&self, input: CreateWarRoomInput) -> Result<WarRoom, AppError>;
    async fn update_room(&self, id: i64, input: UpdateWarRoomInput) -> Result<Option<WarRoom>, AppError>;
    async fn delete_room(&self, id: i64) -> Result<bool, AppError>;

    // Groups
    async fn list_groups(&self, war_room_id: i64) -> Result<Vec<WarRoomGroup>, AppError>;
    async fn create_group(&self, war_room_id: i64, input: CreateGroupInput) -> Result<WarRoomGroup, AppError>;
    async fn update_group(&self, id: i64, input: UpdateGroupInput) -> Result<Option<WarRoomGroup>, AppError>;
    async fn delete_group(&self, id: i64) -> Result<bool, AppError>;

    // Items (listed for a whole room, ordered by group then position)
    async fn list_items(&self, war_room_id: i64) -> Result<Vec<WarRoomItem>, AppError>;
    async fn create_item(&self, group_id: i64, input: CreateItemInput) -> Result<WarRoomItem, AppError>;
    async fn update_item(&self, id: i64, input: UpdateItemInput) -> Result<Option<WarRoomItem>, AppError>;
    async fn delete_item(&self, id: i64) -> Result<bool, AppError>;
}

pub struct SqliteWarRoomRepository {
    pool: SqlitePool,
}

impl SqliteWarRoomRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn find_group(&self, id: i64) -> Result<Option<WarRoomGroup>, AppError> {
        Ok(sqlx::query_as::<_, WarRoomGroup>("SELECT * FROM war_room_groups WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }

    async fn find_item(&self, id: i64) -> Result<Option<WarRoomItem>, AppError> {
        Ok(sqlx::query_as::<_, WarRoomItem>("SELECT * FROM war_room_items WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }
}

#[async_trait]
impl WarRoomRepository for SqliteWarRoomRepository {
    // ---- Rooms ----

    async fn list_rooms(&self) -> Result<Vec<WarRoom>, AppError> {
        Ok(sqlx::query_as::<_, WarRoom>("SELECT * FROM war_rooms ORDER BY position, created_at DESC")
            .fetch_all(&self.pool)
            .await?)
    }

    async fn find_room(&self, id: i64) -> Result<Option<WarRoom>, AppError> {
        Ok(sqlx::query_as::<_, WarRoom>("SELECT * FROM war_rooms WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }

    async fn create_room(&self, input: CreateWarRoomInput) -> Result<WarRoom, AppError> {
        Ok(sqlx::query_as::<_, WarRoom>(
            "INSERT INTO war_rooms (name, description, position)
             VALUES (?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM war_rooms))
             RETURNING *",
        )
        .bind(&input.name)
        .bind(input.description.unwrap_or_default())
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_room(&self, id: i64, input: UpdateWarRoomInput) -> Result<Option<WarRoom>, AppError> {
        let existing = match self.find_room(id).await? {
            Some(r) => r,
            None => return Ok(None),
        };
        let config = match input.config {
            Some(v) => serde_json::to_string(&v)?,
            None => existing.config,
        };
        Ok(sqlx::query_as::<_, WarRoom>(
            "UPDATE war_rooms SET name = ?, description = ?, status = ?, config = ?, links = ?, position = ?,
                updated_at = datetime('now')
             WHERE id = ? RETURNING *",
        )
        .bind(input.name.unwrap_or(existing.name))
        .bind(input.description.unwrap_or(existing.description))
        .bind(input.status.unwrap_or(existing.status))
        .bind(config)
        .bind(input.notes.unwrap_or(existing.notes))
        .bind(input.position.unwrap_or(existing.position))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete_room(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM war_rooms WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }

    // ---- Groups ----

    async fn list_groups(&self, war_room_id: i64) -> Result<Vec<WarRoomGroup>, AppError> {
        Ok(sqlx::query_as::<_, WarRoomGroup>(
            "SELECT * FROM war_room_groups WHERE war_room_id = ? ORDER BY position, id",
        )
        .bind(war_room_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn create_group(&self, war_room_id: i64, input: CreateGroupInput) -> Result<WarRoomGroup, AppError> {
        Ok(sqlx::query_as::<_, WarRoomGroup>(
            "INSERT INTO war_room_groups (war_room_id, repo_id, name, repo, position)
             VALUES (?, ?, ?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM war_room_groups WHERE war_room_id = ?))
             RETURNING *",
        )
        .bind(war_room_id)
        .bind(input.repo_id)
        .bind(&input.name)
        .bind(&input.repo)
        .bind(war_room_id)
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_group(&self, id: i64, input: UpdateGroupInput) -> Result<Option<WarRoomGroup>, AppError> {
        let existing = match self.find_group(id).await? {
            Some(g) => g,
            None => return Ok(None),
        };
        Ok(sqlx::query_as::<_, WarRoomGroup>(
            "UPDATE war_room_groups SET repo_id = ?, name = ?, repo = ?, position = ?,
                updated_at = datetime('now')
             WHERE id = ? RETURNING *",
        )
        // repo_id / repo are replace-style (nullable): the client always sends
        // the full group, so `None` means "cleared", not "unchanged".
        .bind(input.repo_id)
        .bind(input.name.unwrap_or(existing.name))
        .bind(input.repo)
        .bind(input.position.unwrap_or(existing.position))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete_group(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM war_room_groups WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }

    // ---- Items ----

    async fn list_items(&self, war_room_id: i64) -> Result<Vec<WarRoomItem>, AppError> {
        Ok(sqlx::query_as::<_, WarRoomItem>(
            "SELECT i.* FROM war_room_items i
             JOIN war_room_groups g ON g.id = i.group_id
             WHERE g.war_room_id = ?
             ORDER BY g.position, i.position, i.id",
        )
        .bind(war_room_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn create_item(&self, group_id: i64, input: CreateItemInput) -> Result<WarRoomItem, AppError> {
        let checklist = serde_json::to_string(&input.checklist.unwrap_or_default())?;
        Ok(sqlx::query_as::<_, WarRoomItem>(
            "INSERT INTO war_room_items (group_id, label, ref_type, ref_number, note, stage, checklist, depends_on, source_item_id, position)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM war_room_items WHERE group_id = ?))
             RETURNING *",
        )
        .bind(group_id)
        .bind(&input.label)
        .bind(&input.ref_type)
        .bind(input.ref_number)
        .bind(input.note.unwrap_or_default())
        .bind(input.stage.unwrap_or_else(|| "todo".to_string()))
        .bind(&checklist)
        .bind(input.depends_on)
        .bind(input.source_item_id)
        .bind(group_id)
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_item(&self, id: i64, input: UpdateItemInput) -> Result<Option<WarRoomItem>, AppError> {
        let existing = match self.find_item(id).await? {
            Some(i) => i,
            None => return Ok(None),
        };
        let checklist = match input.checklist {
            Some(c) => serde_json::to_string(&c)?,
            None => existing.checklist,
        };
        Ok(sqlx::query_as::<_, WarRoomItem>(
            "UPDATE war_room_items SET
                label = ?, ref_type = ?, ref_number = ?, note = ?, stage = ?,
                checklist = ?, depends_on = ?, position = ?, updated_at = datetime('now')
             WHERE id = ? RETURNING *",
        )
        .bind(input.label.unwrap_or(existing.label))
        // ref_type / ref_number / depends_on are replace-style (nullable): the
        // client always sends the full item, so `None` means "cleared".
        .bind(input.ref_type)
        .bind(input.ref_number)
        .bind(input.note.unwrap_or(existing.note))
        .bind(input.stage.unwrap_or(existing.stage))
        .bind(&checklist)
        .bind(input.depends_on)
        .bind(input.position.unwrap_or(existing.position))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete_item(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM war_room_items WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }
}
