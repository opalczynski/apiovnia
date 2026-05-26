//! GraphQL request bodies.
//!
//! A GraphQL request is, on the wire, just an HTTP `POST` of a JSON envelope
//! `{"query": …, "variables": …}`. Apiovnia models it as a [`BodyType::GraphQl`]
//! request whose `body_content` stores a [`GraphQlBody`] — the query document
//! and the variables text the user typed — serialised as JSON. That keeps the
//! storage schema single-column (same trick as `Form`/`Multipart`) and means
//! `BodyType::GraphQl` rides the existing REST executor with zero migration.
//!
//! [`BodyType::GraphQl`]: crate::model::BodyType::GraphQl

use serde::{Deserialize, Serialize};

/// Internal representation of a `BodyType::GraphQl` request body.
///
/// Stored as JSON inside `Request::body_content`; mirrored by the
/// `GraphQlBody` type on the TypeScript side. `variables` holds the **raw
/// text** the user typed (expected to be a JSON object) rather than a parsed
/// value — so an in-progress, not-yet-valid edit round-trips without loss.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GraphQlBody {
    /// The GraphQL document — a query, mutation, or subscription.
    pub query: String,
    /// Operation variables as raw JSON-object text. Empty = no variables.
    pub variables: String,
}

impl GraphQlBody {
    /// Parse a `body_content` string into a [`GraphQlBody`].
    ///
    /// Lenient by design: an empty or malformed `body_content` yields an
    /// empty body rather than an error, so the editor can always recover
    /// (the first keystroke rewrites it into a well-formed envelope).
    #[must_use]
    pub fn parse(body_content: &str) -> Self {
        if body_content.trim().is_empty() {
            return Self::default();
        }
        serde_json::from_str(body_content).unwrap_or_default()
    }

