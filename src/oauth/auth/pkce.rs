//! PKCE (Proof Key for Code Exchange) implementation.
//!
//! This module provides PKCE functionality for OAuth 2.0 authorization code flows:
//!
//! - Code verifier generation (43-128 URL-safe characters)
//! - S256 code challenge derivation using SHA-256
//! - Verification that a challenge matches a verifier
//!
//! # Security
//!
//! PKCE protects against authorization code interception attacks by requiring
//! the client to prove possession of a secret (the verifier) when exchanging
//! the authorization code for tokens.
//!
//! # Example
//!
//! ```rust,ignore
//! use gate::auth::pkce::Pkce;
//!
//! // Generate a new PKCE pair
//! let pkce = Pkce::generate();
//! println!("Verifier: {}", pkce.verifier);
//! println!("Challenge: {}", pkce.challenge);
//! println!("Method: {}", pkce.method);
//!
//! // Verify a verifier matches a challenge
//! assert!(Pkce::verify(&pkce.verifier, &pkce.challenge));
//! ```

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};

/// PKCE verifier length in bytes.
///
/// 32 bytes produces a 43-character base64url string, which is within
/// the RFC 7636 requirement of 43-128 characters.
const PKCE_VERIFIER_LENGTH: usize = 32;

/// PKCE challenge method constant.
const PKCE_METHOD: &str = "S256";

/// PKCE (Proof Key for Code Exchange) data.
///
/// Contains a code verifier and its corresponding challenge for use
/// in the OAuth 2.0 authorization code flow with PKCE.
///
/// # Example
///
/// ```rust,ignore
/// use gate::auth::pkce::Pkce;
///
/// let pkce = Pkce::generate();
///
/// // Verifier is 43 URL-safe characters (32 bytes base64url encoded)
/// assert_eq!(pkce.verifier.len(), 43);
///
/// // Challenge is SHA-256 hash of verifier, base64url encoded
/// assert!(pkce.challenge.len() >= 43);
///
/// // Method is always S256
/// assert_eq!(pkce.method, "S256");
/// ```
#[derive(Debug, Clone)]
pub struct Pkce {
    /// The code verifier (secret, used during token exchange).
    ///
    /// This is a cryptographically random string that must be kept secret
    /// and sent during token exchange to prove possession.
    pub verifier: String,

    /// The code challenge (sent in authorization URL).
    ///
    /// This is the SHA-256 hash of the verifier, base64url encoded.
    /// It is sent with the authorization request so the server can
    /// verify the verifier during token exchange.
    pub challenge: String,

    /// The challenge method (always "S256").
    ///
    /// S256 indicates SHA-256 hashing, which is the recommended method
    /// per RFC 7636.
    pub method: &'static str,
}

impl Pkce {
    /// Generate a new PKCE verifier/challenge pair.
    ///
    /// Uses 32 cryptographically random bytes for the verifier,
    /// producing a 43-character base64url string. The challenge is
    /// the SHA-256 hash of the verifier, also base64url encoded.
    ///
    /// # Returns
    ///
    /// A new `Pkce` instance with generated verifier and challenge.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use gate::auth::pkce::Pkce;
    ///
    /// let pkce = Pkce::generate();
    /// assert!(!pkce.verifier.is_empty());
    /// assert!(!pkce.challenge.is_empty());
    /// assert_eq!(pkce.method, "S256");
    /// ```
    #[must_use]
    pub fn generate() -> Self {
        // Generate random bytes for verifier
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; PKCE_VERIFIER_LENGTH] = rng.gen();

        // Base64url encode the verifier (no padding)
        let verifier = URL_SAFE_NO_PAD.encode(random_bytes);

        // SHA256 hash the verifier and base64url encode for challenge
        let challenge = Self::compute_challenge(&verifier);

        Self {
            verifier,
            challenge,
            method: PKCE_METHOD,
        }
    }

    /// Verify that a challenge matches a verifier.
    ///
    /// Computes the SHA-256 hash of the verifier and compares it to
    /// the provided challenge.
    ///
    /// # Arguments
    ///
    /// * `verifier` - The PKCE code verifier
    /// * `challenge` - The PKCE code challenge to verify against
    ///
    /// # Returns
    ///
    /// `true` if the challenge matches the verifier, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use gate::auth::pkce::Pkce;
    ///
    /// let pkce = Pkce::generate();
    /// assert!(Pkce::verify(&pkce.verifier, &pkce.challenge));
    /// assert!(!Pkce::verify("wrong_verifier", &pkce.challenge));
    /// ```
    #[must_use]
    pub fn verify(verifier: &str, challenge: &str) -> bool {
        let expected = Self::compute_challenge(verifier);
        expected == challenge
    }

    /// Compute the S256 challenge from a verifier.
    ///
    /// # Arguments
    ///
    /// * `verifier` - The code verifier string
    ///
    /// # Returns
    ///
    /// The base64url-encoded SHA-256 hash of the verifier.
    fn compute_challenge(verifier: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        URL_SAFE_NO_PAD.encode(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = Pkce::generate();
        assert!(!pkce.verifier.is_empty());
        assert!(!pkce.challenge.is_empty());
        assert_eq!(pkce.method, "S256");

        // Verify that the challenge matches the verifier
        assert!(Pkce::verify(&pkce.verifier, &pkce.challenge));
    }

    #[test]
    fn test_verifier_length() {
        let pkce = Pkce::generate();
        // 32 bytes base64url encoded = 43 characters
        assert_eq!(pkce.verifier.len(), 43);
    }

    #[test]
    fn test_verifier_url_safe() {
        let pkce = Pkce::generate();
        // Should only contain URL-safe characters (no + or /)
        assert!(
            pkce.verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "Verifier contains non-URL-safe characters: {}",
            pkce.verifier
        );
    }

    #[test]
    fn test_challenge_url_safe() {
        let pkce = Pkce::generate();
        // Challenge should also be URL-safe
        assert!(
            pkce.challenge
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "Challenge contains non-URL-safe characters: {}",
            pkce.challenge
        );
    }

    #[test]
    fn test_challenge_deterministic_for_verifier() {
        let pkce = Pkce::generate();

        // Manually compute challenge from the verifier
        let mut hasher = Sha256::new();
        hasher.update(pkce.verifier.as_bytes());
        let hash = hasher.finalize();
        let expected_challenge = URL_SAFE_NO_PAD.encode(hash);

        assert_eq!(pkce.challenge, expected_challenge);
    }

    #[test]
    fn test_verification_success() {
        let pkce = Pkce::generate();
        assert!(Pkce::verify(&pkce.verifier, &pkce.challenge));
    }

    #[test]
    fn test_verification_failure_wrong_verifier() {
        let pkce = Pkce::generate();
        assert!(!Pkce::verify("wrong_verifier", &pkce.challenge));
    }

    #[test]
    fn test_verification_failure_wrong_challenge() {
        let pkce = Pkce::generate();
        assert!(!Pkce::verify(&pkce.verifier, "wrong_challenge"));
    }

    #[test]
    fn test_unique_generation() {
        let pkce1 = Pkce::generate();
        let pkce2 = Pkce::generate();

        assert_ne!(pkce1.verifier, pkce2.verifier);
        assert_ne!(pkce1.challenge, pkce2.challenge);
    }

    #[test]
    fn test_clone() {
        let pkce = Pkce::generate();
        let cloned = pkce.clone();

        assert_eq!(pkce.verifier, cloned.verifier);
        assert_eq!(pkce.challenge, cloned.challenge);
        assert_eq!(pkce.method, cloned.method);
    }
}
