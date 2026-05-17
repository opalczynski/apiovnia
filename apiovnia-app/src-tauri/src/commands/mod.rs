//! Tauri IPC commands. Each command is a thin wrapper that forwards into a
//! domain crate. Business logic lives in the domain crates; this layer only
//! adapts between Tauri's invoke signature and the repository API.

pub mod collections;
pub mod crypto;
pub mod environments;
pub mod execution;
pub mod openapi;
pub mod overrides;
pub mod projects;
pub mod requests;

/// Health check used by the frontend bootstrap to confirm the Rust side is up.
#[tauri::command]
pub fn ping() -> &'static str {
    "pong"
}
