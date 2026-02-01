//! Unified error types for the gate authentication framework.
//!
//! This module provides a comprehensive error taxonomy for OAuth-based API clients.
//! The error types are designed to:
//!
//! - Support multiple authentication providers (Claude, Gemini, etc.)
//! - Provide actionable error information for retry and recovery logic
//! - Be `Send + Sync + 'static` for async compatibility
//! - Include structured context for debugging
//!
//! # Error Hierarchy
//!
//! The error system uses two levels:
//!
//! 1. [`Error`] - Top-level errors covering all failure modes
//! 2. [`AuthError`] - Authentication-specific errors nested under `Error::Auth`
//!
//! # Example
//!
//! ```rust
//! use ttrpg_assistant::gate::error::{Error, AuthError};
//! use std::time::Duration;
//!
//! fn handle_error(err: &Error) {
//!     if err.requires_reauth() {
//!         println!("User must re-authenticate");
//!     } else if let Some(retry_after) = err.retry_after() {
//!         println!("Rate limited, retry after {:?}", retry_after);
//!     } else if err.is_recoverable() {
//!         println!("Transient error, safe to retry");
//!     }
//! }
//! ```

use std::time::Duration;
use thiserror::Error;

/// Result type alias using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Unified error type for the gate authentication framework.
///
/// This enum covers all failure modes that can occur when interacting
/// with OAuth-based API providers. Each variant includes contextual
/// information to aid debugging and recovery.
///
/// # Error Categories
///
/// - **Authentication**: Token-related failures requiring user action
/// - **API**: Server-side errors from the provider
/// - **Network**: Connection and transport failures
/// - **Data**: Serialization and parsing errors
/// - **Configuration**: Setup and environment issues
/// - **Storage**: Token persistence failures
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Authentication-related errors requiring user action.
    ///
    /// These errors indicate the user needs to re-authenticate or
    /// there's an issue with the OAuth flow.
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    /// API error returned by the provider.
    ///
    /// Includes the HTTP status code and any retry-after duration
    /// for rate limit responses.
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code from the API response.
        status: u16,
        /// Human-readable error message from the API.
        message: String,
        /// Duration to wait before retrying (for rate limits).
        retry_after: Option<Duration>,
    },

    /// Network or HTTP transport error.
    ///
    /// Covers connection failures, timeouts, and TLS errors.
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Configuration error.
    ///
    /// Indicates missing or invalid configuration such as
    /// OAuth client IDs, endpoints, or feature flags.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Token storage error.
    ///
    /// Covers failures reading from or writing to the token store.
    #[error("Storage error: {0}")]
    Storage(String),

    /// I/O error.
    ///
    /// File system operations, typically during token persistence.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// URL parsing error.
    ///
    /// Invalid OAuth redirect URLs or API endpoints.
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
}

impl Error {
    /// Create a configuration error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ttrpg_assistant::gate::error::Error;
    ///
    /// let err = Error::config("Missing OAuth client ID");
    /// assert!(err.to_string().contains("client ID"));
    /// ```
    #[must_use]
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a storage error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ttrpg_assistant::gate::error::Error;
    ///
    /// let err = Error::storage("Failed to write token file");
    /// assert!(err.to_string().contains("token file"));
    /// ```
    #[must_use]
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    /// Create an API error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ttrpg_assistant::gate::error::Error;
    /// use std::time::Duration;
    ///
    /// // Rate limit error with retry-after
    /// let err = Error::api(429, "Rate limit exceeded", Some(Duration::from_secs(60)));
    /// assert!(err.is_rate_limit());
    /// assert_eq!(err.retry_after(), Some(Duration::from_secs(60)));
    ///
    /// // Server error without retry
    /// let err = Error::api(500, "Internal server error", None);
    /// assert!(!err.is_rate_limit());
    /// ```
    #[must_use]
    pub fn api(status: u16, message: impl Into<String>, retry_after: Option<Duration>) -> Self {
        Self::Api {
            status,
            message: message.into(),
            retry_after,
        }
    }

