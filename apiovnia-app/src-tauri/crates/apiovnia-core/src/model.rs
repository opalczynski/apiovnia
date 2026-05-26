//! Domain models — projects, collections, requests, environments, overrides.
//!
//! `serde` is configured with `camelCase` everywhere so the TypeScript
//! mirror in `src/lib/types/domain.ts` reads the same JSON unchanged.

use serde::{Deserialize, Serialize};

use crate::ids::{CollectionId, EnvironmentId, ProjectId, RequestId};

// ---------------------------------------------------------------------------
// HTTP primitives
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BodyType {
    #[default]
    None,
    Json,
    /// `application/x-www-form-urlencoded` — text-only key/value pairs.
    Form,
    /// `multipart/form-data` — text OR file per field. Required for uploads.
    Multipart,
    Raw,
    /// GraphQL — `body_content` holds a [`crate::graphql::GraphQlBody`]
    /// (query + variables) as JSON; the executor sends it as a JSON `POST`.
    GraphQl,
}

/// One row in a key/value table (headers, query params, form fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    /// Disabled rows are kept around (so users can toggle them) but
    /// excluded from the resolved request that hits the wire.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

const fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthConfig {
    #[default]
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    #[serde(rename = "apikey")]
    ApiKey {
        name: String,
        value: String,
        /// `header` (default) or `query`.
        #[serde(default = "default_apikey_in")]
        r#in: ApiKeyLocation,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

const fn default_apikey_in() -> ApiKeyLocation {
    ApiKeyLocation::Header
}

// ---------------------------------------------------------------------------
// Entities
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub sort_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: CollectionId,
    pub project_id: ProjectId,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub sort_order: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub id: RequestId,
    pub collection_id: CollectionId,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub params: Vec<KeyValue>,
    #[serde(default)]
    pub body_type: BodyType,
    #[serde(default)]
    pub body_content: String,
    #[serde(default)]
    pub auth: AuthConfig,
    pub created_at: i64,
    pub updated_at: i64,
    pub sort_order: i64,
}

impl Request {
    /// Builds a freshly-created request with sane defaults. The caller is
    /// expected to fill in `id`, `collection_id`, and the timestamps.
    #[must_use]
    pub fn new_blank(
        id: RequestId,
        collection_id: CollectionId,
        name: String,
        now_ms: i64,
        sort_order: i64,
    ) -> Self {
        Self {
            id,
            collection_id,
            name,
            method: HttpMethod::Get,
            url: String::new(),
            headers: Vec::new(),
            params: Vec::new(),
            body_type: BodyType::None,
            body_content: String::new(),
            auth: AuthConfig::None,
            created_at: now_ms,
            updated_at: now_ms,
            sort_order,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: EnvironmentId,
    pub project_id: ProjectId,
    pub name: String,
    /// `true` once the env is sealed behind a master password.
    #[serde(default)]
    pub requires_unlock: bool,
    /// Mirrors `requires_unlock` for now — kept separate so we can later
    /// have unlocked-but-encrypted envs (post-MVP).
    #[serde(default)]
    pub is_encrypted: bool,
    pub created_at: i64,
}

/// A named value usable as `{{name}}` inside URLs, headers, body, and auth.
/// `is_secret` marks values that get encrypted once the parent env is sealed
/// (Phase 6); for Phase 5 it's just a UI hint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVariable {
    pub id: String,
    pub environment_id: EnvironmentId,
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub is_secret: bool,
}

/// Per-`(request, environment)` patch. Every field is optional: `None` means
/// "inherit base from the underlying request". For headers and params,
/// `Some(vec)` is a **full replacement** of the base list (not a per-key merge
/// — that semantics lives in the brief and avoids confusing edge cases).
///
/// Used by [`crate::resolver::resolve_request`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvOverride {
    pub request_id: RequestId,
    pub environment_id: EnvironmentId,
    #[serde(default)]
    pub method: Option<HttpMethod>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub headers: Option<Vec<KeyValue>>,
    #[serde(default)]
    pub params: Option<Vec<KeyValue>>,
    #[serde(default)]
    pub body_type: Option<BodyType>,
    #[serde(default)]
    pub body_content: Option<String>,
    #[serde(default)]
    pub auth: Option<AuthConfig>,
}

impl EnvOverride {
    /// Returns true when every override field is `None` — i.e. the row is
    /// effectively empty and could be deleted from storage.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.method.is_none()
            && self.url.is_none()
            && self.headers.is_none()
            && self.params.is_none()
            && self.body_type.is_none()
            && self.body_content.is_none()
            && self.auth.is_none()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_method_round_trips() {
        let v = serde_json::to_value(HttpMethod::Patch).unwrap();
        assert_eq!(v, serde_json::json!("PATCH"));
        let back: HttpMethod = serde_json::from_value(v).unwrap();
        assert_eq!(back, HttpMethod::Patch);
    }

    #[test]
    fn auth_serializes_with_type_tag() {
        let a = AuthConfig::Bearer {
            token: "tok".into(),
        };
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v, serde_json::json!({"type": "bearer", "token": "tok"}));
    }

    #[test]
    fn auth_default_is_none() {
        let a = AuthConfig::default();
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v, serde_json::json!({"type": "none"}));
    }

    #[test]
    fn key_value_enabled_defaults_to_true_on_deserialize() {
        let kv: KeyValue = serde_json::from_str(r#"{"key":"a","value":"b"}"#).unwrap();
        assert!(kv.enabled);
    }

    #[test]
    fn new_blank_request_has_get_and_no_body() {
        let req = Request::new_blank(
            RequestId::new(),
            CollectionId::new(),
            "My req".into(),
            42,
            7,
        );
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.body_type, BodyType::None);
        assert_eq!(req.created_at, 42);
        assert_eq!(req.sort_order, 7);
    }
}
