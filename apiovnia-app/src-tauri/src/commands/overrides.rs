//! Per-`(request, env)` override IPC commands.
//!
//! Plaintext envs go through the typed [`OverrideRepo::get`] / `upsert` /
//! `delete` path unchanged. Encrypted envs round-trip the secret-bearing
//! columns through `commands::crypto::{encrypt,decrypt}_override_cols`; the
//! `method`/`url`/`body_type` fields stay plaintext so `sqlite3` browsing remains
//! useful for non-secret bookkeeping.

use apiovnia_core::{
    ids::{EnvironmentId, RequestId},
    model::EnvOverride,
};
use apiovnia_storage::{
    EnvironmentRepo, OverrideRepo, RawOverrideCols, Result, StorageError,
};
use tauri::State;

use crate::{
    app_state::AppState,
    commands::crypto::{decrypt_override_cols, encrypt_override_cols},
};

#[tauri::command]
pub async fn get_override(
    state: State<'_, AppState>,
    request_id: RequestId,
    env_id: EnvironmentId,
) -> Result<Option<EnvOverride>> {
    let env = EnvironmentRepo::get(state.db.pool(), &env_id).await?;
    if env.is_encrypted {
        // EnvLocked propagates if the session key isn't loaded — frontend
        // will translate that into the unlock modal.
        let Some(raw) = OverrideRepo::get_raw(state.db.pool(), &request_id, &env_id).await?
        else {
            return Ok(None);
        };
        let decrypted = decrypt_override_cols(&state, &env_id, &raw.cols)?;
        return Ok(Some(decrypted.into_domain(request_id, env_id)?));
    }
    OverrideRepo::get(state.db.pool(), &request_id, &env_id).await
}

#[tauri::command]
pub async fn list_overrides_for_request(
    state: State<'_, AppState>,
    request_id: RequestId,
) -> Result<Vec<EnvOverride>> {
    // Used by debug / future "diff" views — kept on the plaintext path; an
    // encrypted env's row that nobody has unlocked simply won't decode. We
    // skip those silently rather than failing the whole list.
    OverrideRepo::list_for_request(state.db.pool(), &request_id).await
}

/// Upsert the override. If the patch is entirely empty (every field `None`),
/// we delete the row instead so storage stays tidy. Encrypted envs require
/// the session key; otherwise `EnvLocked` propagates.
#[tauri::command]
pub async fn upsert_override(
    state: State<'_, AppState>,
    patch: EnvOverride,
) -> Result<Option<EnvOverride>> {
    if patch.is_empty() {
        OverrideRepo::delete(state.db.pool(), &patch.request_id, &patch.environment_id).await?;
        return Ok(None);
    }

    let env = EnvironmentRepo::get(state.db.pool(), &patch.environment_id).await?;
    if env.is_encrypted {
        let raw = RawOverrideCols::from_domain(&patch)?;
        let encrypted = encrypt_override_cols(&state, &patch.environment_id, &raw)?;
        OverrideRepo::upsert_raw(
            state.db.pool(),
            &patch.request_id,
            &patch.environment_id,
            &encrypted,
        )
        .await?;
        // Echo the plaintext patch back to the caller — the frontend reuses
        // it as its new in-memory truth, so we mustn't surface ciphertext.
        return Ok(Some(patch));
    }

    OverrideRepo::upsert(state.db.pool(), &patch).await.map(Some)
}

#[tauri::command]
pub async fn delete_override(
    state: State<'_, AppState>,
    request_id: RequestId,
    env_id: EnvironmentId,
) -> Result<()> {
    let env = EnvironmentRepo::get(state.db.pool(), &env_id).await?;
    // Deleting a row in a locked env doesn't leak secrets, but the editor UX
    // assumes "if locked, hide everything", so require unlock for symmetry.
    if env.is_encrypted && !state.session_keys.is_unlocked(&env_id) {
        return Err(StorageError::EnvLocked(env_id.as_str().to_string()));
    }
    OverrideRepo::delete(state.db.pool(), &request_id, &env_id).await
}
