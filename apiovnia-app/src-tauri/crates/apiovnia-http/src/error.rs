//! HTTP execution errors.
//!
//! We avoid leaking `reqwest`-specific structure to the frontend; everything
//! becomes a flat string on the IPC boundary via `Serialize`.

use serde::{Serialize, Serializer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("timeout after {0} ms")]
    Timeout(u64),

    #[error("response too large to handle")]
    ResponseTooLarge,
}

impl Serialize for ExecutionError {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl From<reqwest::Error> for ExecutionError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            // Reqwest doesn't surface the configured timeout — pass 0 here and
            // let the executor wrap the real value via `Timeout::new` if
            // ever needed.
            return Self::Timeout(0);
        }
        if e.is_builder() || e.is_request() {
            return Self::InvalidRequest(e.to_string());
        }
        Self::Network(e.to_string())
    }
}

impl From<url::ParseError> for ExecutionError {
    fn from(e: url::ParseError) -> Self {
        Self::InvalidUrl(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ExecutionError>;
