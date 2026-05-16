//! Environment IPC commands.
//!
//! Phase 5 surface: list / create / rename / delete. Marking an env as
//! `requires_unlock` + master-password sealing lands in Phase 6.

use apiovnia_core::{
    ids::{EnvironmentId, ProjectId},
    model::{EnvVariable, Environment},
};
use apiovnia_storage::{EnvVariableRepo, EnvironmentRepo, Result};
use tauri::State;

use crate::app_state::AppState;

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
    EnvironmentRepo::delete(state.db.pool(), &id).await
}

// ---------------------------------------------------------------------------
// Variables
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_env_variables(
    state: State<'_, AppState>,
    env_id: EnvironmentId,
) -> Result<Vec<EnvVariable>> {
    EnvVariableRepo::list_for_env(state.db.pool(), &env_id).await
}

#[tauri::command]
pub async fn upsert_env_variable(
    state: State<'_, AppState>,
    env_id: EnvironmentId,
    name: String,
    value: String,
    is_secret: bool,
) -> Result<EnvVariable> {
    EnvVariableRepo::upsert(state.db.pool(), &env_id, &name, &value, is_secret).await
}

#[tauri::command]
pub async fn delete_env_variable(
    state: State<'_, AppState>,
    env_id: EnvironmentId,
    name: String,
) -> Result<()> {
    EnvVariableRepo::delete(state.db.pool(), &env_id, &name).await
}
