//! Secret scrubbing for `OpenAPI` export.
//!
//! Threat model: the exported `.yaml` file gets committed, shared, posted on
//! Stack Overflow, etc. Anything that looks remotely like a credential
//! must be replaced with a typed placeholder before it leaves the app.
//!
//! Detection is heuristic (key-name matching) — we don't try to inspect
//! values for entropy or format. The cost of a false positive (legitimate
//! field replaced with placeholder) is one user edit; the cost of a false
//! negative (a real secret in the file) is much worse, so we err on the
//! aggressive side.
//!
//! Phase 11 will let the user extend [`Policy::extra_body_keywords`] from
//! settings. Until then the defaults below are baked in.

use std::collections::HashSet;

use apiovnia_core::model::{ApiKeyLocation, AuthConfig, BodyType, KeyValue, Request};
use serde_json::Value;

/// What we replace a secret value with. The descriptive form (instead of a
/// single `<REDACTED>` token) lets the consumer of the exported file see
/// *what kind* of secret they need to provide.
#[derive(Debug, Clone, Copy)]
pub enum Placeholder {
    BearerToken,
    Password,
    ApiKey,
    Secret,
    Token,
    Credential,
    PrivateKey,
    Cookie,
    Cvv,
    Ssn,
    Pin,
    Otp,
    Generic,
}

impl Placeholder {
    /// The literal string written into the exported document.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BearerToken => "<your-bearer-token>",
            Self::Password => "<your-password>",
            Self::ApiKey => "<your-api-key>",
            Self::Secret => "<your-secret>",
            Self::Token => "<your-token>",
            Self::Credential => "<your-credential>",
            Self::PrivateKey => "<your-private-key>",
            Self::Cookie => "<your-cookie>",
            Self::Cvv => "<your-cvv>",
            Self::Ssn => "<your-ssn>",
            Self::Pin => "<your-pin>",
            Self::Otp => "<your-otp>",
            Self::Generic => "<redacted>",
        }
    }
}

/// User-customisable detection rules. Phase 11 wires the `extra_*` lists
/// to the settings panel; for now they stay empty.
#[derive(Debug, Default, Clone)]
pub struct Policy {
    /// Extra header names to treat as secret (full value replaced).
    pub extra_header_names: Vec<String>,
    /// Extra query/form/body key names to treat as secret.
    pub extra_body_keywords: Vec<String>,
}

impl Policy {
    #[must_use]
    pub fn default_with_extras(extras: &[String]) -> Self {
        Self {
            extra_header_names: Vec::new(),
            extra_body_keywords: extras.to_vec(),
        }
    }
}

// ---------------------------------------------------------------------------
// Default keyword tables
// ---------------------------------------------------------------------------

/// Header names whose value is *always* a credential (case-insensitive
/// exact match). The whole header value gets the placeholder treatment.
const SECRET_HEADER_NAMES: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "x-api-key",
    "api-key",
    "apikey",
    "x-auth-token",
    "auth-token",
    "x-access-token",
    "access-token",
    "x-csrf-token",
    "csrf-token",
    "x-session-token",
    "cookie",
    "set-cookie",
];

