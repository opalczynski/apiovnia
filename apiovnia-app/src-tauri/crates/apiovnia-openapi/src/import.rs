//! Parse an `OpenAPI` 3.x document into the Apiovnia domain.
//!
//! Best-effort: we map the fields we have a home for and warn about the
//! rest. Nothing in `oas3`'s output is required — if a field is missing
//! we substitute a sensible default and keep going.
//!
//! Output shape: one [`Collection`] plus a flat list of [`Request`]s
//! ready to be persisted by the storage layer. `Environment`s are derived
//! from `servers[]`; the IPC command layer is responsible for actually
//! writing them and wiring the URL overrides per-(request, env).

use std::collections::BTreeMap;

use apiovnia_core::ids::{CollectionId, ProjectId, RequestId};
use apiovnia_core::model::{
    ApiKeyLocation, AuthConfig, BodyType, Collection, EnvOverride, Environment, HttpMethod,
    KeyValue, Request,
};
use apiovnia_core::time::epoch_millis_now;
use apiovnia_core::ids::EnvironmentId;
use oas3::spec::{
    ObjectOrReference, ObjectSchema, Operation, Parameter, ParameterIn, RequestBody,
    SchemaType as Type, SecurityScheme, Spec,
};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("failed to parse OpenAPI document: {0}")]
    Parse(String),
    #[error("io: {0}")]
    Io(String),
}

/// Output of a single import call.
#[derive(Debug)]
pub struct ImportResult {
    pub collection: Collection,
    pub requests: Vec<Request>,
    /// Environments derived from `servers[]`. Empty when the spec had no
    /// `servers` block (which is legal).
    pub environments: Vec<Environment>,
    /// Per-(request, env) URL overrides — same length as
    /// `requests.len() * environments.len()` in the worst case. Caller
    /// persists what isn't empty.
    pub overrides: Vec<EnvOverride>,
    /// Per-row summary for the `OpLog` table.
    pub rows: Vec<ImportRow>,
    /// Soft notices that don't fail the import.
    pub warnings: Vec<String>,
    /// Full textual log.
    pub log: String,
}

#[derive(Debug, Clone)]
pub struct ImportRow {
    pub name: String,
    pub method: String,
    pub path: String,
}

/// Parse a YAML or JSON document into the Apiovnia domain.
///
/// `project_id` and `collection_id` are pre-allocated by the caller (the
/// IPC layer) so the new collection links up cleanly without a roundtrip
/// to the DB layer.
///
/// # Errors
///
/// Returns [`ImportError::Parse`] when neither the YAML nor JSON parser
/// can make sense of `source`.
#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
pub fn import_document(
    source: &str,
    project_id: &ProjectId,
    collection_id: CollectionId,
) -> Result<ImportResult, ImportError> {
    let spec = parse_any(source)?;
    let now = epoch_millis_now();
    let mut warnings: Vec<String> = Vec::new();

    let title = spec.info.title.trim().to_string();
    let collection_name = if title.is_empty() {
        "Imported".into()
    } else {
        title
    };
    let collection = Collection {
        id: collection_id.clone(),
        project_id: project_id.clone(),
        name: collection_name.clone(),
        created_at: now,
        updated_at: now,
        sort_order: now,
    };

    // Environments from servers[]. Skipped when only one (or zero) server —
    // the user doesn't gain anything from a single env in that case.
    let environments = build_environments(&spec, project_id, now, &mut warnings);

    // Walk operations. Skip unsupported methods (TRACE, CONNECT — neither
    // is in our HttpMethod enum) with a warning.
    let mut requests: Vec<Request> = Vec::new();
    let mut rows: Vec<ImportRow> = Vec::new();
    let mut overrides: Vec<EnvOverride> = Vec::new();

    if let Some(paths) = &spec.paths {
        let mut order_offset: i64 = 0;
        for (path, item) in paths {
            for (raw_method, op) in path_item_operations(item) {
                let Some(method) = parse_method(raw_method) else {
                    warnings.push(format!(
                        "skipped {raw_method} {path}: method not supported in Apiovnia"
                    ));
                    continue;
                };

                let name = pick_request_name(op, raw_method, path);
                let url = build_request_url(&spec, path);
                let (headers, params, path_params) = collect_parameters(op);
                if !path_params.is_empty() {
                    warnings.push(format!(
                        "request '{name}': {} path param(s) preserved as {{name}} in URL ({}); replace before sending",
                        path_params.len(),
                        path_params.join(", "),
                    ));
                }

                let (body_type, body_content) = build_body(&spec, op, &mut warnings, &name);
                let (auth, auth_warning) = build_auth(op, &spec);
                if let Some(w) = auth_warning {
                    warnings.push(format!("request '{name}': {w}"));
                }

                let req_id = RequestId::new();
                let request = Request {
                    id: req_id.clone(),
                    collection_id: collection_id.clone(),
                    name: name.clone(),
                    method,
                    url,
                    headers,
                    params,
                    body_type,
                    body_content,
                    auth,
                    created_at: now,
                    updated_at: now + order_offset,
                    sort_order: now + order_offset,
                };
                order_offset += 1;

                // Per-(request, env) URL override — only when we have envs
                // *and* the env's server URL differs from the request's base.
                for (env, server) in environments.iter().zip(servers_iter(&spec)) {
                    let override_url = format!("{}{}", strip_trailing_slash(&server), path);
                    if override_url != request.url {
                        overrides.push(EnvOverride {
                            request_id: req_id.clone(),
                            environment_id: env.id.clone(),
                            method: None,
                            url: Some(override_url),
                            headers: None,
                            params: None,
                            body_type: None,
                            body_content: None,
                            auth: None,
                        });
                    }
                }

                rows.push(ImportRow {
                    name,
                    method: raw_method.to_string(),
                    path: path.clone(),
                });
                requests.push(request);
            }
        }
    } else {
        warnings.push("document has no paths".into());
    }

    let log = render_log(&collection, &rows, &environments, &warnings);

    Ok(ImportResult {
        collection,
        requests,
        environments,
        overrides,
        rows,
        warnings,
        log,
    })
}

