//! Request CRUD.
//!
//! Headers/params/auth live in the row as JSON strings; we (de)serialise
//! through `apiovnia-core` types so the JSON shape stays in one place.

use apiovnia_core::{
    ids::{CollectionId, RequestId},
    model::{AuthConfig, BodyType, HttpMethod, KeyValue, Request},
    time::epoch_millis_now,
};
use sqlx::SqlitePool;

use crate::error::{Result, StorageError};

/// Lightweight row for the middle-panel listing — full `Request` with its
/// JSON columns is loaded on demand via [`RequestRepo::get`].
#[derive(Debug, Clone)]
pub struct RequestSummary {
    pub id: RequestId,
    pub collection_id: CollectionId,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub sort_order: i64,
}

pub struct RequestRepo;

impl RequestRepo {
    pub async fn list_in_collection(
        pool: &SqlitePool,
        collection_id: &CollectionId,
    ) -> Result<Vec<RequestSummary>> {
        let rows = sqlx::query_as::<_, SummaryRow>(
            "SELECT id, collection_id, name, method, url, sort_order \
             FROM requests WHERE collection_id = ? \
             ORDER BY sort_order ASC, created_at ASC",
        )
        .bind(collection_id.as_str())
        .fetch_all(pool)
        .await?;

        rows.into_iter().map(SummaryRow::try_into_domain).collect()
    }

    pub async fn get(pool: &SqlitePool, id: &RequestId) -> Result<Request> {
        let row = sqlx::query_as::<_, FullRow>(
            "SELECT id, collection_id, name, method, url, headers_json, params_json, \
                    body_type, body_content, auth_json, sort_order, created_at, updated_at \
             FROM requests WHERE id = ?",
        )
        .bind(id.as_str())
        .fetch_optional(pool)
        .await?
        .ok_or(StorageError::NotFound)?;

        row.try_into_domain()
    }

    /// Creates a blank `GET ""` request named `name`, ready for the user to
    /// fill in via the editor.
    pub async fn create_blank(
        pool: &SqlitePool,
        collection_id: &CollectionId,
        name: &str,
    ) -> Result<Request> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StorageError::InvalidData("request name is empty".into()));
        }
        let id = RequestId::new();
        let now = epoch_millis_now();
        let req = Request::new_blank(id.clone(), collection_id.clone(), name.into(), now, now);

        sqlx::query(
            "INSERT INTO requests (id, collection_id, name, method, url, headers_json, \
                                   params_json, body_type, body_content, auth_json, \
                                   sort_order, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(req.id.as_str())
        .bind(req.collection_id.as_str())
        .bind(&req.name)
        .bind(req.method.as_str())
        .bind(&req.url)
        .bind(serde_json::to_string(&req.headers)?)
        .bind(serde_json::to_string(&req.params)?)
        .bind(body_type_str(req.body_type))
        .bind(&req.body_content)
        .bind(serde_json::to_string(&req.auth)?)
        .bind(req.sort_order)
        .bind(req.created_at)
        .bind(req.updated_at)
        .execute(pool)
        .await?;

        Ok(req)
    }

    pub async fn rename(pool: &SqlitePool, id: &RequestId, name: &str) -> Result<Request> {
        let name = name.trim();
        if name.is_empty() {
            return Err(StorageError::InvalidData("request name is empty".into()));
        }
        let now = epoch_millis_now();
        let res = sqlx::query("UPDATE requests SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(now)
            .bind(id.as_str())
            .execute(pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Self::get(pool, id).await
    }

    /// Full update — the frontend sends the entire request and we overwrite
    /// the row. `id`, `collection_id`, and `created_at` are preserved from the
    /// existing row; everything else comes from the patch.
    pub async fn update_full(
        pool: &SqlitePool,
        id: &RequestId,
        patch: &Request,
    ) -> Result<Request> {
        let now = epoch_millis_now();
        let res = sqlx::query(
            "UPDATE requests SET name = ?, method = ?, url = ?, headers_json = ?, \
                                  params_json = ?, body_type = ?, body_content = ?, \
                                  auth_json = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&patch.name)
        .bind(patch.method.as_str())
        .bind(&patch.url)
        .bind(serde_json::to_string(&patch.headers)?)
        .bind(serde_json::to_string(&patch.params)?)
        .bind(body_type_str(patch.body_type))
        .bind(&patch.body_content)
        .bind(serde_json::to_string(&patch.auth)?)
        .bind(now)
        .bind(id.as_str())
        .execute(pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Self::get(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &RequestId) -> Result<()> {
        let res = sqlx::query("DELETE FROM requests WHERE id = ?")
            .bind(id.as_str())
            .execute(pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(StorageError::NotFound);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Row helpers
// ---------------------------------------------------------------------------

fn body_type_str(b: BodyType) -> &'static str {
    match b {
        BodyType::None => "none",
        BodyType::Json => "json",
        BodyType::Form => "form",
        BodyType::Multipart => "multipart",
        BodyType::Raw => "raw",
    }
}

fn parse_body_type(s: &str) -> Result<BodyType> {
    Ok(match s {
        "none" => BodyType::None,
        "json" => BodyType::Json,
        "form" => BodyType::Form,
        "multipart" => BodyType::Multipart,
        "raw" => BodyType::Raw,
        other => {
            return Err(StorageError::InvalidData(format!(
                "unknown body_type {other:?}"
            )))
        }
    })
}

fn parse_method(s: &str) -> Result<HttpMethod> {
    Ok(match s {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "PATCH" => HttpMethod::Patch,
        "DELETE" => HttpMethod::Delete,
        "HEAD" => HttpMethod::Head,
        "OPTIONS" => HttpMethod::Options,
        other => {
            return Err(StorageError::InvalidData(format!(
                "unknown method {other:?}"
            )))
        }
    })
}

#[derive(sqlx::FromRow)]
struct SummaryRow {
    id: String,
    collection_id: String,
    name: String,
    method: String,
    url: String,
    sort_order: i64,
}

impl SummaryRow {
    fn try_into_domain(self) -> Result<RequestSummary> {
        Ok(RequestSummary {
            id: RequestId::from_trusted(self.id),
            collection_id: CollectionId::from_trusted(self.collection_id),
            name: self.name,
            method: parse_method(&self.method)?,
            url: self.url,
            sort_order: self.sort_order,
        })
    }
}

#[derive(sqlx::FromRow)]
struct FullRow {
    id: String,
    collection_id: String,
    name: String,
    method: String,
    url: String,
    headers_json: String,
    params_json: String,
    body_type: String,
    body_content: String,
    auth_json: String,
    sort_order: i64,
    created_at: i64,
    updated_at: i64,
}

impl FullRow {
    fn try_into_domain(self) -> Result<Request> {
        let headers: Vec<KeyValue> = serde_json::from_str(&self.headers_json)?;
        let params: Vec<KeyValue> = serde_json::from_str(&self.params_json)?;
        let auth: AuthConfig = serde_json::from_str(&self.auth_json)?;
        Ok(Request {
            id: RequestId::from_trusted(self.id),
            collection_id: CollectionId::from_trusted(self.collection_id),
            name: self.name,
            method: parse_method(&self.method)?,
            url: self.url,
            headers,
            params,
            body_type: parse_body_type(&self.body_type)?,
            body_content: self.body_content,
            auth,
            sort_order: self.sort_order,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}
