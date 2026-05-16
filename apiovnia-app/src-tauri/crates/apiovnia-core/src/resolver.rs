//! Env-aware request resolution.
//!
//! `resolve_request` takes a base [`Request`], an optional [`EnvOverride`],
//! and a set of `{{var}}` substitutions, and returns the [`ResolvedRequest`]
//! that the HTTP executor will actually run.
//!
//! Field-wise rule: `override.field == Some(v)` → use `v`; `None` → keep the
//! base value. Headers and params are **full replacements** (not per-key
//! merges) — the brief asks for this so users don't have to reason about
//! how individual override rows interact with base rows.
//!
//! Pure function. Idempotent. No I/O. Easy to test, easy to reason about.

use std::collections::HashMap;
use std::hash::BuildHasher;

use crate::interpolate::{interpolate, interpolate_auth, interpolate_pairs};
use crate::model::{EnvOverride, Request};

/// Same shape as a [`Request`] — the alias makes call sites read clearly.
pub type ResolvedRequest = Request;

/// Resolve `base` against `override` and `vars`. Returns a brand-new owned
/// [`Request`] suitable for the executor.
#[must_use]
pub fn resolve_request<S: BuildHasher>(
    base: &Request,
    over: Option<&EnvOverride>,
    vars: &HashMap<String, String, S>,
) -> ResolvedRequest {
    let mut out = base.clone();

    if let Some(o) = over {
        if let Some(m) = o.method {
            out.method = m;
        }
        if let Some(u) = &o.url {
            out.url.clone_from(u);
        }
        if let Some(h) = &o.headers {
            out.headers.clone_from(h);
        }
        if let Some(p) = &o.params {
            out.params.clone_from(p);
        }
        if let Some(bt) = o.body_type {
            out.body_type = bt;
        }
        if let Some(bc) = &o.body_content {
            out.body_content.clone_from(bc);
        }
        if let Some(a) = &o.auth {
            out.auth.clone_from(a);
        }
    }

    // Interpolate after the override fold: env vars apply uniformly whether
    // a field comes from base or override.
    out.url = interpolate(&out.url, vars);
    interpolate_pairs(&mut out.headers, vars);
    interpolate_pairs(&mut out.params, vars);
    out.body_content = interpolate(&out.body_content, vars);
    interpolate_auth(&mut out.auth, vars);

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CollectionId, EnvironmentId, RequestId};
    use crate::model::{AuthConfig, BodyType, HttpMethod, KeyValue, Request};
    use std::collections::HashMap;

    fn base() -> Request {
        Request {
            id: RequestId::new(),
            collection_id: CollectionId::new(),
            name: "Login".into(),
            method: HttpMethod::Post,
            url: "https://{{base_url}}/auth/login".into(),
            headers: vec![
                KeyValue { key: "X-Workspace-Id".into(), value: "ws_udl".into(), enabled: true },
                KeyValue { key: "X-Client-Version".into(), value: "1.0".into(), enabled: true },
            ],
            params: vec![
                KeyValue { key: "trace".into(), value: "false".into(), enabled: true },
            ],
            body_type: BodyType::Json,
            body_content: r#"{"token":"{{token}}"}"#.into(),
            auth: AuthConfig::None,
            created_at: 0,
            updated_at: 0,
            sort_order: 0,
        }
    }

    fn empty_vars() -> HashMap<String, String> {
        HashMap::new()
    }

    fn dev_vars() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("base_url".into(), "api-dev.udl.test".into());
        m.insert("token".into(), "dev_token".into());
        m
    }

    fn over_for(req: &Request, env: &EnvironmentId) -> EnvOverride {
        EnvOverride {
            request_id: req.id.clone(),
            environment_id: env.clone(),
            ..Default::default()
        }
    }

    // -----------------------------------------------------------------------
    // Field-fold semantics (brief: "all None / some Some / all Some")
    // -----------------------------------------------------------------------

    #[test]
    fn no_override_passes_base_through_unchanged() {
        let b = base();
        let out = resolve_request(&b, None, &empty_vars());
        // Interpolation leaves unknowns as-is, so url + body still carry the
        // {{var}} placeholders.
        assert_eq!(out.method, b.method);
        assert_eq!(out.url, b.url);
        assert_eq!(out.headers, b.headers);
        assert_eq!(out.params, b.params);
        assert_eq!(out.body_type, b.body_type);
        assert_eq!(out.body_content, b.body_content);
        assert_eq!(out.auth, b.auth);
    }

    #[test]
    fn empty_override_passes_base_through_unchanged() {
        let b = base();
        let env = EnvironmentId::new();
        let o = over_for(&b, &env);
        assert!(o.is_empty());
        let out = resolve_request(&b, Some(&o), &empty_vars());
        assert_eq!(out.method, b.method);
        assert_eq!(out.url, b.url);
        assert_eq!(out.headers, b.headers);
    }

    #[test]
    fn url_only_override_replaces_url_only() {
        let b = base();
        let env = EnvironmentId::new();
        let mut o = over_for(&b, &env);
        o.url = Some("https://prod.example.com/login".into());
        let out = resolve_request(&b, Some(&o), &empty_vars());
        assert_eq!(out.url, "https://prod.example.com/login");
        // Method and headers untouched.
        assert_eq!(out.method, HttpMethod::Post);
        assert_eq!(out.headers, b.headers);
    }

    #[test]
    fn method_override_replaces_method_only() {
        let b = base();
        let env = EnvironmentId::new();
        let mut o = over_for(&b, &env);
        o.method = Some(HttpMethod::Put);
        let out = resolve_request(&b, Some(&o), &empty_vars());
        assert_eq!(out.method, HttpMethod::Put);
        assert_eq!(out.url, b.url);
    }

    #[test]
    fn headers_override_fully_replaces_base_headers() {
        let b = base();
        let env = EnvironmentId::new();
        let mut o = over_for(&b, &env);
        o.headers = Some(vec![KeyValue {
            key: "Authorization".into(),
            value: "Bearer xyz".into(),
            enabled: true,
        }]);
        let out = resolve_request(&b, Some(&o), &empty_vars());
        // Replacement, not merge: base headers gone.
        assert_eq!(out.headers.len(), 1);
        assert_eq!(out.headers[0].key, "Authorization");
        assert_eq!(out.headers[0].value, "Bearer xyz");
    }

    #[test]
    fn params_override_fully_replaces_base_params() {
        let b = base();
        let env = EnvironmentId::new();
        let mut o = over_for(&b, &env);
        o.params = Some(vec![]);
        let out = resolve_request(&b, Some(&o), &empty_vars());
        assert!(out.params.is_empty());
    }

    #[test]
    fn body_override_replaces_type_and_content() {
        let b = base();
        let env = EnvironmentId::new();
        let mut o = over_for(&b, &env);
        o.body_type = Some(BodyType::Raw);
        o.body_content = Some("hello".into());
        let out = resolve_request(&b, Some(&o), &empty_vars());
        assert_eq!(out.body_type, BodyType::Raw);
        assert_eq!(out.body_content, "hello");
    }

    #[test]
    fn auth_override_replaces_whole_auth_block() {
        let b = base();
        let env = EnvironmentId::new();
        let mut o = over_for(&b, &env);
        o.auth = Some(AuthConfig::Bearer { token: "xyz".into() });
        let out = resolve_request(&b, Some(&o), &empty_vars());
        assert_eq!(out.auth, AuthConfig::Bearer { token: "xyz".into() });
    }

    #[test]
    fn all_fields_override_replaces_everything() {
        let b = base();
        let env = EnvironmentId::new();
        let o = EnvOverride {
            request_id: b.id.clone(),
            environment_id: env,
            method: Some(HttpMethod::Get),
            url: Some("https://full.example.com/x".into()),
            headers: Some(vec![]),
            params: Some(vec![]),
            body_type: Some(BodyType::None),
            body_content: Some(String::new()),
            auth: Some(AuthConfig::None),
        };
        let out = resolve_request(&b, Some(&o), &empty_vars());
        assert_eq!(out.method, HttpMethod::Get);
        assert_eq!(out.url, "https://full.example.com/x");
        assert!(out.headers.is_empty());
        assert!(out.params.is_empty());
        assert_eq!(out.body_type, BodyType::None);
        assert!(out.body_content.is_empty());
        assert_eq!(out.auth, AuthConfig::None);
    }

    // -----------------------------------------------------------------------
    // Interpolation
    // -----------------------------------------------------------------------

    #[test]
    fn vars_interpolate_into_url_and_body() {
        let b = base();
        let out = resolve_request(&b, None, &dev_vars());
        assert_eq!(out.url, "https://api-dev.udl.test/auth/login");
        assert_eq!(out.body_content, r#"{"token":"dev_token"}"#);
    }

    #[test]
    fn vars_interpolate_into_override_url() {
        let b = base();
        let env = EnvironmentId::new();
        let mut o = over_for(&b, &env);
        o.url = Some("https://{{base_url}}/v2/login".into());
        let out = resolve_request(&b, Some(&o), &dev_vars());
        assert_eq!(out.url, "https://api-dev.udl.test/v2/login");
    }

    #[test]
    fn vars_interpolate_into_header_values_after_override_replace() {
        let b = base();
        let env = EnvironmentId::new();
        let mut o = over_for(&b, &env);
        o.headers = Some(vec![KeyValue {
            key: "Authorization".into(),
            value: "Bearer {{token}}".into(),
            enabled: true,
        }]);
        let out = resolve_request(&b, Some(&o), &dev_vars());
        assert_eq!(out.headers[0].value, "Bearer dev_token");
    }

    #[test]
    fn missing_vars_left_verbatim_so_user_sees_the_gap() {
        let b = base();
        // No vars at all — placeholders stay.
        let out = resolve_request(&b, None, &empty_vars());
        assert!(out.url.contains("{{base_url}}"));
        assert!(out.body_content.contains("{{token}}"));
    }
}
