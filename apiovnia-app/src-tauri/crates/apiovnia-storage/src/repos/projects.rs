//! Project CRUD.

use apiovnia_core::{ids::ProjectId, model::Project, time::epoch_millis_now};
use sqlx::SqlitePool;

use crate::error::{Result, StorageError};

pub struct ProjectRepo;

impl ProjectRepo {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Project>> {
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, name, created_at, updated_at, sort_order \
             FROM projects ORDER BY sort_order ASC, created_at ASC",
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Row::into_domain).collect())
    }

    pub async fn get(pool: &SqlitePool, id: &ProjectId) -> Result<Project> {
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, name, created_at, updated_at, sort_order \
             FROM projects WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        Ok(row.into_domain())
    }

    pub async fn create(pool: &SqlitePool, name: &str) -> Result<Project> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StorageError::InvalidData("project name is empty".into()));
        }
        let id = ProjectId::new();
        let now = epoch_millis_now();
        // Sort_order defaults to now (millis) so newest goes to the bottom by
        // default; users reorder later via drag.
        sqlx::query(
            "INSERT INTO projects (id, name, created_at, updated_at, sort_order) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.as_str())
        .bind(name)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        Self::get(pool, &id).await
    }

    pub async fn rename(pool: &SqlitePool, id: &ProjectId, name: &str) -> Result<Project> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StorageError::InvalidData("project name is empty".into()));
        }
        let now = epoch_millis_now();
        let res = sqlx::query("UPDATE projects SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(now)
            .bind(id.as_str())
            .execute(pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Self::get(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &ProjectId) -> Result<()> {
        let res = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id.as_str())
            .execute(pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}

// Internal row shape — kept private so the public API stays in domain types.
#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    name: String,
    created_at: i64,
    updated_at: i64,
    sort_order: i64,
}

impl Row {
    fn into_domain(self) -> Project {
        Project {
            id: ProjectId::from_trusted(self.id),
            name: self.name,
            created_at: self.created_at,
            updated_at: self.updated_at,
            sort_order: self.sort_order,
        }
    }
}
