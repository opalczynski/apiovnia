//! Master-password / encrypted-environment IPC + helpers.
//!
//! This module owns the cross-cutting "is this env locked? do I need to
//! encrypt/decrypt?" logic. The storage repos stay crypto-agnostic; here we
//! sit between Tauri and storage, calling into `apiovnia-crypto` whenever a
//! value crosses the encryption boundary.
//!
//! Public IPC commands:
//!   - `enable_env_encryption(env_id, password)` — seal a plaintext env
//!   - `disable_env_encryption(env_id, password)` — un-seal back to plaintext
//!   - `unlock_env(env_id, password)` — load the session key for this session
//!   - `lock_env(env_id)` — drop the session key
//!   - `is_env_unlocked(env_id)` — quick poll for the frontend
//!   - `list_unlocked_envs()` — bulk version, used on startup
//!
//! Helpers used by other command modules:
//!   - [`require_key`] — look up the session key for an env or return `EnvLocked`
//!   - [`encrypt_value_for_env`] / [`decrypt_value_for_env`] — env-var values
//!   - [`encrypt_override_cols`] / [`decrypt_override_cols`] — override fields

use apiovnia_core::ids::EnvironmentId;
use apiovnia_crypto::{
    derive_key, make_password_check, open_str, random_salt, score_password as score_password_impl,
    seal_str, validate_policy, verify_password_check, CryptoError, EnvKey, PasswordStrength,
};
use apiovnia_storage::{
    EncryptionMeta, EnvVariableRepo, EnvironmentRepo, OverrideRepo, RawOverrideCols, Result,
    StorageError,
};
use tauri::State;

use crate::app_state::AppState;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Map a crypto failure into the storage error vocabulary so it travels
/// through IPC the same way every other storage error does. We don't
/// distinguish wrong-password from tampered-ciphertext — the AEAD layer
/// already can't tell those apart, and the user-facing message is the same.
#[allow(clippy::needless_pass_by_value)] // signature shaped for `Result::map_err`
fn crypto_err(e: CryptoError) -> StorageError {
    StorageError::Crypto(e.to_string())
}

/// Return a fresh copy of the session key for `env_id`, or `EnvLocked` if
/// the env isn't unlocked. The copy lives in its own buffer, zeroed on drop;
/// the canonical copy stays inside the session store.
fn require_key(state: &AppState, env_id: &EnvironmentId) -> Result<EnvKey> {
    state
        .session_keys
        .with_key(env_id, |k| EnvKey::from_bytes(*k.as_bytes()))
        .ok_or_else(|| StorageError::EnvLocked(env_id.as_str().to_string()))
}

/// Encrypt one plaintext env-variable value for the given env's session key.
pub fn encrypt_value_for_env(
    state: &AppState,
    env_id: &EnvironmentId,
    plaintext: &str,
) -> Result<String> {
    let key = require_key(state, env_id)?;
    seal_str(plaintext, &key).map_err(crypto_err)
}

/// Inverse of [`encrypt_value_for_env`] for a single value.
pub fn decrypt_value_for_env(
    state: &AppState,
    env_id: &EnvironmentId,
    ciphertext_b64: &str,
) -> Result<String> {
    let key = require_key(state, env_id)?;
    open_str(ciphertext_b64, &key).map_err(crypto_err)
}

