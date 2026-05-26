//! Render a resolved request as a `curl` command-line.
//!
//! Single logical command, line-wrapped with `\` continuations so it
//! pastes cleanly into bash/zsh/fish. Decisions:
//!   * `-X METHOD` omitted for plain GET — curl's default.
//!   * Query params from `req.params` folded into the URL.
//!   * `--user 'u:p'` for Basic (the canonical idiom).
//!   * `--data-raw` (not `--data`) so a JSON body starting with `@` isn't
//!     mistakenly treated as a file reference.
//!   * `--form 'key=@/path[;type=mime]'` for multipart file parts.

use crate::model::{ApiKeyLocation, AuthConfig, BodyType, HttpMethod, Request};

use super::{
    effective_query_params, parse_kv_list, parse_multipart_list, url_with_params,
    user_has_content_type,
};

#[must_use]
pub fn to_curl(req: &Request) -> String {
    let mut out = String::with_capacity(128);
    out.push_str("curl");

    if req.method != HttpMethod::Get {
        out.push_str(" -X ");
        out.push_str(req.method.as_str());
    }

    let params = effective_query_params(req);
    let url = url_with_params(&req.url, &params);
    out.push(' ');
    out.push_str(&sh_escape(&url));

    // Auth — emit before user headers so a user-set Authorization wins.
    match &req.auth {
        AuthConfig::None | AuthConfig::ApiKey { r#in: ApiKeyLocation::Query, .. } => {}
        AuthConfig::Bearer { token } => {
            push_header(&mut out, "Authorization", &format!("Bearer {token}"));
        }
        AuthConfig::Basic { username, password } => {
            out.push_str(" \\\n  --user ");
            out.push_str(&sh_escape(&format!("{username}:{password}")));
        }
        AuthConfig::ApiKey {
            name,
            value,
            r#in: ApiKeyLocation::Header,
        } => {
            push_header(&mut out, name, value);
        }
    }

    let user_set_ct = user_has_content_type(req);
    for h in &req.headers {
        if !h.enabled {
            continue;
        }
        push_header(&mut out, &h.key, &h.value);
    }

    match req.body_type {
        BodyType::None => {}
        // GraphQL is folded into a JSON body upstream by `SnippetFormat::render`.
        BodyType::Json | BodyType::GraphQl => {
            if !user_set_ct {
                push_header(&mut out, "Content-Type", "application/json");
            }
            out.push_str(" \\\n  --data-raw ");
            out.push_str(&sh_escape(&req.body_content));
        }
        BodyType::Raw => {
            out.push_str(" \\\n  --data-binary ");
            out.push_str(&sh_escape(&req.body_content));
        }
        BodyType::Form => {
            for kv in parse_kv_list(&req.body_content) {
                if !kv.enabled {
                    continue;
                }
                out.push_str(" \\\n  --data-urlencode ");
                out.push_str(&sh_escape(&format!("{}={}", kv.key, kv.value)));
            }
        }
        BodyType::Multipart => {
            for f in parse_multipart_list(&req.body_content) {
                if !f.enabled {
                    continue;
                }
                let part = if f.kind == "file" {
                    let mut s = format!("{}=@{}", f.key, f.file_path);
                    if !f.content_type.is_empty() {
                        s.push_str(";type=");
                        s.push_str(&f.content_type);
                    }
                    s
                } else {
                    format!("{}={}", f.key, f.value)
                };
                out.push_str(" \\\n  --form ");
                out.push_str(&sh_escape(&part));
            }
        }
    }

    out
}

fn push_header(out: &mut String, name: &str, value: &str) {
    out.push_str(" \\\n  -H ");
    out.push_str(&sh_escape(&format!("{name}: {value}")));
}

/// POSIX-safe single-quote shell escape. `it's` → `'it'\''s'`.
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
    use crate::model::{ApiKeyLocation, AuthConfig, BodyType, HttpMethod, KeyValue, Request};

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
    fn plain_get_omits_method_flag() {
        let r = base(HttpMethod::Get, "https://example.com");
        let c = to_curl(&r);
        assert!(c.starts_with("curl 'https://example.com'"), "got: {c}");
        assert!(!c.contains("-X"));
    }

    #[test]
    fn post_emits_method() {
        let r = base(HttpMethod::Post, "https://example.com");
        let c = to_curl(&r);
        assert!(c.contains("-X POST"), "got: {c}");
    }

    #[test]
    fn shell_escapes_single_quotes_in_url() {
        let r = base(HttpMethod::Get, "https://example.com/it's-fine");
        let c = to_curl(&r);
        assert!(c.contains(r"'https://example.com/it'\''s-fine'"), "got: {c}");
    }

    #[test]
    fn folds_enabled_params_into_url() {
        let mut r = base(HttpMethod::Get, "https://example.com/x");
        r.params = vec![
            KeyValue { key: "a".into(), value: "1".into(), enabled: true },
            KeyValue { key: "b".into(), value: "two words".into(), enabled: true },
            KeyValue { key: "c".into(), value: "skip".into(), enabled: false },
        ];
        let c = to_curl(&r);
        assert!(c.contains("'https://example.com/x?a=1&b=two%20words'"), "got: {c}");
        assert!(!c.contains("skip"));
    }

    #[test]
    fn appends_to_existing_query() {
        let mut r = base(HttpMethod::Get, "https://example.com/?keep=yes");
        r.params = vec![KeyValue { key: "extra".into(), value: "added".into(), enabled: true }];
        let c = to_curl(&r);
        assert!(c.contains("?keep=yes&extra=added"), "got: {c}");
    }

    #[test]
    fn enabled_headers_emitted_disabled_skipped() {
        let mut r = base(HttpMethod::Get, "https://example.com");
        r.headers = vec![
            KeyValue { key: "X-A".into(), value: "1".into(), enabled: true },
            KeyValue { key: "X-Off".into(), value: "off".into(), enabled: false },
        ];
        let c = to_curl(&r);
        assert!(c.contains(r"-H 'X-A: 1'"), "got: {c}");
        assert!(!c.contains("X-Off"));
    }

    #[test]
    fn bearer_emits_authorization_header() {
        let mut r = base(HttpMethod::Get, "https://example.com");
        r.auth = AuthConfig::Bearer { token: "tok-123".into() };
        let c = to_curl(&r);
        assert!(c.contains(r"-H 'Authorization: Bearer tok-123'"), "got: {c}");
    }

    #[test]
    fn basic_uses_user_flag() {
        let mut r = base(HttpMethod::Get, "https://example.com");
        r.auth = AuthConfig::Basic { username: "u".into(), password: "p".into() };
        let c = to_curl(&r);
        assert!(c.contains(r"--user 'u:p'"), "got: {c}");
    }

    #[test]
    fn apikey_in_header_emits_header() {
        let mut r = base(HttpMethod::Get, "https://example.com");
        r.auth = AuthConfig::ApiKey { name: "X-API-Key".into(), value: "secret".into(), r#in: ApiKeyLocation::Header };
        let c = to_curl(&r);
        assert!(c.contains(r"-H 'X-API-Key: secret'"), "got: {c}");
    }

    #[test]
    fn apikey_in_query_folds_into_url() {
        let mut r = base(HttpMethod::Get, "https://example.com/x");
        r.auth = AuthConfig::ApiKey { name: "token".into(), value: "abc def".into(), r#in: ApiKeyLocation::Query };
        let c = to_curl(&r);
        assert!(c.contains("?token=abc%20def"), "got: {c}");
        assert!(!c.contains("-H 'token:"));
    }

    #[test]
    fn json_body_adds_content_type_when_missing() {
        let mut r = base(HttpMethod::Post, "https://example.com");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"name":"alice"}"#.into();
        let c = to_curl(&r);
        assert!(c.contains("'Content-Type: application/json'"), "got: {c}");
        assert!(c.contains(r#"--data-raw '{"name":"alice"}'"#), "got: {c}");
    }

    #[test]
    fn json_body_respects_user_content_type() {
        let mut r = base(HttpMethod::Post, "https://example.com");
        r.body_type = BodyType::Json;
        r.body_content = "{}".into();
        r.headers = vec![KeyValue { key: "content-type".into(), value: "application/vnd.api+json".into(), enabled: true }];
        let c = to_curl(&r);
        assert_eq!(
            c.matches("Content-Type").count() + c.matches("content-type").count(),
            1,
            "expected exactly one content-type, got: {c}"
        );
    }

    #[test]
    fn raw_body_uses_data_binary() {
        let mut r = base(HttpMethod::Post, "https://example.com");
        r.body_type = BodyType::Raw;
        r.body_content = "hello\nworld".into();
        let c = to_curl(&r);
        assert!(c.contains("--data-binary 'hello\nworld'"), "got: {c}");
    }

    #[test]
    fn form_body_emits_one_data_urlencode_per_enabled_row() {
        let mut r = base(HttpMethod::Post, "https://example.com");
        r.body_type = BodyType::Form;
        r.body_content = serde_json::to_string(&vec![
            KeyValue { key: "a".into(), value: "1".into(), enabled: true },
            KeyValue { key: "b".into(), value: "2".into(), enabled: false },
            KeyValue { key: "c".into(), value: "3 4".into(), enabled: true },
        ]).unwrap();
        let c = to_curl(&r);
        assert!(c.contains("--data-urlencode 'a=1'"), "got: {c}");
        assert!(c.contains("--data-urlencode 'c=3 4'"), "got: {c}");
        assert!(!c.contains("'b=2'"));
    }

    #[test]
    fn multipart_text_and_file_parts() {
        let mut r = base(HttpMethod::Post, "https://example.com");
        r.body_type = BodyType::Multipart;
        r.body_content = serde_json::json!([
            {"key": "comment", "value": "hi", "kind": "text", "filePath": "", "contentType": "", "enabled": true},
            {"key": "upload",  "value": "",   "kind": "file", "filePath": "/tmp/x.png", "contentType": "image/png", "enabled": true},
            {"key": "off",     "value": "no", "kind": "text", "filePath": "", "contentType": "", "enabled": false},
        ]).to_string();
        let c = to_curl(&r);
        assert!(c.contains("--form 'comment=hi'"), "got: {c}");
        assert!(c.contains("--form 'upload=@/tmp/x.png;type=image/png'"), "got: {c}");
        assert!(!c.contains("'off="));
    }

    #[test]
    fn multipart_file_without_explicit_mime_omits_type() {
        let mut r = base(HttpMethod::Post, "https://example.com");
        r.body_type = BodyType::Multipart;
        r.body_content = serde_json::json!([
            {"key": "f", "value": "", "kind": "file", "filePath": "/tmp/x", "contentType": "", "enabled": true},
        ]).to_string();
        let c = to_curl(&r);
        assert!(c.contains("--form 'f=@/tmp/x'"), "got: {c}");
        assert!(!c.contains(";type="));
    }

    #[test]
    fn empty_string_escapes_to_double_single() {
        assert_eq!(sh_escape(""), "''");
    }
}
