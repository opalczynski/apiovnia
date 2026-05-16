//! Per-`(request, environment)` overrides — `request_env_overrides` table.
//!
//! Storage maps directly to the [`EnvOverride`] domain type. Collection
//! fields (`headers`, `params`) are stored as JSON strings; primitive
//! columns are nullable so `NULL` means "inherit base".

use apiovnia_core::{
    ids::{EnvironmentId, RequestId},
    model::{AuthConfig, BodyType, EnvOverride, HttpMethod, KeyValue},
};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::{Result, StorageError};

pub struct OverrideRepo;

impl OverrideRepo {
    /// Returns the override row for `(request_id, environment_id)` if present.
    pub async fn get(
        pool: &SqlitePool,
        request_id: &RequestId,
        env_id: &EnvironmentId,
    ) -> Result<Option<EnvOverride>> {
        let row = sqlx::query_as::<_, Row>(
            "SELECT id, request_id, environment_id, \
                    method_override, url_override, headers_override_json, \
                    params_override_json, body_type_override, body_content_override, \
                    auth_override_json \
             FROM request_env_overrides WHERE request_id = ? AND environment_id = ?",
        )
        .bind(request_id.as_str())
        .bind(env_id.as_str())
        .fetch_optional(pool)
        .await?;
        row.map(Row::try_into_domain).transpose()
    }

    /// Returns every override row for a given request, keyed by env id.
    /// Phase 9 will probably use this to drive a "show me where this request
    /// behaves differently" view.
    pub async fn list_for_request(
        pool: &SqlitePool,
        request_id: &RequestId,
    ) -> Result<Vec<EnvOverride>> {
        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, request_id, environment_id, \
                    method_override, url_override, headers_override_json, \
                    params_override_json, body_type_override, body_content_override, \
                    auth_override_json \
             FROM request_env_overrides WHERE request_id = ?",
        )
        .bind(request_id.as_str())
        .fetch_all(pool)
        .await?;
        rows.into_iter().map(Row::try_into_domain).collect()
    }

    /// Insert-or-update by `(request_id, environment_id)`. Caller controls
    /// the payload; an entirely-empty patch is allowed and is the natural
    /// way to leave the row in place "but inheriting everything" — see
    /// [`OverrideRepo::delete`] for the "actually remove" variant.
    pub async fn upsert(pool: &SqlitePool, patch: &EnvOverride) -> Result<EnvOverride> {
        let new_id = format!("ovr_{}", Uuid::new_v4().simple());

        let method_str = patch.method.map(http_method_str);
        let body_type_str = patch.body_type.map(body_type_str);
        let headers_json = patch
            .headers
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let params_json = patch
            .params
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let auth_json = patch.auth.as_ref().map(serde_json::to_string).transpose()?;

        sqlx::query(
            "INSERT INTO request_env_overrides (id, request_id, environment_id, \
                method_override, url_override, headers_override_json, params_override_json, \
                body_type_override, body_content_override, auth_override_json) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(request_id, environment_id) DO UPDATE SET \
                method_override = excluded.method_override, \
                url_override = excluded.url_override, \
                headers_override_json = excluded.headers_override_json, \
                params_override_json = excluded.params_override_json, \
                body_type_override = excluded.body_type_override, \
                body_content_override = excluded.body_content_override, \
                auth_override_json = excluded.auth_override_json",
        )
        .bind(&new_id)
        .bind(patch.request_id.as_str())
        .bind(patch.environment_id.as_str())
        .bind(method_str)
        .bind(patch.url.as_deref())
        .bind(headers_json.as_deref())
        .bind(params_json.as_deref())
        .bind(body_type_str)
        .bind(patch.body_content.as_deref())
        .bind(auth_json.as_deref())
        .execute(pool)
        .await?;

        Self::get(pool, &patch.request_id, &patch.environment_id)
            .await?
            .ok_or(StorageError::NotFound)
    }

    pub async fn delete(
        pool: &SqlitePool,
        request_id: &RequestId,
        env_id: &EnvironmentId,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM request_env_overrides WHERE request_id = ? AND environment_id = ?",
        )
        .bind(request_id.as_str())
        .bind(env_id.as_str())
        .execute(pool)
        .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Row decode
// ---------------------------------------------------------------------------

#[derive(sqlx::FromRow)]
struct Row {
    #[allow(dead_code)] // surfaced for future debugging / referential reads.
    id: String,
    request_id: String,
    environment_id: String,
    method_override: Option<String>,
    url_override: Option<String>,
    headers_override_json: Option<String>,
    params_override_json: Option<String>,
    body_type_override: Option<String>,
    body_content_override: Option<String>,
    auth_override_json: Option<String>,
}

impl Row {
    fn try_into_domain(self) -> Result<EnvOverride> {
        let method = self.method_override.as_deref().map(parse_method).transpose()?;
        let body_type = self
            .body_type_override
            .as_deref()
            .map(parse_body_type)
            .transpose()?;
        let headers: Option<Vec<KeyValue>> = self
            .headers_override_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?;
        let params: Option<Vec<KeyValue>> = self
            .params_override_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?;
        let auth: Option<AuthConfig> = self
            .auth_override_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?;
        Ok(EnvOverride {
            request_id: RequestId::from_trusted(self.request_id),
            environment_id: EnvironmentId::from_trusted(self.environment_id),
            method,
            url: self.url_override,
            headers,
            params,
            body_type,
            body_content: self.body_content_override,
            auth,
        })
    }
}

fn http_method_str(m: HttpMethod) -> &'static str {
    m.as_str()
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
                "unknown method override {other:?}"
            )))
        }
    })
}

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
                "unknown body_type override {other:?}"
            )))
        }
    })
}