/// Encrypt every non-NULL field of an override row in place. Method / `body_type`
/// are intentionally NOT encrypted: they are low-entropy enum strings whose
/// presence already leaks from header counts, and storing them ciphered would
/// break our debug-friendly `sqlite3` workflow without any real benefit.
/// Same for URL — keep that plain so `sqlite3` browsing remains useful for
/// non-secret bookkeeping. Secret material lives in `headers_json`,
/// `params_json`, `body_content`, `auth_json`.
pub fn encrypt_override_cols(
    state: &AppState,
    env_id: &EnvironmentId,
    cols: &RawOverrideCols,
) -> Result<RawOverrideCols> {
    let key = require_key(state, env_id)?;
    Ok(RawOverrideCols {
        method: cols.method.clone(),
        url: cols.url.as_deref().map(|s| seal_str(s, &key)).transpose().map_err(crypto_err)?,
        headers_json: cols
            .headers_json
            .as_deref()
            .map(|s| seal_str(s, &key))
            .transpose()
            .map_err(crypto_err)?,
        params_json: cols
            .params_json
            .as_deref()
            .map(|s| seal_str(s, &key))
            .transpose()
            .map_err(crypto_err)?,
        body_type: cols.body_type.clone(),
        body_content: cols
            .body_content
            .as_deref()
            .map(|s| seal_str(s, &key))
            .transpose()
            .map_err(crypto_err)?,
        auth_json: cols
            .auth_json
            .as_deref()
            .map(|s| seal_str(s, &key))
            .transpose()
            .map_err(crypto_err)?,
    })
}

/// Inverse of [`encrypt_override_cols`].
pub fn decrypt_override_cols(
    state: &AppState,
    env_id: &EnvironmentId,
    cols: &RawOverrideCols,
) -> Result<RawOverrideCols> {
    let key = require_key(state, env_id)?;
    Ok(RawOverrideCols {
        method: cols.method.clone(),
        url: cols.url.as_deref().map(|s| open_str(s, &key)).transpose().map_err(crypto_err)?,
        headers_json: cols
            .headers_json
            .as_deref()
            .map(|s| open_str(s, &key))
            .transpose()
            .map_err(crypto_err)?,
        params_json: cols
            .params_json
            .as_deref()
            .map(|s| open_str(s, &key))
            .transpose()
            .map_err(crypto_err)?,
        body_type: cols.body_type.clone(),
        body_content: cols
            .body_content
            .as_deref()
            .map(|s| open_str(s, &key))
            .transpose()
            .map_err(crypto_err)?,
        auth_json: cols
            .auth_json
            .as_deref()
            .map(|s| open_str(s, &key))
            .transpose()
            .map_err(crypto_err)?,
    })
}

// ---------------------------------------------------------------------------
// IPC commands
// ---------------------------------------------------------------------------

