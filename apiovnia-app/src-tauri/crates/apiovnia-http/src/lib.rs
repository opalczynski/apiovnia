//! Apiovnia HTTP execution layer.
//!
//! `Executor` wraps a long-lived `reqwest::Client` and runs a `Request`
//! against the wire, returning a structured `ExecutionResult`. Body bytes
//! over `BODY_LIMIT_BYTES` are truncated; binary bodies are base64-encoded.
//!
//! Variable interpolation (`{{var}}`) and per-environment resolution land
//! in Phase 5. For now we send the request as-is.

mod error;
mod executor;
mod result;

pub use error::ExecutionError;
pub use executor::{Executor, ExecutorConfig};
pub use result::{ExecutionResult, HeaderEntry, ResponseBodyKind, SentRequest};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Cap on the body we store in history / send to the frontend. Anything past
/// this is truncated and `body_truncated` is flagged.
pub const BODY_LIMIT_BYTES: usize = 2 * 1024 * 1024;
