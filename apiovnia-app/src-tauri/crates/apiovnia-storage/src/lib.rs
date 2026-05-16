//! Apiovnia `SQLite` persistence layer.
//!
//! Owns the connection pool, runs migrations on startup, and exposes one
//! repository struct per entity. Repositories take `&SqlitePool` and return
//! domain models from `apiovnia-core`.

pub mod db;
pub mod error;
pub mod repos;

pub use db::Db;
pub use error::{Result, StorageError};
pub use repos::{
    collections::CollectionRepo, env_variables::EnvVariableRepo, environments::EnvironmentRepo,
    history::HistoryRepo, overrides::OverrideRepo, projects::ProjectRepo, requests::RequestRepo,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