/// Seal an env with a master password. Generates a fresh salt, derives the
/// key, encrypts every existing variable value + override field, writes the
/// `password_check` blob, and loads the key into the session store so the
/// caller can immediately use the env.
///
/// Fails if the env is already encrypted (`Conflict`) or if `password` is
/// empty (`InvalidData`).
///
/// `bypass_policy = true` skips the zxcvbn score + length floor — the
/// "pro user" escape hatch. The non-empty check still applies (the KDF
/// needs something to derive from, and an empty master password is a
/// footgun far past even pro-user territory).
#[tauri::command]
pub async fn enable_env_encryption(
    state: State<'_, AppState>,
    env_id: EnvironmentId,
    password: String,
    bypass_policy: bool,
) -> Result<()> {
    if password.is_empty() {
        return Err(StorageError::InvalidData("password is empty".into()));
    }
    if !bypass_policy {
        // Defense in depth: frontend already gates on meets_policy, but the
        // backend re-validates so a custom IPC caller can't bypass the floor.
        validate_policy(&password).map_err(|e| StorageError::InvalidData(e.user_message()))?;
    }

    let env = EnvironmentRepo::get(state.db.pool(), &env_id).await?;
    if env.is_encrypted {
        return Err(StorageError::Conflict(
            "environment is already encrypted".into(),
        ));
    }

    let salt = random_salt();
    let key = derive_key(&password, &salt).map_err(crypto_err)?;
    let password_check = make_password_check(&key).map_err(crypto_err)?;

    // Snapshot everything we need to migrate before opening the tx so we
    // can fail fast on encrypt errors without holding a write lock.
    let pool = state.db.pool();
    let plaintext_vars = EnvVariableRepo::list_for_env(pool, &env_id).await?;
    let plaintext_overrides = OverrideRepo::list_raw_for_env(pool, &env_id).await?;

    let encrypted_vars: Vec<(String, String)> = plaintext_vars
        .iter()
        .map(|v| seal_str(&v.value, &key).map(|ct| (v.name.clone(), ct)))
        .collect::<std::result::Result<_, _>>()
        .map_err(crypto_err)?;

    // Encrypt overrides with the freshly-derived key (which isn't in the
    // session store yet — we use it directly to avoid a chicken-and-egg
    // unlock dance).
    let encrypted_overrides: Vec<(_, RawOverrideCols)> = plaintext_overrides
        .iter()
        .map(|row| {
            let cols = &row.cols;
            let out = RawOverrideCols {
                method: cols.method.clone(),
                url: opt_seal(cols.url.as_deref(), &key)?,
                headers_json: opt_seal(cols.headers_json.as_deref(), &key)?,
                params_json: opt_seal(cols.params_json.as_deref(), &key)?,
                body_type: cols.body_type.clone(),
                body_content: opt_seal(cols.body_content.as_deref(), &key)?,
                auth_json: opt_seal(cols.auth_json.as_deref(), &key)?,
            };
            Ok::<_, CryptoError>((row.request_id.clone(), out))
        })
        .collect::<std::result::Result<_, _>>()
        .map_err(crypto_err)?;

    // One transaction for all four mutations — the env flip, the bulk
    // variable rewrite, and the per-row override rewrites. Splitting these
    // across `pool` calls deadlocks on SQLite's single-writer lock.
    let mut tx = pool.begin().await?;
    EnvironmentRepo::set_encryption_in_tx(
        &mut tx,
        &env_id,
        &EncryptionMeta {
            salt: salt.to_vec(),
            password_check,
        },
    )
    .await?;
    EnvVariableRepo::rewrite_values_in_tx(&mut tx, &env_id, &encrypted_vars).await?;
    for (req_id, cols) in &encrypted_overrides {
        OverrideRepo::rewrite_row_in_tx(&mut tx, &env_id, req_id, cols).await?;
    }
    tx.commit().await?;

    // Load the session key so the user can keep working without re-typing.
    state.session_keys.unlock(env_id, key);
    Ok(())
}

/// Inverse of [`enable_env_encryption`]. Verifies the password against
/// `password_check`, decrypts every variable + override row, clears the
/// crypto metadata, and drops the session key.
#[tauri::command]
pub async fn disable_env_encryption(
    state: State<'_, AppState>,
    env_id: EnvironmentId,
    password: String,
) -> Result<()> {
    let env = EnvironmentRepo::get(state.db.pool(), &env_id).await?;
    if !env.is_encrypted {
        return Err(StorageError::Conflict(
            "environment is already plaintext".into(),
        ));
    }
    let meta = EnvironmentRepo::encryption_meta(state.db.pool(), &env_id)
        .await?
        .ok_or_else(|| StorageError::InvalidData("encrypted env is missing metadata".into()))?;

    let key = derive_key(&password, &meta.salt).map_err(crypto_err)?;
    if !verify_password_check(&meta.password_check, &key) {
        return Err(StorageError::Crypto("wrong password".into()));
    }

    let pool = state.db.pool();
    let encrypted_vars = EnvVariableRepo::list_for_env(pool, &env_id).await?;
    let encrypted_overrides = OverrideRepo::list_raw_for_env(pool, &env_id).await?;

    let plaintext_vars: Vec<(String, String)> = encrypted_vars
        .iter()
        .map(|v| open_str(&v.value, &key).map(|pt| (v.name.clone(), pt)))
        .collect::<std::result::Result<_, _>>()
        .map_err(crypto_err)?;

    let plaintext_overrides: Vec<(_, RawOverrideCols)> = encrypted_overrides
        .iter()
        .map(|row| {
            let cols = &row.cols;
            let out = RawOverrideCols {
                method: cols.method.clone(),
                url: opt_open(cols.url.as_deref(), &key)?,
                headers_json: opt_open(cols.headers_json.as_deref(), &key)?,
                params_json: opt_open(cols.params_json.as_deref(), &key)?,
                body_type: cols.body_type.clone(),
                body_content: opt_open(cols.body_content.as_deref(), &key)?,
                auth_json: opt_open(cols.auth_json.as_deref(), &key)?,
            };
            Ok::<_, CryptoError>((row.request_id.clone(), out))
        })
        .collect::<std::result::Result<_, _>>()
        .map_err(crypto_err)?;

    // Same atomicity argument as `enable_env_encryption` — all four mutations
    // share one transaction so we don't trip SQLite's writer lock.
    let mut tx = pool.begin().await?;
    EnvVariableRepo::rewrite_values_in_tx(&mut tx, &env_id, &plaintext_vars).await?;
    for (req_id, cols) in &plaintext_overrides {
        OverrideRepo::rewrite_row_in_tx(&mut tx, &env_id, req_id, cols).await?;
    }
    EnvironmentRepo::clear_encryption_in_tx(&mut tx, &env_id).await?;
    tx.commit().await?;

    state.session_keys.lock(&env_id);
    Ok(())
}

