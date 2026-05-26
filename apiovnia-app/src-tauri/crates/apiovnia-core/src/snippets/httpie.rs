//! Render a resolved request as an `http` (`HTTPie`) command-line.
//!
//! `HTTPie`'s mini-DSL packs headers/params/body into positional args:
//!   * `Name:value`  → header
//!   * `name==value` → query param
//!   * `name=value`  → form field (or `JSON` string field)
//!   * `name:=value` → `JSON` raw (number / bool / nested)
//!
//! We use `--raw '…'` for `JSON` bodies — preserves the body exactly as the
//! user typed it (better than reshaping a parsed value back into `HTTPie`'s
//! key=value DSL). `--form` for urlencoded, `--multipart` + `file@/path`
//! for multipart, `--auth user:pass` for Basic.

use crate::model::{ApiKeyLocation, AuthConfig, BodyType, HttpMethod, Request};

use super::{parse_kv_list, parse_multipart_list};

#[must_use]
pub fn to_httpie(req: &Request) -> String {
    let mut out = String::with_capacity(128);
    out.push_str("http");

    // HTTPie infers GET by default; specify the verb otherwise.
    if req.method != HttpMethod::Get {
        out.push(' ');
        out.push_str(req.method.as_str());
    }

    // Basic auth as a top-level flag.
    if let AuthConfig::Basic { username, password } = &req.auth {
        out.push_str(" \\\n  --auth ");
        out.push_str(&sh_escape(&format!("{username}:{password}")));
    }

    // Body-mode flag — has to come before positional args so HTTPie knows
    // how to interpret them.
    match req.body_type {
        BodyType::Form => out.push_str(" \\\n  --form"),
        BodyType::Multipart => out.push_str(" \\\n  --multipart"),
        _ => {}
    }

    // URL.
    out.push_str(" \\\n  ");
    out.push_str(&sh_escape(&req.url));

    // Auth-derived headers / params before user-supplied ones.
    match &req.auth {
        AuthConfig::None | AuthConfig::Basic { .. } => {}
        AuthConfig::Bearer { token } => {
            push_token(&mut out, "Authorization", &format!("Bearer {token}"), ':');
        }
        AuthConfig::ApiKey {
            name,
            value,
            r#in: ApiKeyLocation::Header,
        } => {
            push_token(&mut out, name, value, ':');
        }
        AuthConfig::ApiKey {
            name,
            value,
            r#in: ApiKeyLocation::Query,
        } => {
            push_token(&mut out, name, value, '=');
            out.push('='); // == for query
        }
    }

    // Query params: `name==value`.
    for p in &req.params {
        if !p.enabled {
            continue;
        }
        out.push_str(" \\\n  ");
        out.push_str(&sh_escape(&format!("{}=={}", p.key, p.value)));
    }

    // Headers: `Name:value`.
    for h in &req.headers {
        if !h.enabled {
            continue;
        }
        push_token(&mut out, &h.key, &h.value, ':');
    }

    // Body.
    match req.body_type {
        BodyType::None => {}
        // GraphQL is folded into a JSON body upstream by `SnippetFormat::render`.
        BodyType::Json | BodyType::GraphQl => {
            // Prefer HTTPie's native JSON shorthand — `key=value` for
            // string values, `key:=jsonval` for everything else (numbers,
            // bools, arrays, objects). More idiomatic than `--raw` for
            // anyone who actually uses HTTPie. Falls back to `--raw`
            // when the body can't be expressed as a flat top-level
            // object (top-level array, malformed JSON, …).
            if let Some(tokens) = json_body_as_httpie_shorthand(&req.body_content) {
                for tok in tokens {
                    out.push_str(" \\\n  ");
                    out.push_str(&sh_escape(&tok));
                }
            } else {
                out.push_str(" \\\n  --raw ");
                out.push_str(&sh_escape(&req.body_content));
            }
        }
        BodyType::Raw => {
            out.push_str(" \\\n  --raw ");
            out.push_str(&sh_escape(&req.body_content));
        }
        BodyType::Form => {
            for kv in parse_kv_list(&req.body_content) {
                if !kv.enabled {
                    continue;
                }
                out.push_str(" \\\n  ");
                out.push_str(&sh_escape(&format!("{}={}", kv.key, kv.value)));
            }
        }
        BodyType::Multipart => {
            for f in parse_multipart_list(&req.body_content) {
                if !f.enabled {
                    continue;
                }
                let token = if f.kind == "file" {
                    // HTTPie's file-upload syntax: `field@/path/to/file`.
                    format!("{}@{}", f.key, f.file_path)
                } else {
                    format!("{}={}", f.key, f.value)
                };
                out.push_str(" \\\n  ");
                out.push_str(&sh_escape(&token));
            }
        }
    }

    out
}

fn push_token(out: &mut String, name: &str, value: &str, sep: char) {
    out.push_str(" \\\n  ");
    out.push_str(&sh_escape(&format!("{name}{sep}{value}")));
}

