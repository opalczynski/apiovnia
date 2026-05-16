//! Project IPC commands.

use apiovnia_core::{ids::ProjectId, model::Project};
use apiovnia_storage::{ProjectRepo, Result};
use tauri::State;

use crate::app_state::AppState;

#[tauri::command]
pub async fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>> {
    ProjectRepo::list(state.db.pool()).await
}

#[tauri::command]
pub async fn create_project(state: State<'_, AppState>, name: String) -> Result<Project> {
    ProjectRepo::create(state.db.pool(), &name).await
}

#[tauri::command]
pub async fn rename_project(
    state: State<'_, AppState>,
    id: ProjectId,
    name: String,
) -> Result<Project> {
    ProjectRepo::rename(state.db.pool(), &id, &name).await
}

#[tauri::command]
pub async fn delete_project(state: State<'_, AppState>, id: ProjectId) -> Result<()> {
    ProjectRepo::delete(state.db.pool(), &id).await
}
