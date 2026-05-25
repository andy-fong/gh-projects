use async_trait::async_trait;
use sqlx::SqlitePool;
use crate::{error::AppError, models::note::{CreateNoteInput, Note, NoteFilter, UpdateNoteInput}};

#[async_trait]
pub trait NoteRepository: Send + Sync {
    async fn create(&self, input: CreateNoteInput) -> Result<Note, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Note>, AppError>;
    async fn list(&self, filter: NoteFilter) -> Result<Vec<Note>, AppError>;
    async fn update(&self, id: i64, input: UpdateNoteInput) -> Result<Option<Note>, AppError>;
    async fn delete(&self, id: i64) -> Result<bool, AppError>;
}

pub struct SqliteNoteRepository {
    pool: SqlitePool,
}

impl SqliteNoteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl NoteRepository for SqliteNoteRepository {
    async fn create(&self, input: CreateNoteInput) -> Result<Note, AppError> {
        let tags = serde_json::to_string(&input.tags.unwrap_or_default())?;
        let note = sqlx::query_as::<_, Note>(
            "INSERT INTO notes (title, body, repo, ref_type, ref_number, tags)
             VALUES (?, ?, ?, ?, ?, ?)
             RETURNING *"
        )
        .bind(&input.title)
        .bind(input.body.unwrap_or_default())
        .bind(&input.repo)
        .bind(&input.ref_type)
        .bind(input.ref_number)
        .bind(&tags)
        .fetch_one(&self.pool)
        .await?;
        Ok(note)
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Note>, AppError> {
        let note = sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(note)
    }

    async fn list(&self, filter: NoteFilter) -> Result<Vec<Note>, AppError> {
        // Build query dynamically based on filter
        let mut query = "SELECT * FROM notes WHERE 1=1".to_string();
        let mut conditions_repo: Option<String> = None;
        let mut conditions_ref_type: Option<String> = None;
        let mut conditions_ref_number: Option<i64> = None;

        if filter.repo.is_some() {
            query.push_str(" AND repo = ?");
            conditions_repo = filter.repo;
        }
        if filter.ref_type.is_some() {
            query.push_str(" AND ref_type = ?");
            conditions_ref_type = filter.ref_type;
        }
        if filter.ref_number.is_some() {
            query.push_str(" AND ref_number = ?");
            conditions_ref_number = filter.ref_number;
        }
        query.push_str(" ORDER BY updated_at DESC");

        let mut q = sqlx::query_as::<_, Note>(&query);
        if let Some(r) = conditions_repo { q = q.bind(r); }
        if let Some(rt) = conditions_ref_type { q = q.bind(rt); }
        if let Some(rn) = conditions_ref_number { q = q.bind(rn); }

        Ok(q.fetch_all(&self.pool).await?)
    }

    async fn update(&self, id: i64, input: UpdateNoteInput) -> Result<Option<Note>, AppError> {
        let existing = match self.find_by_id(id).await? {
            Some(n) => n,
            None => return Ok(None),
        };
        let tags = match input.tags {
            Some(t) => serde_json::to_string(&t)?,
            None => existing.tags.clone(),
        };
        let note = sqlx::query_as::<_, Note>(
            "UPDATE notes SET
                title = ?,
                body = ?,
                repo = ?,
                ref_type = ?,
                ref_number = ?,
                tags = ?,
                updated_at = datetime('now')
             WHERE id = ?
             RETURNING *"
        )
        .bind(input.title.unwrap_or(existing.title))
        .bind(input.body.unwrap_or(existing.body))
        .bind(input.repo.or(existing.repo))
        .bind(input.ref_type.or(existing.ref_type))
        .bind(input.ref_number.or(existing.ref_number))
        .bind(&tags)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(note)
    }

    async fn delete(&self, id: i64) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM notes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
