//! Request IPC commands.
//!
//! `list_requests` returns a lighter `RequestSummary` for the middle panel;
//! the full `Request` body (headers/params/body/auth) is loaded on demand via
//! `get_request` when the user opens it.

use apiovnia_core::{
    ids::{CollectionId, RequestId},
    model::{HttpMethod, Request},
};
use apiovnia_storage::{repos::requests::RequestSummary, RequestRepo, Result};
use serde::Serialize;
use tauri::State;

use crate::app_state::AppState;

/// Frontend-facing summary — adds `#[serde(rename_all = "camelCase")]` for
/// the IPC boundary so the TS mirror reads naturally (`collectionId` etc).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestSummaryDto {
    pub id: RequestId,
    pub collection_id: CollectionId,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub sort_order: i64,
}

impl From<RequestSummary> for RequestSummaryDto {
    fn from(s: RequestSummary) -> Self {
        Self {
            id: s.id,
            collection_id: s.collection_id,
            name: s.name,
            method: s.method,
            url: s.url,
            sort_order: s.sort_order,
        }
    }
}

#[tauri::command]
pub async fn list_requests(
    state: State<'_, AppState>,
    collection_id: CollectionId,
) -> Result<Vec<RequestSummaryDto>> {
    let rows = RequestRepo::list_in_collection(state.db.pool(), &collection_id).await?;
    Ok(rows.into_iter().map(RequestSummaryDto::from).collect())
}

#[tauri::command]
pub async fn get_request(state: State<'_, AppState>, id: RequestId) -> Result<Request> {
    RequestRepo::get(state.db.pool(), &id).await
}

#[tauri::command]
pub async fn create_request(
    state: State<'_, AppState>,
    collection_id: CollectionId,
    name: String,
) -> Result<Request> {
    RequestRepo::create_blank(state.db.pool(), &collection_id, &name).await
}

#[tauri::command]
pub async fn rename_request(
    state: State<'_, AppState>,
    id: RequestId,
    name: String,
) -> Result<Request> {
    RequestRepo::rename(state.db.pool(), &id, &name).await
}

#[tauri::command]
pub async fn update_request(
    state: State<'_, AppState>,
    id: RequestId,
    patch: Request,
) -> Result<Request> {
    RequestRepo::update_full(state.db.pool(), &id, &patch).await
}

#[tauri::command]
pub async fn delete_request(state: State<'_, AppState>, id: RequestId) -> Result<()> {
    RequestRepo::delete(state.db.pool(), &id).await
}
