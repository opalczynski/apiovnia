//! `Executor` — owns a shared `reqwest::Client` and runs a domain `Request`
//! against the wire.

use std::time::{Duration, Instant};

use apiovnia_core::model::{AuthConfig, BodyType, HttpMethod, KeyValue, Request};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Method, Url};

use crate::error::{ExecutionError, Result};
use crate::result::{ExecutionResult, HeaderEntry, ResponseBodyKind, SentRequest};
use crate::BODY_LIMIT_BYTES;

/// Cap on how much of the outgoing body we echo back to the frontend in the
/// `SentRequest.body_preview`. Plenty for debugging; we don't want to ship
/// a multi-megabyte upload twice across the IPC boundary.
const SENT_BODY_PREVIEW_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    pub request_timeout: Duration,
    /// Max redirects to follow (default 10). 0 disables.
    pub max_redirects: usize,
    /// User-Agent header value sent on every request.
    pub user_agent: String,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(30),
            max_redirects: 10,
            user_agent: format!("Apiovnia/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Executor {
    client: reqwest::Client,
    max_redirects: usize,
}

impl Executor {
    pub fn new(cfg: &ExecutorConfig) -> Result<Self> {
        // We always disable reqwest's built-in redirect follower and walk the
        // chain ourselves (see `send_following_redirects`). reqwest only sheds
        // Authorization / Cookie / Proxy-Authorization on a cross-origin hop,
        // which leaks any *custom* secret header (an ApiKey-in-Header, or any
        // value the user set on the Headers tab) to the redirect target. Owning
        // the loop lets us strip those too.
        let client = reqwest::Client::builder()
            .timeout(cfg.request_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(&cfg.user_agent)
            .build()?;
        Ok(Self {
            client,
            max_redirects: cfg.max_redirects,
        })
    }

    pub async fn execute(&self, req: &Request) -> Result<ExecutionResult> {
        let started = Instant::now();

        let mut url = Url::parse(req.url.trim())?;
        attach_query_params(&mut url, &req.params);

        // GraphQL-over-HTTP GET: the {query, variables} envelope rides the URL
        // query string, not a body (spec — GET stays safe/idempotent, queries
        // only). Must happen before the builder captures the URL below.
        if req.body_type == BodyType::GraphQl && req.method == HttpMethod::Get {
            let params = apiovnia_core::graphql::GraphQlBody::parse(&req.body_content)
                .to_get_query_params_checked()
                .map_err(ExecutionError::InvalidRequest)?;
            for (k, v) in params {
                url.query_pairs_mut().append_pair(k, &v);
            }
        }

        let method = method_to_reqwest(req.method);

        let mut builder = self.client.request(method.clone(), url.clone());
        builder = builder.headers(build_headers(&req.headers)?);
        builder = apply_auth(builder, &req.auth, &mut url)?;

        // Snapshot the pre-body state — auth + headers fully applied. We use
        // this for multipart, since reqwest stores the multipart body as a
        // `Body::stream(...)` and `RequestBuilder::try_clone()` returns
        // `None` for streaming bodies (so the post-body try_clone would lose
        // headers + body). For non-streaming bodies we keep using the
        // post-body probe below — it picks up the body bytes for the preview.
        let pre_body_probe = builder.try_clone().and_then(|b| b.build().ok());

        builder = apply_body(builder, req).await?;

        let sent = if req.body_type == BodyType::Multipart {
            synthesize_multipart_snapshot(pre_body_probe.as_ref(), req, &method, &url)
        } else {
            match builder.try_clone().and_then(|b| b.build().ok()) {
                Some(probe) => snapshot_sent(&probe),
                None => SentRequest {
                    method: method.to_string(),
                    url: url.to_string(),
                    headers: Vec::new(),
                    body_preview: String::new(),
                    body_size_bytes: 0,
                },
            }
        };

        let resp = self
            .send_following_redirects(req, builder, &method, url)
            .await?;
        let status = resp.status();
        let final_url = resp.url().to_string();

        let headers: Vec<HeaderEntry> = resp
            .headers()
            .iter()
            .map(|(n, v)| HeaderEntry {
                name: n.as_str().to_ascii_lowercase(),
                value: v.to_str().unwrap_or("").to_string(),
            })
            .collect();

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(';').next().unwrap_or(s).trim().to_ascii_lowercase());

        let bytes = resp.bytes().await?;
        let total_size = bytes.len() as u64;
        let truncated = bytes.len() > BODY_LIMIT_BYTES;
        let body_bytes = if truncated {
            &bytes[..BODY_LIMIT_BYTES]
        } else {
            &bytes[..]
        };

        let (body_kind, body) = if body_bytes.is_empty() {
            (ResponseBodyKind::Empty, String::new())
        } else if is_text_like(content_type.as_deref()) {
            match std::str::from_utf8(body_bytes) {
                Ok(s) => (ResponseBodyKind::Text, s.to_string()),
                Err(_) => (
                    ResponseBodyKind::BinaryBase64,
                    base64_encode(body_bytes),
                ),
            }
        } else {
            (ResponseBodyKind::BinaryBase64, base64_encode(body_bytes))
        };

        Ok(ExecutionResult {
            status: status.as_u16(),
            status_text: status.canonical_reason().unwrap_or("").to_string(),
            headers,
            content_type,
            body_kind,
            body,
            body_truncated: truncated,
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            size_bytes: total_size,
            final_url,
            sent,
        })
    }

