//! Execute IPC.
//!
//! `execute_request` loads the request from storage, resolves it against the
//! optional environment (override + `{{var}}` interpolation), runs it through
//! the HTTP executor, and persists a history row.

use std::collections::HashMap;

use apiovnia_core::{
    ids::{EnvironmentId, RequestId},
    model::EnvOverride,
    resolve_request, SnippetFormat,
};
use apiovnia_http::{ExecutionError, ExecutionResult, HeaderEntry, ResponseBodyKind, SentRequest};
use apiovnia_storage::{
    repos::history::{HistoryEntry, NewHistoryEntry},
    EnvVariableRepo, EnvironmentRepo, HistoryRepo, OverrideRepo, RequestRepo, StorageError,
};
use serde::Serialize;
use tauri::State;
use thiserror::Error;

use crate::{
    app_state::AppState,
    commands::crypto::{decrypt_override_cols, decrypt_value_for_env},
};

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

    let (override_opt, vars, encrypted) =
        load_env_context(&state, &request_id, env_id.as_ref()).await?;
    let resolved = resolve_request(&base, override_opt.as_ref(), &vars);
    let outcome = state.executor.execute(&resolved).await;

    // Persist history regardless of success/failure so the user always has
    // a trail. We store enough to reconstruct the full ExecutionResult on
    // restart (sent snapshot, headers, body, kind, finalUrl); failures store
    // the error message instead.
    //
    // When the env is encrypted we deliberately drop every field that can
    // carry resolved secrets — the sent snapshot (resolved Authorization +
    // expanded `{{var}}` body), the response body and headers (server-issued
    // tokens / Set-Cookie), the resolved final URL (an apikey may ride the
    // query string) and the error string (reqwest embeds the URL). Otherwise
    // those secrets would sit in the same at-rest-untrusted SQLite file the
    // env encryption exists to protect, silently defeating it. Only
    // non-secret metadata (status, timing, size, content-type) survives.
    match &outcome {
        Ok(result) => {
            let (headers_json, sent_json, body_for_history, final_url) = if encrypted {
                (None, None, None, None)
            } else {
                let headers_json = serde_json::to_string(&result.headers).ok();
                let sent_json = serde_json::to_string(&result.sent).ok();
                let body = if result.body.len() > 64 * 1024 {
                    // 64 KiB cap in history to keep the DB lean — the in-memory
                    // viewer always sees the full body during the session.
                    Some(format!("{}…", &result.body[..64 * 1024]))
                } else {
                    Some(result.body.clone())
                };
                (headers_json, sent_json, body, Some(result.final_url.clone()))
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
                    final_url: final_url.as_deref(),
                    content_type: result.content_type.as_deref(),
                    body_kind: Some(body_kind),
                },
            )
            .await;
        }
        Err(e) => {
            let err_string = e.to_string();
            let error_message = if encrypted {
                "request failed (details omitted — encrypted environment)"
            } else {
                err_string.as_str()
            };
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
                    error_message: Some(error_message),
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
    Ok(Some(rehydrate(row)))
}

/// One row in the History panel (Phase 9). Slim DTO — only the columns
/// the list needs (status pill / timing / where it points). Bodies live
/// in storage but only flow back through `get_history_response` when the
/// user clicks a row.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRowDto {
    pub id: String,
    pub request_id: Option<String>,
    pub request_name: Option<String>,
    pub project_id: Option<String>,
    pub project_name: Option<String>,
    pub collection_id: Option<String>,
    pub collection_name: Option<String>,
    pub environment_id: Option<String>,
    pub environment_name: Option<String>,
    pub executed_at: i64,
    pub status_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub method: Option<String>,
    pub url: Option<String>,
    pub final_url: Option<String>,
    pub content_type: Option<String>,
    /// Truncated to ~140 chars — only for the secondary line in the row.
    pub error_message: Option<String>,
}

/// Recent execution history, newest first. The default limit (200) matches
/// the design intent; callers can bump it for "show more" if we ever ship that.
#[tauri::command]
pub async fn list_history(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<HistoryRowDto>, ExecuteError> {
    let pool = state.db.pool();
    let lim = limit.unwrap_or(200).clamp(1, 1000);
    let rows = HistoryRepo::list_recent(pool, lim).await?;

    // Enrich rows with the request/collection/project/env names so the
    // History panel doesn't need to do N+1 fetches frontend-side. Small
    // per-call caches keep us at O(distinct ids) lookups, not O(rows).
    let mut req_cache: std::collections::HashMap<String, Option<apiovnia_core::model::Request>> =
        std::collections::HashMap::new();
    let mut coll_cache: std::collections::HashMap<
        String,
        Option<apiovnia_core::model::Collection>,
    > = std::collections::HashMap::new();
    let mut proj_cache: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    let mut env_cache: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let req = if let Some(rid) = &row.request_id {
            let key = rid.as_str().to_owned();
            if !req_cache.contains_key(&key) {
                let v = RequestRepo::get(pool, rid).await.ok();
                req_cache.insert(key.clone(), v);
            }
            req_cache.get(&key).and_then(|o| o.as_ref())
        } else {
            None
        };

        let coll = if let Some(r) = req {
            let key = r.collection_id.as_str().to_owned();
            if !coll_cache.contains_key(&key) {
                let v = apiovnia_storage::CollectionRepo::get(pool, &r.collection_id)
                    .await
                    .ok();
                coll_cache.insert(key.clone(), v);
            }
            coll_cache.get(&key).and_then(|o| o.as_ref())
        } else {
            None
        };

        let project_name = if let Some(c) = coll {
            let key = c.project_id.as_str().to_owned();
            if !proj_cache.contains_key(&key) {
                let v = apiovnia_storage::ProjectRepo::get(pool, &c.project_id)
                    .await
                    .ok()
                    .map(|p| p.name);
                proj_cache.insert(key.clone(), v);
            }
            proj_cache.get(&key).cloned().flatten()
        } else {
            None
        };

        let env_name = if let Some(eid) = &row.environment_id {
            let key = eid.as_str().to_owned();
            if !env_cache.contains_key(&key) {
                let v = EnvironmentRepo::get(pool, eid).await.ok().map(|e| e.name);
                env_cache.insert(key.clone(), v);
            }
            env_cache.get(&key).cloned().flatten()
        } else {
            None
        };

        out.push(HistoryRowDto {
            id: row.id,
            request_id: row.request_id.map(|i| i.as_str().to_owned()),
            request_name: req.map(|r| r.name.clone()),
            project_id: coll.map(|c| c.project_id.as_str().to_owned()),
            project_name,
            collection_id: coll.map(|c| c.id.as_str().to_owned()),
            collection_name: coll.map(|c| c.name.clone()),
            environment_id: row.environment_id.map(|i| i.as_str().to_owned()),
            environment_name: env_name,
            executed_at: row.executed_at,
            status_code: row.status_code,
            duration_ms: row.duration_ms,
            method: req.map(|r| r.method.as_str().to_owned()),
            url: req.map(|r| r.url.clone()),
            final_url: row.final_url,
            content_type: row.content_type,
            error_message: row.error_message.map(truncate_140),
        });
    }
    Ok(out)
}

