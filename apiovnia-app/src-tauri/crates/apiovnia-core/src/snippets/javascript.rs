//! Render a resolved request as native `fetch()` JavaScript.
//!
//! Idiomatic top-level-await shape (works in Node 18+, modern browsers,
//! Deno, Bun). For multipart we use `FormData`; for form-urlencoded we
//! use `URLSearchParams`. Basic auth is converted to an `Authorization`
//! header via `btoa` since `fetch` has no native flag.

use std::fmt::Write as _;

use crate::model::{ApiKeyLocation, AuthConfig, BodyType, Request};

use super::{
    effective_query_params, parse_kv_list, parse_multipart_list, url_with_params,
    user_has_content_type,
};

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn to_javascript_fetch(req: &Request) -> String {
    // Headers (auth-derived first, then user-supplied).
    let mut headers: Vec<(String, String)> = Vec::new();
    match &req.auth {
        AuthConfig::None | AuthConfig::ApiKey { r#in: ApiKeyLocation::Query, .. } => {}
        AuthConfig::Bearer { token } => {
            headers.push(("Authorization".into(), format!("Bearer {token}")));
        }
        AuthConfig::Basic { username, password } => {
            // No native flag — emit a `btoa(...)` call so the snippet is
            // self-contained and obvious about what's happening.
            headers.push((
                "Authorization".into(),
                format!("__BASIC__:{username}:{password}"),
            ));
        }
        AuthConfig::ApiKey {
            name,
            value,
            r#in: ApiKeyLocation::Header,
        } => {
            headers.push((name.clone(), value.clone()));
        }
    }
    for h in &req.headers {
        if !h.enabled {
            continue;
        }
        headers.push((h.key.clone(), h.value.clone()));
    }
    // Auto Content-Type for JSON bodies when the user didn't set one.
    if matches!(req.body_type, BodyType::Json) && !user_has_content_type(req) {
        headers.push(("Content-Type".into(), "application/json".into()));
    }

    let params = effective_query_params(req);
    let final_url = url_with_params(&req.url, &params);

    // Body lines — emitted as a `const body = …` set up before the fetch
    // call so the fetch options stay readable.
    let mut prelude: Vec<String> = Vec::new();
    let body_expr: Option<String> = match req.body_type {
        BodyType::None => None,
        BodyType::Json => {
            // The literal is embedded inside `body: JSON.stringify(...)` at
            // the fetch-options indent (2 spaces). Start the nested JSON
            // at depth 1 so its keys land at 4 spaces and the closing
            // brace lines up with `body:` rather than column 0.
            Some(format!(
                "JSON.stringify({})",
                js_value_or_string(&req.body_content, 1)
            ))
        }
        BodyType::Raw => Some(js_str(&req.body_content)),
        BodyType::Form => {
            let rows: Vec<(String, String)> = parse_kv_list(&req.body_content)
                .into_iter()
                .filter(|kv| kv.enabled)
                .map(|kv| (kv.key, kv.value))
                .collect();
            prelude.push(format!(
                "const body = new URLSearchParams({});",
                js_pairs_array(&rows)
            ));
            Some("body".into())
        }
        BodyType::Multipart => {
            let rows = parse_multipart_list(&req.body_content);
            let mut lines: Vec<String> = vec!["const body = new FormData();".into()];
            for f in rows {
                if !f.enabled {
                    continue;
                }
                if f.kind == "file" {
                    // Browser `fetch` doesn't have filesystem access — be
                    // explicit so the user knows they have to wire it up.
                    lines.push(format!(
                        "// TODO: replace with a File / Blob — fetch can't read {} from disk",
                        js_str(&f.file_path)
                    ));
                    lines.push(format!(
                        "body.append({}, /* File or Blob */);",
                        js_str(&f.key)
                    ));
                } else {
                    lines.push(format!(
                        "body.append({}, {});",
                        js_str(&f.key),
                        js_str(&f.value)
                    ));
                }
            }
            prelude.push(lines.join("\n"));
            Some("body".into())
        }
    };

    let mut out = String::with_capacity(256);
    if !prelude.is_empty() {
        for line in &prelude {
            out.push_str(line);
            out.push('\n');
        }
        out.push('\n');
    }

    writeln!(out, "const response = await fetch({}, {{", js_str(&final_url))
        .expect("write! into String is infallible");
    writeln!(out, "  method: {},", js_str(req.method.as_str()))
        .expect("write! into String is infallible");

    if !headers.is_empty() {
        out.push_str("  headers: {\n");
        for (k, v) in &headers {
            if let Some(rest) = v.strip_prefix("__BASIC__:") {
                let (u, p) = rest.split_once(':').unwrap_or((rest, ""));
                writeln!(
                    out,
                    "    {}: \"Basic \" + btoa({} + \":\" + {}),",
                    js_str(k),
                    js_str(u),
                    js_str(p)
                )
                .expect("write! into String is infallible");
            } else {
                writeln!(out, "    {}: {},", js_str(k), js_str(v))
                    .expect("write! into String is infallible");
            }
        }
        out.push_str("  },\n");
    }

    if let Some(b) = body_expr {
        writeln!(out, "  body: {b},").expect("write! into String is infallible");
    }

    out.push_str("});\n");
    out.push_str("const data = await response.text();\n");
    out.push_str("console.log(response.status, data);\n");
    out
}

// ---------------------------------------------------------------------------
// JS literal helpers
// ---------------------------------------------------------------------------

/// Try to parse as JSON so the output stays as a pretty-printed object
/// literal; if not valid JSON, fall back to a double-quoted string.
///
/// `depth` controls the starting indentation level — keys land at
/// `(depth + 1) * 2` spaces, closing brace at `depth * 2`. Use `0` for
/// a top-level literal, `1` when embedding inside another two-space
/// scope (e.g. inside `JSON.stringify(...)` at the fetch-options level).
fn js_value_or_string(s: &str, depth: usize) -> String {
    match serde_json::from_str::<serde_json::Value>(s) {
        Ok(v) => js_value(&v, depth),
        Err(_) => js_str(s),
    }
}

fn js_value(v: &serde_json::Value, depth: usize) -> String {
    use serde_json::Value;
    let pad = "  ".repeat(depth + 1);
    let close_pad = "  ".repeat(depth);
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => js_str(s),
        Value::Array(arr) => {
            if arr.is_empty() {
                return "[]".into();
            }
            let mut out = String::from("[\n");
            for item in arr {
                writeln!(out, "{}{},", pad, js_value(item, depth + 1))
                    .expect("write! into String is infallible");
            }
            write!(out, "{close_pad}]").expect("write! into String is infallible");
            out
        }
        Value::Object(map) => {
            if map.is_empty() {
                return "{}".into();
            }
            let mut out = String::from("{\n");
            for (k, v) in map {
                writeln!(out, "{}{}: {},", pad, js_str(k), js_value(v, depth + 1))
                    .expect("write! into String is infallible");
            }
            write!(out, "{close_pad}}}").expect("write! into String is infallible");
            out
        }
    }
}

