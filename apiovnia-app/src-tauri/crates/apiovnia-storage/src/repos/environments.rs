//! Environment CRUD — `environments` table.
//!
//! Phase 5: plain rows (no encryption). Phase 6 will fill `salt` /
//! `password_check` when the user seals an env behind a master password.

use apiovnia_core::{
    ids::{EnvironmentId, ProjectId},
    model::Environment,
    time::epoch_millis_now,
};
use sqlx::SqlitePool;

use crate::error::{Result, StorageError};

pub struct EnvironmentRepo;

impl EnvironmentRepo {
    pub async fn list_for_project(
        pool: &SqlitePool,
        project_id: &ProjectId,
    ) -> Result<Vec<Environment>> {
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, project_id, name, requires_unlock, is_encrypted, created_at \
             FROM environments WHERE project_id = ? ORDER BY created_at ASC",
        )
        .bind(project_id.as_str())
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(Row::into_domain).collect())
    }

    pub async fn get(pool: &SqlitePool, id: &EnvironmentId) -> Result<Environment> {
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, project_id, name, requires_unlock, is_encrypted, created_at \
             FROM environments WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(pool)
        .await?
        .ok_or(StorageError::NotFound)?;
        Ok(row.into_domain())
    }

    pub async fn create(
        pool: &SqlitePool,
        project_id: &ProjectId,
        name: &str,
    ) -> Result<Environment> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StorageError::InvalidData("environment name is empty".into()));
        }
        let id = EnvironmentId::new();
        let now = epoch_millis_now();
        sqlx::query(
            "INSERT INTO environments (id, project_id, name, requires_unlock, is_encrypted, \
                                       created_at) VALUES (?, ?, ?, 0, 0, ?)",
        )
        .bind(id.as_str())
        .bind(project_id.as_str())
        .bind(name)
        .bind(now)
        .execute(pool)
        .await
        .map_err(map_unique_conflict)?;
        Self::get(pool, &id).await
    }

    pub async fn rename(
        pool: &SqlitePool,
        id: &EnvironmentId,
        name: &str,
    ) -> Result<Environment> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StorageError::InvalidData("environment name is empty".into()));
        }
        let res = sqlx::query("UPDATE environments SET name = ? WHERE id = ?")
            .bind(name)
            .bind(id.as_str())
            .execute(pool)
            .await
            .map_err(map_unique_conflict)?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Self::get(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &EnvironmentId) -> Result<()> {
        let res = sqlx::query("DELETE FROM environments WHERE id = ?")
            .bind(id.as_str())
            .execute(pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}

fn map_unique_conflict(e: sqlx::Error) -> StorageError {
    if let sqlx::Error::Database(db) = &e {
        if db.message().contains("UNIQUE") {
            return StorageError::Conflict("environment name already exists in project".into());
        }
    }
    e.into()
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    project_id: String,
    name: String,
    requires_unlock: i64,
    is_encrypted: i64,
    created_at: i64,
}

impl Row {
    fn into_domain(self) -> Environment {
        Environment {
            id: EnvironmentId::from_trusted(self.id),
            project_id: ProjectId::from_trusted(self.project_id),
            name: self.name,
            requires_unlock: self.requires_unlock != 0,
            is_encrypted: self.is_encrypted != 0,
            created_at: self.created_at,
        }
    }
}
