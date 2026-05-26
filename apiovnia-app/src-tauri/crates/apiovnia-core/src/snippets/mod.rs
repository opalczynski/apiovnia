//! Render a resolved request as a pasteable snippet in one of several
//! formats: curl, Python (`requests`), `HTTPie`, JavaScript (`fetch`), or
//! `PowerShell` (`Invoke-RestMethod`).
//!
//! All generators are pure functions over [`Request`] — same input, same
//! output, no I/O. They assume the request has already been through
//! `resolve_request` (env override folded + `{{vars}}` interpolated) and
//! that any decryption of encrypted-env fields has happened upstream.
//!
//! Disabled `KeyValue` rows are silently dropped — matches the executor
//! and the user's "checkbox off → not sent" mental model.

use serde::Deserialize;

use crate::graphql::GraphQlBody;
use crate::model::{ApiKeyLocation, AuthConfig, BodyType, HttpMethod, KeyValue, Request};

pub mod curl;
pub mod httpie;
pub mod javascript;
pub mod powershell;
pub mod python;

pub use curl::to_curl;
pub use httpie::to_httpie;
pub use javascript::to_javascript_fetch;
pub use powershell::to_powershell;
pub use python::to_python_requests;

/// The set of supported output formats. The frontend names map 1:1 to
/// these variants over IPC via serde (camelCase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SnippetFormat {
    Curl,
    PythonRequests,
    Httpie,
    JavaScriptFetch,
    PowerShell,
}

impl SnippetFormat {
    /// Dispatch to the right generator. The single public entry point —
    /// IPC, palette, and context menu all funnel through here.
    ///
    /// A `BodyType::GraphQl` request is folded into a plain REST request
    /// first, so every generator stays purely REST-shaped and never has to
    /// know GraphQL exists: a `POST` becomes a JSON body of the
    /// `{query, variables}` envelope; a `GET` moves `query`/`variables` into
    /// the query string (GraphQL-over-HTTP spec).
    #[must_use]
    pub fn render(self, req: &Request) -> String {
        let folded;
        let req = if req.body_type == BodyType::GraphQl {
            let mut r = req.clone();
            let gql = GraphQlBody::parse(&r.body_content);
            if r.method == HttpMethod::Get {
                for (k, v) in gql.to_get_query_params() {
                    r.params.push(KeyValue {
                        key: k.to_string(),
                        value: v,
                        enabled: true,
                    });
                }
                r.body_type = BodyType::None;
                r.body_content = String::new();
            } else {
                r.body_content = gql.to_wire_json();
                r.body_type = BodyType::Json;
            }
            folded = r;
            &folded
        } else {
            req
        };
        match self {
            Self::Curl => to_curl(req),
            Self::PythonRequests => to_python_requests(req),
            Self::Httpie => to_httpie(req),
            Self::JavaScriptFetch => to_javascript_fetch(req),
            Self::PowerShell => to_powershell(req),
        }
    }
}

// ---------------------------------------------------------------------------
// Shared helpers — used across most generators so each file stays focused
// on its language's idioms.
// ---------------------------------------------------------------------------

