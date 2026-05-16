//! Collection IPC commands.

use apiovnia_core::{
    ids::{CollectionId, ProjectId},
    model::Collection,
};
use apiovnia_storage::{CollectionRepo, Result};
use tauri::State;

use crate::app_state::AppState;

#[tauri::command]
pub async fn list_collections(
    state: State<'_, AppState>,
    project_id: ProjectId,
) -> Result<Vec<Collection>> {
    CollectionRepo::list_in_project(state.db.pool(), &project_id).await
}

#[tauri::command]
pub async fn create_collection(
    state: State<'_, AppState>,
    project_id: ProjectId,
    name: String,
) -> Result<Collection> {
    CollectionRepo::create(state.db.pool(), &project_id, &name).await
}

#[tauri::command]
pub async fn rename_collection(
    state: State<'_, AppState>,
    id: CollectionId,
    name: String,
) -> Result<Collection> {
    CollectionRepo::rename(state.db.pool(), &id, &name).await
}

#[tauri::command]
pub async fn delete_collection(state: State<'_, AppState>, id: CollectionId) -> Result<()> {
    CollectionRepo::delete(state.db.pool(), &id).await
}
