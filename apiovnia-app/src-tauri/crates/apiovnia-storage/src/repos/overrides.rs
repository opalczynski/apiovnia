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

    /// Upsert from raw column data (already encrypted if the env is sealed).
    /// Used by the encrypted-env path so the command layer can apply
    /// encryption between serialisation and storage.
    pub async fn upsert_raw(
        pool: &SqlitePool,
        request_id: &RequestId,
        env_id: &EnvironmentId,
        cols: &RawOverrideCols,
    ) -> Result<()> {
        let new_id = format!("ovr_{}", Uuid::new_v4().simple());
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
        .bind(request_id.as_str())
        .bind(env_id.as_str())
        .bind(cols.method.as_deref())
        .bind(cols.url.as_deref())
        .bind(cols.headers_json.as_deref())
        .bind(cols.params_json.as_deref())
        .bind(cols.body_type.as_deref())
        .bind(cols.body_content.as_deref())
        .bind(cols.auth_json.as_deref())
        .execute(pool)
        .await?;
        Ok(())
    }

    /// As [`get`] but returns the raw stored TEXT columns. Used by the
    /// encrypted-env path, which can't go through the JSON parse in `get`
    /// because the columns hold base64 ciphertext.
    pub async fn get_raw(
        pool: &SqlitePool,
        request_id: &RequestId,
        env_id: &EnvironmentId,
    ) -> Result<Option<RawOverrideRow>> {
        let row = sqlx::query_as::<_, RawRow>(
            "SELECT request_id, environment_id, \
                    method_override, url_override, headers_override_json, \
                    params_override_json, body_type_override, body_content_override, \
                    auth_override_json \
             FROM request_env_overrides WHERE request_id = ? AND environment_id = ?",
        )
        .bind(request_id.as_str())
        .bind(env_id.as_str())
        .fetch_optional(pool)
        .await?;
        Ok(row.map(RawRow::into_public))
    }

    /// Returns every override row for an env, as the raw TEXT columns —
    /// callers needing decryption (env is encrypted) get to peek at the
    /// stored bytes without paying the JSON-decode tax on each field.
    pub async fn list_raw_for_env(
        pool: &SqlitePool,
        env_id: &EnvironmentId,
    ) -> Result<Vec<RawOverrideRow>> {
        let rows = sqlx::query_as::<_, RawRow>(
            "SELECT request_id, environment_id, \
                    method_override, url_override, headers_override_json, \
                    params_override_json, body_type_override, body_content_override, \
                    auth_override_json \
             FROM request_env_overrides WHERE environment_id = ?",
        )
        .bind(env_id.as_str())
        .fetch_all(pool)
        .await?;
        Ok(rows.into_iter().map(RawRow::into_public).collect())
    }

    /// Tx-aware bulk replacement of one override row's text columns. Used by
    /// enable/disable-encryption flows. `request_id` keys the row.
    pub async fn rewrite_row_in_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        env_id: &EnvironmentId,
        request_id: &RequestId,
        cols: &RawOverrideCols,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE request_env_overrides SET \
                method_override = ?, \
                url_override = ?, \
                headers_override_json = ?, \
                params_override_json = ?, \
                body_type_override = ?, \
                body_content_override = ?, \
                auth_override_json = ? \
             WHERE request_id = ? AND environment_id = ?",
        )
        .bind(cols.method.as_deref())
        .bind(cols.url.as_deref())
        .bind(cols.headers_json.as_deref())
        .bind(cols.params_json.as_deref())
        .bind(cols.body_type.as_deref())
        .bind(cols.body_content.as_deref())
        .bind(cols.auth_json.as_deref())
        .bind(request_id.as_str())
        .bind(env_id.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

/// The override row's seven text columns, exactly as stored. Either all
/// plaintext or all ciphertext-base64 — interpretation is the command
/// layer's job.
#[derive(Debug, Clone, Default)]
pub struct RawOverrideCols {
    pub method: Option<String>,
    pub url: Option<String>,
    pub headers_json: Option<String>,
    pub params_json: Option<String>,
    pub body_type: Option<String>,
    pub body_content: Option<String>,
    pub auth_json: Option<String>,
}

impl RawOverrideCols {
    /// Parse this row's plaintext columns into the typed [`EnvOverride`].
    /// Errors come from malformed JSON or unknown `method`/`body_type` strings.
    pub fn into_domain(
        self,
        request_id: RequestId,
        environment_id: EnvironmentId,
    ) -> Result<EnvOverride> {
        let method = self.method.as_deref().map(parse_method).transpose()?;
        let body_type = self.body_type.as_deref().map(parse_body_type).transpose()?;
        let headers: Option<Vec<KeyValue>> = self
            .headers_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?;
        let params: Option<Vec<KeyValue>> = self
            .params_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?;
        let auth: Option<AuthConfig> = self
            .auth_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?;
        Ok(EnvOverride {
            request_id,
            environment_id,
            method,
            url: self.url,
            headers,
            params,
            body_type,
            body_content: self.body_content,
            auth,
        })
    }

    /// Inverse of [`Self::into_domain`] — serialise typed overrides back into
    /// the raw column layout (no encryption applied; that's a layer up).
    pub fn from_domain(patch: &EnvOverride) -> Result<Self> {
        Ok(Self {
            method: patch.method.map(|m| http_method_str(m).to_string()),
            url: patch.url.clone(),
            headers_json: patch
                .headers
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
            params_json: patch
                .params
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
            body_type: patch.body_type.map(|b| body_type_str(b).to_string()),
            body_content: patch.body_content.clone(),
            auth_json: patch.auth.as_ref().map(serde_json::to_string).transpose()?,
        })
    }
}

/// One override row's identity + raw columns.
#[derive(Debug, Clone)]
pub struct RawOverrideRow {
    pub request_id: RequestId,
    pub environment_id: EnvironmentId,
    pub cols: RawOverrideCols,
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

#[derive(sqlx::FromRow)]
struct RawRow {
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

impl RawRow {
    fn into_public(self) -> RawOverrideRow {
        RawOverrideRow {
            request_id: RequestId::from_trusted(self.request_id),
            environment_id: EnvironmentId::from_trusted(self.environment_id),
            cols: RawOverrideCols {
                method: self.method_override,
                url: self.url_override,
                headers_json: self.headers_override_json,
                params_json: self.params_override_json,
                body_type: self.body_type_override,
                body_content: self.body_content_override,
                auth_json: self.auth_override_json,
            },
        }
    }
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
