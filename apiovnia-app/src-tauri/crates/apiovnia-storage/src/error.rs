//! Storage-layer errors.

use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("entity not found")]
    NotFound,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("invalid data: {0}")]
    InvalidData(String),

    /// The operation touched an encrypted env whose session key isn't loaded.
    /// The id is the env that needs unlocking — the frontend pattern-matches
    /// on this so it can pop the right unlock modal.
    #[error("ENV_LOCKED:{0}")]
    EnvLocked(String),

    /// A crypto operation failed (wrong password, tampered data, etc).
    #[error("crypto: {0}")]
    Crypto(String),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("io: {0}")]
    Io(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// Storage errors travel out through Tauri commands as JSON. We flatten to a
/// string so the frontend gets a clean `Error("…")` payload — full backtraces
/// stay in the logs.
impl Serialize for StorageError {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