    /// Send `initial` and walk the redirect chain by hand. Each hop is built
    /// fresh from the domain `req`, which lets us decide per-hop whether to
    /// attach the request's secrets.
    ///
    /// Once any hop crosses to a different origin (host / port / scheme) we
    /// latch `sensitive_stripped` and never re-attach the auth block or the
    /// user-set headers for the rest of the chain — so a `302 → attacker.com`
    /// can't harvest an `X-Api-Key` or any other Headers-tab value. (The body
    /// for 307/308 is preserved per HTTP semantics; secrets baked into a
    /// request *body* are a separate surface tracked elsewhere.)
    async fn send_following_redirects(
        &self,
        req: &Request,
        initial: reqwest::RequestBuilder,
        initial_method: &Method,
        initial_url: Url,
    ) -> Result<reqwest::Response> {
        let mut resp = initial.send().await?;
        let mut current_url = initial_url;
        let mut current_method = initial_method.clone();
        let mut sensitive_stripped = false;
        let mut hops: usize = 0;

        loop {
            let status = resp.status().as_u16();
            if self.max_redirects == 0 || !is_redirect_status(status) {
                return Ok(resp);
            }

            // A 3xx with no usable Location header isn't actionable — hand the
            // response back as-is rather than erroring.
            let Some(location) = resp
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
            else {
                return Ok(resp);
            };

            let next_url = current_url.join(location)?;
            if next_url.scheme() != "http" && next_url.scheme() != "https" {
                return Err(ExecutionError::InvalidRequest(format!(
                    "redirect to unsupported scheme {:?}",
                    next_url.scheme()
                )));
            }
            if hops >= self.max_redirects {
                return Err(ExecutionError::TooManyRedirects(self.max_redirects));
            }

            // Compare against the hop we just made (reqwest's own semantics).
            // The latch is sticky: a later hop pointing back at the origin
            // still won't get the secrets re-attached.
            if is_cross_origin(&current_url, &next_url) {
                sensitive_stripped = true;
            }

            let (next_method, keep_body) = redirect_transition(status, &current_method);

            let mut hop_url = next_url.clone();
            let mut builder = self.client.request(next_method.clone(), hop_url.clone());
            if !sensitive_stripped {
                builder = builder.headers(build_headers(&req.headers)?);
                builder = apply_auth(builder, &req.auth, &mut hop_url)?;
            }
            if keep_body {
                builder = apply_body(builder, req).await?;
            }

            resp = builder.send().await?;
            current_url = next_url;
            current_method = next_method;
            hops += 1;
        }
    }
}