fn truncate_140(mut s: String) -> String {
    const MAX: usize = 140;
    if s.chars().count() > MAX {
        let cut = s.char_indices().nth(MAX).map_or(s.len(), |(i, _)| i);
        s.truncate(cut);
        s.push('…');
    }
    s
}

/// Rehydrate a single history row into the full `ExecutionResult` shape
/// the response viewer already knows how to render. Returns `None` for a
/// missing id or a row that has no stored body (typical for failures).
#[tauri::command]
pub async fn get_history_response(
    state: State<'_, AppState>,
    history_id: String,
) -> Result<Option<ExecutionResult>, ExecuteError> {
    let Some(row) = HistoryRepo::get(state.db.pool(), &history_id).await? else {
        return Ok(None);
    };
    Ok(Some(rehydrate(row)))
}

fn rehydrate(row: HistoryEntry) -> ExecutionResult {
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

    ExecutionResult {
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
    }
}

/// Render a paste-ready code snippet for the given request, in the
/// requested format (curl / Python requests / `HTTPie` / fetch / `PowerShell`).
/// Shares the full resolution + decryption path with `execute_request`,
/// so secrets get materialised the same way — the resulting string
/// carries plaintext by design.
///
/// Locked encrypted env → `EnvLocked` bubbles up, frontend pops the unlock
/// modal with a retry callback that re-runs this command.
#[tauri::command]
pub async fn build_request_snippet(
    state: State<'_, AppState>,
    request_id: RequestId,
    env_id: Option<EnvironmentId>,
    format: SnippetFormat,
) -> Result<String, ExecuteError> {
    let pool = state.db.pool();
    let base = RequestRepo::get(pool, &request_id).await?;
    let (over, vars, _encrypted) = load_env_context(&state, &request_id, env_id.as_ref()).await?;
    let resolved = resolve_request(&base, over.as_ref(), &vars);
    Ok(format.render(&resolved))
}

/// Shared resolution + (when encrypted) decryption path. Returns the
/// optional override row, the `{{var}}` map to feed `resolve_request`, and
/// whether the env was encrypted (so callers can decide whether the resolved
/// values are secret material that must not be persisted in cleartext).
///
/// `EnvLocked` propagates through `?` — the encrypted-env helpers throw
/// it when the session key isn't loaded.
async fn load_env_context(
    state: &State<'_, AppState>,
    request_id: &RequestId,
    env_id: Option<&EnvironmentId>,
) -> Result<(Option<EnvOverride>, HashMap<String, String>, bool), ExecuteError> {
    let Some(env_ref) = env_id else {
        return Ok((None, HashMap::new(), false));
    };
    let pool = state.db.pool();
    let env = EnvironmentRepo::get(pool, env_ref).await?;

    if env.is_encrypted {
        // Variables: list → decrypt each value.
        let mut vlist = EnvVariableRepo::list_for_env(pool, env_ref).await?;
        for v in &mut vlist {
            v.value = decrypt_value_for_env(state, env_ref, &v.value)?;
        }
        let mut map = HashMap::with_capacity(vlist.len());
        for v in vlist {
            map.insert(v.name, v.value);
        }
        // Override: fetch raw → decrypt secret columns → parse JSON.
        let over = match OverrideRepo::get_raw(pool, request_id, env_ref).await? {
            Some(raw) => {
                let decrypted = decrypt_override_cols(state, env_ref, &raw.cols)?;
                Some(decrypted.into_domain(request_id.clone(), env_ref.clone())?)
            }
            None => None,
        };
        Ok((over, map, true))
    } else {
        let over = OverrideRepo::get(pool, request_id, env_ref).await?;
        let vlist = EnvVariableRepo::list_for_env(pool, env_ref).await?;
        let mut map = HashMap::with_capacity(vlist.len());
        for v in vlist {
            map.insert(v.name, v.value);
        }
        Ok((over, map, false))
    }
}

const fn body_kind_str(k: ResponseBodyKind) -> &'static str {
    match k {
        ResponseBodyKind::Text => "text",
        ResponseBodyKind::BinaryBase64 => "binarybase64",
        ResponseBodyKind::Empty => "empty",
    }
}
