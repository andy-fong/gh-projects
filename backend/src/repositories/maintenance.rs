use async_trait::async_trait;
use sqlx::SqlitePool;

use crate::error::AppError;

/// Database-level housekeeping that isn't tied to one entity.
#[async_trait]
pub trait MaintenanceRepository: Send + Sync {
    /// Write a consistent copy of the whole database to `path`.
    ///
    /// Restore is destructive and not transactional, so it takes one of these
    /// first: if a later step fails, the pre-restore state is still on disk.
    async fn snapshot_to(&self, path: &str) -> Result<(), AppError>;
}

pub struct SqliteMaintenanceRepository {
    pool: SqlitePool,
}

impl SqliteMaintenanceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MaintenanceRepository for SqliteMaintenanceRepository {
    async fn snapshot_to(&self, path: &str) -> Result<(), AppError> {
        // `VACUUM INTO` fails if the target exists, which is the behaviour we
        // want — a snapshot is never silently overwritten.
        sqlx::query("VACUUM INTO ?")
            .bind(path)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