// ---------------------------------------------------------------------------
// Spec parsing
// ---------------------------------------------------------------------------

/// `oas3` ships `from_yaml` + `from_json`. We try YAML first because
/// YAML is a superset of JSON syntactically — if a `.json` document
/// is valid YAML it'll parse, but if it's not we fall back.
fn parse_any(source: &str) -> Result<Spec, ImportError> {
    if let Ok(spec) = oas3::from_yaml(source) {
        return Ok(spec);
    }
    oas3::from_json(source).map_err(|e| ImportError::Parse(e.to_string()))
}

fn path_item_operations(
    item: &oas3::spec::PathItem,
) -> Vec<(&'static str, &Operation)> {
    let mut out = Vec::new();
    if let Some(op) = &item.get { out.push(("GET", op)); }
    if let Some(op) = &item.post { out.push(("POST", op)); }
    if let Some(op) = &item.put { out.push(("PUT", op)); }
    if let Some(op) = &item.patch { out.push(("PATCH", op)); }
    if let Some(op) = &item.delete { out.push(("DELETE", op)); }
    if let Some(op) = &item.head { out.push(("HEAD", op)); }
    if let Some(op) = &item.options { out.push(("OPTIONS", op)); }
    out
}

fn parse_method(m: &str) -> Option<HttpMethod> {
    Some(match m {
        "GET" => HttpMethod::Get,
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "PATCH" => HttpMethod::Patch,
        "DELETE" => HttpMethod::Delete,
        "HEAD" => HttpMethod::Head,
        "OPTIONS" => HttpMethod::Options,
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Per-request mapping
// ---------------------------------------------------------------------------

fn pick_request_name(op: &Operation, method: &str, path: &str) -> String {
    op.summary
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| op.operation_id.clone().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| format!("{method} {path}"))
}

/// Default URL the user sees in the editor: derived from the spec's primary
/// `server.url` (if any) plus the path. The IPC layer also generates
/// per-(req, env) overrides from the full `servers[]` list — so the user
/// can switch envs in the UI and hit the right one.
fn build_request_url(spec: &Spec, path: &str) -> String {
    let server = spec
        .servers
        .first()
        .map(|s| strip_trailing_slash(&s.url))
        .unwrap_or_default();
    format!("{server}{path}")
}

fn collect_parameters(op: &Operation) -> (Vec<KeyValue>, Vec<KeyValue>, Vec<String>) {
    let mut headers: Vec<KeyValue> = Vec::new();
    let mut params: Vec<KeyValue> = Vec::new();
    let mut path_params: Vec<String> = Vec::new();
    for p in &op.parameters {
        let ObjectOrReference::Object(param) = p else {
            // We don't follow $ref — would need the full spec context;
            // OAS specs rarely $ref query params anyway.
            continue;
        };
        match param.location {
            ParameterIn::Query => params.push(param_to_kv(param)),
            ParameterIn::Header => headers.push(param_to_kv(param)),
            ParameterIn::Path => path_params.push(param.name.clone()),
            ParameterIn::Cookie => {
                // Cookies aren't first-class in our request model. Treat as
                // a header. Better than silently losing them.
                headers.push(KeyValue {
                    key: "Cookie".into(),
                    value: format!("{}={}", param.name, example_for(param)),
                    enabled: true,
                });
            }
        }
    }
    (headers, params, path_params)
}

fn param_to_kv(p: &Parameter) -> KeyValue {
    KeyValue {
        key: p.name.clone(),
        value: example_for(p),
        enabled: true,
    }
}

/// Pulls a sensible default value out of the parameter's `example` /
/// `examples` / `schema.example`. Empty string when nothing's there —
/// the user fills it in.
fn example_for(p: &Parameter) -> String {
    if let Some(ex) = &p.example {
        return value_to_string(ex);
    }
    if let Some(ObjectOrReference::Object(e)) = p.examples.values().next() {
        if let Some(v) = &e.value {
            return value_to_string(v);
        }
    }
    String::new()
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn build_body(
    spec: &Spec,
    op: &Operation,
    warnings: &mut Vec<String>,
    req_name: &str,
) -> (BodyType, String) {
    let Some(ObjectOrReference::Object(body)) = &op.request_body else {
        return (BodyType::None, String::new());
    };
    body_to_apiovnia(spec, body, warnings, req_name).unwrap_or((BodyType::None, String::new()))
}

fn body_to_apiovnia(
    spec: &Spec,
    body: &RequestBody,
    warnings: &mut Vec<String>,
    req_name: &str,
) -> Option<(BodyType, String)> {
    // Preference order: JSON > form > multipart > text/* > anything-else.
    if let Some(mt) = body.content.get("application/json") {
        if let Some(ex) = first_example(mt) {
            return Some((
                BodyType::Json,
                serde_json::to_string_pretty(&ex).unwrap_or_else(|_| ex.to_string()),
            ));
        }
        // No example — resolve the schema (`$ref` followed where present)
        // and synthesise a dummy object from its properties. Falls back to
        // `{}` only when even the schema can't be reached.
        let dummy = mt.schema(spec).ok().map_or_else(
            || Value::Object(serde_json::Map::new()),
            |s| synthesize_from_schema(spec, &s, 0),
        );
        let pretty = serde_json::to_string_pretty(&dummy).unwrap_or_else(|_| "{}".into());
        return Some((BodyType::Json, pretty));
    }
    if let Some(mt) = body.content.get("application/x-www-form-urlencoded") {
        let rows = kv_list_from_media_type(spec, mt);
        return Some((
            BodyType::Form,
            serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into()),
        ));
    }
    if let Some(mt) = body.content.get("multipart/form-data") {
        let rows: Vec<serde_json::Value> = kv_list_from_media_type(spec, mt)
            .into_iter()
            .map(|kv| {
                serde_json::json!({
                    "key": kv.key,
                    "value": kv.value,
                    "kind": "text",
                    "filePath": "",
                    "contentType": "",
                    "enabled": true,
                })
            })
            .collect();
        return Some((
            BodyType::Multipart,
            serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into()),
        ));
    }
    // Any remaining content type → raw text using the first content type's
    // example (or empty).
    if let Some((ct, mt)) = body.content.iter().next() {
        warnings.push(format!(
            "request '{req_name}': body content-type '{ct}' imported as Raw"
        ));
        let content = first_example(mt)
            .map(|v| value_to_string(&v))
            .unwrap_or_default();
        return Some((BodyType::Raw, content));
    }
    None
}

/// Walk a schema (resolving `$ref`s via the spec) and emit a JSON value
/// representing a "dummy" example. For objects: every property gets a
/// type-appropriate placeholder. For primitives: zero/false/"string".
/// `format` hints (date-time, email, uuid, uri) give nicer defaults.
///
/// `depth` guards against pathological recursive schemas — past 8 levels
/// we emit `null` rather than risk an infinite loop on a self-referential
/// schema (`type: object, properties: { self: $ref: this }`).
fn synthesize_from_schema(spec: &Spec, schema: &ObjectSchema, depth: u8) -> Value {
    if depth > 8 {
        return Value::Null;
    }

    // Honour an explicit `example` / `default` / first `enum` value first —
    // any of these is a strong signal of what the field should look like.
    if let Some(v) = &schema.example {
        return v.clone();
    }
    if let Some(v) = &schema.default {
        return v.clone();
    }
    if let Some(v) = schema.enum_values.first() {
        return v.clone();
    }

    // allOf: merge synthesised objects (Pet = NewPet ∪ {id}).
    if !schema.all_of.is_empty() {
        let mut merged = serde_json::Map::new();
        for sub in &schema.all_of {
            if let Ok(s) = sub.resolve(spec) {
                if let Value::Object(map) = synthesize_from_schema(spec, &s, depth + 1) {
                    merged.extend(map);
                }
            }
        }
        return Value::Object(merged);
    }

    // anyOf / oneOf: pick the first branch — best-effort dummy.
    if let Some(branch) = schema.any_of.first().or_else(|| schema.one_of.first()) {
        if let Ok(s) = branch.resolve(spec) {
            return synthesize_from_schema(spec, &s, depth + 1);
        }
    }

    // Object with declared properties.
    if !schema.properties.is_empty() {
        let mut obj = serde_json::Map::new();
        for (key, prop) in &schema.properties {
            let val = prop
                .resolve(spec)
                .map_or(Value::Null, |s| synthesize_from_schema(spec, &s, depth + 1));
            obj.insert(key.clone(), val);
        }
        return Value::Object(obj);
    }

    // Array.
    if schema
        .schema_type
        .as_ref()
        .is_some_and(|t| t.contains(Type::Array))
    {
        if let Some(items) = &schema.items {
            if let Ok(item_schema) = items.resolve(spec) {
                return Value::Array(vec![synthesize_from_schema(
                    spec,
                    &item_schema,
                    depth + 1,
                )]);
            }
        }
        return Value::Array(Vec::new());
    }

    // Primitive — pick a default for the first matching type.
    if let Some(t) = &schema.schema_type {
        if t.contains(Type::String) {
            return Value::String(string_default_for_format(schema.format.as_deref()));
        }
        if t.contains(Type::Integer) {
            return Value::Number(0_i64.into());
        }
        if t.contains(Type::Number) {
            return serde_json::Number::from_f64(0.0).map_or(Value::Null, Value::Number);
        }
        if t.contains(Type::Boolean) {
            return Value::Bool(false);
        }
        if t.contains(Type::Null) {
            return Value::Null;
        }
    }

    // Unknown / untyped — empty object is the least surprising default.
    Value::Object(serde_json::Map::new())
}

fn string_default_for_format(format: Option<&str>) -> String {
    match format {
        Some("date-time") => "2024-01-01T00:00:00Z".into(),
        Some("date") => "2024-01-01".into(),
        Some("time") => "00:00:00".into(),
        Some("email") => "user@example.com".into(),
        Some("uuid") => "00000000-0000-0000-0000-000000000000".into(),
        Some("uri" | "url") => "https://example.com".into(),
        Some("ipv4") => "0.0.0.0".into(),
        Some("ipv6") => "::1".into(),
        // base64 + binary — placeholder is fine empty in both cases
        Some("byte" | "binary") => String::new(),
        _ => "string".into(),
    }
}

/// Form / multipart row list. Prefers the media type's example; falls back
/// to synthesising one from the schema (same as JSON path) so users get a
/// populated row list even when only `$ref`s are present.
fn kv_list_from_media_type(spec: &Spec, mt: &oas3::spec::MediaType) -> Vec<KeyValue> {
    let example = first_example(mt).or_else(|| {
        mt.schema(spec)
            .ok()
            .map(|s| synthesize_from_schema(spec, &s, 0))
    });
    let Some(Value::Object(map)) = example else {
        return Vec::new();
    };
    map.into_iter()
        .map(|(k, v)| KeyValue {
            key: k,
            value: value_to_string(&v),
            enabled: true,
        })
        .collect()
}

/// First example value across the various `OpenAPI` ways to express one.
fn first_example(mt: &oas3::spec::MediaType) -> Option<Value> {
    use oas3::spec::MediaTypeExamples;
    match &mt.examples {
        Some(MediaTypeExamples::Example { example }) => Some(example.clone()),
        Some(MediaTypeExamples::Examples { examples }) => {
            for ex in examples.values() {
                if let ObjectOrReference::Object(e) = ex {
                    if let Some(v) = &e.value {
                        return Some(v.clone());
                    }
                }
            }
            None
        }
        None => None,
    }
}

/// Pick an auth strategy from the operation's `security` requirements +
/// the spec's `components.securitySchemes`. We honour the FIRST scheme
/// referenced by the operation; `OpenAPI` lets ops list multiple alternatives
/// but Apiovnia's auth field holds one config.
fn build_auth(op: &Operation, spec: &Spec) -> (AuthConfig, Option<String>) {
    // Operation-level security wins; fall back to global security.
    let security = if op.security.is_empty() {
        &spec.security
    } else {
        &op.security
    };
    let Some(first) = security.first() else {
        return (AuthConfig::None, None);
    };
    let Some((scheme_name, _)) = first.0.iter().next() else {
        return (AuthConfig::None, None);
    };

    let schemes: &BTreeMap<String, ObjectOrReference<SecurityScheme>> = match spec
        .components
        .as_ref()
        .map(|c| &c.security_schemes)
    {
        Some(m) => m,
        None => return (AuthConfig::None, Some("operation declares security but spec has no securitySchemes".into())),
    };
    let Some(ObjectOrReference::Object(scheme)) = schemes.get(scheme_name) else {
        return (AuthConfig::None, Some(format!("security scheme '{scheme_name}' is a $ref or missing")));
    };

    match scheme {
        SecurityScheme::Http { scheme: kind, .. } => match kind.to_ascii_lowercase().as_str() {
            "bearer" => (AuthConfig::Bearer { token: String::new() }, None),
            "basic" => (
                AuthConfig::Basic {
                    username: String::new(),
                    password: String::new(),
                },
                None,
            ),
            other => (
                AuthConfig::None,
                Some(format!("HTTP auth scheme '{other}' not supported — left as None")),
            ),
        },
        SecurityScheme::ApiKey { name, location, .. } => {
            let loc = match location.to_ascii_lowercase().as_str() {
                "header" => ApiKeyLocation::Header,
                "query" => ApiKeyLocation::Query,
                "cookie" => {
                    return (
                        AuthConfig::None,
                        Some("API key in cookie not supported — left as None".into()),
                    );
                }
                other => {
                    return (
                        AuthConfig::None,
                        Some(format!("API key in '{other}' not supported — left as None")),
                    );
                }
            };
            (
                AuthConfig::ApiKey {
                    name: name.clone(),
                    value: String::new(),
                    r#in: loc,
                },
                None,
            )
        }
        SecurityScheme::OAuth2 { .. } => (
            AuthConfig::None,
            Some("OAuth2 not supported in MVP — configure manually".into()),
        ),
        SecurityScheme::OpenIdConnect { .. } => (
            AuthConfig::None,
            Some("OpenID Connect not supported in MVP — configure manually".into()),
        ),
        SecurityScheme::MutualTls { .. } => (
            AuthConfig::None,
            Some("mutual TLS not supported in MVP — configure manually".into()),
        ),
    }
}

// ---------------------------------------------------------------------------
// Environments
// ---------------------------------------------------------------------------

fn build_environments(
    spec: &Spec,
    project_id: &ProjectId,
    now: i64,
    warnings: &mut Vec<String>,
) -> Vec<Environment> {
    if spec.servers.len() < 2 {
        // One server (or none) → no env needed; the URL alone suffices.
        return Vec::new();
    }
    let mut out: Vec<Environment> = Vec::with_capacity(spec.servers.len());
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, server) in spec.servers.iter().enumerate() {
        let raw = server.description.clone().unwrap_or_default();
        let mut name = env_name_from_description(&raw).unwrap_or_else(|| format!("env{}", i + 1));
        // Deduplicate.
        let base = name.clone();
        let mut n = 2;
        while seen_names.contains(&name) {
            name = format!("{base}-{n}");
            n += 1;
        }
        seen_names.insert(name.clone());
        out.push(Environment {
            id: EnvironmentId::new(),
            project_id: project_id.clone(),
            name,
            requires_unlock: false,
            is_encrypted: false,
            created_at: now + i64::try_from(i).unwrap_or(0),
        });
    }
    if !out.is_empty() {
        warnings.push(format!(
            "created {} environment(s) from servers[] — variable substitution and overrides are configured per-request",
            out.len()
        ));
    }
    out
}

