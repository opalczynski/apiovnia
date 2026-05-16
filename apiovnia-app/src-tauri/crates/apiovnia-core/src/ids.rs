//! Newtype IDs.
//!
//! Each entity gets a strongly-typed ID so the type system catches mix-ups
//! (e.g. passing a `CollectionId` where a `ProjectId` is expected). The
//! underlying representation is a `UUIDv4` stringified for `SQLite` storage.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Returned when parsing a string into a typed ID fails.
#[derive(Debug, Error)]
#[error("invalid id: {0}")]
pub struct InvalidId(String);

macro_rules! define_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Generates a fresh ID.
            #[must_use]
            pub fn new() -> Self {
                Self(format!("{}_{}", $prefix, Uuid::new_v4().simple()))
            }

            /// Wraps an already-validated string. Use when reading from `SQLite`.
            #[must_use]
            pub fn from_trusted(s: impl Into<String>) -> Self {
                Self(s.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            #[must_use]
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = InvalidId;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                if s.is_empty() {
                    return Err(InvalidId(s.to_string()));
                }
                Ok(Self(s.to_string()))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

define_id!(ProjectId, "prj");
define_id!(CollectionId, "col");
define_id!(RequestId, "req");
define_id!(EnvironmentId, "env");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let a = ProjectId::new();
        let b = ProjectId::new();
        assert_ne!(a, b);
        assert!(a.as_str().starts_with("prj_"));
    }

    #[test]
    fn parse_rejects_empty() {
        assert!("".parse::<RequestId>().is_err());
    }

    #[test]
    fn round_trip_via_serde_json() {
        let id = CollectionId::new();
        let s = serde_json::to_string(&id).unwrap();
        let back: CollectionId = serde_json::from_str(&s).unwrap();
        assert_eq!(id, back);
    }
}
