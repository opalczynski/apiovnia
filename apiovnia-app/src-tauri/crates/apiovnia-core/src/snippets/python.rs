//! Render a resolved request as Python `requests` code.
//!
//! Idiomatic shape: a single `requests.<verb>(...)` call with kwargs for
//! `headers`, `params`, `json`/`data`/`files`/`auth`. We use `json=` for
//! JSON bodies (lets requests set Content-Type), `data=` for form +
//! `files=` for multipart (matches the executor's behaviour).

use std::fmt::Write as _;

use crate::model::{ApiKeyLocation, AuthConfig, BodyType, Request};

use super::{parse_kv_list, parse_multipart_list};

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn to_python_requests(req: &Request) -> String {
    let verb = req.method.as_str().to_ascii_lowercase();
    let url = py_str(&req.url);

    let mut kwargs: Vec<String> = Vec::new();

    // Query params — only the enabled rows. The ApiKey-in-query auth
    // goes through `params=` too for transparency (instead of being
    // hidden in the URL).
    let mut params_pairs: Vec<(String, String)> = req
        .params
        .iter()
        .filter(|p| p.enabled)
        .map(|p| (p.key.clone(), p.value.clone()))
        .collect();
    if let AuthConfig::ApiKey {
        name,
        value,
        r#in: ApiKeyLocation::Query,
    } = &req.auth
    {
        params_pairs.push((name.clone(), value.clone()));
    }
    if !params_pairs.is_empty() {
        kwargs.push(format!("    params={}", py_dict(&params_pairs)));
    }

    // Headers — user rows + auth-derived ones (Bearer / ApiKey-in-header).
    let mut header_pairs: Vec<(String, String)> = Vec::new();
    match &req.auth {
        AuthConfig::None
        | AuthConfig::Basic { .. }
        | AuthConfig::ApiKey { r#in: ApiKeyLocation::Query, .. } => {}
        AuthConfig::Bearer { token } => {
            header_pairs.push(("Authorization".into(), format!("Bearer {token}")));
        }
        AuthConfig::ApiKey {
            name,
            value,
            r#in: ApiKeyLocation::Header,
        } => {
            header_pairs.push((name.clone(), value.clone()));
        }
    }
    for h in &req.headers {
        if !h.enabled {
            continue;
        }
        header_pairs.push((h.key.clone(), h.value.clone()));
    }
    if !header_pairs.is_empty() {
        kwargs.push(format!("    headers={}", py_dict(&header_pairs)));
    }

    // Basic auth — requests has a native tuple form.
    if let AuthConfig::Basic { username, password } = &req.auth {
        kwargs.push(format!(
            "    auth=({}, {})",
            py_str(username),
            py_str(password)
        ));
    }

    // Body.
    match req.body_type {
        BodyType::None => {}
        BodyType::Json => {
            // Use `json=` so requests handles serialisation + Content-Type.
            // If the user already set a Content-Type header we still pass
            // `json=`; requests respects an explicit header.
            match serde_json::from_str::<serde_json::Value>(&req.body_content) {
                Ok(v) => kwargs.push(format!("    json={}", py_value(&v, 1))),
                Err(_) => {
                    // Body isn't valid JSON (template like `{{var}}`). Fall
                    // back to `data=` to ship it verbatim.
                    kwargs.push(format!("    data={}", py_str(&req.body_content)));
                }
            }
        }
        BodyType::Raw => {
            kwargs.push(format!("    data={}", py_str(&req.body_content)));
        }
        BodyType::Form => {
            let rows: Vec<(String, String)> = parse_kv_list(&req.body_content)
                .into_iter()
                .filter(|kv| kv.enabled)
                .map(|kv| (kv.key, kv.value))
                .collect();
            kwargs.push(format!("    data={}", py_dict(&rows)));
        }
        BodyType::Multipart => {
            // requests uses `files=` for multipart. Files: tuples of
            // `(filename, fileobj, mime?)`. Text parts: a plain string.
            let rows = parse_multipart_list(&req.body_content);
            let mut entries: Vec<String> = Vec::new();
            for f in rows {
                if !f.enabled {
                    continue;
                }
                if f.kind == "file" {
                    let filename = f
                        .file_path
                        .rsplit('/')
                        .next()
                        .unwrap_or(&f.file_path)
                        .to_string();
                    let mime = if f.content_type.is_empty() {
                        String::from("None")
                    } else {
                        py_str(&f.content_type)
                    };
                    entries.push(format!(
                        "        ({}, ({}, open({}, 'rb'), {})),",
                        py_str(&f.key),
                        py_str(&filename),
                        py_str(&f.file_path),
                        mime,
                    ));
                } else {
                    entries.push(format!(
                        "        ({}, (None, {})),",
                        py_str(&f.key),
                        py_str(&f.value),
                    ));
                }
            }
            if entries.is_empty() {
                kwargs.push("    files=[]".into());
            } else {
                kwargs.push(format!("    files=[\n{}\n    ]", entries.join("\n")));
            }
        }
    }

    let mut out = String::with_capacity(256);
    out.push_str("import requests\n\n");
    writeln!(out, "response = requests.{verb}(").expect("write! into String is infallible");
    writeln!(out, "    {url},").expect("write! into String is infallible");
    if !kwargs.is_empty() {
        out.push_str(&kwargs.join(",\n"));
        out.push_str(",\n");
    }
    out.push_str(")\n");
    out.push_str("print(response.status_code)\n");
    out.push_str("print(response.text)\n");
    out
}

// ---------------------------------------------------------------------------
// Python literal helpers
// ---------------------------------------------------------------------------

/// Python double-quoted string with backslash + double-quote + newline
/// escaping. Adequate for our domain (URLs, header values, JSON bodies).
fn py_str(s: &str) -> String {
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

fn py_dict(pairs: &[(String, String)]) -> String {
    if pairs.is_empty() {
        return "{}".into();
    }
    let mut out = String::from("{\n");
    for (k, v) in pairs {
        writeln!(out, "        {}: {},", py_str(k), py_str(v))
            .expect("write! into String is infallible");
    }
    out.push_str("    }");
    out
}

/// Render a JSON value as a Python literal — recursive so nested structs
/// pretty-print like a Python `dict` / `list` would.
fn py_value(v: &serde_json::Value, depth: usize) -> String {
    use serde_json::Value;
    let pad = "    ".repeat(depth + 1);
    let close_pad = "    ".repeat(depth);
    match v {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => py_str(s),
        Value::Array(arr) => {
            if arr.is_empty() {
                return "[]".into();
            }
            let mut out = String::from("[\n");
            for item in arr {
                writeln!(out, "{}{},", pad, py_value(item, depth + 1))
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
                writeln!(out, "{}{}: {},", pad, py_str(k), py_value(v, depth + 1))
                    .expect("write! into String is infallible");
            }
            write!(out, "{close_pad}}}").expect("write! into String is infallible");
            out
        }
    }
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
    fn imports_and_calls_correct_verb() {
        let p = to_python_requests(&base(HttpMethod::Post, "https://x.com"));
        assert!(p.contains("import requests"));
        assert!(p.contains("requests.post("));
        assert!(p.contains(r#""https://x.com""#));
    }

    #[test]
    fn json_body_uses_json_kwarg() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"a":1,"b":"two"}"#.into();
        let p = to_python_requests(&r);
        assert!(p.contains("json={"));
        assert!(p.contains(r#""a": 1"#));
        assert!(p.contains(r#""b": "two""#));
    }

    #[test]
    fn json_body_preserves_key_order_from_request() {
        // serde_json with `preserve_order` keeps Object keys in their
        // insertion order — so the rendered dict matches what the user
        // typed in the editor, not an alphabetical re-sort.
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"street":"Mill Lane","number":"64","postcode":"123","town":"London"}"#
            .into();
        let p = to_python_requests(&r);
        let street_pos = p.find("street").expect("street present");
        let number_pos = p.find("number").expect("number present");
        let postcode_pos = p.find("postcode").expect("postcode present");
        let town_pos = p.find("town").expect("town present");
        assert!(
            street_pos < number_pos && number_pos < postcode_pos && postcode_pos < town_pos,
            "expected request order street→number→postcode→town, got: {p}"
        );
    }

    #[test]
    fn form_body_uses_data_dict() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Form;
        r.body_content = serde_json::to_string(&vec![
            KeyValue { key: "a".into(), value: "1".into(), enabled: true },
        ]).unwrap();
        let p = to_python_requests(&r);
        assert!(p.contains("data={"));
        assert!(p.contains(r#""a": "1""#));
    }

    #[test]
    fn basic_uses_native_auth_tuple() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.auth = AuthConfig::Basic { username: "u".into(), password: "p".into() };
        let p = to_python_requests(&r);
        assert!(p.contains(r#"auth=("u", "p")"#));
    }

    #[test]
    fn bearer_emits_authorization_header() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.auth = AuthConfig::Bearer { token: "xyz".into() };
        let p = to_python_requests(&r);
        assert!(p.contains(r#""Authorization": "Bearer xyz""#));
    }

    #[test]
    fn multipart_emits_files_kwarg_with_open() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Multipart;
        r.body_content = serde_json::json!([
            {"key": "f", "value": "", "kind": "file", "filePath": "/tmp/x.png", "contentType": "image/png", "enabled": true},
            {"key": "t", "value": "hi", "kind": "text", "filePath": "", "contentType": "", "enabled": true},
        ]).to_string();
        let p = to_python_requests(&r);
        assert!(p.contains("files=["));
        assert!(p.contains(r#"open("/tmp/x.png", 'rb')"#));
        assert!(p.contains(r#""image/png""#));
        assert!(p.contains(r#"("t", (None, "hi"))"#));
    }
}
