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

use crate::model::{ApiKeyLocation, AuthConfig, KeyValue, Request};

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
    #[must_use]
    pub fn render(self, req: &Request) -> String {
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