fn env_name_from_description(desc: &str) -> Option<String> {
    let lc = desc.to_ascii_lowercase();
    for needle in [
        "production",
        "prod",
        "staging",
        "stage",
        "development",
        "dev",
        "local",
        "qa",
        "test",
        "sandbox",
    ] {
        if lc.contains(needle) {
            return Some(short_env(needle));
        }
    }
    None
}

fn short_env(s: &str) -> String {
    match s {
        "production" => "prod".into(),
        "development" => "dev".into(),
        "staging" => "stage".into(),
        other => other.into(),
    }
}

fn servers_iter(spec: &Spec) -> impl Iterator<Item = String> + '_ {
    spec.servers.iter().map(|s| s.url.clone())
}

fn strip_trailing_slash(s: &str) -> String {
    s.strip_suffix('/').map_or_else(|| s.to_string(), str::to_string)
}

// ---------------------------------------------------------------------------
// Log rendering
// ---------------------------------------------------------------------------

fn render_log(
    collection: &Collection,
    rows: &[ImportRow],
    envs: &[Environment],
    warnings: &[String],
) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    // The `<timestamp>` placeholder is replaced by the IPC layer (which has
    // the wall-clock dep) — keep it literal here.
    let _ = writeln!(
        s,
        "# Apiovnia OpenAPI import log\n# collection: {}\n# generated: <timestamp>\n",
        collection.name,
    );
    let _ = writeln!(s, "## Requests ({})", rows.len());
    for r in rows {
        let _ = writeln!(s, "  - {:<6} {:<40}  [{}]", r.method, r.path, r.name);
    }
    let _ = writeln!(s, "\n## Environments ({})", envs.len());
    for e in envs {
        let _ = writeln!(s, "  - {}", e.name);
    }
    if warnings.is_empty() {
        let _ = writeln!(s, "\n## Warnings\n  (none)");
    } else {
        let _ = writeln!(s, "\n## Warnings ({})", warnings.len());
        for w in warnings {
            let _ = writeln!(s, "  - {w}");
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use apiovnia_core::ids::{CollectionId, ProjectId};

    const MINIMAL: &str = r"
openapi: 3.0.0
info: { title: My API, version: '1.0.0' }
paths:
  /things:
    get:
      summary: List things
      responses: { '200': { description: ok } }
    post:
      summary: Create thing
      responses: { '201': { description: created } }
";

    #[test]
    fn parses_minimal_yaml_into_collection_and_requests() {
        let out = import_document(MINIMAL, &ProjectId::new(), CollectionId::new()).unwrap();
        assert_eq!(out.collection.name, "My API");
        assert_eq!(out.requests.len(), 2);
        // Order GET first, POST second (path_item_operations order).
        assert_eq!(out.requests[0].name, "List things");
        assert_eq!(out.requests[0].method, HttpMethod::Get);
        assert_eq!(out.requests[1].method, HttpMethod::Post);
    }

    #[test]
    fn uses_operation_id_when_summary_missing() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /x:
    get:
      operationId: listX
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        assert_eq!(out.requests[0].name, "listX");
    }

    #[test]
    fn falls_back_to_method_path_for_unnamed_op() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /x:
    delete:
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        assert_eq!(out.requests[0].name, "DELETE /x");
    }

    #[test]
    fn maps_query_and_header_params() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /items:
    get:
      parameters:
        - { name: limit, in: query, example: 10 }
        - { name: X-Trace, in: header, example: 'abc' }
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        let r = &out.requests[0];
        assert_eq!(r.params.len(), 1);
        assert_eq!(r.params[0].key, "limit");
        assert_eq!(r.params[0].value, "10");
        assert_eq!(r.headers.len(), 1);
        assert_eq!(r.headers[0].key, "X-Trace");
        assert_eq!(r.headers[0].value, "abc");
    }

    #[test]
    fn path_params_preserved_as_braces_in_url() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
