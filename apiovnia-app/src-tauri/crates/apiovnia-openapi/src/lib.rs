//! Apiovnia `OpenAPI` bridge.
//!
//! Imports `OpenAPI` 3.0/3.1 specs into the Apiovnia domain (collection +
//! requests + environments) and best-effort export back. Filled in Phase 7.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
