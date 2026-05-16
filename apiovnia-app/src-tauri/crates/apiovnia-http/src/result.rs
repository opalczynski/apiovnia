//! Wire-friendly response container handed back to the IPC layer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseBodyKind {
    /// UTF-8 text we managed to decode (HTML, JSON, plain text…).
    Text,
    /// Binary payload encoded as base64 so it round-trips through JSON.
    BinaryBase64,
    /// No body (HEAD, 204, etc).
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    /// HTTP numeric status (e.g. 200).
    pub status: u16,
    /// Reason phrase if known (e.g. "OK").
    pub status_text: String,
    /// Headers in receive order; lowercase names.
    pub headers: Vec<HeaderEntry>,
    /// Convenience copy of the response content-type (lowercased, no params).
    pub content_type: Option<String>,
    /// Body kind: text / binary base64 / empty.
    pub body_kind: ResponseBodyKind,
    /// The body string — UTF-8 text or base64 of binary.
    pub body: String,
    /// True if the body exceeded `BODY_LIMIT_BYTES` and was truncated.
    pub body_truncated: bool,
    /// Total wall time from request build to last byte.
    pub duration_ms: u64,
    /// Number of body bytes we received from the server (before truncation).
    pub size_bytes: u64,
    /// The final URL after redirects.
    pub final_url: String,
    /// Snapshot of what actually went on the wire — useful when an API
    /// rejects a request and you need to confirm we sent the right thing.
    pub sent: SentRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderEntry>,
    /// First N bytes of the request body, decoded as UTF-8 (lossy fallback).
    /// Empty for GET/HEAD.
    pub body_preview: String,
    pub body_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
}