servers: [{ url: https://api.example.com }]
paths:
  /users/{id}:
    get:
      parameters:
        - { name: id, in: path, required: true }
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        let r = &out.requests[0];
        assert_eq!(r.url, "https://api.example.com/users/{id}");
        // Warning about path params being preserved.
        assert!(out.warnings.iter().any(|w| w.contains("path param")));
    }

    #[test]
    fn json_request_body_example_becomes_body_content() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /login:
    post:
      requestBody:
        content:
          application/json:
            example: { username: alice, password: hunter2 }
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        let r = &out.requests[0];
        assert_eq!(r.body_type, BodyType::Json);
        let v: serde_json::Value = serde_json::from_str(&r.body_content).unwrap();
        assert_eq!(v["username"], "alice");
        assert_eq!(v["password"], "hunter2");
    }

    #[test]
    fn json_body_without_example_emits_empty_object() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /x:
    post:
      requestBody: { content: { application/json: { schema: { type: object } } } }
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        assert_eq!(out.requests[0].body_type, BodyType::Json);
        assert_eq!(out.requests[0].body_content, "{}");
    }

    #[test]
    fn json_body_synthesizes_dummy_from_ref_schema() {
        // The petstore-style case: requestBody schema is `$ref: NewPet`.
        // We resolve the ref + walk properties + emit type-default values.
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /pets:
    post:
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/NewPet'
      responses: { '200': { description: ok } }
components:
  schemas:
    NewPet:
      type: object
      required: [name]
      properties:
        name: { type: string }
        tag:  { type: string }
        count: { type: integer }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        assert_eq!(out.requests[0].body_type, BodyType::Json);
        let v: serde_json::Value = serde_json::from_str(&out.requests[0].body_content).unwrap();
        assert_eq!(v["name"], "string");
        assert_eq!(v["tag"], "string");
        assert_eq!(v["count"], 0);
    }

    #[test]
    fn json_body_synthesizes_allof_merge() {
        // Pet is allOf [NewPet, {id}] — synth should merge.
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /pets:
    post:
      requestBody:
        content:
          application/json:
            schema: { $ref: '#/components/schemas/Pet' }
      responses: { '200': { description: ok } }
components:
  schemas:
    NewPet:
      type: object
      properties:
        name: { type: string }
    Pet:
      allOf:
        - $ref: '#/components/schemas/NewPet'
        - type: object
          properties:
            id: { type: integer, format: int64 }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out.requests[0].body_content).unwrap();
        assert_eq!(v["name"], "string");
        assert_eq!(v["id"], 0);
    }

    #[test]
    fn json_body_synthesizes_nested_array_of_refs() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /x:
    post:
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                items:
                  type: array
                  items: { $ref: '#/components/schemas/Item' }
      responses: { '200': { description: ok } }
