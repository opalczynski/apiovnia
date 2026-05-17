//! Build an `OpenAPI` 3.0.3 YAML document from an Apiovnia collection.
//!
//! Best-effort by design: we don't model responses or schemas in our own
//! domain, so the export fills in placeholders (`200 Success`, minimal
//! request-body schemas inferred from the example). Required-fields-only
//! by default plus the parts every editor/codegen actually reads.
//!
//! **Secrets are scrubbed before they touch this module** — every request
//! we receive is expected to have been through [`crate::redact::redact_request`]
//! first; we don't double-check. The caller (IPC command layer) is
//! responsible for the order.
//!
//! Collision policy: `(method, path)` must be unique across the export.
//! Two requests resolving to the same pair → [`ExportError::Collision`].
//! That's the "abort on collision" semantic we agreed on, vs silently
//! overwriting one of them.

use std::collections::{BTreeMap, BTreeSet};

use apiovnia_core::model::{ApiKeyLocation, AuthConfig, BodyType, Collection, HttpMethod, Request};
use oas3::spec::{
    Info, MediaType, MediaTypeExamples, ObjectOrReference, ObjectSchema, Operation, Parameter,
    ParameterIn, PathItem, RequestBody, Response, SchemaType, SchemaTypeSet, SecurityRequirement,
    SecurityScheme, Server, Spec,
};
use serde_json::Value;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error(
        "two or more requests would collide on {method} {path}: {requests:?} — \
         rename one or change its URL/method before exporting"
    )]
    Collision {
        method: String,
        path: String,
        requests: Vec<String>,
    },
    #[error("serialise to YAML: {0}")]
    Serde(String),
}

/// What the export produced. The `log` field is a human-readable audit
/// trail the frontend can drop into a `.log` file via the `OpLog` "Download
/// log" button.
#[derive(Debug)]
pub struct ExportResult {
    /// The `OpenAPI` 3.0.3 document, ready to write to disk.
    pub yaml: String,
    /// Per-request summary lines for the `OpLog` table.
    pub rows: Vec<ExportRow>,
    /// Soft notices that don't fail the export (skipped fields, unparseable
    /// URLs, etc.). Empty when everything went smoothly.
    pub warnings: Vec<String>,
    /// Full textual log — header + per-request lines + warnings. Suitable
    /// for `export_{collection}_{date}.log`.
    pub log: String,
}

/// One row in the `OpLog` table.
#[derive(Debug, Clone)]
pub struct ExportRow {
    pub name: String,
    pub method: String,
    pub path: String,
    /// Number of secrets we replaced (across headers/params/body/auth).
    pub redactions: usize,
}

/// Run the export.
///
/// `requests` should already be redacted (call [`crate::redact::redact_request`]
/// per item and keep the tally for the caller's log alongside the export
/// itself; we don't redact here so the secret-detection logic stays in one
/// module and the caller controls the user's policy).
pub fn export_collection(
    collection: &Collection,
    requests: &[(Request, crate::redact::RedactionTally)],
) -> Result<ExportResult, ExportError> {
    let mut spec = base_spec(collection);
    let mut paths: BTreeMap<String, PathItem> = BTreeMap::new();
    let mut security_schemes: BTreeMap<String, ObjectOrReference<SecurityScheme>> = BTreeMap::new();
    // Per-request body schemas keyed by `{PascalCaseOperationId}Request`.
    // Inferred from the redacted JSON example via `schema_from_example`,
    // emitted into `components.schemas` so the request body can `$ref` them
    // — same pattern as a hand-written spec.
    let mut request_schemas: BTreeMap<String, ObjectOrReference<ObjectSchema>> = BTreeMap::new();
    let mut servers_seen: BTreeSet<String> = BTreeSet::new();
    let mut rows: Vec<ExportRow> = Vec::with_capacity(requests.len());
    let mut warnings: Vec<String> = Vec::new();

    for (req, tally) in requests {
        if req.url.trim().is_empty() {
            warnings.push(format!(
                "skipped request '{}': URL is empty",
                req.name
            ));
            continue;
        }

        let (server_url, path) = split_url(&req.url, &mut warnings, &req.name);
        if let Some(srv) = server_url {
            servers_seen.insert(srv);
        }

        let method_str = req.method.as_str();
        let entry = paths.entry(path.clone()).or_default();

        if pathitem_has_method(entry, req.method) {
            // Find the other request that already occupies this slot.
            let other = rows
                .iter()
                .find(|r| r.method == method_str && r.path == path)
                .map_or_else(|| "<unknown>".into(), |r| r.name.clone());
            return Err(ExportError::Collision {
                method: method_str.into(),
                path,
                requests: vec![other, req.name.clone()],
            });
        }

        // `build_operation` attaches security inside; the returned `_sec_used`
        // marker is informational only — we don't need to act on it here.
        let (op, _sec_used) = build_operation(req, &mut security_schemes, &mut request_schemas);

        place_operation(entry, req.method, op);

        rows.push(ExportRow {
            name: req.name.clone(),
            method: method_str.into(),
            path,
            redactions: tally.total(),
        });
    }

    // Servers: stable BTreeSet order → alphabetical by URL.
    if !servers_seen.is_empty() {
        spec.servers = servers_seen
            .into_iter()
            .map(|url| Server {
                url,
                description: None,
                variables: BTreeMap::new(),
            })
            .collect();
    }

    if !security_schemes.is_empty() || !request_schemas.is_empty() {
        let comps = spec.components.get_or_insert_with(Default::default);
        for (k, v) in security_schemes {
            comps.security_schemes.insert(k, v);
        }
        for (k, v) in request_schemas {
            comps.schemas.insert(k, v);
        }
    }

    spec.paths = Some(paths);

    let yaml = oas3::to_yaml(&spec).map_err(|e| ExportError::Serde(e.to_string()))?;
    let log = render_log(collection, &rows, &warnings);

    Ok(ExportResult {
        yaml,
        rows,
        warnings,
        log,
    })
}

