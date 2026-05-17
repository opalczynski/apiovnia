//! `OpenAPI` import / export IPC.
//!
//! Both commands assemble + return a DTO with the full data the frontend
//! needs to render the `OpLog` panel (table rows + warnings + downloadable
//! log text). The actual file I/O (open/save dialogs, writing the export
//! YAML) is done frontend-side via `tauri-plugin-dialog` + browser-style
//! download; we only expose a `write_text_file` helper for cases where
//! the user can't be in the loop (the timestamped log file).

use std::path::Path;

use apiovnia_core::ids::{CollectionId, ProjectId};
use apiovnia_openapi::{
    export_collection, import_document,
    redact::{redact_request, Policy},
    ExportError, ExportResult, ExportRow, ImportError, ImportResult, ImportRow,
};
use apiovnia_storage::{
    CollectionRepo, EnvVariableRepo, EnvironmentRepo, OverrideRepo, ProjectRepo, RequestRepo,
    StorageError,
};
use chrono::Utc;
use serde::Serialize;
use tauri::State;
use thiserror::Error;

use crate::app_state::AppState;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum OpenApiError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Import(#[from] ImportError),
    #[error(transparent)]
    Export(#[from] ExportError),
    #[error("io: {0}")]
    Io(String),
}

impl Serialize for OpenApiError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

// ---------------------------------------------------------------------------
// DTOs surfaced to the frontend
// ---------------------------------------------------------------------------

/// Mirrors [`apiovnia_openapi::ImportRow`] for IPC (camelCase serde).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRowDto {
    pub name: String,
    pub method: String,
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResultDto {
    /// The new collection's id — frontend uses it to navigate to the new
    /// collection right after the import.
    pub collection_id: CollectionId,
    pub collection_name: String,
    pub request_count: usize,
    pub environment_count: usize,
    pub warning_count: usize,
    pub rows: Vec<ImportRowDto>,
    pub warnings: Vec<String>,
    /// Full text the user can save via the "Download log" button.
    pub log_text: String,
    /// Suggested filename for the downloaded log.
    pub log_filename: String,
}

/// Mirrors [`apiovnia_openapi::ExportRow`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRowDto {
    pub name: String,
    pub method: String,
    pub path: String,
    pub redactions: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResultDto {
    /// The `OpenAPI` 3.0.3 YAML document.
    pub yaml: String,
    /// Suggested filename for the YAML (`{collection}_{date}.yaml`).
    pub yaml_filename: String,
    pub request_count: usize,
    pub redaction_count: usize,
    pub warning_count: usize,
    pub rows: Vec<ExportRowDto>,
    pub warnings: Vec<String>,
    pub log_text: String,
    pub log_filename: String,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn import_openapi(
    state: State<'_, AppState>,
    project_id: ProjectId,
    file_path: String,
) -> Result<ImportResultDto, OpenApiError> {
    let source = std::fs::read_to_string(&file_path).map_err(|e| OpenApiError::Io(e.to_string()))?;

    // The frontend has the project id; we pre-allocate the collection id
    // here so the import + persistence path can wire it through cleanly.
    let collection_id = CollectionId::new();
    let parsed = import_document(&source, &project_id, collection_id.clone())?;

    persist_import(&state, &parsed).await?;

    let stem = Path::new(&file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported");
    let dto = ImportResultDto {
        collection_id,
        collection_name: parsed.collection.name.clone(),
        request_count: parsed.requests.len(),
        environment_count: parsed.environments.len(),
        warning_count: parsed.warnings.len(),
        rows: parsed.rows.iter().map(import_row_to_dto).collect(),
        warnings: parsed.warnings,
        log_text: stamp_log(&parsed.log),
        log_filename: format!("import_{}_{}.log", sanitize(stem), date_slug()),
    };
    Ok(dto)
}

#[tauri::command]
pub async fn export_collection_openapi(
    state: State<'_, AppState>,
    collection_id: CollectionId,
) -> Result<ExportResultDto, OpenApiError> {
    let pool = state.db.pool();
    let collection = CollectionRepo::get(pool, &collection_id).await?;
    // Project name powers the suggested filename `{project}_{collection}.yaml`.
    let project = ProjectRepo::get(pool, &collection.project_id).await?;
    let summaries = RequestRepo::list_in_collection(pool, &collection_id).await?;

    // Load full request bodies — summaries only have name/method/url.
    let mut full_requests = Vec::with_capacity(summaries.len());
    for s in summaries {
        let req = RequestRepo::get(pool, &s.id).await?;
        full_requests.push(req);
    }

    // Redact each request with the default policy (Phase 11 will pass a
    // user-customised one through here).
    let policy = Policy::default();
    let redacted: Vec<_> = full_requests
        .iter()
        .map(|r| redact_request(r, &policy))
        .collect();
    let total_redactions: usize = redacted.iter().map(|(_, t)| t.total()).sum();

    let result: ExportResult = export_collection(&collection, &redacted)?;

    let proj_slug = sanitize(&project.name);
    let coll_slug = sanitize(&collection.name);
    // YAML filename mirrors the project → collection hierarchy: easy to
    // find in `~/Downloads` when you've exported multiple collections.
    let yaml_filename = format!("{proj_slug}_{coll_slug}.yaml");
    // Log filename keeps the date so re-exports don't clobber each other.
    let log_filename = format!("export_{proj_slug}_{coll_slug}_{date}.log", date = date_slug());
    let dto = ExportResultDto {
        yaml: result.yaml,
        yaml_filename,
        request_count: result.rows.len(),
        redaction_count: total_redactions,
        warning_count: result.warnings.len(),
        rows: result.rows.iter().map(export_row_to_dto).collect(),
        warnings: result.warnings,
        log_text: stamp_log(&result.log),
        log_filename,
    };
    Ok(dto)
}

/// Generic "save this string to disk" — used by the `OpLog` "Download log"
/// button after the frontend has picked a path via `tauri-plugin-dialog`.
/// Doesn't append, doesn't validate path beyond what the OS does.
#[tauri::command]
pub async fn save_text_file(path: String, contents: String) -> Result<(), OpenApiError> {
    std::fs::write(&path, contents).map_err(|e| OpenApiError::Io(e.to_string()))
}

// ---------------------------------------------------------------------------
// Persistence helpers
// ---------------------------------------------------------------------------

async fn persist_import(
    state: &State<'_, AppState>,
    parsed: &ImportResult,
) -> Result<(), OpenApiError> {
    let pool = state.db.pool();

    // 1. Create the collection (using the same id we pre-allocated so the
    //    request rows already reference the right collection_id).
    CollectionRepo::insert_full(pool, &parsed.collection).await?;

    // 2. Insert every request as a full row in one go.
    for req in &parsed.requests {
        RequestRepo::insert_full(pool, req).await?;
    }

    // 3. Environments (one per server[] entry, when there are 2+).
    for env in &parsed.environments {
        EnvironmentRepo::create(pool, &env.project_id, &env.name).await?;
    }
    // After create the env has a fresh id (because EnvironmentRepo::create
    // generates one). To wire the imported overrides, we need to remap.
    // Refetch by name and build (parsed_id -> real_id) map.
    let mut id_map: std::collections::HashMap<String, apiovnia_core::ids::EnvironmentId> =
        std::collections::HashMap::new();
    if !parsed.environments.is_empty() {
        let project_id = &parsed.environments[0].project_id;
        let real = EnvironmentRepo::list_for_project(pool, project_id).await?;
        for e in &parsed.environments {
            if let Some(matching) = real.iter().find(|r| r.name == e.name) {
                id_map.insert(e.id.as_str().to_string(), matching.id.clone());
            }
        }
    }

    // 4. Overrides — same remap.
    for ov in &parsed.overrides {
        let env_real = match id_map.get(ov.environment_id.as_str()) {
            Some(id) => id.clone(),
            None => continue,
        };
        let mut patched = ov.clone();
        patched.environment_id = env_real;
        OverrideRepo::upsert(pool, &patched).await?;
    }

    // 5. Touch — no env variables to seed (OpenAPI doesn't carry them).
    //    The EnvVariableRepo dep stays imported for future use (Phase 7
    //    nice-to-have: derive `{{base_url}}` etc from server variables).
    let _ = EnvVariableRepo::list_for_env; // silence unused-import lint
    Ok(())
}

// ---------------------------------------------------------------------------
// Tiny helpers
// ---------------------------------------------------------------------------

fn import_row_to_dto(r: &ImportRow) -> ImportRowDto {
    ImportRowDto {
        name: r.name.clone(),
        method: r.method.clone(),
        path: r.path.clone(),
    }
}

fn export_row_to_dto(r: &ExportRow) -> ExportRowDto {
    ExportRowDto {
        name: r.name.clone(),
        method: r.method.clone(),
        path: r.path.clone(),
        redactions: r.redactions,
    }
}

/// Replace the `<timestamp>` placeholder left in the openapi crate's log
/// (which has no clock dep) with the current wall-clock time.
fn stamp_log(log: &str) -> String {
    log.replacen("<timestamp>", &Utc::now().to_rfc3339(), 1)
}

fn date_slug() -> String {
    Utc::now().format("%Y%m%d-%H%M%S").to_string()
}

/// Filename-safe slug from a free-form name. ASCII letters/digits/`-`/`_`
/// only; everything else collapses to `_`.
fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_underscore = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "untitled".into()
    } else {
        trimmed
    }
}