components:
  schemas:
    Item:
      type: object
      properties:
        sku: { type: string }
        qty: { type: integer }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out.requests[0].body_content).unwrap();
        assert!(v["items"].is_array());
        let first = &v["items"][0];
        assert_eq!(first["sku"], "string");
        assert_eq!(first["qty"], 0);
    }

    #[test]
    fn json_body_format_hints_give_richer_defaults() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /x:
    post:
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                created_at: { type: string, format: date-time }
                email:      { type: string, format: email }
                id:         { type: string, format: uuid }
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out.requests[0].body_content).unwrap();
        assert_eq!(v["created_at"], "2024-01-01T00:00:00Z");
        assert_eq!(v["email"], "user@example.com");
        assert_eq!(v["id"], "00000000-0000-0000-0000-000000000000");
    }

    #[test]
    fn json_body_prefers_explicit_example_over_schema_synth() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /x:
    post:
      requestBody:
        content:
          application/json:
            schema: { $ref: '#/components/schemas/T' }
            example: { name: 'Fluffy' }
      responses: { '200': { description: ok } }
components:
  schemas:
    T:
      type: object
      properties:
        name: { type: string }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out.requests[0].body_content).unwrap();
        // Explicit example wins — the synth wouldn't produce "Fluffy".
        assert_eq!(v["name"], "Fluffy");
    }

    #[test]
    fn form_body_becomes_kv_list() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