    /// The wire body for a GraphQL-over-HTTP `POST`: `{"query":…,"variables":…}`.
    ///
    /// The `variables` text is spliced in verbatim when non-empty, so a valid
    /// JSON object round-trips exactly as the user wrote it; a blank field
    /// becomes `{}`. The `query` is always JSON-string-escaped. This is the
    /// lenient variant — it never fails, so it suits snippet generation and
    /// "best effort" rendering. Use [`Self::to_wire_json_checked`] when the
    /// caller needs the variables to actually be valid JSON.
    #[must_use]
    pub fn to_wire_json(&self) -> String {
        let vars = self.variables.trim();
        let vars = if vars.is_empty() { "{}" } else { vars };
        let query = serde_json::to_string(&self.query).unwrap_or_else(|_| "\"\"".to_string());
        format!(r#"{{"query":{query},"variables":{vars}}}"#)
    }

    /// Like [`Self::to_wire_json`] but fails when `variables` is non-empty and
    /// not valid JSON — used by the executor so a malformed variables block
    /// surfaces as a clear error instead of a confusing `400` from the server.
    ///
    /// # Errors
    /// Returns a human-readable message when `variables` cannot be parsed.
    pub fn to_wire_json_checked(&self) -> Result<String, String> {
        self.validate_variables()?;
        Ok(self.to_wire_json())
    }

    /// The query-string parameters for a GraphQL-over-HTTP **GET** request.
    ///
    /// Per the GraphQL-over-HTTP spec, a `GET` carries `query` and the
    /// JSON-encoded `variables` in the URL query string rather than a body
    /// (`GET` must stay safe/idempotent, so it is for queries only — never
    /// mutations). `query` is always emitted; `variables` only when present.
    /// The caller percent-encodes. Lenient — suits snippets and previews.
    #[must_use]
    pub fn to_get_query_params(&self) -> Vec<(&'static str, String)> {
        let mut out = vec![("query", self.query.clone())];
        let vars = self.variables.trim();
        if !vars.is_empty() {
            out.push(("variables", vars.to_string()));
        }
        out
    }

    /// Like [`Self::to_get_query_params`] but fails when `variables` is
    /// non-empty and not valid JSON — the executor's `GET` counterpart of
    /// [`Self::to_wire_json_checked`].
    ///
    /// # Errors
    /// Returns a human-readable message when `variables` cannot be parsed.
    pub fn to_get_query_params_checked(&self) -> Result<Vec<(&'static str, String)>, String> {
        self.validate_variables()?;
        Ok(self.to_get_query_params())
    }

    /// Errors when `variables` is non-empty and not parseable as JSON.
    fn validate_variables(&self) -> Result<(), String> {
        let vars = self.variables.trim();
        if !vars.is_empty() {
            serde_json::from_str::<serde_json::Value>(vars)
                .map_err(|e| format!("GraphQL variables are not valid JSON: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_yields_default() {
        assert_eq!(GraphQlBody::parse(""), GraphQlBody::default());
        assert_eq!(GraphQlBody::parse("   \n "), GraphQlBody::default());
    }

    #[test]
    fn parse_malformed_yields_default() {
        // A stale plain-JSON body left over from a `BodyType::Json` request
        // must not blow up — it just reads as an empty GraphQL body.
        assert_eq!(GraphQlBody::parse("{not json"), GraphQlBody::default());
        assert_eq!(
            GraphQlBody::parse(r#"{"unrelated":1}"#),
            GraphQlBody::default()
        );
    }

    #[test]
    fn round_trips_through_json() {
        let body = GraphQlBody {
            query: "query Me { me { id } }".into(),
            variables: r#"{"id":1}"#.into(),
        };
        let encoded = serde_json::to_string(&body).unwrap();
        assert_eq!(GraphQlBody::parse(&encoded), body);
    }

    #[test]
    fn wire_json_splices_variables_verbatim() {
        let body = GraphQlBody {
            query: "{ me { id } }".into(),
            variables: r#"{ "id": 42 }"#.into(),
        };
        assert_eq!(
            body.to_wire_json(),
            r#"{"query":"{ me { id } }","variables":{ "id": 42 }}"#
        );
    }

    #[test]
    fn wire_json_blank_variables_become_empty_object() {
        let body = GraphQlBody {
            query: "{ ping }".into(),
            variables: String::new(),
        };
        assert_eq!(body.to_wire_json(), r#"{"query":"{ ping }","variables":{}}"#);
    }

    #[test]
    fn wire_json_escapes_query_string() {
        let body = GraphQlBody {
            query: "mutation { add(note: \"hi\\n\") }".into(),
            variables: String::new(),
        };
        let wire = body.to_wire_json();
        // The whole envelope must parse back as valid JSON.
        let parsed: serde_json::Value = serde_json::from_str(&wire).unwrap();
        assert_eq!(parsed["query"], "mutation { add(note: \"hi\\n\") }");
    }

    #[test]
    fn checked_rejects_invalid_variables() {
        let body = GraphQlBody {
            query: "{ ping }".into(),
            variables: "{ not json".into(),
        };
        assert!(body.to_wire_json_checked().is_err());
    }

    #[test]
    fn checked_accepts_valid_and_blank_variables() {
        let ok = GraphQlBody {
            query: "{ ping }".into(),
            variables: r#"{"a":1}"#.into(),
        };
        assert!(ok.to_wire_json_checked().is_ok());

        let blank = GraphQlBody {
            query: "{ ping }".into(),
            variables: "   ".into(),
        };
        assert_eq!(
            blank.to_wire_json_checked().unwrap(),
            r#"{"query":"{ ping }","variables":{}}"#
        );
    }

    #[test]
    fn get_params_carry_query_and_variables() {
        let body = GraphQlBody {
            query: "query { me { id } }".into(),
            variables: r#"{"limit":5}"#.into(),
        };
        assert_eq!(
            body.to_get_query_params(),
            vec![
                ("query", "query { me { id } }".to_string()),
                ("variables", r#"{"limit":5}"#.to_string()),
            ]
        );
    }

    #[test]
    fn get_params_omit_blank_variables() {
        let body = GraphQlBody {
            query: "{ ping }".into(),
            variables: "  ".into(),
        };
        // `query` is always present; `variables` is dropped when blank.
        assert_eq!(
            body.to_get_query_params(),
            vec![("query", "{ ping }".to_string())]
        );
    }

    #[test]
    fn get_params_checked_rejects_invalid_variables() {
        let body = GraphQlBody {
            query: "{ ping }".into(),
            variables: "{ not json".into(),
        };
        assert!(body.to_get_query_params_checked().is_err());
    }
}
