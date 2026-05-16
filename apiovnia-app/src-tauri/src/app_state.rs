//! Application state shared across IPC commands.
//!
//! Long-lived resources — the `SQLite` handle and the HTTP executor — live
//! here. Phase 6 adds the encryption session-key store.

use std::sync::Arc;

use apiovnia_http::Executor;
use apiovnia_storage::Db;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub executor: Arc<Executor>,
}

impl AppState {
    #[must_use]
    pub fn new(db: Db, executor: Executor) -> Self {
        Self {
            db: Arc::new(db),
            executor: Arc::new(executor),
        }
    }
}