/// Redirect status codes we follow. Anything else (incl. `300 Multiple
/// Choices`, which has no canonical Location) is returned to the caller.
const fn is_redirect_status(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// A redirect leaves the trusted origin when host, effective port, or scheme
/// changes. The scheme check catches an `https → http` downgrade even when the
/// explicit port number stays the same.
fn is_cross_origin(from: &Url, to: &Url) -> bool {
    from.host_str() != to.host_str()
        || from.port_or_known_default() != to.port_or_known_default()
        || from.scheme() != to.scheme()
}

/// The `(method, keep_body)` the next hop should use, mirroring browser/reqwest
/// behaviour:
///   - 303 → always GET, drop body
///   - 301/302 → downgrade non-GET/HEAD to GET and drop body
///   - 307/308 → preserve both method and body
fn redirect_transition(status: u16, method: &Method) -> (Method, bool) {
    match status {
        307 | 308 => (method.clone(), true),
        303 => (Method::GET, false),
        // 301 / 302 (and anything else we got here with).
        _ => {
            if *method == Method::GET || *method == Method::HEAD {
                (method.clone(), false)
            } else {
                (Method::GET, false)
            }
        }
    }
}

fn snapshot_sent(req: &reqwest::Request) -> SentRequest {
    let headers = req
        .headers()
        .iter()
        .map(|(n, v)| HeaderEntry {
            name: n.as_str().to_string(),
            value: v.to_str().unwrap_or("<binary>").to_string(),
        })
        .collect();

    let (body_preview, body_size_bytes) = match req.body().and_then(reqwest::Body::as_bytes) {
        Some(bytes) => {
            let size = bytes.len() as u64;
            let slice = if bytes.len() > SENT_BODY_PREVIEW_BYTES {
                &bytes[..SENT_BODY_PREVIEW_BYTES]
            } else {
                bytes
            };
            (String::from_utf8_lossy(slice).into_owned(), size)
        }
        None => (String::new(), 0),
    };

    SentRequest {
        method: req.method().to_string(),
        url: req.url().to_string(),
        headers,
        body_preview,
        body_size_bytes,
    }
}

// ---------------------------------------------------------------------------
// Request building
// ---------------------------------------------------------------------------

const fn method_to_reqwest(m: HttpMethod) -> Method {
    match m {
        HttpMethod::Get => Method::GET,
        HttpMethod::Post => Method::POST,
        HttpMethod::Put => Method::PUT,
        HttpMethod::Patch => Method::PATCH,
        HttpMethod::Delete => Method::DELETE,
        HttpMethod::Head => Method::HEAD,
        HttpMethod::Options => Method::OPTIONS,
    }
}

fn attach_query_params(url: &mut Url, params: &[KeyValue]) {
    for p in params.iter().filter(|p| p.enabled && !p.key.is_empty()) {
        url.query_pairs_mut().append_pair(&p.key, &p.value);
    }
}

fn build_headers(headers: &[KeyValue]) -> Result<HeaderMap> {
    let mut out = HeaderMap::new();
    for h in headers.iter().filter(|h| h.enabled && !h.key.is_empty()) {
        let name = HeaderName::from_bytes(h.key.as_bytes())
            .map_err(|e| ExecutionError::InvalidRequest(format!("invalid header name: {e}")))?;
        let value = HeaderValue::from_str(&h.value)
            .map_err(|e| ExecutionError::InvalidRequest(format!("invalid header value: {e}")))?;
        out.append(name, value);
    }
    Ok(out)
}

fn apply_auth(
    builder: reqwest::RequestBuilder,
    auth: &AuthConfig,
    url: &mut Url,
) -> Result<reqwest::RequestBuilder> {
    Ok(match auth {
        AuthConfig::Bearer { token } if !token.is_empty() => builder.bearer_auth(token),
        AuthConfig::Basic { username, password } => builder.basic_auth(username, Some(password)),
        AuthConfig::ApiKey { name, value, r#in } if !name.is_empty() => match r#in {
            apiovnia_core::model::ApiKeyLocation::Header => {
                let h = HeaderName::from_bytes(name.as_bytes())
                    .map_err(|e| ExecutionError::InvalidRequest(format!("invalid header: {e}")))?;
                let v = HeaderValue::from_str(value).map_err(|e| {
                    ExecutionError::InvalidRequest(format!("invalid header value: {e}"))
                })?;
                builder.header(h, v)
            }
            apiovnia_core::model::ApiKeyLocation::Query => {
                url.query_pairs_mut().append_pair(name, value);
                builder
            }
        },
        // None / empty Bearer / empty ApiKey → send no auth.
        AuthConfig::None | AuthConfig::Bearer { .. } | AuthConfig::ApiKey { .. } => builder,
    })
}

async fn apply_body(
    builder: reqwest::RequestBuilder,
    req: &Request,
) -> Result<reqwest::RequestBuilder> {
    Ok(match req.body_type {
        BodyType::None => builder,
        BodyType::Json => {
            // Send as text so we keep the user's formatting verbatim. Caller
            // is responsible for setting Content-Type if they want it; we set
            // a sensible default only if not already set.
            let needs_ct = !req
                .headers
                .iter()
                .any(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"));
            let b = builder.body(req.body_content.clone());
            if needs_ct {
                b.header(reqwest::header::CONTENT_TYPE, "application/json")
            } else {
                b
            }
        }
        BodyType::GraphQl if req.method == HttpMethod::Get => {
            // GraphQL-over-HTTP GET carries query/variables in the URL
            // (folded in by `execute`) — no body on the request itself.
            builder
        }
        BodyType::GraphQl => {
            // POST-style GraphQL: a JSON body of the `{query, variables}`
            // envelope. We validate the variables here so a malformed block
            // surfaces as a clear error rather than a confusing 400.
            let wire = apiovnia_core::graphql::GraphQlBody::parse(&req.body_content)
                .to_wire_json_checked()
                .map_err(ExecutionError::InvalidRequest)?;
            let needs_ct = !req
                .headers
                .iter()
                .any(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"));
            let b = builder.body(wire);
            if needs_ct {
                b.header(reqwest::header::CONTENT_TYPE, "application/json")
            } else {
                b
            }
        }
        BodyType::Raw => builder.body(req.body_content.clone()),
        BodyType::Form => {
            // The frontend serialises `KeyValue[]` into `bodyContent` as JSON.
            let rows: Vec<KeyValue> = serde_json::from_str(&req.body_content).map_err(|e| {
                ExecutionError::InvalidRequest(format!("invalid form payload: {e}"))
            })?;
            let pairs: Vec<(String, String)> = rows
                .into_iter()
                .filter(|r| r.enabled && !r.key.is_empty())
                .map(|r| (r.key, r.value))
                .collect();
            builder.form(&pairs)
        }
        BodyType::Multipart => apply_multipart(builder, req).await?,
    })
}

/// Multipart row — JSON shape we expect inside `body_content` for
/// `BodyType::Multipart`. Mirrors `MultipartField` on the TypeScript side.
///
/// `kind = "text"` → use `value` as the part body.
/// `kind = "file"` → read `file_path` from disk; optional `content_type`
/// overrides the MIME we'd otherwise guess from the extension.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultipartField {
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default = "default_kind")]
    kind: MultipartKind,
    #[serde(default)]
    file_path: String,
    #[serde(default)]
    content_type: String,
    #[serde(default = "crate::executor::default_true_serde")]
    enabled: bool,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum MultipartKind {
    Text,
    File,
}

fn default_kind() -> MultipartKind {
    MultipartKind::Text
}

pub(crate) const fn default_true_serde() -> bool {
    true
}

async fn apply_multipart(
    builder: reqwest::RequestBuilder,
    req: &Request,
) -> Result<reqwest::RequestBuilder> {
    let rows: Vec<MultipartField> =
        serde_json::from_str(&req.body_content).map_err(|e| {
            ExecutionError::InvalidRequest(format!("invalid multipart payload: {e}"))
        })?;

    let mut form = reqwest::multipart::Form::new();
    for r in rows
        .into_iter()
        .filter(|r| r.enabled && !r.key.is_empty())
    {
        form = match r.kind {
            MultipartKind::Text => form.text(r.key, r.value),
            MultipartKind::File => {
                if r.file_path.is_empty() {
                    return Err(ExecutionError::InvalidRequest(format!(
                        "multipart field {:?} marked as file but has no path",
                        r.key
                    )));
                }
                let path = std::path::Path::new(&r.file_path);
                let file_name = path
                    .file_name()
                    .map_or_else(|| r.key.clone(), |s| s.to_string_lossy().into_owned());
                let bytes = tokio::fs::read(path).await.map_err(|e| {
                    ExecutionError::InvalidRequest(format!(
                        "failed to read multipart file {:?}: {e}",
                        r.file_path
                    ))
                })?;

                // Resolve MIME: explicit override > extension guess > octet-stream.
                let mime = if r.content_type.is_empty() {
                    mime_for_path(path)
                } else {
                    r.content_type.clone()
                };

                let mut part = reqwest::multipart::Part::bytes(bytes).file_name(file_name);
                part = part.mime_str(&mime).map_err(|e| {
                    ExecutionError::InvalidRequest(format!(
                        "invalid mime {mime:?} for {:?}: {e}",
                        r.file_path
                    ))
                })?;
                form.part(r.key, part)
            }
        };
    }

    Ok(builder.multipart(form))
}

/// Synthesize a [`SentRequest`] snapshot for `BodyType::Multipart`. reqwest
/// stores the multipart body as a stream, so `RequestBuilder::try_clone()`
/// returns `None` once we've called `.multipart(...)` — we'd lose both the
/// body and the auto-injected Content-Type. We rebuild the snapshot by hand:
///   - method + URL from the call site,
///   - headers from the pre-body probe (auth applied, no body yet),
///   - Content-Type with a synthetic boundary appended for the snapshot,
///   - `body_preview` rendered as RFC-7578 multipart, with file contents
///     replaced by `[N bytes — file contents omitted from preview]`.
///
/// The synthetic boundary never reaches the wire — reqwest picks its own.
/// We label ours so it's obvious in the Request tab that this is a
/// reconstruction, not a byte-for-byte capture.
fn synthesize_multipart_snapshot(
    pre_probe: Option<&reqwest::Request>,
    req: &Request,
    method: &reqwest::Method,
    url: &Url,
) -> SentRequest {
    const BOUNDARY: &str = "----apiovnia-snapshot-boundary";

    // Base shape — method/url/headers come from the auth-applied probe so
    // Bearer/Basic/ApiKey-header live in the snapshot too.
    let mut snap = match pre_probe {
        Some(p) => snapshot_sent(p),
        None => SentRequest {
            method: method.to_string(),
            url: url.to_string(),
            headers: req
                .headers
                .iter()
                .filter(|h| h.enabled && !h.key.is_empty())
                .map(|h| HeaderEntry {
                    name: h.key.clone(),
                    value: h.value.clone(),
                })
                .collect(),
            body_preview: String::new(),
            body_size_bytes: 0,
        },
    };

    // Drop any pre-existing user-set Content-Type so the multipart CT below
    // is the only one visible — matches what reqwest would do on the wire.
    snap.headers
        .retain(|h| !h.name.eq_ignore_ascii_case("content-type"));

    let rows: Vec<MultipartField> = serde_json::from_str(&req.body_content).unwrap_or_default();

    let mut body = String::new();
    let mut size: u64 = 0;

    for r in rows.iter().filter(|r| r.enabled && !r.key.is_empty()) {
        match r.kind {
            MultipartKind::Text => {
                let part = format!(
                    "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n",
                    r.key, r.value
                );
                size += part.len() as u64;
                body.push_str(&part);
            }
            MultipartKind::File => {
                let path = std::path::Path::new(&r.file_path);
                let file_size = std::fs::metadata(path).map_or(0, |m| m.len());
                let basename = path
                    .file_name()
                    .map_or_else(|| r.key.clone(), |s| s.to_string_lossy().into_owned());
                let mime = if r.content_type.is_empty() {
                    mime_for_path(path)
                } else {
                    r.content_type.clone()
                };
                let header_part = format!(
                    "--{BOUNDARY}\r\nContent-Disposition: form-data; \
                     name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
                    r.key, basename, mime
                );
                let placeholder = format!(
                    "[{file_size} bytes — file contents omitted from preview]\r\n"
                );
                size += header_part.len() as u64 + file_size;
                body.push_str(&header_part);
                body.push_str(&placeholder);
            }
        }
    }
    let closing = format!("--{BOUNDARY}--\r\n");
    size += closing.len() as u64;
    body.push_str(&closing);

    snap.headers.push(HeaderEntry {
        name: "content-type".into(),
        value: format!("multipart/form-data; boundary={BOUNDARY}"),
    });
    snap.body_preview = body;
    snap.body_size_bytes = size;
    snap
}

/// Tiny MIME guess by extension. We avoid pulling in a full database (`mime_guess`)
/// because users can always set the part content-type explicitly when it matters.
fn mime_for_path(path: &std::path::Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("json") => "application/json",
        Some("xml") => "application/xml",
        Some("yaml" | "yml") => "application/yaml",
        Some("txt") => "text/plain",
        Some("csv") => "text/csv",
        Some("html" | "htm") => "text/html",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("gz" | "tgz") => "application/gzip",
        _ => "application/octet-stream",
    }
    .to_string()
}

