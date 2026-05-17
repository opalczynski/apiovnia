//! Environment IPC commands.
//!
//! Phase 5: list / create / rename / delete + plaintext variable CRUD.
//! Phase 6: when an env is `is_encrypted`, the value column round-trips
//! through `commands::crypto::{encrypt,decrypt}_value_for_env`. Without the
//! session key loaded, list / upsert / delete bounce with `EnvLocked` so
//! the frontend knows to pop the unlock modal.

use apiovnia_core::{
    ids::{EnvironmentId, ProjectId},
    model::{EnvVariable, Environment},
};
use apiovnia_storage::{EnvVariableRepo, EnvironmentRepo, Result, StorageError};
use tauri::State;

use crate::{
    app_state::AppState,
    commands::crypto::{decrypt_value_for_env, encrypt_value_for_env},
};

#[tauri::command]
pub async fn list_envs(
    state: State<'_, AppState>,
    project_id: ProjectId,
) -> Result<Vec<Environment>> {
    EnvironmentRepo::list_for_project(state.db.pool(), &project_id).await
}

#[tauri::command]
pub async fn create_env(
    state: State<'_, AppState>,
    project_id: ProjectId,
    name: String,
) -> Result<Environment> {
    EnvironmentRepo::create(state.db.pool(), &project_id, &name).await
}

#[tauri::command]
pub async fn rename_env(
    state: State<'_, AppState>,
    id: EnvironmentId,
    name: String,
) -> Result<Environment> {
    EnvironmentRepo::rename(state.db.pool(), &id, &name).await
}

#[tauri::command]
pub async fn delete_env(state: State<'_, AppState>, id: EnvironmentId) -> Result<()> {
    EnvironmentRepo::delete(state.db.pool(), &id).await?;
    // Drop any session key for the now-gone env so the next env that
    // happens to reuse the (random, UUIDv4) id wouldn't accidentally inherit
    // a stale key. Belt-and-braces.
    state.session_keys.lock(&id);
    Ok(())
}

// ---------------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_env_variables(
    state: State<'_, AppState>,
    env_id: EnvironmentId,
) -> Result<Vec<EnvVariable>> {
    let env = EnvironmentRepo::get(state.db.pool(), &env_id).await?;
    let mut vars = EnvVariableRepo::list_for_env(state.db.pool(), &env_id).await?;
    if env.is_encrypted {
        // Decrypt each value with the session key. EnvLocked propagates if the
        // user hasn't unlocked yet.
        for v in &mut vars {
            v.value = decrypt_value_for_env(&state, &env_id, &v.value)?;
        }
    }
    Ok(vars)
}

#[tauri::command]
pub async fn upsert_env_variable(
    state: State<'_, AppState>,
    env_id: EnvironmentId,
    name: String,
    value: String,
    is_secret: bool,
) -> Result<EnvVariable> {
    let env = EnvironmentRepo::get(state.db.pool(), &env_id).await?;
    let value_to_store = if env.is_encrypted {
        encrypt_value_for_env(&state, &env_id, &value)?
    } else {
        value.clone()
    };
    let mut stored =
        EnvVariableRepo::upsert(state.db.pool(), &env_id, &name, &value_to_store, is_secret)
            .await?;
    // Return the plaintext value to the caller so the editor doesn't suddenly
    // show ciphertext after save. Same shape as plaintext path.
    if env.is_encrypted {
        stored.value = value;
    }
    Ok(stored)
}

#[tauri::command]
pub async fn delete_env_variable(
    state: State<'_, AppState>,
    env_id: EnvironmentId,
    name: String,
) -> Result<()> {
    // The delete itself doesn't touch the secret, but we still require the
    // env to be unlocked to avoid the "edit while locked" confusion class.
    let env = EnvironmentRepo::get(state.db.pool(), &env_id).await?;
    if env.is_encrypted && !state.session_keys.is_unlocked(&env_id) {
        return Err(StorageError::EnvLocked(env_id.as_str().to_string()));
    }
    EnvVariableRepo::delete(state.db.pool(), &env_id, &name).await
}