/// Double-quoted JS string literal. Doesn't try to escape unicode — the
/// editor renders the file as UTF-8 anyway.
fn js_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn js_pairs_array(pairs: &[(String, String)]) -> String {
    if pairs.is_empty() {
        return "[]".into();
    }
    let mut out = String::from("[\n");
    for (k, v) in pairs {
        writeln!(out, "  [{}, {}],", js_str(k), js_str(v))
            .expect("write! into String is infallible");
    }
    out.push(']');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CollectionId, RequestId};
    use crate::model::{AuthConfig, HttpMethod, KeyValue};

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
    fn awaits_fetch_with_method_and_url() {
        let j = to_javascript_fetch(&base(HttpMethod::Post, "https://x.com"));
        assert!(j.contains(r#"const response = await fetch("https://x.com""#));
        assert!(j.contains(r#"method: "POST""#));
    }

    #[test]
    fn json_body_uses_json_stringify_with_pretty_object() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"a":1}"#.into();
        let j = to_javascript_fetch(&r);
        assert!(j.contains("JSON.stringify({"));
        assert!(j.contains(r#""Content-Type": "application/json""#));
    }

    #[test]
    fn json_stringify_indent_aligns_with_body_column() {
        // The literal sits inside `body: JSON.stringify(...)` at 2-space
        // indent — its inner keys should land at 4 spaces and the closing
        // brace at 2 spaces, so the snippet reads as one continuous shape
        // rather than jumping back to column 0.
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"name":"alice","tag":"cat"}"#.into();
        let j = to_javascript_fetch(&r);
        // 4-space indent on properties inside the stringify literal.
        assert!(j.contains("    \"name\": \"alice\""), "got: {j}");
        assert!(j.contains("    \"tag\": \"cat\""), "got: {j}");
        // 2-space indent on the closing brace, lining up with `body:`.
        assert!(j.contains("  })"), "got: {j}");
    }

    #[test]
    fn form_body_uses_urlsearchparams() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Form;
        r.body_content = serde_json::to_string(&vec![
            KeyValue { key: "a".into(), value: "1".into(), enabled: true },
        ]).unwrap();
        let j = to_javascript_fetch(&r);
        assert!(j.contains("new URLSearchParams(["));
        assert!(j.contains(r#"["a", "1"]"#));
        assert!(j.contains("body: body"));
    }

    #[test]
    fn multipart_uses_formdata_with_file_todo() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Multipart;
        r.body_content = serde_json::json!([
            {"key": "f", "value": "", "kind": "file", "filePath": "/tmp/x", "contentType": "", "enabled": true},
            {"key": "t", "value": "hi", "kind": "text", "filePath": "", "contentType": "", "enabled": true},
        ]).to_string();
        let j = to_javascript_fetch(&r);
        assert!(j.contains("new FormData()"));
        assert!(j.contains("// TODO: replace with a File / Blob"));
        assert!(j.contains(r#"body.append("t", "hi")"#));
    }

    #[test]
    fn basic_auth_uses_btoa_for_authorization() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.auth = AuthConfig::Basic { username: "u".into(), password: "p".into() };
        let j = to_javascript_fetch(&r);
        assert!(j.contains(r#""Authorization": "Basic " + btoa("u" + ":" + "p")"#), "got: {j}");
    }

    #[test]
    fn bearer_emits_authorization_header() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.auth = AuthConfig::Bearer { token: "xyz".into() };
        let j = to_javascript_fetch(&r);
        assert!(j.contains(r#""Authorization": "Bearer xyz""#));
    }
}
