//! Render a resolved request as a `PowerShell` `Invoke-RestMethod` call.
//!
//! Idiomatic shape:
//! ```powershell
//! $headers = @{ "K" = "V" }
//! $body = '{"x": 1}'
//! Invoke-RestMethod -Uri "..." -Method POST -Headers $headers -Body $body
//! ```
//!
//! `PowerShell` single-quotes are literal (no interpolation, no escapes
//! except a doubled `''` for embedded apostrophe). We use them for body
//! bytes so `JSON` travels unchanged.

use std::fmt::Write as _;

use crate::model::{ApiKeyLocation, AuthConfig, BodyType, Request};

use super::{effective_query_params, parse_kv_list, parse_multipart_list, url_with_params};

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn to_powershell(req: &Request) -> String {
    let mut prelude: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();

    let params = effective_query_params(req);
    let final_url = url_with_params(&req.url, &params);

    args.push(format!("-Uri {}", ps_dq(&final_url)));
    args.push(format!("-Method {}", req.method.as_str()));

    // Headers (auth-derived + user).
    let mut headers: Vec<(String, String)> = Vec::new();
    match &req.auth {
        AuthConfig::None | AuthConfig::Basic { .. } | AuthConfig::ApiKey { r#in: ApiKeyLocation::Query, .. } => {}
        AuthConfig::Bearer { token } => {
            headers.push(("Authorization".into(), format!("Bearer {token}")));
        }
        AuthConfig::ApiKey {
            name,
            value,
            r#in: ApiKeyLocation::Header,
        } => {
            headers.push((name.clone(), value.clone()));
        }
    }
    // Content-Type goes through `Invoke-RestMethod`'s dedicated
    // `-ContentType` parameter rather than the `$headers` hashtable —
    // Windows PowerShell 5.1 (still default on plenty of corp Windows
    // boxes) chokes on hashtable Content-Type + JSON body combos.
    let mut content_type_arg: Option<String> = None;
    for h in &req.headers {
        if !h.enabled {
            continue;
        }
        if h.key.eq_ignore_ascii_case("content-type") {
            content_type_arg = Some(h.value.clone());
            continue;
        }
        headers.push((h.key.clone(), h.value.clone()));
    }
    if matches!(req.body_type, BodyType::Json) && content_type_arg.is_none() {
        content_type_arg = Some("application/json".into());
    }
    if !headers.is_empty() {
        let mut block = String::from("$headers = @{\n");
        for (k, v) in &headers {
            writeln!(block, "    {} = {}", ps_dq(k), ps_dq(v))
                .expect("write! into String is infallible");
        }
        block.push('}');
        prelude.push(block);
        args.push("-Headers $headers".into());
    }
    if let Some(ct) = &content_type_arg {
        args.push(format!("-ContentType {}", ps_dq(ct)));
    }

    // Basic auth via `-Credential` requires a SecureString dance that
    // muddies the snippet. PowerShell `Invoke-RestMethod` accepts
    // `-Authentication Basic -Credential …`, but for a paste-and-go
    // script the manual header is clearer.
    if let AuthConfig::Basic { username, password } = &req.auth {
        prelude.push(format!(
            "$pair = {} + \":\" + {}\n$basic = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($pair))",
            ps_dq(username),
            ps_dq(password)
        ));
        // If we already have a $headers block, splice the Authorization
        // line in; otherwise create one.
        if let Some(h) = prelude.iter_mut().find(|p| p.starts_with("$headers")) {
            let insert_at = h.rfind('}').unwrap_or(h.len());
            h.insert_str(
                insert_at,
                "    Authorization = \"Basic $basic\"\n",
            );
        } else {
            prelude.push("$headers = @{ Authorization = \"Basic $basic\" }".into());
            args.push("-Headers $headers".into());
        }
    }

    // Body.
    match req.body_type {
        BodyType::None => {}
        // GraphQL is folded into a JSON body upstream by `SnippetFormat::render`.
        BodyType::Json | BodyType::Raw | BodyType::GraphQl => {
            // Single-quoted PowerShell literal — no escapes apart from
            // doubled apostrophes inside.
            prelude.push(format!("$body = {}", ps_sq(&req.body_content)));
            args.push("-Body $body".into());
        }
        BodyType::Form => {
            let rows: Vec<(String, String)> = parse_kv_list(&req.body_content)
                .into_iter()
                .filter(|kv| kv.enabled)
                .map(|kv| (kv.key, kv.value))
                .collect();
            let mut block = String::from("$body = @{\n");
            for (k, v) in &rows {
                writeln!(block, "    {} = {}", ps_dq(k), ps_dq(v))
                    .expect("write! into String is infallible");
            }
            block.push('}');
            prelude.push(block);
            args.push("-Body $body".into());
            if content_type_arg.is_none() {
                args.push("-ContentType \"application/x-www-form-urlencoded\"".into());
            }
        }
        BodyType::Multipart => {
            // Invoke-RestMethod's -Form param (PowerShell 6+) takes a
            // hashtable where file values can be `Get-Item path`.
            let rows = parse_multipart_list(&req.body_content);
            let mut block = String::from("$form = @{\n");
            for f in rows {
                if !f.enabled {
                    continue;
                }
                let value_expr = if f.kind == "file" {
                    format!("Get-Item {}", ps_dq(&f.file_path))
                } else {
                    ps_dq(&f.value)
                };
                writeln!(block, "    {} = {}", ps_dq(&f.key), value_expr)
                    .expect("write! into String is infallible");
            }
            block.push('}');
            prelude.push(block);
            args.push("-Form $form".into());
        }
    }

    let mut out = String::with_capacity(256);
    for p in &prelude {
        out.push_str(p);
        out.push('\n');
    }
    if !prelude.is_empty() {
        out.push('\n');
    }
    out.push_str("$response = Invoke-RestMethod ");
    out.push_str(&args.join(" `\n    "));
    out.push('\n');
    out.push_str("$response\n");
    out
}

// ---------------------------------------------------------------------------
// PowerShell literal helpers
// ---------------------------------------------------------------------------

/// Double-quoted `PowerShell` string — supports interpolation, so we have
/// to escape `$` and `` ` `` plus the obvious `"` / newline.
fn ps_dq(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '`' => out.push_str("``"),
            '"' => out.push_str("`\""),
            '$' => out.push_str("`$"),
            '\n' => out.push_str("`n"),
            '\r' => out.push_str("`r"),
            '\t' => out.push_str("`t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Single-quoted `PowerShell` string — fully literal, only `'` doubles.
fn ps_sq(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("''");
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
    fn invokes_with_uri_and_method() {
        let p = to_powershell(&base(HttpMethod::Post, "https://x.com"));
        assert!(p.contains("Invoke-RestMethod -Uri \"https://x.com\""));
        assert!(p.contains("-Method POST"));
    }

    #[test]
    fn json_body_emits_content_type_via_dedicated_parameter() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"a":1}"#.into();
        let p = to_powershell(&r);
        assert!(p.contains(r#"$body = '{"a":1}'"#));
        // -ContentType arg, not a $headers entry — PS 5.1 chokes otherwise.
        assert!(p.contains(r#"-ContentType "application/json""#), "got: {p}");
        assert!(!p.contains(r#""Content-Type" = "application/json""#), "got: {p}");
        assert!(p.contains("-Body $body"));
    }

    #[test]
    fn user_content_type_header_promoted_to_dedicated_parameter() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Json;
        r.body_content = "{}".into();
        r.headers = vec![KeyValue {
            key: "content-type".into(),
            value: "application/vnd.api+json".into(),
            enabled: true,
        }];
        let p = to_powershell(&r);
        assert!(p.contains(r#"-ContentType "application/vnd.api+json""#), "got: {p}");
        // And it's NOT also in the $headers hashtable.
        assert!(!p.contains(r#""content-type""#), "got: {p}");
        assert!(!p.contains(r#""Content-Type""#), "got: {p}");
    }

    #[test]
    fn form_body_emits_hashtable_and_content_type() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Form;
        r.body_content = serde_json::to_string(&vec![
            KeyValue { key: "a".into(), value: "1".into(), enabled: true },
        ]).unwrap();
        let p = to_powershell(&r);
        assert!(p.contains("$body = @{"));
        assert!(p.contains(r#""a" = "1""#));
        assert!(p.contains(r#"-ContentType "application/x-www-form-urlencoded""#));
    }

    #[test]
    fn bearer_emits_authorization_header() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.auth = AuthConfig::Bearer { token: "xyz".into() };
        let p = to_powershell(&r);
        assert!(p.contains(r#""Authorization" = "Bearer xyz""#), "got: {p}");
    }

    #[test]
    fn basic_emits_base64_dance() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.auth = AuthConfig::Basic { username: "u".into(), password: "p".into() };
        let p = to_powershell(&r);
        assert!(p.contains("$pair = \"u\" + \":\" + \"p\""), "got: {p}");
        assert!(p.contains("ToBase64String"));
        assert!(p.contains("Authorization = \"Basic $basic\""));
    }

    #[test]
    fn multipart_uses_form_param_with_get_item_for_files() {
        let mut r = base(HttpMethod::Post, "https://x.com");
        r.body_type = BodyType::Multipart;
        r.body_content = serde_json::json!([
            {"key": "f", "value": "", "kind": "file", "filePath": "/tmp/x", "contentType": "", "enabled": true},
            {"key": "t", "value": "hi", "kind": "text", "filePath": "", "contentType": "", "enabled": true},
        ]).to_string();
        let p = to_powershell(&r);
        assert!(p.contains("$form = @{"));
        assert!(p.contains(r#""f" = Get-Item "/tmp/x""#));
        assert!(p.contains(r#""t" = "hi""#));
        assert!(p.contains("-Form $form"));
    }

    #[test]
    fn embedded_dollar_in_value_is_escaped_in_dq() {
        let mut r = base(HttpMethod::Get, "https://x.com");
        r.headers = vec![KeyValue { key: "X-Var".into(), value: "$foo".into(), enabled: true }];
        let p = to_powershell(&r);
        assert!(p.contains("\"`$foo\""), "got: {p}");
    }
}
