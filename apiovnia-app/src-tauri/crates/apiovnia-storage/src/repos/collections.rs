//! Collection CRUD.

use apiovnia_core::{
    ids::{CollectionId, ProjectId},
    model::Collection,
    time::epoch_millis_now,
};
use sqlx::SqlitePool;

use crate::error::{Result, StorageError};

pub struct CollectionRepo;

impl CollectionRepo {
    pub async fn list_in_project(
        pool: &SqlitePool,
        project_id: &ProjectId,
    ) -> Result<Vec<Collection>> {
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, project_id, name, created_at, updated_at, sort_order \
             FROM collections WHERE project_id = ? \
             ORDER BY sort_order ASC, created_at ASC",
        )
        .bind(project_id.as_str())
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Row::into_domain).collect())
    }

    pub async fn get(pool: &SqlitePool, id: &CollectionId) -> Result<Collection> {
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, project_id, name, created_at, updated_at, sort_order \
             FROM collections WHERE id = ?",
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
    ) -> Result<Collection> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StorageError::InvalidData("collection name is empty".into()));
        }
        let id = CollectionId::new();
        let now = epoch_millis_now();
        sqlx::query(
            "INSERT INTO collections (id, project_id, name, sort_order, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_str())
        .bind(project_id.as_str())
        .bind(name)
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        Self::get(pool, &id).await
    }

    pub async fn rename(
        pool: &SqlitePool,
        id: &CollectionId,
        name: &str,
    ) -> Result<Collection> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StorageError::InvalidData("collection name is empty".into()));
        }
        let now = epoch_millis_now();
        let res = sqlx::query("UPDATE collections SET name = ?, updated_at = ? WHERE id = ?")
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

    pub async fn delete(pool: &SqlitePool, id: &CollectionId) -> Result<()> {
        let res = sqlx::query("DELETE FROM collections WHERE id = ?")
            .bind(id.as_str())
            .execute(pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    project_id: String,
    name: String,
    created_at: i64,
    updated_at: i64,
    sort_order: i64,
}

impl Row {
    fn into_domain(self) -> Collection {
        Collection {
            id: CollectionId::from_trusted(self.id),
            project_id: ProjectId::from_trusted(self.project_id),
            name: self.name,
            created_at: self.created_at,
            updated_at: self.updated_at,
            sort_order: self.sort_order,
        }
    }
}
