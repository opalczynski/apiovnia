//! Password strength scoring + policy enforcement.
//!
//! Uses the `zxcvbn` crate (port of Dropbox's estimator) so users see real
//! feedback instead of arbitrary "one uppercase, one digit, one special"
//! rules. The estimator knows about dictionaries, l33t substitutions,
//! keyboard walks, dates, and common passwords — `password123` scores 0
//! even though it satisfies length + char-class rules.
//!
//! Policy thresholds:
//!   - `MIN_LENGTH` (8) — absolute lower bound; below this we reject before
//!     we even score (a 4-char password's "centuries" estimate would lie).
//!   - `MIN_SCORE` (3) — "safely unguessable" per zxcvbn's calibration;
//!     the same threshold Dropbox ships in production.
//!
//! The IPC surface returns a `PasswordStrength` snapshot so the frontend
//! meter doesn't have to duplicate any of the logic.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use zxcvbn::zxcvbn;

/// Absolute minimum length. Below this the offline-slow-hash estimate is
/// effectively meaningless because zxcvbn's score saturates.
pub const MIN_LENGTH: usize = 8;

/// Minimum zxcvbn score we accept. 3 = "safely unguessable; moderate
/// protection from offline slow-hash scenario", which is what we use.
pub const MIN_SCORE: u8 = 3;

/// Snapshot returned to the frontend on every keystroke. Designed so the
/// UI can paint the meter, the crack-time line, and any inline warning
/// without any extra IPC roundtrips.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordStrength {
    /// 0..=4 — zxcvbn's score.
    pub score: u8,
    /// Human-readable label matching the score: "Very weak" → "Excellent".
    pub label: &'static str,
    /// Human-friendly crack-time estimate against a slow offline attacker
    /// (Argon2id-class) — e.g. "less than a second", "3 days", "centuries".
    /// This is the line the user sees as "Your password needs ~X to crack".
    pub crack_time_display: String,
    /// Optional one-liner explaining the main weakness (only set for low
    /// scores). Examples: "This is a top-10 common password", "Common
    /// names and surnames are easy to guess".
    pub warning: Option<String>,
    /// Up to ~3 actionable suggestions from zxcvbn. UI shows the first.
    pub suggestions: Vec<String>,
    /// True when the password clears both the length floor and the score
    /// threshold — i.e. the Lock button should be enabled.
    pub meets_policy: bool,
    /// True iff the input is at least `MIN_LENGTH` characters. Surfaced
    /// separately so the UI can render a "too short" hint distinctly from
    /// "weak content".
    pub long_enough: bool,
}

/// Reasons the policy validator can refuse a password.
#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("password must be at least {MIN_LENGTH} characters")]
    TooShort,
    /// `feedback` is the user-facing message we want to surface in the UI.
    #[error("password is too weak — {feedback}")]
    TooWeak { feedback: String },
}

/// Score a password using zxcvbn. Always returns a snapshot; never errors —
/// `meets_policy` says whether the snapshot would be accepted.
#[must_use]
pub fn score_password(password: &str) -> PasswordStrength {
    let long_enough = password.chars().count() >= MIN_LENGTH;

    // zxcvbn on an empty string still works but the crack-time string is
    // misleading ("less than a second" for "" — true but unhelpful). Short
    // inputs are handled the same way; we just stamp a sensible label.
    let result = zxcvbn(password, &[]);
    let score = u8::from(result.score());
    let crack_time_display = result
        .crack_times()
        .offline_slow_hashing_1e4_per_second()
        .to_string();

    let (warning, suggestions) = match result.feedback() {
        Some(fb) => (
            fb.warning().map(|w| w.to_string()),
            fb.suggestions().iter().map(ToString::to_string).collect(),
        ),
        None => (None, Vec::new()),
    };

    PasswordStrength {
        score,
        label: score_label(score),
        crack_time_display,
        warning,
        suggestions,
        meets_policy: long_enough && score >= MIN_SCORE,
        long_enough,
    }
}

/// Defense-in-depth check at the IPC boundary. Frontend already gates the
/// Lock button on `meets_policy`, but the backend re-validates so a custom
/// IPC caller can't bypass it.
pub fn validate_policy(password: &str) -> Result<(), PolicyError> {
    if password.chars().count() < MIN_LENGTH {
        return Err(PolicyError::TooShort);
    }
    let strength = score_password(password);
    if strength.score >= MIN_SCORE {
        return Ok(());
    }
    // Stitch together the most actionable user-facing one-liner. zxcvbn's
    // feedback is high-signal: warning explains what's wrong, suggestion
    // explains what to do about it.
    let feedback = match (&strength.warning, strength.suggestions.first()) {
        (Some(w), Some(s)) => format!("{w} — {s}"),
        (Some(w), None) => w.clone(),
        (None, Some(s)) => s.clone(),
        (None, None) => format!(
            "estimated crack time: {} (need score ≥ {MIN_SCORE})",
            strength.crack_time_display
        ),
    };
    Err(PolicyError::TooWeak { feedback })
}

const fn score_label(score: u8) -> &'static str {
    match score {
        0 => "Very weak",
        1 => "Weak",
        2 => "Fair",
        3 => "Strong",
        _ => "Excellent",
    }
}

// Bridge for the IPC layer — convert PolicyError into the existing storage
// error vocabulary without dragging StorageError into this crate.
impl PolicyError {
    /// Get the user-facing message — same string thiserror would Display.
    #[must_use]
    pub fn user_message(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejects_short_passwords() {
        let r = score_password("test");
        assert!(!r.long_enough);
        assert!(!r.meets_policy);
    }

    #[test]
    fn test_rejects_dictionary_words_even_when_long() {
        // 12 chars but a common base + suffix → zxcvbn should hate it.
        let r = score_password("password1234");
        assert!(r.long_enough);
        assert!(!r.meets_policy, "got score {}", r.score);
    }

    #[test]
    fn test_accepts_strong_passphrase() {
        let r = score_password("correct horse battery staple");
        assert!(r.long_enough);
        assert!(r.meets_policy, "expected meets_policy, got score {}", r.score);
        assert!(!r.crack_time_display.is_empty());
    }

    #[test]
    fn test_score_label_round() {
        assert_eq!(score_label(0), "Very weak");
        assert_eq!(score_label(4), "Excellent");
    }

    #[test]
    fn test_validate_policy_too_short() {
        assert!(matches!(validate_policy("abc"), Err(PolicyError::TooShort)));
    }

    #[test]
    fn test_validate_policy_too_weak() {
        let err = validate_policy("password1234").unwrap_err();
        assert!(matches!(err, PolicyError::TooWeak { .. }));
        // user_message should not be empty.
        assert!(!err.user_message().is_empty());
    }

    #[test]
    fn test_validate_policy_accepts_strong() {
        assert!(validate_policy("correct horse battery staple").is_ok());
    }

    #[test]
    fn test_crack_time_string_has_known_shape() {
        // We don't pin a specific format ('centuries' / '20 years' depend on
        // zxcvbn) but it must be non-empty and not a raw number.
        let r = score_password("correct horse battery staple");
        let ct = &r.crack_time_display;
        assert!(!ct.is_empty());
        assert!(ct.chars().any(char::is_alphabetic), "{ct} should contain letters");
    }
}