/// Translate a JSON body into `HTTPie`'s positional-argument DSL.
///
/// Returns `Some(tokens)` only when the body is a JSON *object* — that's
/// the one shape `HTTPie`'s shorthand maps onto. Top-level arrays and
/// non-object types fall through (`None`), so the caller can emit `--raw`.
///
/// Per-property rule:
///   * `Value::String(s)`   → `key=s`   (`HTTPie` treats RHS as string)
///   * everything else       → `key:=jsonval` (`HTTPie` parses RHS as JSON)
fn json_body_as_httpie_shorthand(body: &str) -> Option<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let serde_json::Value::Object(map) = value else {
        return None;
    };
    let mut out = Vec::with_capacity(map.len());
    for (k, v) in map {
        match v {
            serde_json::Value::String(s) => out.push(format!("{k}={s}")),
            other => {
                // Minified — HTTPie wants the raw JSON token, no whitespace.
                let json = serde_json::to_string(&other).ok()?;
                out.push(format!("{k}:={json}"));
            }
        }
    }
    Some(out)
}

/// Same POSIX shell escape as `curl.rs`; copy-local so each generator's
/// shell vs language-string escaping is self-contained.
fn sh_escape(s: &str) -> String {
    if s.is_empty() {
        return "''".into();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CollectionId, RequestId};
    use crate::model::{AuthConfig, KeyValue};

    fn base(method: HttpMethod, url: &str) -> Request {
        Request {
            id: RequestId::new(),
            collection_id: CollectionId::new(),
            name: "r".into(),
            method,
            url: url.into(),
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
    fn plain_get_omits_verb() {
        let h = to_httpie(&base(HttpMethod::Get, "https://x.com"));
        assert!(h.starts_with("http "), "got: {h}");
        assert!(!h.contains(" GET "));
        assert!(h.contains("'https://x.com'"));
    }

    #[test]
    fn post_emits_verb() {
        let h = to_httpie(&base(HttpMethod::Post, "https://x.com"));
        assert!(h.contains("http POST"), "got: {h}");
    }

    #[test]
    fn headers_use_colon_syntax() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.headers = vec![KeyValue { key: "X-A".into(), value: "1".into(), enabled: true }];
        let h = to_httpie(&r);
        assert!(h.contains("'X-A:1'"), "got: {h}");
    }

    #[test]
    fn query_params_use_double_equals() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.params = vec![KeyValue { key: "limit".into(), value: "10".into(), enabled: true }];
        let h = to_httpie(&r);
        assert!(h.contains("'limit==10'"), "got: {h}");
    }

    #[test]
    fn bearer_token_emitted_as_header() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.auth = AuthConfig::Bearer { token: "xyz".into() };
        let h = to_httpie(&r);
        assert!(h.contains("'Authorization:Bearer xyz'"), "got: {h}");
    }

    #[test]
    fn basic_uses_auth_flag() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.auth = AuthConfig::Basic { username: "u".into(), password: "p".into() };
        let h = to_httpie(&r);
        assert!(h.contains("--auth 'u:p'"), "got: {h}");
    }

    #[test]
    fn form_body_uses_form_flag_and_equals_syntax() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Form;
        r.body_content = serde_json::to_string(&vec![
            KeyValue { key: "a".into(), value: "1".into(), enabled: true },
        ]).unwrap();
        let h = to_httpie(&r);
        assert!(h.contains("--form"), "got: {h}");
        assert!(h.contains("'a=1'"));
    }

    #[test]
    fn json_string_value_uses_equals_syntax() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"name":"alice"}"#.into();
        let h = to_httpie(&r);
        // String values: `key=value`. No --raw, no JSON quoting around alice.
        assert!(h.contains("'name=alice'"), "got: {h}");
        assert!(!h.contains("--raw"), "got: {h}");
    }

    #[test]
    fn json_non_string_values_use_raw_json_syntax() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Json;
        r.body_content =
            r#"{"name":"alice","age":30,"admin":true,"tags":["a","b"]}"#.into();
        let h = to_httpie(&r);
        // String → key=value
        assert!(h.contains("'name=alice'"), "got: {h}");
        // Number/bool/array → key:=jsonval (minified)
        assert!(h.contains("'age:=30'"), "got: {h}");
        assert!(h.contains("'admin:=true'"), "got: {h}");
        assert!(h.contains(r#"'tags:=["a","b"]'"#), "got: {h}");
        assert!(!h.contains("--raw"), "got: {h}");
    }

    #[test]
    fn top_level_json_array_falls_back_to_raw() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Json;
        r.body_content = r#"[{"a":1},{"a":2}]"#.into();
        let h = to_httpie(&r);
        // No top-level object → no shorthand → --raw.
        assert!(h.contains("--raw"), "got: {h}");
        assert!(h.contains(r#"'[{"a":1},{"a":2}]'"#), "got: {h}");
    }

    #[test]
    fn unparseable_json_body_falls_back_to_raw() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Json;
        r.body_content = "{{template_var}}".into();
        let h = to_httpie(&r);
        assert!(h.contains("--raw"), "got: {h}");
    }

    #[test]
    fn multipart_emits_file_at_syntax() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Multipart;
        r.body_content = serde_json::json!([
            {"key": "f", "value": "", "kind": "file", "filePath": "/tmp/x", "contentType": "", "enabled": true},
        ]).to_string();
        let h = to_httpie(&r);
        assert!(h.contains("--multipart"), "got: {h}");
        assert!(h.contains("'f@/tmp/x'"), "got: {h}");
    }
}