    /// Check if this is a rate limit error (HTTP 429).
    ///
    /// Rate limit errors typically include a `retry_after` duration
    /// indicating when the client can retry.
    #[must_use]
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Self::Api { status: 429, .. })
    }

    /// Check if this is any authentication-related error.
    ///
    /// Returns `true` for:
    /// - All `Error::Auth` variants
    /// - HTTP 401 Unauthorized responses
    /// - HTTP 403 Forbidden responses (may indicate token issues)
    #[must_use]
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            Self::Auth(_) | Self::Api { status: 401 | 403, .. }
        )
    }

    /// Check if this error requires the user to re-authenticate.
    ///
    /// Returns `true` when the current tokens are invalid and cannot
    /// be automatically refreshed. The user must complete a new OAuth
    /// flow to continue.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ttrpg_assistant::gate::error::{Error, AuthError};
    ///
    /// let err = Error::Auth(AuthError::TokenExpired);
    /// assert!(err.requires_reauth());
    ///
    /// let err = Error::Auth(AuthError::InvalidGrant);
    /// assert!(err.requires_reauth());
    /// ```
    #[must_use]
    pub fn requires_reauth(&self) -> bool {
        match self {
            Self::Auth(auth_err) => auth_err.requires_reauth(),
            Self::Api { status: 401, .. } => true,
            _ => false,
        }
    }

    /// Get the retry-after duration if this is a rate limit error.
    ///
    /// Returns `None` for non-rate-limit errors.
    #[must_use]
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Api { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Check if this error is transient and safe to retry.
    ///
    /// Returns `true` for errors that may succeed on retry:
    /// - Network timeouts and connection errors
    /// - Server errors (5xx)
    /// - Rate limits (should wait for retry_after)
    ///
    /// Returns `false` for:
    /// - Authentication errors (need reauth)
    /// - Client errors (4xx except 429)
    /// - Configuration errors
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Network(_) => true,
            Self::Api { status, .. } => *status >= 500 || *status == 429,
            Self::Io(err) => matches!(
                err.kind(),
                std::io::ErrorKind::Interrupted
                    | std::io::ErrorKind::WouldBlock
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::NotConnected
                    | std::io::ErrorKind::UnexpectedEof
            ),
            _ => false,
        }
    }
}

/// Authentication-specific errors.
///
/// These errors indicate issues with the OAuth flow or tokens
/// that typically require user action to resolve.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthError {
    /// No valid credentials are stored.
    ///
    /// The user has not completed OAuth authentication.
    #[error("Not authenticated - please complete OAuth flow")]
    NotAuthenticated,

    /// Access token has expired and cannot be refreshed.
    ///
    /// This occurs when both the access token and refresh token
    /// are invalid or expired.
    #[error("Token expired - please re-authenticate")]
    TokenExpired,

    /// Refresh token is invalid, revoked, or corrupted.
    ///
    /// The refresh token cannot be used to obtain new access tokens.
    /// This typically happens when:
    /// - The user revoked access in the provider's settings
    /// - The refresh token was used too many times (some providers limit this)
    /// - The token data was corrupted
    #[error("Invalid grant - refresh token is invalid or revoked")]
    InvalidGrant,

    /// OAuth state parameter mismatch.
    ///
    /// The state returned by the OAuth callback doesn't match
    /// what was sent in the authorization request. This could
    /// indicate a CSRF attack or a stale OAuth flow.
    #[error("OAuth state mismatch - possible CSRF attack")]
    StateMismatch,

    /// PKCE verification failed.
    ///
    /// The code verifier doesn't match the code challenge
    /// sent in the authorization request.
    #[error("PKCE verification failed")]
    PkceVerificationFailed,

    /// OAuth flow was cancelled by the user or timed out.
    #[error("OAuth flow cancelled")]
    Cancelled,

    /// Project discovery failed.
    ///
    /// Some OAuth providers require discovering a project ID
    /// after authentication. This error indicates that step failed.
    #[error("Failed to discover project: {0}")]
    ProjectDiscovery(String),

    /// Token refresh failed with a specific reason.
    #[error("Token refresh failed: {0}")]
    RefreshFailed(String),
}

impl AuthError {
    /// Check if this error requires the user to re-authenticate.
    ///
    /// Returns `true` for errors that cannot be resolved without
    /// user action (completing a new OAuth flow).
    #[must_use]
    pub fn requires_reauth(&self) -> bool {
        matches!(
            self,
            Self::NotAuthenticated
                | Self::TokenExpired
                | Self::InvalidGrant
                | Self::StateMismatch
                | Self::PkceVerificationFailed
                | Self::RefreshFailed(_)
        )
    }

    /// Create a project discovery error.
    #[must_use]
    pub fn project_discovery(msg: impl Into<String>) -> Self {
        Self::ProjectDiscovery(msg.into())
    }

    /// Create a refresh failed error.
    #[must_use]
    pub fn refresh_failed(msg: impl Into<String>) -> Self {
        Self::RefreshFailed(msg.into())
    }
}