// ---------------------------------------------------------------------------
// Spec construction
// ---------------------------------------------------------------------------

fn base_spec(collection: &Collection) -> Spec {
    Spec {
        openapi: "3.0.3".into(),
        info: Info {
            title: collection.name.clone(),
            summary: None,
            description: Some(
                "Exported from Apiovnia. Secrets have been replaced with `<your-*>` placeholders — replace them before use."
                    .to_string(),
            ),
            terms_of_service: None,
            version: "1.0.0".into(),
            contact: None,
            license: None,
            extensions: BTreeMap::new(),
        },
        servers: Vec::new(),
        paths: None,
        components: None,
        security: Vec::new(),
        tags: Vec::new(),
        webhooks: BTreeMap::new(),
        external_docs: None,
        extensions: BTreeMap::new(),
    }
}

fn build_operation(
    req: &Request,
    security_schemes: &mut BTreeMap<String, ObjectOrReference<SecurityScheme>>,
    request_schemas: &mut BTreeMap<String, ObjectOrReference<ObjectSchema>>,
) -> (Operation, Option<()>) {
    let op_id = slug(&req.name);
    let mut op = Operation {
        summary: Some(req.name.clone()),
        operation_id: Some(op_id.clone()),
        ..Operation::default()
    };

    // Parameters: enabled query + header rows.
    let mut params: Vec<ObjectOrReference<Parameter>> = Vec::new();
    for kv in &req.params {
        if !kv.enabled {
            continue;
        }
        params.push(ObjectOrReference::Object(simple_param(
            kv.key.clone(),
            ParameterIn::Query,
            kv.value.clone(),
        )));
    }
    for kv in &req.headers {
        if !kv.enabled {
            continue;
        }
        // Skip content-type — it's expressed via requestBody.content key.
        if kv.key.eq_ignore_ascii_case("content-type") {
            continue;
        }
        params.push(ObjectOrReference::Object(simple_param(
            kv.key.clone(),
            ParameterIn::Header,
            kv.value.clone(),
        )));
    }
    op.parameters = params;

    // Path parameters: detect `{name}` segments — emit them as required.
    for pp in extract_path_params(&req.url) {
        op.parameters.push(ObjectOrReference::Object(Parameter {
            name: pp,
            location: ParameterIn::Path,
            description: None,
            required: Some(true),
            deprecated: None,
            allow_empty_value: None,
            style: None,
            explode: None,
            allow_reserved: None,
            schema: None,
            example: Some(Value::String("<value>".into())),
            examples: BTreeMap::new(),
            content: None,
            extensions: BTreeMap::new(),
        }));
    }

    // Request body — JSON path infers a schema from the example, registers
    // it in `components.schemas`, and `$ref`s it from the media type. Other
    // body types keep the existing example-only shape (no schema inference).
    if let Some(body) = build_request_body(req, &op_id, request_schemas) {
        op.request_body = Some(ObjectOrReference::Object(body));
    }

    // Security
    if let Some((name, scheme)) = build_security_scheme(&req.auth) {
        security_schemes
            .entry(name.clone())
            .or_insert(ObjectOrReference::Object(scheme));
        let mut req_obj: BTreeMap<String, Vec<String>> = BTreeMap::new();
        req_obj.insert(name, Vec::new());
        op.security = vec![SecurityRequirement(req_obj)];
    }

    // Responses placeholder.
    let mut responses = BTreeMap::new();
    responses.insert(
        "200".into(),
        ObjectOrReference::Object(Response {
            description: Some("Successful response".into()),
            headers: BTreeMap::new(),
            content: BTreeMap::new(),
            links: BTreeMap::new(),
            extensions: BTreeMap::new(),
        }),
    );
    op.responses = Some(responses);

    (op, None)
}

