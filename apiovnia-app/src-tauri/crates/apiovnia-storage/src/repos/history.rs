//! Per-request execution history.
//!
//! Each invocation of `execute_request` writes one row here. The History
//! panel (Phase 9) reads from `list_recent` to show "what did I send and
//! when". Bodies are stored as text (or base64 for binaries — encoded by
//! the caller before we ever see them).

use apiovnia_core::{
    ids::{EnvironmentId, RequestId},
    time::epoch_millis_now,
};
use serde::Serialize;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::Result;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub request_id: Option<RequestId>,
    pub environment_id: Option<EnvironmentId>,
    pub executed_at: i64,
    pub status_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub response_size_bytes: Option<i64>,
    pub response_headers_json: Option<String>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    /// JSON-encoded `SentRequest` snapshot, when available.
    pub sent_json: Option<String>,
    pub final_url: Option<String>,
    pub content_type: Option<String>,
    /// `"text" | "binarybase64" | "empty"` — matches `apiovnia-http::ResponseBodyKind`.
    pub body_kind: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewHistoryEntry<'a> {
    pub request_id: &'a RequestId,
    pub environment_id: Option<&'a EnvironmentId>,
    pub status_code: Option<i64>,
    pub duration_ms: Option<i64>,
    pub response_size_bytes: Option<i64>,
    pub response_headers_json: Option<&'a str>,
    pub response_body: Option<&'a str>,
    pub error_message: Option<&'a str>,
    pub sent_json: Option<&'a str>,
    pub final_url: Option<&'a str>,
    pub content_type: Option<&'a str>,
    pub body_kind: Option<&'a str>,
}

pub struct HistoryRepo;

impl HistoryRepo {
    pub async fn insert(pool: &SqlitePool, entry: NewHistoryEntry<'_>) -> Result<String> {
        let id = format!("hist_{}", Uuid::new_v4().simple());
        let now = epoch_millis_now();
        sqlx::query(
            "INSERT INTO request_history (id, request_id, environment_id, executed_at, \
                                          status_code, duration_ms, response_size_bytes, \
                                          response_headers_json, response_body, error_message, \
                                          sent_json, final_url, content_type, body_kind) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(entry.request_id.as_str())
        .bind(entry.environment_id.map(apiovnia_core::ids::EnvironmentId::as_str))
        .bind(now)
        .bind(entry.status_code)
        .bind(entry.duration_ms)
        .bind(entry.response_size_bytes)
        .bind(entry.response_headers_json)
        .bind(entry.response_body)
        .bind(entry.error_message)
        .bind(entry.sent_json)
        .bind(entry.final_url)
        .bind(entry.content_type)
        .bind(entry.body_kind)
        .execute(pool)
        .await?;
        Ok(id)
    }

    pub async fn list_recent(pool: &SqlitePool, limit: i64) -> Result<Vec<HistoryEntry>> {
        let rows = sqlx::query_as::<_, Row>(SELECT_COLUMNS)
            .bind(limit)
            .fetch_all(pool)
            .await?;
        Ok(rows.into_iter().map(Row::into_domain).collect())
    }

    /// Most recent **successful** history entry for a given request — i.e.
    /// `error_message IS NULL`. Used to restore the last response after an
    /// app restart so the right pane isn't empty until the user re-Sends.
    pub async fn latest_success_for(
        pool: &SqlitePool,
        request_id: &RequestId,
    ) -> Result<Option<HistoryEntry>> {
        let row = sqlx::query_as::<_, Row>(SELECT_LATEST_SUCCESS)
            .bind(request_id.as_str())
            .fetch_optional(pool)
            .await?;
        Ok(row.map(Row::into_domain))
    }
}

const SELECT_COLUMNS: &str =
    "SELECT id, request_id, environment_id, executed_at, status_code, duration_ms, \
            response_size_bytes, response_headers_json, response_body, error_message, \
            sent_json, final_url, content_type, body_kind \
     FROM request_history ORDER BY executed_at DESC LIMIT ?";

const SELECT_LATEST_SUCCESS: &str =
    "SELECT id, request_id, environment_id, executed_at, status_code, duration_ms, \
            response_size_bytes, response_headers_json, response_body, error_message, \
            sent_json, final_url, content_type, body_kind \
     FROM request_history \
     WHERE request_id = ? AND error_message IS NULL AND status_code IS NOT NULL \
     ORDER BY executed_at DESC LIMIT 1";

#[derive(sqlx::FromRow)]
struct Row {
    id: String,
    request_id: Option<String>,
    environment_id: Option<String>,
    executed_at: i64,
    status_code: Option<i64>,
    duration_ms: Option<i64>,
    response_size_bytes: Option<i64>,
    response_headers_json: Option<String>,
    response_body: Option<String>,
    error_message: Option<String>,
    sent_json: Option<String>,
    final_url: Option<String>,
    content_type: Option<String>,
    body_kind: Option<String>,
}

impl Row {
    fn into_domain(self) -> HistoryEntry {
        HistoryEntry {
            id: self.id,
            request_id: self.request_id.map(RequestId::from_trusted),
            environment_id: self.environment_id.map(EnvironmentId::from_trusted),
            executed_at: self.executed_at,
            status_code: self.status_code,
            duration_ms: self.duration_ms,
            response_size_bytes: self.response_size_bytes,
            response_headers_json: self.response_headers_json,
            response_body: self.response_body,
            error_message: self.error_message,
            sent_json: self.sent_json,
            final_url: self.final_url,
            content_type: self.content_type,
            body_kind: self.body_kind,
        }
    }
}