/// Mirror of the executor's multipart row layout. Local to this module
/// so `apiovnia-core` doesn't have to pull in `apiovnia-http`.
#[derive(Debug, Deserialize)]
pub(crate) struct MultipartRow {
    pub key: String,
    #[serde(default)]
    pub value: String,
    #[serde(default = "default_text_kind")]
    pub kind: String,
    #[serde(default, rename = "filePath")]
    pub file_path: String,
    #[serde(default, rename = "contentType")]
    pub content_type: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_text_kind() -> String {
    "text".into()
}
const fn default_true() -> bool {
    true
}

pub(crate) fn parse_kv_list(json: &str) -> Vec<KeyValue> {
    serde_json::from_str(json).unwrap_or_default()
}

pub(crate) fn parse_multipart_list(json: &str) -> Vec<MultipartRow> {
    serde_json::from_str(json).unwrap_or_default()
}

/// Query params we should emit on the wire = enabled rows plus the
/// `ApiKey { in: Query }` virtual one. Mirrors the curl + executor logic.
pub(crate) fn effective_query_params(req: &Request) -> Vec<(&str, &str)> {
    let mut out: Vec<(&str, &str)> = req
        .params
        .iter()
        .filter(|p| p.enabled)
        .map(|p| (p.key.as_str(), p.value.as_str()))
        .collect();
    if let AuthConfig::ApiKey {
        name,
        value,
        r#in: ApiKeyLocation::Query,
    } = &req.auth
    {
        out.push((name.as_str(), value.as_str()));
    }
    out
}

/// URL with enabled query params folded in, percent-encoded.
pub(crate) fn url_with_params(url: &str, params: &[(&str, &str)]) -> String {
    if params.is_empty() {
        return url.to_string();
    }
    let sep = if url.contains('?') { '&' } else { '?' };
    let qs: Vec<String> = params
        .iter()
        .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
        .collect();
    format!("{url}{sep}{}", qs.join("&"))
}

/// RFC 3986 unreserved-char percent-encoding for query string pieces.
pub(crate) fn percent_encode(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => write!(out, "%{b:02X}").expect("write! into String is infallible"),
        }
    }
    out
}

/// True iff the user already set a `Content-Type` header.
pub(crate) fn user_has_content_type(req: &Request) -> bool {
    req.headers
        .iter()
        .any(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CollectionId, RequestId};
    use crate::model::HttpMethod;

    fn graphql_request() -> Request {
        Request {
            id: RequestId::new(),
            collection_id: CollectionId::new(),
            name: "List users".into(),
            method: HttpMethod::Post,
            url: "https://api.example.com/graphql".into(),
            headers: vec![],
            params: vec![],
            body_type: BodyType::GraphQl,
            body_content: serde_json::json!({
                "query": "query Users($limit: Int) { users(limit: $limit) { id } }",
                "variables": r#"{"limit":5}"#,
            })
            .to_string(),
            auth: AuthConfig::None,
            created_at: 0,
            updated_at: 0,
            sort_order: 0,
        }
    }

    #[test]
    fn render_folds_graphql_into_a_json_post_body() {
        // Every format runs through the same fold — spot-check that the wire
        // envelope (not the internal {query,variables}-as-strings shape) is
        // what each generator emits.
        let req = graphql_request();
        let curl = SnippetFormat::Curl.render(&req);
        assert!(
            curl.contains(r#"{"query":"query Users($limit: Int) { users(limit: $limit) { id } }","variables":{"limit":5}}"#),
            "curl should embed the GraphQL wire body, got: {curl}"
        );
        assert!(
            curl.contains("Content-Type: application/json"),
            "GraphQL POST should advertise JSON, got: {curl}"
        );

        // The other generators must not panic and must mention the operation.
        for fmt in [
            SnippetFormat::PythonRequests,
            SnippetFormat::Httpie,
            SnippetFormat::JavaScriptFetch,
            SnippetFormat::PowerShell,
        ] {
            let out = fmt.render(&req);
            assert!(out.contains("users(limit: $limit)"), "{fmt:?} → {out}");
        }
    }

    #[test]
    fn render_folds_graphql_get_into_query_params() {
        // A GraphQL GET moves query/variables into the URL — no body.
        let mut req = graphql_request();
        req.method = HttpMethod::Get;
        let curl = SnippetFormat::Curl.render(&req);
        assert!(curl.contains("query="), "query should ride the URL, got: {curl}");
        assert!(
            curl.contains("variables=%7B%22limit%22%3A5%7D"),
            "variables should be percent-encoded JSON in the URL, got: {curl}"
        );
        assert!(
            !curl.contains("--data-raw"),
            "a GraphQL GET must not carry a body, got: {curl}"
        );
    }
}
