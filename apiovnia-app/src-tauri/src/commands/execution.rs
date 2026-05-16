//! Execute IPC.
//!
//! `execute_request` loads the request from storage, resolves it against the
//! optional environment (override + `{{var}}` interpolation), runs it through
//! the HTTP executor, and persists a history row.

use std::collections::HashMap;

use apiovnia_core::{
    ids::{EnvironmentId, RequestId},
    resolve_request,
};
use apiovnia_http::{ExecutionError, ExecutionResult, HeaderEntry, ResponseBodyKind, SentRequest};
use apiovnia_storage::{
    repos::history::NewHistoryEntry, EnvVariableRepo, HistoryRepo, OverrideRepo, RequestRepo,
    StorageError,
};
use serde::Serialize;
use tauri::State;
use thiserror::Error;

use crate::app_state::AppState;

#[derive(Debug, Error)]
pub enum ExecuteError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Execution(#[from] ExecutionError),
}

impl Serialize for ExecuteError {
    fn serialize<S: serde::Serializer>(
        &self,
        s: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

#[tauri::command]
pub async fn execute_request(
    state: State<'_, AppState>,
    request_id: RequestId,
    env_id: Option<EnvironmentId>,
) -> Result<ExecutionResult, ExecuteError> {
    let pool = state.db.pool();
    let base = RequestRepo::get(pool, &request_id).await?;

    // Resolve against env if one is active. Missing env / missing override /
    // missing vars all degrade gracefully — we still send the base request.
    let (override_opt, vars) = if let Some(env) = env_id.as_ref() {
        let o = OverrideRepo::get(pool, &request_id, env).await?;
        let vlist = EnvVariableRepo::list_for_env(pool, env).await?;
        let mut map = HashMap::with_capacity(vlist.len());
        for v in vlist {
            map.insert(v.name, v.value);
        }
        (o, map)
    } else {
        (None, HashMap::new())
    };

    let resolved = resolve_request(&base, override_opt.as_ref(), &vars);
    let outcome = state.executor.execute(&resolved).await;

    // Persist history regardless of success/failure so the user always has
    // a trail. We store enough to reconstruct the full ExecutionResult on
    // restart (sent snapshot, headers, body, kind, finalUrl); failures store
    // the error message instead.
    match &outcome {
        Ok(result) => {
            let headers_json = serde_json::to_string(&result.headers).ok();
            let sent_json = serde_json::to_string(&result.sent).ok();
            let body_for_history = if result.body.len() > 64 * 1024 {
                // 64 KiB cap in history to keep the DB lean — the in-memory
                // viewer always sees the full body during the session.
                Some(format!("{}…", &result.body[..64 * 1024]))
            } else {
                Some(result.body.clone())
            };
            let body_kind = body_kind_str(result.body_kind);
            let _ = HistoryRepo::insert(
                pool,
                NewHistoryEntry {
                    request_id: &request_id,
                    environment_id: env_id.as_ref(),
                    status_code: Some(i64::from(result.status)),
                    duration_ms: i64::try_from(result.duration_ms).ok(),
                    response_size_bytes: i64::try_from(result.size_bytes).ok(),
                    response_headers_json: headers_json.as_deref(),
                    response_body: body_for_history.as_deref(),
                    error_message: None,
                    sent_json: sent_json.as_deref(),
                    final_url: Some(result.final_url.as_str()),
                    content_type: result.content_type.as_deref(),
                    body_kind: Some(body_kind),
                },
            )
            .await;
        }
        Err(e) => {
            let _ = HistoryRepo::insert(
                pool,
                NewHistoryEntry {
                    request_id: &request_id,
                    environment_id: env_id.as_ref(),
                    status_code: None,
                    duration_ms: None,
                    response_size_bytes: None,
                    response_headers_json: None,
                    response_body: None,
                    error_message: Some(&e.to_string()),
                    sent_json: None,
                    final_url: None,
                    content_type: None,
                    body_kind: None,
                },
            )
            .await;
        }
    }

    Ok(outcome?)
}

/// Last successful response for a given request, restored from history.
/// Returns `None` if there's nothing on file or the most recent entry is
/// missing the rich fields (legacy rows from before migration 0002).
#[tauri::command]
pub async fn get_last_response(
    state: State<'_, AppState>,
    request_id: RequestId,
) -> Result<Option<ExecutionResult>, ExecuteError> {
    let Some(row) = HistoryRepo::latest_success_for(state.db.pool(), &request_id).await? else {
        return Ok(None);
    };

    // Rehydrate. Anything missing falls back to a sane default — better to
    // show *something* than nothing.
    let headers: Vec<HeaderEntry> = row
        .response_headers_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let sent: SentRequest = row
        .sent_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(SentRequest {
            method: String::new(),
            url: String::new(),
            headers: Vec::new(),
            body_preview: String::new(),
            body_size_bytes: 0,
        });

    let body_kind = match row.body_kind.as_deref() {
        Some("binarybase64") => ResponseBodyKind::BinaryBase64,
        Some("empty") => ResponseBodyKind::Empty,
        _ => ResponseBodyKind::Text,
    };

    let status = u16::try_from(row.status_code.unwrap_or(0).max(0)).unwrap_or(0);

    Ok(Some(ExecutionResult {
        status,
        status_text: String::new(),
        headers,
        content_type: row.content_type,
        body_kind,
        body: row.response_body.unwrap_or_default(),
        body_truncated: false,
        duration_ms: u64::try_from(row.duration_ms.unwrap_or(0).max(0)).unwrap_or(0),
        size_bytes: u64::try_from(row.response_size_bytes.unwrap_or(0).max(0)).unwrap_or(0),
        final_url: row.final_url.unwrap_or_default(),
        sent,
    }))
}

const fn body_kind_str(k: ResponseBodyKind) -> &'static str {
    match k {
        ResponseBodyKind::Text => "text",
        ResponseBodyKind::BinaryBase64 => "binarybase64",
        ResponseBodyKind::Empty => "empty",
    }
}