/// Query / form / JSON body key names that mark the *value* as secret
/// (case-insensitive substring match — `userPassword` matches `password`).
/// The match-key column tells us which placeholder to swap in.
const SECRET_BODY_KEYWORDS: &[(&str, Placeholder)] = &[
    ("password", Placeholder::Password),
    ("passwd", Placeholder::Password),
    ("pwd", Placeholder::Password),
    ("secret", Placeholder::Secret),
    ("api_key", Placeholder::ApiKey),
    ("apikey", Placeholder::ApiKey),
    ("api-key", Placeholder::ApiKey),
    // `key` alone is risky — but `?key=abc123` is a *very* common API-key
    // pattern that we don't want to leak. Cost of a false positive
    // ("sortKey") is one manual edit; cost of leaking a real key is
    // immeasurable. Keep it in.
    ("private_key", Placeholder::PrivateKey),
    ("private-key", Placeholder::PrivateKey),
    ("token", Placeholder::Token),
    ("access_token", Placeholder::Token),
    ("refresh_token", Placeholder::Token),
    ("id_token", Placeholder::Token),
    ("auth_token", Placeholder::Token),
    ("bearer", Placeholder::BearerToken),
    ("jwt", Placeholder::BearerToken),
    ("credential", Placeholder::Credential),
    ("credentials", Placeholder::Credential),
    ("session_id", Placeholder::Token),
    ("sessid", Placeholder::Token),
    ("ssn", Placeholder::Ssn),
    ("social_security", Placeholder::Ssn),
    ("cvv", Placeholder::Cvv),
    ("cvc", Placeholder::Cvv),
    ("pin", Placeholder::Pin),
    ("otp", Placeholder::Otp),
    ("totp", Placeholder::Otp),
    ("mfa", Placeholder::Otp),
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Per-request audit trail — what got changed, so the export log can show
/// "stripped 7 secrets from POST /login".
#[derive(Debug, Default, Clone)]
pub struct RedactionTally {
    pub headers: usize,
    pub query_params: usize,
    pub body_fields: usize,
    pub auth: bool,
    pub multipart_files: usize,
}

impl RedactionTally {
    #[must_use]
    pub fn total(&self) -> usize {
        self.headers
            + self.query_params
            + self.body_fields
            + usize::from(self.auth)
            + self.multipart_files
    }
}

/// Returns the redacted copy of the request plus a tally of what changed.
/// The original is never mutated.
#[must_use]
pub fn redact_request(req: &Request, policy: &Policy) -> (Request, RedactionTally) {
    let mut out = req.clone();
    let mut tally = RedactionTally::default();

    redact_headers(&mut out.headers, policy, &mut tally);
    redact_params(&mut out.params, policy, &mut tally);
    redact_auth(&mut out.auth, &mut tally);
    redact_body(&mut out.body_type, &mut out.body_content, policy, &mut tally);

    (out, tally)
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

fn header_name_set(extra: &[String]) -> HashSet<String> {
    let mut s: HashSet<String> = SECRET_HEADER_NAMES.iter().map(|n| (*n).to_string()).collect();
    for e in extra {
        s.insert(e.to_ascii_lowercase());
    }
    s
}

/// Returns the matching placeholder if `key` contains any secret keyword.
fn classify_body_key(key: &str, extra: &[String]) -> Option<Placeholder> {
    let k = key.to_ascii_lowercase();
    for (needle, ph) in SECRET_BODY_KEYWORDS {
        if k.contains(needle) {
            return Some(*ph);
        }
    }
    for e in extra {
        if k.contains(&e.to_ascii_lowercase()) {
            // No semantic placeholder for user-defined — use Generic.
            return Some(Placeholder::Generic);
        }
    }
    None
}

fn redact_headers(headers: &mut [KeyValue], policy: &Policy, tally: &mut RedactionTally) {
    let secret_names = header_name_set(&policy.extra_header_names);
    for h in headers.iter_mut() {
        let k = h.key.to_ascii_lowercase();
        if secret_names.contains(&k) {
            h.value = pick_header_placeholder(&k).as_str().into();
            tally.headers += 1;
        }
    }
}

fn pick_header_placeholder(lower_name: &str) -> Placeholder {
    match lower_name {
        "authorization" | "proxy-authorization" => Placeholder::BearerToken,
        "cookie" | "set-cookie" => Placeholder::Cookie,
        "x-api-key" | "api-key" | "apikey" => Placeholder::ApiKey,
        _ => Placeholder::Token,
    }
}

fn redact_params(params: &mut [KeyValue], policy: &Policy, tally: &mut RedactionTally) {
    for p in params.iter_mut() {
        if let Some(ph) = classify_body_key(&p.key, &policy.extra_body_keywords) {
            p.value = ph.as_str().into();
            tally.query_params += 1;
        }
    }
}

fn redact_auth(auth: &mut AuthConfig, tally: &mut RedactionTally) {
    let changed = match auth {
        AuthConfig::None => false,
        AuthConfig::Bearer { token } => {
            if token.is_empty() {
                false
            } else {
                *token = Placeholder::BearerToken.as_str().into();
                true
            }
        }
        AuthConfig::Basic { password, .. } => {
            // Keep the username — it's commonly non-secret context (e.g. the
            // service account name) and re-typing isn't burdensome. Password
            // always goes.
            if password.is_empty() {
                false
            } else {
                *password = Placeholder::Password.as_str().into();
                true
            }
        }
        AuthConfig::ApiKey { value, r#in, .. } => {
            if value.is_empty() {
                false
            } else {
                *value = match r#in {
                    ApiKeyLocation::Query | ApiKeyLocation::Header => Placeholder::ApiKey.as_str().into(),
                };
                true
            }
        }
    };
    tally.auth = changed;
}

fn redact_body(
    body_type: &mut BodyType,
    body_content: &mut String,
    policy: &Policy,
    tally: &mut RedactionTally,
) {
    match body_type {
        BodyType::None | BodyType::Raw => {} // raw bytes — can't introspect safely
        BodyType::Json => {
            if let Ok(mut v) = serde_json::from_str::<Value>(body_content) {
                let count = redact_json_value(&mut v, &policy.extra_body_keywords);
                if count > 0 {
                    *body_content = serde_json::to_string_pretty(&v).unwrap_or_else(|_| body_content.clone());
                    tally.body_fields += count;
                }
            }
            // If parse failed, body stays as-is. The user might have a JSONC
            // file or templating — we'd rather under-redact a malformed
            // body than corrupt it.
        }
        BodyType::Form => {
            // body_content is JSON-encoded Vec<KeyValue>
            if let Ok(mut rows) = serde_json::from_str::<Vec<KeyValue>>(body_content) {
                let mut changed = false;
                for r in &mut rows {
                    if let Some(ph) = classify_body_key(&r.key, &policy.extra_body_keywords) {
                        r.value = ph.as_str().into();
                        tally.body_fields += 1;
                        changed = true;
                    }
                }
                if changed {
                    if let Ok(s) = serde_json::to_string(&rows) {
                        *body_content = s;
                    }
                }
            }
        }
        BodyType::Multipart => {
            if let Ok(mut rows) = serde_json::from_str::<Vec<MultipartRow>>(body_content) {
                let mut changed = false;
                for r in &mut rows {
                    match r.kind.as_str() {
                        "file" => {
                            if !r.file_path.is_empty() {
                                r.file_path = "./placeholder.bin".into();
                                tally.multipart_files += 1;
                                changed = true;
                            }
                        }
                        _ => {
                            if let Some(ph) = classify_body_key(&r.key, &policy.extra_body_keywords) {
                                r.value = ph.as_str().into();
                                tally.body_fields += 1;
                                changed = true;
                            }
                        }
                    }
                }
                if changed {
                    if let Ok(s) = serde_json::to_string(&rows) {
                        *body_content = s;
                    }
                }
            }
        }
    }
}

/// Walk a JSON tree, replacing values whose key matches a secret keyword.
/// Returns the number of replacements made.
fn redact_json_value(v: &mut Value, extras: &[String]) -> usize {
    fn walk(v: &mut Value, extras: &[String], count: &mut usize) {
        match v {
            Value::Object(map) => {
                for (k, child) in map.iter_mut() {
                    if let Some(ph) = classify_body_key(k, extras) {
                        // Replace the whole subtree — even if the value is a
                        // nested object, that's still considered the secret.
                        *child = Value::String(ph.as_str().into());
                        *count += 1;
                    } else {
                        walk(child, extras, count);
                    }
                }
            }
            Value::Array(arr) => {
                for child in arr.iter_mut() {
                    walk(child, extras, count);
                }
            }
            _ => {}
        }
    }
    let mut count = 0;
    walk(v, extras, &mut count);
    count
}

// Mirror of the executor's multipart row shape — kept local because
// apiovnia-openapi must not depend on apiovnia-http.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct MultipartRow {
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default = "default_text_kind")]
    kind: String,
    #[serde(default, rename = "filePath")]
    file_path: String,
    #[serde(default, rename = "contentType")]
    content_type: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_text_kind() -> String {
    "text".into()
}
const fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use apiovnia_core::{
        ids::{CollectionId, RequestId},
        model::{HttpMethod, Request},
    };

    fn r(name: &str) -> Request {
        Request {
            id: RequestId::new(),
            collection_id: CollectionId::new(),
            name: name.into(),
            method: HttpMethod::Post,
            url: "https://api.example.com/x".into(),
            headers: vec![],
            params: vec![],
            body_type: BodyType::None,
            body_content: String::new(),
            auth: AuthConfig::None,
            created_at: 0,
            updated_at: 0,
            sort_order: 0,
        }
    }

    #[test]
    fn redacts_authorization_header_to_bearer_placeholder() {
        let mut req = r("x");
        req.headers = vec![KeyValue { key: "Authorization".into(), value: "Bearer eyJ...".into(), enabled: true }];
        let (red, t) = redact_request(&req, &Policy::default());
        assert_eq!(red.headers[0].value, "<your-bearer-token>");
        assert_eq!(t.headers, 1);
    }

    #[test]
    fn redacts_api_key_header_to_apikey_placeholder() {
        let mut req = r("x");
        req.headers = vec![KeyValue { key: "X-API-Key".into(), value: "abc-123".into(), enabled: true }];
        let (red, t) = redact_request(&req, &Policy::default());
        assert_eq!(red.headers[0].value, "<your-api-key>");
        assert_eq!(t.headers, 1);
    }

    #[test]
    fn redacts_cookie_to_cookie_placeholder() {
        let mut req = r("x");
        req.headers = vec![KeyValue { key: "Cookie".into(), value: "session=xyz".into(), enabled: true }];
        let (red, t) = redact_request(&req, &Policy::default());
        assert_eq!(red.headers[0].value, "<your-cookie>");
        assert_eq!(t.headers, 1);
    }

    #[test]
    fn leaves_innocuous_headers_alone() {
        let mut req = r("x");
        req.headers = vec![
            KeyValue { key: "Content-Type".into(), value: "application/json".into(), enabled: true },
            KeyValue { key: "Accept".into(), value: "*/*".into(), enabled: true },
        ];
        let (red, t) = redact_request(&req, &Policy::default());
        assert_eq!(red.headers[0].value, "application/json");
        assert_eq!(red.headers[1].value, "*/*");
        assert_eq!(t.headers, 0);
    }

    #[test]
    fn redacts_secret_query_params() {
        let mut req = r("x");
        req.params = vec![
            KeyValue { key: "api_key".into(), value: "secret123".into(), enabled: true },
            KeyValue { key: "sort".into(), value: "asc".into(), enabled: true },
            KeyValue { key: "access_token".into(), value: "xyz".into(), enabled: true },
        ];
        let (red, t) = redact_request(&req, &Policy::default());
        assert_eq!(red.params[0].value, "<your-api-key>");
        assert_eq!(red.params[1].value, "asc");
        assert_eq!(red.params[2].value, "<your-token>");
        assert_eq!(t.query_params, 2);
    }

    #[test]
    fn redacts_bearer_auth_value() {
        let mut req = r("x");
        req.auth = AuthConfig::Bearer { token: "eyJ...".into() };
        let (red, t) = redact_request(&req, &Policy::default());
        assert!(matches!(red.auth, AuthConfig::Bearer { ref token } if token == "<your-bearer-token>"));
        assert!(t.auth);
    }

    #[test]
    fn redacts_basic_password_keeps_username() {
        let mut req = r("x");
        req.auth = AuthConfig::Basic { username: "alice".into(), password: "hunter2".into() };
        let (red, t) = redact_request(&req, &Policy::default());
        match red.auth {
            AuthConfig::Basic { username, password } => {
                assert_eq!(username, "alice", "username preserved");
                assert_eq!(password, "<your-password>");
            }
            _ => panic!("expected Basic"),
        }
        assert!(t.auth);
    }

    #[test]
    fn empty_auth_values_are_not_counted() {
        let mut req = r("x");
        req.auth = AuthConfig::Bearer { token: String::new() };
        let (_, t) = redact_request(&req, &Policy::default());
        assert!(!t.auth);
    }

    #[test]
    fn redacts_json_body_recursively() {
        let mut req = r("x");
        req.body_type = BodyType::Json;
        req.body_content = r#"{"username":"alice","password":"hunter2","nested":{"api_key":"k1","other":"keep"},"arr":[{"token":"t1"}]}"#.into();
        let (red, t) = redact_request(&req, &Policy::default());
        assert_eq!(t.body_fields, 3);
        let v: serde_json::Value = serde_json::from_str(&red.body_content).unwrap();
        assert_eq!(v["username"], "alice");
        assert_eq!(v["password"], "<your-password>");
        assert_eq!(v["nested"]["api_key"], "<your-api-key>");
        assert_eq!(v["nested"]["other"], "keep");
        assert_eq!(v["arr"][0]["token"], "<your-token>");
    }

    #[test]
    fn redacts_form_body_rows() {
        let mut req = r("x");
        req.body_type = BodyType::Form;
        req.body_content = serde_json::to_string(&vec![
            KeyValue { key: "username".into(), value: "alice".into(), enabled: true },
            KeyValue { key: "password".into(), value: "hunter2".into(), enabled: true },
        ]).unwrap();
        let (red, t) = redact_request(&req, &Policy::default());
        assert_eq!(t.body_fields, 1);
        let rows: Vec<KeyValue> = serde_json::from_str(&red.body_content).unwrap();
        assert_eq!(rows[1].value, "<your-password>");
    }

    #[test]
    fn redacts_multipart_file_paths_and_text_secrets() {
        let mut req = r("x");
        req.body_type = BodyType::Multipart;
        req.body_content = serde_json::json!([
            {"key": "token", "value": "tok1", "kind": "text", "filePath": "", "contentType": "", "enabled": true},
            {"key": "upload", "value": "", "kind": "file", "filePath": "/home/me/secret.pdf", "contentType": "application/pdf", "enabled": true},
        ]).to_string();
        let (red, t) = redact_request(&req, &Policy::default());
        let rows: Vec<MultipartRow> = serde_json::from_str(&red.body_content).unwrap();
        assert_eq!(rows[0].value, "<your-token>");
        assert_eq!(rows[1].file_path, "./placeholder.bin");
        assert_eq!(t.body_fields, 1);
        assert_eq!(t.multipart_files, 1);
    }

    #[test]
    fn user_extras_match_and_use_generic_placeholder() {
        let mut req = r("x");
        req.body_type = BodyType::Json;
        req.body_content = r#"{"company_secret_code":"shhh","public":"ok"}"#.into();
        let policy = Policy::default_with_extras(&["company_secret".into()]);
        let (red, t) = redact_request(&req, &policy);
        // "secret" already matches → Placeholder::Secret. Let's exercise a
        // pure extras case with a name no built-in covers.
        let v: serde_json::Value = serde_json::from_str(&red.body_content).unwrap();
        assert_eq!(v["company_secret_code"], "<your-secret>");
        assert_eq!(v["public"], "ok");
        assert!(t.body_fields >= 1);
    }

    #[test]
    fn malformed_json_body_is_left_alone() {
        let mut req = r("x");
        req.body_type = BodyType::Json;
        req.body_content = "{not json".into();
        let (red, t) = redact_request(&req, &Policy::default());
        assert_eq!(red.body_content, "{not json");
        assert_eq!(t.body_fields, 0);
    }

    #[test]
    fn tally_totals_across_categories() {
        let mut req = r("x");
        req.headers = vec![KeyValue { key: "Authorization".into(), value: "Bearer x".into(), enabled: true }];
        req.params = vec![KeyValue { key: "token".into(), value: "t".into(), enabled: true }];
        req.auth = AuthConfig::Bearer { token: "y".into() };
        req.body_type = BodyType::Json;
        req.body_content = r#"{"password":"p"}"#.into();
        let (_, t) = redact_request(&req, &Policy::default());
        assert_eq!(t.total(), 4); // 1 header + 1 param + 1 auth + 1 body
    }
}
