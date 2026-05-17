//! Apiovnia secret sealing — Argon2id KDF + AES-256-GCM authenticated encryption.
//!
//! Threat model: the local `SQLite` database is treated as untrusted at rest.
//! When the user marks an environment as encrypted, every variable value and
//! every override field for that env is sealed with a key derived from the
//! user's master password via Argon2id (OWASP 2024 baseline). The key never
//! leaves the Rust side — it lives in a `SessionKeyStore` (see the tauri
//! crate) for the duration of the unlock and gets zeroed on lock/quit.
//!
//! Ciphertext format (raw bytes): `nonce(12) || ciphertext || tag(16)`.
//! For TEXT columns we wrap the bytes in standard base64.
//!
//! Salt format (per env, stored in `environments.salt`): 16 random bytes.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key as AesKey, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use rand::RngCore;
use thiserror::Error;
use zeroize::Zeroize;

pub mod password_policy;
pub use password_policy::{score_password, validate_policy, PasswordStrength, PolicyError, MIN_LENGTH, MIN_SCORE};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Salt size (bytes) for Argon2id. Stored alongside the env.
pub const SALT_LEN: usize = 16;
/// AES-GCM nonce size (bytes). Random per encryption op.
pub const NONCE_LEN: usize = 12;
/// AES-GCM tag size (bytes). Appended by the AEAD.
pub const TAG_LEN: usize = 16;

/// Fixed marker encrypted with the derived key when an env is first sealed.
/// On unlock we re-derive the key from the supplied password and try to
/// open this blob — success proves the password matches without ever
/// keeping the password itself around.
pub const PASSWORD_CHECK_MARKER: &[u8] = b"apiovnia/v1/password-check";

// Argon2id cost parameters — OWASP 2024 baseline for interactive logins.
const MEMORY_KIB: u32 = 19_456;
const ITERATIONS: u32 = 2;
const PARALLELISM: u32 = 1;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("argon2: {0}")]
    Argon2(String),
    /// AEAD authentication failed: wrong key, wrong nonce, tampered ciphertext,
    /// or wrong associated data. We collapse all of these into one variant —
    /// distinguishing them would leak information to an attacker.
    #[error("decryption failed (wrong password or data was tampered with)")]
    Aead,
    #[error("ciphertext too short")]
    ShortCiphertext,
    #[error("invalid base64: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("decrypted payload was not valid UTF-8")]
    Utf8,
    #[error("invalid salt length (expected {SALT_LEN} bytes)")]
    InvalidSalt,
}

/// 32-byte symmetric key. The inner buffer zeroes itself on drop so the
/// key doesn't linger in freed memory.
///
/// `Clone` is implemented because the session-key store hands out copies
/// across await boundaries; every clone is its own zeroized-on-drop
/// allocation, so there is no aliasing surprise.
pub struct EnvKey([u8; 32]);

impl EnvKey {
    /// Build an `EnvKey` directly from 32 bytes. Use [`derive_key`] in user
    /// flows — this exists for the session-key store, which needs to round-trip
    /// a key back out of its `RwLock<HashMap>` to use across an `await`.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl Clone for EnvKey {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl Drop for EnvKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

/// 16 random bytes. Use this once per env at the moment it gets encrypted.
#[must_use]
pub fn random_salt() -> [u8; SALT_LEN] {
    let mut s = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut s);
    s
}

/// Argon2id key derivation. Always returns a fresh `EnvKey`.
pub fn derive_key(password: &str, salt: &[u8]) -> Result<EnvKey, CryptoError> {
    if salt.len() != SALT_LEN {
        return Err(CryptoError::InvalidSalt);
    }
    let params = Params::new(MEMORY_KIB, ITERATIONS, PARALLELISM, Some(32))
        .map_err(|e| CryptoError::Argon2(e.to_string()))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .map_err(|e| CryptoError::Argon2(e.to_string()))?;
    Ok(EnvKey(out))
}

/// Encrypt `plaintext` and return `nonce || ciphertext || tag`.
pub fn seal(plaintext: &[u8], key: &EnvKey) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(key.as_bytes()));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher.encrypt(nonce, plaintext).map_err(|_| CryptoError::Aead)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Inverse of [`seal`]. Returns the plaintext bytes on success.
pub fn open(blob: &[u8], key: &EnvKey) -> Result<Vec<u8>, CryptoError> {
    if blob.len() < NONCE_LEN + TAG_LEN {
        return Err(CryptoError::ShortCiphertext);
    }
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(key.as_bytes()));
    let (nonce_bytes, ct) = blob.split_at(NONCE_LEN);
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher.decrypt(nonce, ct).map_err(|_| CryptoError::Aead)
}

/// Convenience: encrypt a string and return base64-encoded ciphertext. Used
/// when the destination column is `TEXT` rather than `BLOB`.
pub fn seal_str(plaintext: &str, key: &EnvKey) -> Result<String, CryptoError> {
    Ok(B64.encode(seal(plaintext.as_bytes(), key)?))
}