// ---------------------------------------------------------------------------
// Body classification
// ---------------------------------------------------------------------------

fn is_text_like(content_type: Option<&str>) -> bool {
    let Some(ct) = content_type else {
        // No CT — assume binary; we'd rather over-encode than corrupt utf-8.
        return false;
    };
    if ct.starts_with("text/") {
        return true;
    }
    matches!(
        ct,
        "application/json"
            | "application/xml"
            | "application/javascript"
            | "application/x-www-form-urlencoded"
            | "application/yaml"
            | "application/x-yaml"
    ) || ct.ends_with("+json")
        || ct.ends_with("+xml")
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_content_types() {
        assert!(is_text_like(Some("application/json")));
        assert!(is_text_like(Some("application/vnd.api+json")));
        assert!(is_text_like(Some("text/html")));
        assert!(!is_text_like(Some("image/png")));
        assert!(!is_text_like(None));
    }

    #[test]
    fn method_translation_is_total() {
        for m in [
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Patch,
            HttpMethod::Delete,
            HttpMethod::Head,
            HttpMethod::Options,
        ] {
            let _ = method_to_reqwest(m);
        }
    }

    #[test]
    fn follows_only_real_redirect_statuses() {
        for s in [301, 302, 303, 307, 308] {
            assert!(is_redirect_status(s), "{s} should be followed");
        }
        for s in [200, 201, 204, 300, 304, 305, 400, 404, 500] {
            assert!(!is_redirect_status(s), "{s} should not be followed");
        }
    }

    #[test]
    fn cross_origin_detects_host_port_and_scheme() {
        let u = |s: &str| Url::parse(s).unwrap();

        // Same origin, different path → not cross-origin.
        assert!(!is_cross_origin(&u("https://api.example.com/a"), &u("https://api.example.com/b")));
        // Default vs explicit-but-equal port stays same-origin.
        assert!(!is_cross_origin(&u("https://h.com/"), &u("https://h.com:443/")));

        // Different host (even a subdomain) → cross-origin.
        assert!(is_cross_origin(&u("https://api.example.com/"), &u("https://evil.com/")));
        assert!(is_cross_origin(&u("https://api.example.com/"), &u("https://cdn.example.com/")));
        // https → http downgrade on the same explicit port is still crossing.
        assert!(is_cross_origin(&u("https://h.com:8443/"), &u("http://h.com:8443/")));
        // Different port.
        assert!(is_cross_origin(&u("http://h.com:8080/"), &u("http://h.com:9090/")));
    }

    #[test]
    fn redirect_transition_matches_http_semantics() {
        // 307/308 preserve method + body.
        assert_eq!(redirect_transition(307, &Method::POST), (Method::POST, true));
        assert_eq!(redirect_transition(308, &Method::PUT), (Method::PUT, true));
        // 303 always downgrades to GET, drops body.
        assert_eq!(redirect_transition(303, &Method::POST), (Method::GET, false));
        // 301/302 downgrade non-GET/HEAD to GET, drop body.
        assert_eq!(redirect_transition(301, &Method::POST), (Method::GET, false));
        assert_eq!(redirect_transition(302, &Method::DELETE), (Method::GET, false));
        // GET/HEAD survive a 301/302 unchanged.
        assert_eq!(redirect_transition(302, &Method::GET), (Method::GET, false));
        assert_eq!(redirect_transition(301, &Method::HEAD), (Method::HEAD, false));
    }
}
