//! OAuth flow state for in-progress authentication.
//!
//! This module provides the [`OAuthFlowState`] struct which holds the transient
//! state needed during an OAuth authorization flow, including PKCE data and
//! the state parameter for CSRF protection.
//!
//! # Usage
//!
//! ```rust,ignore
//! use gate::auth::state::OAuthFlowState;
//!
//! // Create a new flow state with generated PKCE and state
//! let flow_state = OAuthFlowState::new();
//!
//! // Store these while the user completes authorization
//! let verifier = &flow_state.code_verifier;
//! let state = &flow_state.state;
//!
//! // Later, validate the received state matches
//! if received_state != flow_state.state {
//!     return Err(AuthError::StateMismatch);
//! }
//!
//! // Use the verifier during token exchange
//! let token = exchange_code(code, verifier).await?;
//! ```

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;

use super::pkce::Pkce;

/// State for an in-progress OAuth authorization flow.
///
/// Contains the PKCE verifier, challenge, and state parameter needed
/// to complete the OAuth exchange. This should be stored temporarily
/// (in memory or session storage) while the user completes authorization.
///
/// # Security
///
/// - The `code_verifier` is secret and must never be logged or exposed to users
/// - The `state` parameter protects against CSRF attacks
/// - Flow state should be cleared after successful token exchange or timeout
///
/// # Example
///
/// ```rust,ignore
/// use gate::auth::state::OAuthFlowState;
///
/// let flow_state = OAuthFlowState::new();
///
/// // Build authorization URL with challenge and state
/// let url = format!(
///     "https://auth.example.com/authorize?code_challenge={}&state={}",
///     flow_state.code_challenge, flow_state.state
/// );
///
/// // Store flow_state while user authenticates...
///
/// // When callback received, validate state and use verifier
/// assert_eq!(callback_state, flow_state.state);
/// let token = exchange_code(code, &flow_state.code_verifier).await?;
/// ```
#[derive(Debug, Clone)]
pub struct OAuthFlowState {
    /// The PKCE code verifier (secret, used during token exchange).
    ///
    /// This is a 43-character base64url-encoded string generated from
    /// 32 random bytes. It is sent during token exchange to prove
    /// possession of the original authorization request.
    pub code_verifier: String,

    /// The PKCE code challenge (sent in authorization URL).
    ///
    /// This is the SHA-256 hash of the verifier, base64url encoded.
    /// It is sent with the authorization request so the token endpoint
    /// can verify the verifier during exchange.
    pub code_challenge: String,

    /// Random state parameter for CSRF protection.
    ///
    /// This is a 22-character base64url-encoded string generated from
    /// 16 random bytes. It must be validated when the OAuth callback
    /// is received to prevent CSRF attacks.
    pub state: String,
}

impl OAuthFlowState {
    /// Create a new OAuthFlowState with generated PKCE and state values.
    ///
    /// Uses cryptographically secure random generation for both
    /// the PKCE verifier and the state parameter.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use gate::auth::state::OAuthFlowState;
    ///
    /// let state = OAuthFlowState::new();
    /// assert_eq!(state.code_verifier.len(), 43);
    /// assert_eq!(state.state.len(), 22);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let pkce = Pkce::generate();
        let state = generate_state();
        Self {
            code_verifier: pkce.verifier,
            code_challenge: pkce.challenge,
            state,
        }
    }

    /// Create a flow state from existing PKCE data.
    ///
    /// Useful when you need to recreate state from stored values.
    ///
    /// # Arguments
    ///
    /// * `pkce` - Pre-generated PKCE data
    /// * `state` - Pre-generated state parameter
    #[must_use]
    pub fn from_pkce(pkce: Pkce, state: String) -> Self {
        Self {
            code_verifier: pkce.verifier,
            code_challenge: pkce.challenge,
            state,
        }
    }

    /// Validate that a received state matches the expected state.
    ///
    /// # Arguments
    ///
    /// * `received_state` - The state parameter received in the OAuth callback
    ///
    /// # Returns
    ///
    /// `true` if the states match, `false` if there's a mismatch
    /// (indicating a potential CSRF attack or stale flow).
    #[must_use]
    pub fn validate_state(&self, received_state: &str) -> bool {
        self.state == received_state
    }
}

impl Default for OAuthFlowState {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a random state parameter for CSRF protection.
///
/// The state is a 16-byte random value encoded as base64url,
/// resulting in 22 characters. This should be stored and validated
/// when the OAuth callback is received.
///
/// # Example
///
/// ```rust,ignore
/// use gate::auth::state::generate_state;
///
/// let state1 = generate_state();
/// let state2 = generate_state();
///
/// // States are unique
/// assert_ne!(state1, state2);
///
/// // 16 bytes = 22 base64url characters (no padding)
/// assert_eq!(state1.len(), 22);
/// ```
#[must_use]
pub fn generate_state() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_flow_state_new() {
        let state = OAuthFlowState::new();

        // Verify verifier is valid (43 characters, URL-safe)
        assert_eq!(state.code_verifier.len(), 43);
        assert!(state
            .code_verifier
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));

        // Verify challenge is valid (base64url encoded SHA-256)
        assert!(!state.code_challenge.is_empty());
        assert!(state
            .code_challenge
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));

        // Verify state is valid (22 characters, URL-safe)
        assert_eq!(state.state.len(), 22);
        assert!(state
            .state
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn test_oauth_flow_state_unique() {
        let state1 = OAuthFlowState::new();
        let state2 = OAuthFlowState::new();

        assert_ne!(state1.code_verifier, state2.code_verifier);
        assert_ne!(state1.code_challenge, state2.code_challenge);
        assert_ne!(state1.state, state2.state);
    }

    #[test]
    fn test_oauth_flow_state_default() {
        let state = OAuthFlowState::default();
        assert_eq!(state.code_verifier.len(), 43);
        assert_eq!(state.state.len(), 22);
    }

    #[test]
    fn test_oauth_flow_state_from_pkce() {
        let pkce = Pkce::generate();
        let state_param = generate_state();

        let flow_state = OAuthFlowState::from_pkce(pkce.clone(), state_param.clone());

        assert_eq!(flow_state.code_verifier, pkce.verifier);
        assert_eq!(flow_state.code_challenge, pkce.challenge);
        assert_eq!(flow_state.state, state_param);
    }

    #[test]
    fn test_validate_state_success() {
        let flow_state = OAuthFlowState::new();
        let received = flow_state.state.clone();

        assert!(flow_state.validate_state(&received));
    }

    #[test]
    fn test_validate_state_failure() {
        let flow_state = OAuthFlowState::new();

        assert!(!flow_state.validate_state("wrong_state"));
        assert!(!flow_state.validate_state(""));
        assert!(!flow_state.validate_state(&generate_state()));
    }

    #[test]
    fn test_generate_state_length() {
        let state = generate_state();
        // 16 bytes base64url encoded = 22 characters
        assert_eq!(state.len(), 22);
    }

    #[test]
    fn test_generate_state_url_safe() {
        let state = generate_state();
        assert!(
            state
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "State contains non-URL-safe characters: {}",
            state
        );
    }

    #[test]
    fn test_generate_state_unique() {
        let state1 = generate_state();
        let state2 = generate_state();
        assert_ne!(state1, state2);
    }

    #[test]
    fn test_clone() {
        let original = OAuthFlowState::new();
        let cloned = original.clone();

        assert_eq!(original.code_verifier, cloned.code_verifier);
        assert_eq!(original.code_challenge, cloned.code_challenge);
        assert_eq!(original.state, cloned.state);
    }
}