/// Inverse of [`seal_str`].
pub fn open_str(blob_b64: &str, key: &EnvKey) -> Result<String, CryptoError> {
    let bytes = B64.decode(blob_b64.as_bytes())?;
    let pt = open(&bytes, key)?;
    String::from_utf8(pt).map_err(|_| CryptoError::Utf8)
}

/// Build the `password_check` ciphertext written when an env is sealed.
pub fn make_password_check(key: &EnvKey) -> Result<Vec<u8>, CryptoError> {
    seal(PASSWORD_CHECK_MARKER, key)
}

/// True when the supplied key successfully decrypts a previously-stored
/// `password_check` blob. Used by the unlock flow to validate the password
/// before we trust the key.
#[must_use]
pub fn verify_password_check(check_blob: &[u8], key: &EnvKey) -> bool {
    matches!(open(check_blob, key), Ok(v) if v == PASSWORD_CHECK_MARKER)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_key() -> EnvKey {
        // Derived from a fixed password/salt so tests are deterministic but
        // still exercise the real KDF.
        derive_key("hunter2", &[0u8; SALT_LEN]).unwrap()
    }

    #[test]
    fn derive_key_is_deterministic_for_same_inputs() {
        let salt = [7u8; SALT_LEN];
        let k1 = derive_key("pw", &salt).unwrap();
        let k2 = derive_key("pw", &salt).unwrap();
        assert_eq!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn derive_key_rejects_wrong_salt_length() {
        assert!(matches!(
            derive_key("pw", &[0u8; 8]),
            Err(CryptoError::InvalidSalt)
        ));
    }

    #[test]
    fn round_trip_short_and_long() {
        let key = fixed_key();
        for pt in [b"".as_ref(), b"hello", &[0u8; 4096][..]] {
            let blob = seal(pt, &key).unwrap();
            let back = open(&blob, &key).unwrap();
            assert_eq!(back, pt);
        }
    }

    #[test]
    fn round_trip_string() {
        let key = fixed_key();
        let blob = seal_str("Bearer abc-123 ✓", &key).unwrap();
        let back = open_str(&blob, &key).unwrap();
        assert_eq!(back, "Bearer abc-123 ✓");
    }

    #[test]
    fn nonces_are_unique_across_seals() {
        let key = fixed_key();
        let a = seal(b"same plaintext", &key).unwrap();
        let b = seal(b"same plaintext", &key).unwrap();
        // Same plaintext + same key → different ciphertext, because the nonce
        // is fresh per call. (This is what AES-GCM requires for safety.)
        assert_ne!(a, b);
    }

    #[test]
    fn wrong_key_fails_with_aead_error() {
        let key = fixed_key();
        let other = derive_key("different", &[0u8; SALT_LEN]).unwrap();
        let blob = seal(b"secret", &key).unwrap();
        assert!(matches!(open(&blob, &other), Err(CryptoError::Aead)));
    }

    #[test]
    fn tampered_tag_fails() {
        let key = fixed_key();
        let mut blob = seal(b"secret", &key).unwrap();
        let last = blob.len() - 1;
        blob[last] ^= 0x01;
        assert!(matches!(open(&blob, &key), Err(CryptoError::Aead)));
    }

    #[test]
    fn tampered_nonce_fails() {
        let key = fixed_key();
        let mut blob = seal(b"secret", &key).unwrap();
        blob[0] ^= 0x01;
        assert!(matches!(open(&blob, &key), Err(CryptoError::Aead)));
    }

    #[test]
    fn too_short_ciphertext_is_rejected() {
        let key = fixed_key();
        let tiny = vec![0u8; NONCE_LEN + TAG_LEN - 1];
        assert!(matches!(
            open(&tiny, &key),
            Err(CryptoError::ShortCiphertext)
        ));
    }

    #[test]
    fn password_check_round_trip() {
        let key = fixed_key();
        let check = make_password_check(&key).unwrap();
        assert!(verify_password_check(&check, &key));
    }

    #[test]
    fn password_check_rejects_wrong_key() {
        let key = fixed_key();
        let other = derive_key("nope", &[0u8; SALT_LEN]).unwrap();
        let check = make_password_check(&key).unwrap();
        assert!(!verify_password_check(&check, &other));
    }

    #[test]
    fn open_str_rejects_non_utf8() {
        let key = fixed_key();
        // Encrypt invalid-UTF-8 bytes then try to open as string.
        let blob = seal(&[0xC3, 0x28], &key).unwrap();
        let b64 = B64.encode(blob);
        assert!(matches!(open_str(&b64, &key), Err(CryptoError::Utf8)));
    }

    #[test]
    fn random_salt_is_not_all_zero() {
        let s = random_salt();
        assert_ne!(s, [0u8; SALT_LEN]);
    }
}