paths:
  /x:
    post:
      requestBody:
        content:
          application/x-www-form-urlencoded:
            example: { a: '1', b: '2' }
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        let r = &out.requests[0];
        assert_eq!(r.body_type, BodyType::Form);
        let rows: Vec<KeyValue> = serde_json::from_str(&r.body_content).unwrap();
        assert_eq!(rows.len(), 2);
        let mut keys: Vec<&str> = rows.iter().map(|kv| kv.key.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn bearer_security_recognized_with_empty_token() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
components:
  securitySchemes:
    bearerAuth: { type: http, scheme: bearer }
security:
  - bearerAuth: []
paths:
  /me:
    get:
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        match &out.requests[0].auth {
            AuthConfig::Bearer { token } => assert!(token.is_empty()),
            _ => panic!("expected Bearer"),
        }
    }

    #[test]
    fn apikey_in_header_recognized() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
components:
  securitySchemes:
    apiKeyAuth: { type: apiKey, in: header, name: X-API-Key }
paths:
  /x:
    get:
      security: [{ apiKeyAuth: [] }]
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        match &out.requests[0].auth {
            AuthConfig::ApiKey { name, value, r#in } => {
                assert_eq!(name, "X-API-Key");
                assert!(value.is_empty());
                assert!(matches!(r#in, ApiKeyLocation::Header));
            }
            _ => panic!("expected ApiKey"),
        }
    }

    #[test]
    fn oauth2_security_falls_back_to_none_with_warning() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
components:
  securitySchemes:
    oauth: { type: oauth2, flows: {} }
paths:
  /x:
    get:
      security: [{ oauth: [] }]
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        assert!(matches!(out.requests[0].auth, AuthConfig::None));
        assert!(out.warnings.iter().any(|w| w.contains("OAuth2")));
    }

    #[test]
    fn multiple_servers_become_environments_with_overrides() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
