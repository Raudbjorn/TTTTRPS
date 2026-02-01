//! Error types for the claude-gate crate.

use thiserror::Error;

/// Result type alias using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when using the claude-gate crate.
#[derive(Error, Debug)]
pub enum Error {
    /// OAuth flow errors
    #[error("OAuth error: {0}")]
    OAuth(String),

    /// Token storage errors
    #[error("Storage error: {0}")]
    Storage(String),

    /// Token is expired and refresh failed
    #[error("Token expired: {0}")]
    TokenExpired(String),

    /// Token refresh failed
    #[error("Token refresh failed: {0}")]
    RefreshFailed(String),

    /// Not authenticated
    #[error("Not authenticated: no valid token available")]
    NotAuthenticated,

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing error
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// API error response from Anthropic
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Error message from API
        message: String,
        /// Error type from API
        error_type: Option<String>,
    },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Invalid state during OAuth flow
    #[error("Invalid OAuth state: expected {expected}, got {actual}")]
    InvalidState {
        /// Expected state value
        expected: String,
        /// Actual state value received
        actual: String,
    },

    /// PKCE verification failed
    #[error("PKCE verification failed")]
    PkceVerificationFailed,

    /// Streaming error
    #[error("Streaming error: {0}")]
    Stream(String),

    /// Keyring error (when keyring feature is enabled)
    #[cfg(feature = "keyring")]
    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),
}

impl Error {
    /// Create an OAuth error
    pub fn oauth(msg: impl Into<String>) -> Self {
        Self::OAuth(msg.into())
    }

    /// Create a storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::Storage(msg.into())
    }

    /// Create a config error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create an API error
    pub fn api(status: u16, message: impl Into<String>, error_type: Option<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
            error_type,
        }
    }

    /// Create a stream error
    pub fn stream(msg: impl Into<String>) -> Self {
        Self::Stream(msg.into())
    }

    /// Check if this is an authentication error that requires re-login
    #[must_use]
    pub fn requires_reauth(&self) -> bool {
        matches!(
            self,
            Self::NotAuthenticated
                | Self::TokenExpired(_)
                | Self::RefreshFailed(_)
                | Self::Api { status: 401, .. }
        )
    }
}

impl From<crate::oauth::error::Error> for Error {
    fn from(err: crate::oauth::error::Error) -> Self {
        match err {
            crate::oauth::error::Error::Auth(auth_err) => match auth_err {
                crate::oauth::error::AuthError::NotAuthenticated => Error::NotAuthenticated,
                crate::oauth::error::AuthError::TokenExpired => Error::TokenExpired("Token expired".into()),
                crate::oauth::error::AuthError::InvalidGrant => Error::OAuth("Invalid grant".into()),
                crate::oauth::error::AuthError::StateMismatch => Error::InvalidState {
                    expected: "unknown".into(),
                    actual: "mismatch".into(),
                },
                crate::oauth::error::AuthError::PkceVerificationFailed => Error::PkceVerificationFailed,
                crate::oauth::error::AuthError::Cancelled => Error::OAuth("Cancelled".into()),
                crate::oauth::error::AuthError::ProjectDiscovery(msg) => Error::OAuth(format!("Project discovery failed: {}", msg)),
                crate::oauth::error::AuthError::RefreshFailed(msg) => Error::RefreshFailed(msg),
            },
            crate::oauth::error::Error::Api { status, message, .. } => Error::Api {
                status,
                message,
                error_type: None,
            },
            crate::oauth::error::Error::Network(e) => Error::Http(e),
            crate::oauth::error::Error::Json(e) => Error::Json(e),
            crate::oauth::error::Error::Config(msg) => Error::Config(msg),
            crate::oauth::error::Error::Storage(msg) => Error::Storage(msg),
            crate::oauth::error::Error::Io(e) => Error::Io(e),
            crate::oauth::error::Error::Url(e) => Error::Url(e),
        }
    }
}
