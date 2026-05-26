//! Application state shared across IPC commands.
//!
//! Long-lived resources — the `SQLite` handle, the HTTP executor, and the
//! Phase 6 session-key store for encrypted envs.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use apiovnia_core::ids::EnvironmentId;
use apiovnia_crypto::EnvKey;
use apiovnia_http::Executor;
use apiovnia_storage::Db;

/// Idle window after which an unlocked env auto-locks (lazy — evaluated on
/// the next access). Industry baseline for password managers: 10–15 min.
/// We pick 10 because each request is cheap to unlock again and the cost
/// of "left for lunch" leaking secrets to anyone with desktop access is
/// asymmetric.
pub const IDLE_AUTO_LOCK: Duration = Duration::from_mins(10);

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub executor: Arc<Executor>,
    pub session_keys: Arc<SessionKeyStore>,
}

impl AppState {
    #[must_use]
    pub fn new(db: Db, executor: Executor) -> Self {
        Self {
            db: Arc::new(db),
            executor: Arc::new(executor),
            session_keys: Arc::new(SessionKeyStore::default()),
        }
    }
}

/// Holds the AES-256-GCM keys for every currently-unlocked environment,
/// each tagged with `last_used` so we can lazily evict idle entries
/// (see [`IDLE_AUTO_LOCK`]).
///
/// Keys are dropped (and zeroized) when:
///   - `lock` is called explicitly,
///   - the env is deleted,
///   - an access happens more than `IDLE_AUTO_LOCK` after the last use,
///   - the app quits and the whole store goes out of scope.
///
/// The store is intentionally *not* persisted — restarting the app forces
/// the user to re-enter the master password.
#[derive(Default)]
pub struct SessionKeyStore {
    inner: RwLock<HashMap<EnvironmentId, StoredKey>>,
}

/// One unlocked env entry. `last_used` is the wall-clock instant of the
/// most recent operation that actually used the key (encrypt/decrypt) —
/// peeks via `is_unlocked` don't refresh it, so leaving the modal open
/// doesn't extend the session forever.
struct StoredKey {
    key: EnvKey,
    last_used: Instant,
}

impl SessionKeyStore {
    pub fn unlock(&self, id: EnvironmentId, key: EnvKey) {
        self.inner.write().expect("session_keys poisoned").insert(
            id,
            StoredKey {
                key,
                last_used: Instant::now(),
            },
        );
    }

    pub fn lock(&self, id: &EnvironmentId) {
        self.inner.write().expect("session_keys poisoned").remove(id);
    }

    /// Peek — true iff the key is still loaded AND hasn't gone stale. Stale
    /// entries are evicted as a side effect so the answer is always
    /// consistent with what the next `with_key` would do.
    #[must_use]
    pub fn is_unlocked(&self, id: &EnvironmentId) -> bool {
        let mut g = self.inner.write().expect("session_keys poisoned");
        match g.get(id) {
            Some(entry) if entry.last_used.elapsed() <= IDLE_AUTO_LOCK => true,
            Some(_) => {
                g.remove(id); // expired — drop runs zeroize
                false
            }
            None => false,
        }
    }

    /// Run `f` against the env's key. Returns `None` (mapped to
    /// `StorageError::EnvLocked` by the caller) when the env was never
    /// unlocked OR when its idle window has elapsed. Successful access
    /// refreshes `last_used`.
    pub fn with_key<R>(&self, id: &EnvironmentId, f: impl FnOnce(&EnvKey) -> R) -> Option<R> {
        let mut g = self.inner.write().expect("session_keys poisoned");
        let entry = g.get_mut(id)?;
        if entry.last_used.elapsed() > IDLE_AUTO_LOCK {
            g.remove(id); // expired — auto-lock
            return None;
        }
        entry.last_used = Instant::now();
        Some(f(&entry.key))
    }

    /// Snapshot of every env id currently unlocked. Used by the frontend on
    /// rehydrate to stay in sync; in practice this starts empty on each
    /// app launch. Evicts any stale entries on the way out so the returned
    /// list matches what `is_unlocked` would say one-by-one.
    #[must_use]
    pub fn unlocked_ids(&self) -> Vec<EnvironmentId> {
        let mut g = self.inner.write().expect("session_keys poisoned");
        g.retain(|_, entry| entry.last_used.elapsed() <= IDLE_AUTO_LOCK);
        g.keys().cloned().collect()
    }
}
