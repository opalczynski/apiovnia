//! Apiovnia — Tauri runtime entry point.
//!
//! Thin layer: builds the Tauri application, opens the local `SQLite` database
//! into application state, and registers the IPC commands defined in
//! `commands::*`. All business logic lives in the domain crates.

mod app_state;
mod commands;

use std::path::PathBuf;

use apiovnia_http::{Executor, ExecutorConfig};
use apiovnia_storage::Db;
use tauri::{async_runtime, Manager};

use app_state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let db_path = resolve_db_path(app)?;
            tracing::info!(?db_path, "opening apiovnia database");

            // Setup runs once at startup — synchronous block_on is fine here.
            let db = async_runtime::block_on(Db::open(&db_path)).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!(
                    "failed to open db at {}: {e}",
                    db_path.display()
                ))
            })?;

            let executor = Executor::new(&ExecutorConfig::default()).map_err(|e| {
                Box::<dyn std::error::Error>::from(format!("failed to init http executor: {e}"))
            })?;

            app.manage(AppState::new(db, executor));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::projects::list_projects,
            commands::projects::create_project,
            commands::projects::rename_project,
            commands::projects::delete_project,
            commands::collections::list_collections,
            commands::collections::create_collection,
            commands::collections::rename_collection,
            commands::collections::delete_collection,
            commands::requests::list_requests,
            commands::requests::get_request,
            commands::requests::create_request,
            commands::requests::rename_request,
            commands::requests::update_request,
            commands::requests::delete_request,
            commands::environments::list_envs,
            commands::environments::create_env,
            commands::environments::rename_env,
            commands::environments::delete_env,
            commands::environments::list_env_variables,
            commands::environments::upsert_env_variable,
            commands::environments::delete_env_variable,
            commands::overrides::get_override,
            commands::overrides::list_overrides_for_request,
            commands::overrides::upsert_override,
            commands::overrides::delete_override,
            commands::execution::execute_request,
            commands::execution::get_last_response,
            commands::execution::build_request_snippet,
            commands::crypto::enable_env_encryption,
            commands::crypto::disable_env_encryption,
            commands::crypto::unlock_env,
            commands::crypto::lock_env,
            commands::crypto::is_env_unlocked,
            commands::crypto::list_unlocked_envs,
            commands::crypto::score_password,
            commands::openapi::import_openapi,
            commands::openapi::export_collection_openapi,
            commands::openapi::save_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Database location: `<app_data_dir>/apiovnia.db`. On Linux this resolves to
/// `~/.local/share/tech.trurl.apiovnia/apiovnia.db` (XDG); macOS uses
/// `~/Library/Application Support/tech.trurl.apiovnia/`.
fn resolve_db_path(app: &tauri::App) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = app.path().app_data_dir()?;
    Ok(dir.join("apiovnia.db"))
}