fn simple_param(name: String, location: ParameterIn, example_value: String) -> Parameter {
    Parameter {
        name,
        location,
        description: None,
        required: None,
        deprecated: None,
        allow_empty_value: None,
        style: None,
        explode: None,
        allow_reserved: None,
        schema: None,
        example: if example_value.is_empty() {
            None
        } else {
            Some(Value::String(example_value))
        },
        examples: BTreeMap::new(),
        content: None,
        extensions: BTreeMap::new(),
    }
}

fn build_request_body(
    req: &Request,
    op_id: &str,
    request_schemas: &mut BTreeMap<String, ObjectOrReference<ObjectSchema>>,
) -> Option<RequestBody> {
    if matches!(req.body_type, BodyType::None) {
        return None;
    }
    let content_type = match req.body_type {
        BodyType::Json => "application/json",
        BodyType::Form => "application/x-www-form-urlencoded",
        BodyType::Multipart => "multipart/form-data",
        BodyType::Raw => content_type_from_headers(req).unwrap_or("text/plain"),
        BodyType::None => unreachable!(),
    };
    let mt = match req.body_type {
        BodyType::Json => media_type_with_json_example(&req.body_content, op_id, request_schemas),
        BodyType::Form | BodyType::Multipart => media_type_with_kv_example(&req.body_content),
        BodyType::Raw => media_type_with_text_example(&req.body_content),
        BodyType::None => unreachable!(),
    };
    let mut content = BTreeMap::new();
    content.insert(content_type.to_string(), mt);
    Some(RequestBody {
        description: None,
        content,
        required: Some(true),
    })
}

fn content_type_from_headers(req: &Request) -> Option<&'static str> {
    for h in &req.headers {
        if h.enabled && h.key.eq_ignore_ascii_case("content-type") {
            // We only return a 'static — fall back to text/plain if user
            // set something exotic. Round-tripping the original string
            // isn't worth the lifetime complexity here.
            let lc = h.value.to_ascii_lowercase();
            if lc.contains("xml") {
                return Some("application/xml");
            }
            if lc.contains("html") {
                return Some("text/html");
            }
            if lc.contains("csv") {
                return Some("text/csv");
            }
            return Some("text/plain");
        }
    }
    None
}

/// JSON body media type — registers an inferred schema in
/// `request_schemas` (keyed by `{PascalCaseOpId}Request`) and points the
/// media type at it via `$ref`. Falls back to a no-schema inline-example
/// shape when the body isn't parseable JSON (rare for `BodyType::Json`,
/// but possible if the user pasted a template like `{{var}}`).
fn media_type_with_json_example(
    body_content: &str,
    op_id: &str,
    request_schemas: &mut BTreeMap<String, ObjectOrReference<ObjectSchema>>,
) -> MediaType {
    // Try to parse the body as JSON. If we can, register an inferred
    // schema + ref it. Either way we emit an `example` for legibility.
    let (example, schema): (Value, Option<ObjectOrReference<ObjectSchema>>) =
        match serde_json::from_str::<Value>(body_content) {
            Ok(parsed) => {
                let schema_name = unique_schema_name(op_id, request_schemas);
                let inferred = schema_from_example(&parsed);
                request_schemas.insert(schema_name.clone(), ObjectOrReference::Object(inferred));
                let ref_link = ObjectOrReference::Ref {
                    ref_path: format!("#/components/schemas/{schema_name}"),
                };
                (parsed, Some(ref_link))
            }
            // Body wasn't valid JSON (rare for BodyType::Json; possible if
            // the user pasted a `{{var}}` template). Emit example only.
            Err(_) => (Value::String(body_content.to_string()), None),
        };

    MediaType {
        schema,
        examples: Some(MediaTypeExamples::Example { example }),
        encoding: BTreeMap::new(),
    }
}

