//! Apiovnia secret sealing.
//!
//! Argon2id key derivation (OWASP 2024 baseline) + AES-256-GCM authenticated
//! encryption. Used by the encrypted-environment feature. Filled in Phase 6.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
