//! Apiovnia domain core.
//!
//! Pure types and functions that model the application's vocabulary —
//! projects, collections, requests, environments, overrides — plus utility
//! types shared across crates. No I/O, no Tauri, no SQL.
//!
//! Everything in this crate must be unit-testable without a runtime.

pub mod graphql;
pub mod ids;
pub mod interpolate;
pub mod model;
pub mod resolver;
pub mod snippets;
pub mod time;

pub use snippets::{to_curl, SnippetFormat};
pub use graphql::GraphQlBody;
pub use ids::{CollectionId, EnvironmentId, ProjectId, RequestId};
pub use model::{
    AuthConfig, BodyType, Collection, Environment, EnvOverride, EnvVariable, HttpMethod, KeyValue,
    Project, Request,
};
pub use resolver::{resolve_request, ResolvedRequest};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
