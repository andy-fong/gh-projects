use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::{
    error::AppError,
    models::team_member::{
        CreateTeamMemberInput, CreateTeamMemberSourceInput, TeamMember, TeamMemberSource,
        UpdateTeamMemberInput,
    },
};

#[async_trait]
pub trait TeamMemberRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<TeamMember>, AppError>;
    async fn create(&self, input: CreateTeamMemberInput) -> Result<TeamMember, AppError>;
    async fn update(
        &self,
        id: i64,
        input: UpdateTeamMemberInput,
    ) -> Result<Option<TeamMember>, AppError>;
    async fn delete(&self, id: i64) -> Result<bool, AppError>;
    async fn delete_all(&self) -> Result<(), AppError>;
    /// Insert the logins that aren't on the roster yet, leaving existing rows
    /// (and therefore their group) untouched. Returns `(added, skipped)`.
    async fn insert_missing(
        &self,
        logins: &[String],
        group: &str,
        source: &str,
    ) -> Result<(usize, usize), AppError>;

    async fn list_sources(&self) -> Result<Vec<TeamMemberSource>, AppError>;
    async fn create_source(
        &self,
        input: CreateTeamMemberSourceInput,
    ) -> Result<TeamMemberSource, AppError>;
    async fn delete_source(&self, id: i64) -> Result<bool, AppError>;
    async fn delete_all_sources(&self) -> Result<(), AppError>;
}

pub struct SqliteTeamMemberRepository {
    pool: SqlitePool,
}

impl SqliteTeamMemberRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<TeamMember>, AppError> {
        Ok(
            sqlx::query_as::<_, TeamMember>("SELECT * FROM team_members WHERE id = ?")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }
}

#[async_trait]
impl TeamMemberRepository for SqliteTeamMemberRepository {
    async fn list(&self) -> Result<Vec<TeamMember>, AppError> {
        Ok(sqlx::query_as::<_, TeamMember>(
            "SELECT * FROM team_members ORDER BY member_group, login COLLATE NOCASE",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    async fn create(&self, input: CreateTeamMemberInput) -> Result<TeamMember, AppError> {
        Ok(sqlx::query_as::<_, TeamMember>(
            "INSERT INTO team_members (login, member_group, source, position)
             VALUES (?, ?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM team_members))
             RETURNING *",
        )
        .bind(input.login.trim())
        .bind(&input.member_group)
        .bind(input.source.as_deref().unwrap_or("manual"))
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update(
        &self,
        id: i64,
        input: UpdateTeamMemberInput,
    ) -> Result<Option<TeamMember>, AppError> {
        let existing = match self.find_by_id(id).await? {
            Some(m) => m,
            None => return Ok(None),
        };
        Ok(sqlx::query_as::<_, TeamMember>(
            "UPDATE team_members
             SET login = ?, member_group = ?, position = ?, updated_at = datetime('now')
             WHERE id = ? RETURNING *",
        )
        .bind(
            input
                .login
                .map(|l| l.trim().to_string())
                .unwrap_or(existing.login),
        )
        .bind(input.member_group.unwrap_or(existing.member_group))
        .bind(input.position.unwrap_or(existing.position))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn delete(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM team_members WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }

    async fn delete_all(&self) -> Result<(), AppError> {
        sqlx::query("DELETE FROM team_members")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn insert_missing(
        &self,
        logins: &[String],
        group: &str,
        source: &str,
    ) -> Result<(usize, usize), AppError> {
        let mut added = 0usize;
        let mut skipped = 0usize;
        for login in logins {
            let login = login.trim();
            if login.is_empty() {
                continue;
            }
            let rows = sqlx::query(
                "INSERT INTO team_members (login, member_group, source, position)
                 VALUES (?, ?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM team_members))
                 ON CONFLICT DO NOTHING",
            )
            .bind(login)
            .bind(group)
            .bind(source)
            .execute(&self.pool)
            .await?
            .rows_affected();
            if rows > 0 {
                added += 1;
            } else {
                skipped += 1;
            }
        }
        Ok((added, skipped))
    }

    async fn list_sources(&self) -> Result<Vec<TeamMemberSource>, AppError> {
        Ok(sqlx::query_as::<_, TeamMemberSource>(
            "SELECT * FROM team_member_sources ORDER BY position, org_team",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    async fn create_source(
        &self,
        input: CreateTeamMemberSourceInput,
    ) -> Result<TeamMemberSource, AppError> {
        Ok(sqlx::query_as::<_, TeamMemberSource>(
            "INSERT INTO team_member_sources (member_group, org_team, position)
             VALUES (?, ?, (SELECT COALESCE(MAX(position) + 1, 0) FROM team_member_sources))
             RETURNING *",
        )
        .bind(&input.member_group)
        .bind(input.org_team.trim().trim_matches('/'))
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete_source(&self, id: i64) -> Result<bool, AppError> {
        Ok(sqlx::query("DELETE FROM team_member_sources WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected()
            > 0)
    }

    async fn delete_all_sources(&self) -> Result<(), AppError> {
        sqlx::query("DELETE FROM team_member_sources")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