/// Derive the session key from the supplied password and load it into the
/// session store. Returns `Crypto("wrong password")` on failure.
#[tauri::command]
pub async fn unlock_env(
    state: State<'_, AppState>,
    env_id: EnvironmentId,
    password: String,
) -> Result<()> {
    let env = EnvironmentRepo::get(state.db.pool(), &env_id).await?;
    if !env.is_encrypted {
        return Err(StorageError::Conflict(
            "environment is not encrypted".into(),
        ));
    }
    let meta = EnvironmentRepo::encryption_meta(state.db.pool(), &env_id)
        .await?
        .ok_or_else(|| StorageError::InvalidData("encrypted env is missing metadata".into()))?;

    let key = derive_key(&password, &meta.salt).map_err(crypto_err)?;
    if !verify_password_check(&meta.password_check, &key) {
        return Err(StorageError::Crypto("wrong password".into()));
    }
    state.session_keys.unlock(env_id, key);
    Ok(())
}

/// Drop the session key for an env. No-op when it wasn't loaded.
#[tauri::command]
pub async fn lock_env(state: State<'_, AppState>, env_id: EnvironmentId) -> Result<()> {
    state.session_keys.lock(&env_id);
    Ok(())
}

#[tauri::command]
pub async fn is_env_unlocked(
    state: State<'_, AppState>,
    env_id: EnvironmentId,
) -> Result<bool> {
    Ok(state.session_keys.is_unlocked(&env_id))
}

#[tauri::command]
pub async fn list_unlocked_envs(state: State<'_, AppState>) -> Result<Vec<EnvironmentId>> {
    Ok(state.session_keys.unlocked_ids())
}

/// Live password-strength snapshot for the `SetEnvPasswordModal` meter.
///
/// Cheap to call on every keystroke (zxcvbn is microseconds for short
/// inputs) — the frontend still debounces ~120 ms to avoid IPC churn.
/// Stateless — no password is stored or logged.
#[tauri::command]
pub async fn score_password(password: String) -> Result<PasswordStrength> {
    Ok(score_password_impl(&password))
}

// ---------------------------------------------------------------------------
// Tiny internal helpers
// ---------------------------------------------------------------------------

fn opt_seal(s: Option<&str>, key: &EnvKey) -> std::result::Result<Option<String>, CryptoError> {
    s.map(|t| seal_str(t, key)).transpose()
}

fn opt_open(s: Option<&str>, key: &EnvKey) -> std::result::Result<Option<String>, CryptoError> {
    s.map(|t| open_str(t, key)).transpose()
}
