//! `{{var}}` template substitution.
//!
//! Used by the resolver to splice environment variables into URLs, header
//! values, query params, body content, and auth fields. Missing variables
//! are **left as-is** in the output so the user can still see them in the
//! request snapshot — better than silently producing an invalid URL.

use std::collections::HashMap;
use std::hash::BuildHasher;

use crate::model::{AuthConfig, KeyValue};

/// Substitute every `{{name}}` occurrence in `template` with the matching
/// value from `vars`. Unknown placeholders are kept verbatim.
///
/// Names accepted: ASCII letters, digits, underscore, hyphen, dot — the same
/// set every other API client uses for env variables.
#[must_use]
pub fn interpolate<S: BuildHasher>(template: &str, vars: &HashMap<String, String, S>) -> String {
    if !template.contains("{{") {
        return template.to_string();
    }

    let bytes = template.as_bytes();
    let mut out = String::with_capacity(template.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // Look for the matching `}}` and capture the name in between.
            if let Some(rel) = find_close(&bytes[i + 2..]) {
                let name_start = i + 2;
                let name_end = name_start + rel;
                let name = template[name_start..name_end].trim();
                if is_valid_name(name) {
                    if let Some(v) = vars.get(name) {
                        out.push_str(v);
                    } else {
                        // Unknown placeholder — emit verbatim so the user
                        // sees the gap in their request snapshot.
                        out.push_str(&template[i..name_end + 2]);
                    }
                    i = name_end + 2;
                    continue;
                }
            }
        }
        // Default — copy a single character.
        let ch_end = next_char_boundary(template, i);
        out.push_str(&template[i..ch_end]);
        i = ch_end;
    }
    out
}

/// Apply [`interpolate`] in place across the value side of a `KeyValue`
/// list. Names (keys) are left untouched on purpose — `{{var}}` in a header
/// name would be a footgun.
pub fn interpolate_pairs<S: BuildHasher>(
    pairs: &mut [KeyValue],
    vars: &HashMap<String, String, S>,
) {
    for kv in pairs {
        kv.value = interpolate(&kv.value, vars);
    }
}

/// Interpolate the secret-bearing fields of an [`AuthConfig`].
pub fn interpolate_auth<S: BuildHasher>(
    auth: &mut AuthConfig,
    vars: &HashMap<String, String, S>,
) {
    match auth {
        AuthConfig::None => {}
        AuthConfig::Bearer { token } => {
            *token = interpolate(token, vars);
        }
        AuthConfig::Basic { username, password } => {
            *username = interpolate(username, vars);
            *password = interpolate(password, vars);
        }
        AuthConfig::ApiKey { name, value, .. } => {
            // The header / query name itself is left untouched (see above);
            // only the value gets interpolated.
            *name = interpolate(name, vars);
            *value = interpolate(value, vars);
        }
    }
}

fn is_valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
}

fn find_close(haystack: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i + 1 < haystack.len() {
        if haystack[i] == b'}' && haystack[i + 1] == b'}' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn next_char_boundary(s: &str, i: usize) -> usize {
    let mut j = i + 1;
    while j < s.len() && !s.is_char_boundary(j) {
        j += 1;
    }
    j
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("base_url".into(), "api.example.com".into());
        m.insert("token".into(), "abc123".into());
        m.insert("locale".into(), "pl-PL".into());
        m
    }

    #[test]
    fn replaces_known_placeholder() {
        let out = interpolate("https://{{base_url}}/users", &vars());
        assert_eq!(out, "https://api.example.com/users");
    }

    #[test]
    fn multiple_placeholders_in_one_string() {
        let out = interpolate("Bearer {{token}}; lang={{locale}}", &vars());
        assert_eq!(out, "Bearer abc123; lang=pl-PL");
    }

    #[test]
    fn unknown_placeholder_is_left_verbatim() {
        let out = interpolate("hello {{nope}} world", &vars());
        assert_eq!(out, "hello {{nope}} world");
    }

    #[test]
    fn malformed_placeholder_passes_through() {
        let out = interpolate("hello {{ world", &vars());
        assert_eq!(out, "hello {{ world");
    }

    #[test]
    fn whitespace_inside_placeholder_is_trimmed() {
        let out = interpolate("{{  base_url  }}", &vars());
        assert_eq!(out, "api.example.com");
    }

    #[test]
    fn names_can_contain_dots_and_hyphens() {
        let mut v = vars();
        v.insert("api.key-v2".into(), "k-v2".into());
        let out = interpolate("k={{api.key-v2}}", &v);
        assert_eq!(out, "k=k-v2");
    }

    #[test]
    fn no_braces_short_circuits_to_clone() {
        let out = interpolate("plain string", &vars());
        assert_eq!(out, "plain string");
    }

    #[test]
    fn unicode_in_surrounding_text_is_preserved() {
        let out = interpolate("zażółć {{base_url}} gęślą", &vars());
        assert_eq!(out, "zażółć api.example.com gęślą");
    }

    #[test]
    fn interpolate_pairs_only_touches_values() {
        let mut pairs = vec![
            KeyValue { key: "X-Token".into(), value: "{{token}}".into(), enabled: true },
            KeyValue { key: "{{name_var}}".into(), value: "plain".into(), enabled: true },
        ];
        interpolate_pairs(&mut pairs, &vars());
        assert_eq!(pairs[0].value, "abc123");
        // Key untouched even though it looks interpolable.
        assert_eq!(pairs[1].key, "{{name_var}}");
        assert_eq!(pairs[1].value, "plain");
    }

    #[test]
    fn interpolate_auth_bearer_and_basic() {
        let mut auth = AuthConfig::Bearer { token: "{{token}}".into() };
        interpolate_auth(&mut auth, &vars());
        assert_eq!(auth, AuthConfig::Bearer { token: "abc123".into() });

        let mut basic = AuthConfig::Basic {
            username: "{{base_url}}".into(),
            password: "{{token}}".into(),
        };
        interpolate_auth(&mut basic, &vars());
        assert_eq!(
            basic,
            AuthConfig::Basic {
                username: "api.example.com".into(),
                password: "abc123".into(),
            }
        );
    }
}