servers:
  - { url: 'https://api.example.com',     description: production }
  - { url: 'https://api-stage.example.com', description: staging }
paths:
  /x:
    get:
      responses: { '200': { description: ok } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        assert_eq!(out.environments.len(), 2);
        let names: Vec<&str> = out.environments.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"prod"));
        assert!(names.contains(&"stage"));
        // Base URL is the first server; second server has an override.
        assert_eq!(out.requests[0].url, "https://api.example.com/x");
        assert_eq!(out.overrides.len(), 1);
        assert_eq!(out.overrides[0].url.as_deref(), Some("https://api-stage.example.com/x"));
    }

    #[test]
    fn single_server_does_not_create_environment() {
        let yaml = r"
openapi: 3.0.0
info: { title: t, version: '1' }
servers: [{ url: 'https://api.example.com' }]
paths:
  /x: { get: { responses: { '200': { description: ok } } } }
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        assert!(out.environments.is_empty());
        assert!(out.overrides.is_empty());
    }

    #[test]
    fn log_lists_requests_and_environments() {
        let out = import_document(MINIMAL, &ProjectId::new(), CollectionId::new()).unwrap();
        assert!(out.log.contains("## Requests (2)"));
        assert!(out.log.contains("List things"));
        assert!(out.log.contains("Create thing"));
    }

    #[test]
    fn invalid_yaml_returns_parse_error() {
        let err = import_document("not: valid: openapi: at all", &ProjectId::new(), CollectionId::new())
            .expect_err("expected error");
        assert!(matches!(err, ImportError::Parse(_)));
    }

    #[test]
    fn empty_title_falls_back_to_imported() {
        let yaml = r"
openapi: 3.0.0
info: { title: '', version: '1' }
paths: {}
";
        let out = import_document(yaml, &ProjectId::new(), CollectionId::new()).unwrap();
        assert_eq!(out.collection.name, "Imported");
    }
}
