//! Environment variable CRUD — `environment_variables` table.
//!
//! Values stored as plaintext in Phase 5; they become ciphertext (AES-256-GCM
//! with an Argon2id-derived key) once the parent env flips `is_encrypted` in
//! Phase 6. Until then `is_secret` is just a UI hint.

use apiovnia_core::{ids::EnvironmentId, model::EnvVariable};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{Result, StorageError};

pub struct EnvVariableRepo;

impl EnvVariableRepo {
    pub async fn list_for_env(
        pool: &SqlitePool,
        env_id: &EnvironmentId,
    ) -> Result<Vec<EnvVariable>> {
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, environment_id, name, value, is_secret \
             FROM environment_variables WHERE environment_id = ? ORDER BY name ASC",
        )
        .bind(env_id.as_str())
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(Row::into_domain).collect())
    }

    /// Insert or update by `(env_id, name)`. Returns the persisted row.
    pub async fn upsert(
        pool: &SqlitePool,
        env_id: &EnvironmentId,
        name: &str,
        value: &str,
        is_secret: bool,
    ) -> Result<EnvVariable> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StorageError::InvalidData("variable name is empty".into()));
        }
        // Generate an id we'll only use if this is an insert; SQLite ignores
        // it on conflict because `excluded.id` keeps the existing value.
        let new_id = format!("evar_{}", Uuid::new_v4().simple());
        sqlx::query(
            "INSERT INTO environment_variables (id, environment_id, name, value, is_secret) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(environment_id, name) DO UPDATE SET \
                value = excluded.value, is_secret = excluded.is_secret",
        )
        .bind(&new_id)
        .bind(env_id.as_str())
        .bind(name)
        .bind(value)
        .bind(i64::from(is_secret))
        .execute(pool)
        .await?;

        // Read back — `name` is the natural key inside an env.
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, environment_id, name, value, is_secret \
             FROM environment_variables WHERE environment_id = ? AND name = ?",
        )
        .bind(env_id.as_str())
        .bind(name)
        .fetch_one(pool)
        .await?;
        Ok(row.into_domain())
    }

    pub async fn delete(pool: &SqlitePool, env_id: &EnvironmentId, name: &str) -> Result<()> {
        let res = sqlx::query(
            "DELETE FROM environment_variables WHERE environment_id = ? AND name = ?",
        )
        .bind(env_id.as_str())
        .bind(name)
        .execute(pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }

    /// Tx-aware bulk update of every variable's `value` column for one env.
    /// Used by the enable/disable-encryption flows to flip the entire env in
    /// one go (encrypt all → SET; decrypt all → SET). Caller supplies the
    /// `(name, new_value)` pairs they want written.
    pub async fn rewrite_values_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        env_id: &EnvironmentId,
        new_values_by_name: &[(String, String)],
    ) -> Result<()> {
        for (name, value) in new_values_by_name {
            sqlx::query(
                "UPDATE environment_variables \
                 SET value = ? WHERE environment_id = ? AND name = ?",
            )
            .bind(value)
            .bind(env_id.as_str())
            .bind(name)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    environment_id: String,
    name: String,
    value: String,
    is_secret: i64,
}

impl Row {
    fn into_domain(self) -> EnvVariable {
        EnvVariable {
            id: self.id,
            environment_id: EnvironmentId::from_trusted(self.environment_id),
            name: self.name,
            value: self.value,
            is_secret: self.is_secret != 0,
        }
    }
}