/// Produce a unique key in `request_schemas` derived from `op_id` (snake)
/// → `PascalCase{Request}`. Collision suffix `_2`, `_3`, … keeps the
/// invariant that every body has its own entry.
fn unique_schema_name(
    op_id: &str,
    existing: &BTreeMap<String, ObjectOrReference<ObjectSchema>>,
) -> String {
    let base = format!("{}Request", pascal_case(op_id));
    if !existing.contains_key(&base) {
        return base;
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{base}_{n}");
        if !existing.contains_key(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn pascal_case(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut cap_next = true;
    for c in snake.chars() {
        if c == '_' || c == '-' {
            cap_next = true;
        } else if cap_next {
            out.push(c.to_ascii_uppercase());
            cap_next = false;
        } else {
            out.push(c);
        }
    }
    if out.is_empty() {
        "Body".into()
    } else {
        out
    }
}

// ---------------------------------------------------------------------------
// Schema inference (best-effort) — walks a JSON example, emits an
// `ObjectSchema` with type + format hints. Conservative: no `nullable`,
// no `additionalProperties: false`, no `minLength`/`maxLength`. The point
// is to give tooling something to bind to + signal the field shape, not
// to be authoritative.
// ---------------------------------------------------------------------------

/// Top-level entry. Object keys are added to `required` unless their value
/// is `null` — a `null` example is the only signal we have for "optional".
fn schema_from_example(value: &Value) -> ObjectSchema {
    match value {
        Value::String(_) => primitive_schema(SchemaType::String, detect_string_format(value), value),
        Value::Bool(_) => primitive_schema(SchemaType::Boolean, None, value),
        Value::Number(n) => {
            let (ty, fmt) = if n.is_i64() || n.is_u64() {
                (SchemaType::Integer, Some("int64".into()))
            } else {
                (SchemaType::Number, Some("double".into()))
            };
            primitive_schema(ty, fmt, value)
        }
        Value::Null => ObjectSchema {
            // OpenAPI 3.0 doesn't have a clean `null` type; leave the schema
            // empty so consumers don't reject the field on validation.
            ..ObjectSchema::default()
        },
        Value::Array(arr) => {
            let item_schema = arr.first().map(schema_from_example).unwrap_or_default();
            ObjectSchema {
                schema_type: Some(SchemaTypeSet::Single(SchemaType::Array)),
                items: Some(Box::new(ObjectOrReference::Object(item_schema))),
                ..ObjectSchema::default()
            }
        }
        Value::Object(map) => {
            let mut properties: BTreeMap<String, ObjectOrReference<ObjectSchema>> = BTreeMap::new();
            let mut required: Vec<String> = Vec::new();
            for (k, v) in map {
                if !matches!(v, Value::Null) {
                    required.push(k.clone());
                }
                properties.insert(
                    k.clone(),
                    ObjectOrReference::Object(schema_from_example(v)),
                );
            }
            ObjectSchema {
                schema_type: Some(SchemaTypeSet::Single(SchemaType::Object)),
                properties,
                required,
                ..ObjectSchema::default()
            }
        }
    }
}

fn primitive_schema(ty: SchemaType, format: Option<String>, example: &Value) -> ObjectSchema {
    ObjectSchema {
        schema_type: Some(SchemaTypeSet::Single(ty)),
        format,
        example: Some(example.clone()),
        ..ObjectSchema::default()
    }
}

/// Heuristic format detection from a string value. Pure ASCII / char
/// checks (no regex dep) — recognises the formats `OpenAPI` tooling
/// actually cares about: date / date-time / email / uuid / uri.
fn detect_string_format(v: &Value) -> Option<String> {
    let Value::String(s) = v else {
        return None;
    };
    if is_iso_datetime(s) {
        return Some("date-time".into());
    }
    if is_iso_date(s) {
        return Some("date".into());
    }
    if is_uuid(s) {
        return Some("uuid".into());
    }
    if is_url(s) {
        return Some("uri".into());
    }
    if is_email(s) {
        return Some("email".into());
    }
    None
}

fn is_iso_datetime(s: &str) -> bool {
    // YYYY-MM-DDTHH:MM:SS[...timezone]. Don't validate the suffix — just
    // that the prefix shape matches.
    let b = s.as_bytes();
    b.len() >= 19
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
        && b[10] == b'T'
        && b[11..13].iter().all(u8::is_ascii_digit)
        && b[13] == b':'
        && b[14..16].iter().all(u8::is_ascii_digit)
        && b[16] == b':'
        && b[17..19].iter().all(u8::is_ascii_digit)
}

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
}

fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 36 {
        return false;
    }
    for (i, c) in b.iter().enumerate() {
        let dash = matches!(i, 8 | 13 | 18 | 23);
        if dash {
            if *c != b'-' {
                return false;
            }
        } else if !c.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

fn is_email(s: &str) -> bool {
    let parts: Vec<&str> = s.split('@').collect();
    parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.') && !parts[1].starts_with('.')
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn media_type_with_kv_example(body_content: &str) -> MediaType {
    // body_content here is JSON-encoded Vec<KeyValue> (form) or
    // Vec<MultipartRow> (multipart). For the example we flatten to a
    // simple {key: value} object — that's what most consumers expect.
    let mut obj = serde_json::Map::new();
    if let Ok(arr) = serde_json::from_str::<Vec<Value>>(body_content) {
        for row in arr {
            if let (Some(k), v) = (row.get("key").and_then(Value::as_str), row.get("value")) {
                obj.insert(k.to_string(), v.cloned().unwrap_or(Value::String(String::new())));
            }
        }
    }
    MediaType {
        schema: None,
        examples: Some(MediaTypeExamples::Example {
            example: Value::Object(obj),
        }),
        encoding: BTreeMap::new(),
    }
}

fn media_type_with_text_example(body_content: &str) -> MediaType {
    MediaType {
        schema: None,
        examples: Some(MediaTypeExamples::Example {
            example: Value::String(body_content.to_string()),
        }),
        encoding: BTreeMap::new(),
    }
}

/// Produces `(name, scheme)` for the components.securitySchemes section.
/// Names are stable so multiple requests using the same auth type share one
/// scheme entry instead of bloating the spec.
fn build_security_scheme(auth: &AuthConfig) -> Option<(String, SecurityScheme)> {
    match auth {
        AuthConfig::None => None,
        AuthConfig::Bearer { .. } => Some((
            "bearerAuth".into(),
            SecurityScheme::Http {
                description: Some("Bearer token in the Authorization header.".into()),
                scheme: "bearer".into(),
                bearer_format: None,
            },
        )),
        AuthConfig::Basic { .. } => Some((
            "basicAuth".into(),
            SecurityScheme::Http {
                description: Some("HTTP Basic auth.".into()),
                scheme: "basic".into(),
                bearer_format: None,
            },
        )),
        AuthConfig::ApiKey { name, r#in, .. } => Some((
            "apiKeyAuth".into(),
            SecurityScheme::ApiKey {
                description: Some("API key.".into()),
                name: name.clone(),
                location: match r#in {
                    ApiKeyLocation::Header => "header".into(),
                    ApiKeyLocation::Query => "query".into(),
                },
            },
        )),
    }
}

// ---------------------------------------------------------------------------
// URL / path helpers
// ---------------------------------------------------------------------------

/// Split `url` into `(server, path)`. Strips any query string from the
/// path — params live in `request.params` and are emitted separately.
///
/// Handles three cases:
///   1. Real absolute URL → `Url::parse` gives us scheme/host/path cleanly.
///   2. `{{var}}` placeholder for the base (`{{base_url}}/login`) → strip
///      the var, server stays `{{base_url}}`, path is `/login`. Warning.
///   3. Anything else (relative path, malformed) → no server, path = url.
fn split_url(url: &str, warnings: &mut Vec<String>, req_name: &str) -> (Option<String>, String) {
    let trimmed = url.trim();
    // Strip user-supplied query string — params travel via OpenAPI parameters.
    let (no_query, _) = trimmed.split_once('?').unwrap_or((trimmed, ""));

    if let Ok(parsed) = Url::parse(no_query) {
        let path = if parsed.path().is_empty() {
            "/".into()
        } else {
            parsed.path().to_string()
        };
        let server = format!(
            "{}://{}{}",
            parsed.scheme(),
            parsed.host_str().unwrap_or(""),
            parsed
                .port()
                .map(|p| format!(":{p}"))
                .unwrap_or_default()
        );
        return (Some(server), path);
    }

    // {{var}}/path → treat var as server placeholder.
    if no_query.starts_with("{{") {
        if let Some(end) = no_query.find("}}") {
            let var = &no_query[..end + 2]; // includes the closing braces
            let rest = &no_query[end + 2..];
            let path = if rest.is_empty() { "/".into() } else { rest.to_string() };
            warnings.push(format!(
                "request '{req_name}': URL starts with template var {var} — emitted as literal server placeholder; replace with a real URL in the exported spec",
            ));
            return (Some(var.to_string()), path);
        }
    }

    warnings.push(format!(
        "request '{req_name}': could not parse URL '{trimmed}' as absolute — emitted as path-only without a server entry"
    ));
    (None, trimmed.to_string())
}

/// `/api/users/{id}/posts/{post_id}` → `["id", "post_id"]`.
/// Only single-brace pairs; double-braces (`{{var}}`) are interpolation
/// templates, not `OpenAPI` path params, so we skip them entirely (consume
/// up to and including the matching `}}`).
fn extract_path_params(url: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = url.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '{' {
            continue;
        }
        // Double-brace — Apiovnia interpolation, not OpenAPI path param.
        // Consume the inner content plus the closing `}}`.
        if chars.peek() == Some(&'{') {
            chars.next(); // second `{`
            while let Some(n) = chars.next() {
                if n == '}' && chars.peek() == Some(&'}') {
                    chars.next();
                    break;
                }
            }
            continue;
        }
        // Single-brace path param: read until the closing `}`.
        let mut name = String::new();
        for next in chars.by_ref() {
            if next == '}' {
                break;
            }
            if next == '{' {
                name.clear();
                break;
            }
            name.push(next);
        }
        if !name.is_empty() && !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

fn pathitem_has_method(item: &PathItem, m: HttpMethod) -> bool {
    match m {
        HttpMethod::Get => item.get.is_some(),
        HttpMethod::Post => item.post.is_some(),
        HttpMethod::Put => item.put.is_some(),
        HttpMethod::Patch => item.patch.is_some(),
        HttpMethod::Delete => item.delete.is_some(),
        HttpMethod::Head => item.head.is_some(),
        HttpMethod::Options => item.options.is_some(),
    }
}

fn place_operation(item: &mut PathItem, m: HttpMethod, op: Operation) {
    match m {
        HttpMethod::Get => item.get = Some(op),
        HttpMethod::Post => item.post = Some(op),
        HttpMethod::Put => item.put = Some(op),
        HttpMethod::Patch => item.patch = Some(op),
        HttpMethod::Delete => item.delete = Some(op),
        HttpMethod::Head => item.head = Some(op),
        HttpMethod::Options => item.options = Some(op),
    }
}

/// Turn a free-form name into a snake-ish operationId. `OpenAPI` lets you
/// use almost any string here, but stable, ASCII identifiers play nicer
/// with codegen.
fn slug(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_underscore = true;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    out.trim_matches('_').to_string()
}

// ---------------------------------------------------------------------------
// Log rendering
// ---------------------------------------------------------------------------

fn render_log(collection: &Collection, rows: &[ExportRow], warnings: &[String]) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    // We don't have a chrono import here — the IPC layer fills in the
    // wall-clock timestamp by replacing the `<timestamp>` placeholder.
    let _ = writeln!(
        s,
        "# Apiovnia OpenAPI export log\n# collection: {}\n# generated: <timestamp>\n",
        collection.name,
    );
    let _ = writeln!(s, "## Requests ({})", rows.len());
    for r in rows {
        let _ = writeln!(
            s,
            "  - {:<6} {:<40}  ({} secret{} stripped)  [{}]",
            r.method,
            r.path,
            r.redactions,
            if r.redactions == 1 { "" } else { "s" },
            r.name
        );
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
    use crate::redact::{redact_request, Policy};
    use apiovnia_core::{
        ids::{CollectionId, ProjectId, RequestId},
        model::{HttpMethod, KeyValue},
    };

    fn coll(name: &str) -> Collection {
        Collection {
            id: CollectionId::new(),
            project_id: ProjectId::new(),
            name: name.into(),
            created_at: 0,
            updated_at: 0,
            sort_order: 0,
        }
    }
    fn req(name: &str, method: HttpMethod, url: &str) -> Request {
        Request {
            id: RequestId::new(),
            collection_id: CollectionId::new(),
            name: name.into(),
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
    fn with_tally(r: &Request) -> (Request, crate::redact::RedactionTally) {
        let (r, t) = redact_request(r, &Policy::default());
        (r, t)
    }

    #[test]
    fn minimal_spec_has_required_fields() {
        let c = coll("Auth");
        let r = req("Login", HttpMethod::Post, "https://api.example.com/auth/login");
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(out.yaml.contains("openapi: 3.0.3"));
        assert!(out.yaml.contains("title: Auth"));
        assert!(out.yaml.contains("version: 1.0.0"));
        // Path keyed by relative path with leading slash.
        assert!(out.yaml.contains("/auth/login:"));
        // Method as nested key.
        assert!(out.yaml.contains("post:"));
        // Required responses block.
        assert!(out.yaml.contains("'200':"));
    }

    #[test]
    fn extracts_server_from_absolute_url() {
        let c = coll("c");
        let r = req("r", HttpMethod::Get, "https://api.example.com/v1/things");
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(out.yaml.contains("servers:"));
        assert!(out.yaml.contains("url: https://api.example.com"));
        assert!(!out.yaml.contains("/v1/things?")); // no spurious query
    }

    #[test]
    fn strips_query_string_from_path() {
        let c = coll("c");
        let r = req("r", HttpMethod::Get, "https://example.com/x?a=1&b=2");
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(out.yaml.contains("/x:"));
        assert!(!out.yaml.contains("/x?"));
    }

    #[test]
    fn emits_one_server_per_unique_base() {
        let c = coll("c");
        let r1 = req("a", HttpMethod::Get, "https://api.example.com/x");
        let r2 = req("b", HttpMethod::Get, "https://api.example.com/y");
        let r3 = req("c", HttpMethod::Get, "https://other.example.com/z");
        let out = export_collection(&c, &[with_tally(&r1), with_tally(&r2), with_tally(&r3)]).unwrap();
        let n = out.yaml.matches("- url:").count();
        assert_eq!(n, 2, "got: {}", out.yaml);
    }

    #[test]
    fn collision_aborts_export() {
        let c = coll("c");
        let r1 = req("First", HttpMethod::Get, "https://example.com/x");
        let r2 = req("Second", HttpMethod::Get, "https://example.com/x");
        let err = export_collection(&c, &[with_tally(&r1), with_tally(&r2)]).unwrap_err();
        match err {
            ExportError::Collision { method, path, requests } => {
                assert_eq!(method, "GET");
                assert_eq!(path, "/x");
                assert_eq!(requests, vec!["First", "Second"]);
            }
            ExportError::Serde(_) => panic!("expected Collision"),
        }
    }

    #[test]
    fn different_methods_share_one_path() {
        let c = coll("c");
        let r1 = req("Get user", HttpMethod::Get, "https://example.com/u");
        let r2 = req("Create user", HttpMethod::Post, "https://example.com/u");
        let out = export_collection(&c, &[with_tally(&r1), with_tally(&r2)]).unwrap();
        // One path entry, two methods nested under it.
        assert_eq!(out.yaml.matches("/u:").count(), 1, "got: {}", out.yaml);
        assert!(out.yaml.contains("get:"));
        assert!(out.yaml.contains("post:"));
    }

    #[test]
    fn enabled_query_params_become_parameters() {
        let c = coll("c");
        let mut r = req("r", HttpMethod::Get, "https://example.com/x");
        r.params = vec![
            KeyValue { key: "limit".into(), value: "10".into(), enabled: true },
            KeyValue { key: "off".into(), value: "skipme".into(), enabled: false },
        ];
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(out.yaml.contains("name: limit"));
        assert!(out.yaml.contains("in: query"));
        assert!(!out.yaml.contains("name: off"));
    }

    #[test]
    fn header_content_type_is_not_duplicated_as_parameter() {
        let c = coll("c");
        let mut r = req("r", HttpMethod::Post, "https://example.com/x");
        r.headers = vec![KeyValue {
            key: "Content-Type".into(),
            value: "application/json".into(),
            enabled: true,
        }];
        r.body_type = BodyType::Json;
        r.body_content = "{}".into();
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        // No parameter row for content-type; the requestBody.content key
        // already conveys it.
        assert!(!out.yaml.contains("name: Content-Type"));
    }

    #[test]
    fn path_params_promoted_when_braces_present() {
        let c = coll("c");
        let r = req("Get user", HttpMethod::Get, "https://example.com/users/{id}");
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(out.yaml.contains("in: path"));
        assert!(out.yaml.contains("name: id"));
        assert!(out.yaml.contains("required: true"));
    }

    #[test]
    fn double_brace_template_not_treated_as_path_param() {
        let c = coll("c");
        let r = req("r", HttpMethod::Get, "https://example.com/{{var}}/x");
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(!out.yaml.contains("name: var"));
    }

    #[test]
    fn template_url_becomes_literal_server_with_warning() {
        let c = coll("c");
        let r = req("r", HttpMethod::Get, "{{base_url}}/login");
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(out.yaml.contains("url: '{{base_url}}'") || out.yaml.contains("url: \"{{base_url}}\""));
        assert!(!out.warnings.is_empty());
    }

    #[test]
    fn bearer_auth_adds_security_scheme_and_requirement() {
        let c = coll("c");
        let mut r = req("r", HttpMethod::Get, "https://example.com/x");
        r.auth = AuthConfig::Bearer { token: "tok".into() };
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(out.yaml.contains("securitySchemes:"));
        assert!(out.yaml.contains("bearerAuth:"));
        assert!(out.yaml.contains("scheme: bearer"));
        // Original token nowhere in output (redacted upstream).
        assert!(!out.yaml.contains("tok\n"));
        assert!(!out.yaml.contains("'tok'"));
    }

    #[test]
    fn apikey_auth_records_in_and_name() {
        let c = coll("c");
        let mut r = req("r", HttpMethod::Get, "https://example.com/x");
        r.auth = AuthConfig::ApiKey {
            name: "X-API-Key".into(),
            value: "secret".into(),
            r#in: ApiKeyLocation::Header,
        };
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(out.yaml.contains("apiKeyAuth:"));
        assert!(out.yaml.contains("in: header"));
        assert!(out.yaml.contains("name: X-API-Key"));
    }

    #[test]
    fn secrets_in_body_are_already_redacted_before_emission() {
        let c = coll("c");
        let mut r = req("Login", HttpMethod::Post, "https://example.com/login");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"username":"alice","password":"hunter2"}"#.into();
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(!out.yaml.contains("hunter2"));
        assert!(out.yaml.contains("<your-password>"));
    }

    #[test]
    fn json_body_emits_ref_to_components_schemas() {
        let c = coll("c");
        let mut r = req("addPet", HttpMethod::Post, "https://example.com/pets");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"name":"Fluffy","tag":"cat","count":3}"#.into();
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        // The media type points to a $ref…
        assert!(
            out.yaml.contains("$ref: '#/components/schemas/AddpetRequest'"),
            "got: {}",
            out.yaml
        );
        // …and the schema lives in components.
        assert!(out.yaml.contains("schemas:"));
        assert!(out.yaml.contains("AddpetRequest:"));
        // Properties typed from example.
        assert!(out.yaml.contains("name:"));
        assert!(out.yaml.contains("type: string"));
        assert!(out.yaml.contains("type: integer"));
        assert!(out.yaml.contains("format: int64"));
        // Required list excludes nothing (no nulls in example).
        assert!(out.yaml.contains("- name"));
        assert!(out.yaml.contains("- tag"));
        assert!(out.yaml.contains("- count"));
        // Example still present alongside the schema.
        assert!(out.yaml.contains("Fluffy"));
    }

    #[test]
    fn format_hints_detected_in_example_strings() {
        let c = coll("c");
        let mut r = req("createUser", HttpMethod::Post, "https://example.com/u");
        r.body_type = BodyType::Json;
        r.body_content = r#"{
            "email": "alice@example.com",
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "createdAt": "2024-01-15T10:30:00Z",
            "homepage": "https://example.com"
        }"#
        .into();
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(out.yaml.contains("format: email"));
        assert!(out.yaml.contains("format: uuid"));
        assert!(out.yaml.contains("format: date-time"));
        assert!(out.yaml.contains("format: uri"));
    }

    #[test]
    fn nested_array_of_objects_gets_recursive_schema() {
        let c = coll("c");
        let mut r = req("bulk", HttpMethod::Post, "https://example.com/x");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"items":[{"sku":"abc","qty":5}]}"#.into();
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        // Array → type: array, items: { properties: {sku, qty} }.
        assert!(out.yaml.contains("type: array"));
        assert!(out.yaml.contains("sku:"));
        assert!(out.yaml.contains("qty:"));
    }

    #[test]
    fn null_field_is_omitted_from_required() {
        let c = coll("c");
        let mut r = req("op", HttpMethod::Post, "https://example.com/x");
        r.body_type = BodyType::Json;
        r.body_content = r#"{"name":"x","nickname":null}"#.into();
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(out.yaml.contains("- name"));
        // `nickname` is in properties but not in required.
        assert!(out.yaml.contains("nickname:"));
        // Crude check: the required list shouldn't include nickname.
        let req_idx = out.yaml.find("required:").unwrap();
        let req_block = &out.yaml[req_idx..req_idx + 200];
        assert!(!req_block.contains("- nickname"), "got: {req_block}");
    }

    #[test]
    fn unparseable_body_falls_back_to_example_without_schema() {
        let c = coll("c");
        let mut r = req("op", HttpMethod::Post, "https://example.com/x");
        r.body_type = BodyType::Json;
        // Template placeholder — not valid JSON. We shouldn't crash; we
        // shouldn't invent a schema. Example carries the raw string.
        r.body_content = "{{base_payload}}".into();
        let out = export_collection(&c, &[with_tally(&r)]).unwrap();
        assert!(!out.yaml.contains("$ref:"));
        assert!(out.yaml.contains("base_payload"));
    }

    #[test]
    fn log_contains_per_request_lines_and_redaction_count() {
        let c = coll("Auth");
        let mut r1 = req("Login", HttpMethod::Post, "https://example.com/login");
        r1.body_type = BodyType::Json;
        r1.body_content = r#"{"password":"x"}"#.into();
        let r2 = req("Get user", HttpMethod::Get, "https://example.com/me");
        let out = export_collection(&c, &[with_tally(&r1), with_tally(&r2)]).unwrap();
        assert!(out.log.contains("Login"));
        assert!(out.log.contains("Get user"));
        assert!(out.log.contains("(1 secret stripped)"));
        assert!(out.log.contains("(0 secrets stripped)"));
    }

    #[test]
    fn empty_url_skipped_with_warning() {
        let c = coll("c");
        let r1 = req("empty", HttpMethod::Get, "");
        let r2 = req("ok", HttpMethod::Get, "https://example.com/x");
        let out = export_collection(&c, &[with_tally(&r1), with_tally(&r2)]).unwrap();
        assert_eq!(out.rows.len(), 1);
        assert!(out.warnings.iter().any(|w| w.contains("empty")));
    }
}