#[cfg(feature = "keyring")]
impl From<keyring::Error> for Error {
    fn from(e: keyring::Error) -> Self {
        Self::Storage(format!("Keyring error: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::config("missing required field");
        assert_eq!(
            err.to_string(),
            "Configuration error: missing required field"
        );

        let err = Error::api(429, "rate limited", Some(Duration::from_secs(60)));
        assert!(err.to_string().contains("429"));
        assert!(err.to_string().contains("rate limited"));
    }

    #[test]
    fn test_is_rate_limit() {
        let err = Error::api(429, "rate limited", Some(Duration::from_secs(60)));
        assert!(err.is_rate_limit());
        assert_eq!(err.retry_after(), Some(Duration::from_secs(60)));

        let err = Error::api(500, "server error", None);
        assert!(!err.is_rate_limit());
        assert_eq!(err.retry_after(), None);
    }

    #[test]
    fn test_is_auth_error() {
        let err = Error::Auth(AuthError::NotAuthenticated);
        assert!(err.is_auth_error());

        let err = Error::api(401, "unauthorized", None);
        assert!(err.is_auth_error());

        let err = Error::api(403, "forbidden", None);
        assert!(err.is_auth_error());

        let err = Error::api(404, "not found", None);
        assert!(!err.is_auth_error());
    }

    #[test]
    fn test_requires_reauth() {
        let err = Error::Auth(AuthError::NotAuthenticated);
        assert!(err.requires_reauth());

        let err = Error::Auth(AuthError::TokenExpired);
        assert!(err.requires_reauth());

        let err = Error::Auth(AuthError::InvalidGrant);
        assert!(err.requires_reauth());

        let err = Error::Auth(AuthError::Cancelled);
        assert!(!err.requires_reauth());

        let err = Error::api(401, "unauthorized", None);
        assert!(err.requires_reauth());

        let err = Error::api(500, "server error", None);
        assert!(!err.requires_reauth());
    }

    #[test]
    fn test_is_recoverable() {
        // Configuration errors are not recoverable
        let err = Error::config("test");
        assert!(!err.is_recoverable());

        // Server errors are recoverable
        let err = Error::api(500, "server error", None);
        assert!(err.is_recoverable());

        let err = Error::api(502, "bad gateway", None);
        assert!(err.is_recoverable());

        // Rate limits are recoverable (with delay)
        let err = Error::api(429, "rate limited", Some(Duration::from_secs(60)));
        assert!(err.is_recoverable());

        // Client errors are not recoverable
        let err = Error::api(400, "bad request", None);
        assert!(!err.is_recoverable());

        let err = Error::api(401, "unauthorized", None);
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_auth_error_display() {
        let err = AuthError::NotAuthenticated;
        assert!(err.to_string().contains("Not authenticated"));

        let err = AuthError::TokenExpired;
        assert!(err.to_string().contains("expired"));

        let err = AuthError::InvalidGrant;
        assert!(err.to_string().contains("Invalid grant"));

        let err = AuthError::StateMismatch;
        assert!(err.to_string().contains("CSRF"));

        let err = AuthError::project_discovery("project not found");
        assert!(err.to_string().contains("project not found"));
    }

    #[test]
    fn test_auth_error_requires_reauth() {
        assert!(AuthError::NotAuthenticated.requires_reauth());
        assert!(AuthError::TokenExpired.requires_reauth());
        assert!(AuthError::InvalidGrant.requires_reauth());
        assert!(AuthError::StateMismatch.requires_reauth());
        assert!(AuthError::PkceVerificationFailed.requires_reauth());
        assert!(AuthError::refresh_failed("test").requires_reauth());

        assert!(!AuthError::Cancelled.requires_reauth());
        assert!(!AuthError::project_discovery("test").requires_reauth());
    }

    #[test]
    fn test_error_conversions() {
        // JSON error
        let json_err: std::result::Result<serde_json::Value, _> = serde_json::from_str("invalid");
        assert!(json_err.is_err());
        let err: Error = json_err.unwrap_err().into();
        assert!(matches!(err, Error::Json(_)));

        // IO error
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));

        // URL error
        let url_err: std::result::Result<url::Url, _> = "not a url".parse();
        assert!(url_err.is_err());
        let err: Error = url_err.unwrap_err().into();
        assert!(matches!(err, Error::Url(_)));

        // Auth error
        let auth_err = AuthError::NotAuthenticated;
        let err: Error = auth_err.into();
        assert!(matches!(err, Error::Auth(_)));
    }

    #[test]
    fn test_auth_error_clone_and_eq() {
        let err1 = AuthError::NotAuthenticated;
        let err2 = err1.clone();
        assert_eq!(err1, err2);

        let err3 = AuthError::TokenExpired;
        assert_ne!(err1, err3);
    }
}
