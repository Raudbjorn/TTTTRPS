//! Error types for the Copilot API client.
//!
//! This module defines the error types used throughout the copilot module,
//! using thiserror for ergonomic error handling.

use thiserror::Error;

/// Result type alias for Copilot operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the Copilot API client.
#[derive(Debug, Error)]
pub enum Error {
    /// Configuration error (invalid settings, missing values).
    #[error("Configuration error: {0}")]
    Config(String),

    /// Not authenticated - no token available.
    #[error("Not authenticated - please sign in first")]
    NotAuthenticated,

    /// Token refresh failed.
    #[error("Token refresh failed: {0}")]
    RefreshFailed(String),

    /// Device flow authentication error.
    #[error("Device flow error: {0}")]
    DeviceFlow(String),

    /// User denied authorization during device flow.
    #[error("Authorization denied by user")]
    AuthorizationDenied,

    /// Device code expired before user completed authorization.
    #[error("Device code expired - please try again")]
    DeviceCodeExpired,

    /// HTTP request error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// API returned an error response.
    #[error("API error ({status}): {message}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Error message from the API.
        message: String,
    },

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Storage error (load/save token).
    #[error("Storage error: {0}")]
    Storage(String),

    /// Stream parsing error.
    #[error("Stream error: {0}")]
    Stream(String),

    /// Timeout waiting for response.
    #[error("Request timeout")]
    Timeout,

    /// Rate limited by the API.
    #[error("Rate limited - retry after {retry_after:?} seconds")]
    RateLimited {
        /// Seconds to wait before retrying.
        retry_after: Option<u64>,
    },

    /// Invalid model specified.
    #[error("Invalid model: {0}")]
    InvalidModel(String),

    /// Internal error (unexpected state).
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Returns true if this error is retriable.
    #[must_use]
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            Self::Http(_) | Self::Timeout | Self::RateLimited { .. } | Self::RefreshFailed(_)
        )
    }

    /// Returns true if this error indicates authentication is needed.
    #[must_use]
    pub fn needs_auth(&self) -> bool {
        matches!(
            self,
            Self::NotAuthenticated | Self::AuthorizationDenied | Self::DeviceCodeExpired
        )
    }

    /// Creates an API error from a status code and message.
    #[must_use]
    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            message: message.into(),
        }
    }

    /// Creates a rate limit error with optional retry-after.
    #[must_use]
    pub fn rate_limited(retry_after: Option<u64>) -> Self {
        Self::RateLimited { retry_after }
    }
}

/// Convert from gate errors (for storage compatibility)
impl From<crate::oauth::error::Error> for Error {
    fn from(err: crate::oauth::error::Error) -> Self {
        Self::Storage(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::NotAuthenticated;
        assert!(err.to_string().contains("Not authenticated"));

        let err = Error::Api {
            status: 401,
            message: "Unauthorized".to_string(),
        };
        assert!(err.to_string().contains("401"));
        assert!(err.to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_is_retriable() {
        assert!(Error::Timeout.is_retriable());
        assert!(Error::RateLimited { retry_after: Some(5) }.is_retriable());
        assert!(!Error::NotAuthenticated.is_retriable());
        assert!(!Error::AuthorizationDenied.is_retriable());
    }

    #[test]
    fn test_needs_auth() {
        assert!(Error::NotAuthenticated.needs_auth());
        assert!(Error::AuthorizationDenied.needs_auth());
        assert!(Error::DeviceCodeExpired.needs_auth());
        assert!(!Error::Timeout.needs_auth());
    }

    #[test]
    fn test_error_constructors() {
        let err = Error::api(404, "Not Found");
        assert!(matches!(err, Error::Api { status: 404, .. }));

        let err = Error::rate_limited(Some(30));
        assert!(matches!(err, Error::RateLimited { retry_after: Some(30) }));
    }
}
