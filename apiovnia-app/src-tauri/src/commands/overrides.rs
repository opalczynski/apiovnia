//! Per-`(request, env)` override IPC commands.

use apiovnia_core::{
    ids::{EnvironmentId, RequestId},
    model::EnvOverride,
};
use apiovnia_storage::{OverrideRepo, Result};
use tauri::State;

use crate::app_state::AppState;

#[tauri::command]
pub async fn get_override(
    state: State<'_, AppState>,
    request_id: RequestId,
    env_id: EnvironmentId,
) -> Result<Option<EnvOverride>> {
    OverrideRepo::get(state.db.pool(), &request_id, &env_id).await
}

#[tauri::command]
pub async fn list_overrides_for_request(
    state: State<'_, AppState>,
    request_id: RequestId,
) -> Result<Vec<EnvOverride>> {
    OverrideRepo::list_for_request(state.db.pool(), &request_id).await
}

/// Upsert the override. If the patch is entirely empty (every field `None`),
/// we delete the row instead so storage stays tidy.
#[tauri::command]
pub async fn upsert_override(
    state: State<'_, AppState>,
    patch: EnvOverride,
) -> Result<Option<EnvOverride>> {
    if patch.is_empty() {
        OverrideRepo::delete(state.db.pool(), &patch.request_id, &patch.environment_id).await?;
        return Ok(None);
    }
    OverrideRepo::upsert(state.db.pool(), &patch).await.map(Some)
}

#[tauri::command]
pub async fn delete_override(
    state: State<'_, AppState>,
    request_id: RequestId,
    env_id: EnvironmentId,
) -> Result<()> {
    OverrideRepo::delete(state.db.pool(), &request_id, &env_id).await
}
