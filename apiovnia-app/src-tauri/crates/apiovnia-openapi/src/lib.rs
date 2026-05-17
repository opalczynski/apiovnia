//! Apiovnia `OpenAPI` bridge.
//!
//! Imports `OpenAPI` 3.0/3.1 specs into the Apiovnia domain (collection +
//! requests + environments) and best-effort export back. Secrets get
//! scrubbed before they leave the app (see [`redact`]).

pub mod export;
pub mod import;
pub mod redact;

pub use export::{export_collection, ExportError, ExportResult, ExportRow};
pub use import::{import_document, ImportError, ImportResult, ImportRow};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
