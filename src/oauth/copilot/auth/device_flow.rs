//! GitHub OAuth Device Code Flow implementation.
//!
//! This module implements the OAuth 2.0 Device Authorization Grant (RFC 8628)
//! for authenticating with GitHub without requiring a web browser redirect.
//!
//! ## Flow Overview
//!
//! 1. Call [`start_device_flow`] to get a user code and verification URL
//! 2. Display the user code and direct the user to the verification URL
//! 3. Call [`poll_for_token`] to poll for completion
//! 4. Exchange the GitHub token for a Copilot token using [`token_exchange`]

use reqwest::header::{ACCEPT, CONTENT_TYPE};
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::oauth::copilot::auth::constants::{
    DEFAULT_POLL_INTERVAL_SECS, DEVICE_CODE_GRANT_TYPE, ERROR_ACCESS_DENIED,
    ERROR_AUTHORIZATION_PENDING, ERROR_EXPIRED_TOKEN, ERROR_SLOW_DOWN, GITHUB_ACCESS_TOKEN_URL,
    GITHUB_CLIENT_ID, GITHUB_DEVICE_CODE_URL, GITHUB_OAUTH_SCOPE, MAX_POLL_ATTEMPTS,
    POLL_INTERVAL_BUFFER_SECS,
};
use crate::oauth::copilot::error::{Error, Result};
use crate::oauth::copilot::models::auth::{DeviceCodeResponse, GitHubTokenResponse};

/// Pending device flow state, returned after starting the flow.
///
/// Contains the information needed to complete the device code flow:
/// the device code for polling and the user-facing code for display.
#[derive(Debug, Clone)]
pub struct DeviceFlowPending {
    /// The device verification code (internal, for polling).
    pub device_code: String,
    /// The user-facing code to enter at the verification URL.
    pub user_code: String,
    /// URL where the user should enter the code.
    pub verification_uri: String,
    /// Seconds until the device code expires.
    pub expires_in: u64,
    /// Minimum seconds between polling attempts.
    pub interval: u64,
}

impl DeviceFlowPending {
    /// Returns the complete verification URL with the code pre-filled.
    ///
    /// GitHub supports adding the code as a query parameter.
    #[must_use]
    pub fn verification_url_with_code(&self) -> String {
        format!("{}?user_code={}", self.verification_uri, self.user_code)
    }
}

/// Result of polling for token completion.
#[derive(Debug, Clone)]
pub enum PollResult {
    /// User has not yet completed authorization, keep polling.
    Pending,
    /// Received a "slow_down" response, increase poll interval.
    SlowDown,
    /// Successfully obtained the access token.
    Complete(String),
}

/// Starts the device code flow by requesting a device code from GitHub.
///
/// This initiates the OAuth 2.0 Device Authorization Grant by requesting
/// a device code and user code from GitHub's authorization server.
///
/// # Arguments
///
/// * `client` - HTTP client for making the request
///
/// # Returns
///
/// A `DeviceFlowPending` containing the device code, user code, and
/// verification URL to display to the user.
///
/// # Errors
///
/// Returns an error if:
/// - Network error occurs
/// - GitHub returns an error response
/// - Response cannot be parsed
///
/// # Example
///
/// ```no_run
/// use crate::oauth::copilot::auth::device_flow::start_device_flow;
///
/// # async fn example() -> crate::oauth::copilot::Result<()> {
/// let client = reqwest::Client::new();
/// let pending = start_device_flow(&client).await?;
///
/// println!("Please visit: {}", pending.verification_uri);
/// println!("And enter code: {}", pending.user_code);
/// # Ok(())
/// # }
/// ```
pub async fn start_device_flow(client: &reqwest::Client) -> Result<DeviceFlowPending> {
    info!("Starting GitHub device code flow");

    let response = client
        .post(GITHUB_DEVICE_CODE_URL)
        .header(ACCEPT, "application/json")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&[
            ("client_id", GITHUB_CLIENT_ID),
            ("scope", GITHUB_OAUTH_SCOPE),
        ])
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let message = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(Error::DeviceFlow(format!(
            "Failed to start device flow: {} - {}",
            status, message
        )));
    }

    let device_response: DeviceCodeResponse = response.json().await?;

    debug!(
        user_code = %device_response.user_code,
        verification_uri = %device_response.verification_uri,
        expires_in = device_response.expires_in,
        interval = device_response.interval,
        "Device code flow started"
    );

    Ok(DeviceFlowPending {
        device_code: device_response.device_code,
        user_code: device_response.user_code,
        verification_uri: device_response.verification_uri,
        expires_in: device_response.expires_in,
        interval: device_response.interval,
    })
}

/// Error response from GitHub OAuth.
#[derive(Debug, Deserialize)]
struct OAuthError {
    error: String,
    #[serde(default)]
    error_description: Option<String>,
    #[serde(default)]
    interval: Option<u64>,
}

/// Polls GitHub for token completion.
///
/// This should be called repeatedly with appropriate delays until
/// `PollResult::Complete` is returned or an error occurs.
///
/// # Arguments
///
/// * `client` - HTTP client for making the request
/// * `device_code` - The device code from `start_device_flow`
///
/// # Returns
///
/// - `Ok(PollResult::Pending)` - User hasn't completed authorization yet
/// - `Ok(PollResult::SlowDown)` - Polling too fast, increase interval
/// - `Ok(PollResult::Complete(token))` - Got the access token
/// - `Err(_)` - Flow failed (user denied, code expired, or network error)
///
/// # Example
///
/// ```no_run
/// use crate::oauth::copilot::auth::device_flow::{poll_for_token, PollResult};
/// use tokio::time::{sleep, Duration};
///
/// # async fn example(pending: &crate::oauth::copilot::auth::device_flow::DeviceFlowPending) -> crate::oauth::copilot::Result<String> {
/// let client = reqwest::Client::new();
/// let mut interval = pending.interval;
///
/// loop {
///     sleep(Duration::from_secs(interval)).await;
///
///     match poll_for_token(&client, &pending.device_code).await? {
///         PollResult::Pending => continue,
///         PollResult::SlowDown => interval += 5,
///         PollResult::Complete(token) => return Ok(token),
///     }
/// }
/// # }
/// ```
pub async fn poll_for_token(client: &reqwest::Client, device_code: &str) -> Result<PollResult> {
    debug!("Polling for device code authorization");

    let response = client
        .post(GITHUB_ACCESS_TOKEN_URL)
        .header(ACCEPT, "application/json")
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&[
            ("client_id", GITHUB_CLIENT_ID),
            ("device_code", device_code),
            ("grant_type", DEVICE_CODE_GRANT_TYPE),
        ])
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;

    // Try to parse as success response first
    if let Ok(token_response) = serde_json::from_str::<GitHubTokenResponse>(&body) {
        info!(
            token_type = %token_response.token_type,
            scope = %token_response.scope,
            token_preview = %mask_token(&token_response.access_token),
            "Device flow completed successfully"
        );
        return Ok(PollResult::Complete(token_response.access_token));
    }

    // Try to parse as error response
    if let Ok(error_response) = serde_json::from_str::<OAuthError>(&body) {
        match error_response.error.as_str() {
            ERROR_AUTHORIZATION_PENDING => {
                debug!("Authorization pending, will retry");
                return Ok(PollResult::Pending);
            }
            ERROR_SLOW_DOWN => {
                let new_interval = error_response.interval.unwrap_or(10);
                debug!(new_interval, "Received slow_down, increasing interval");
                return Ok(PollResult::SlowDown);
            }
            ERROR_ACCESS_DENIED => {
                warn!("User denied authorization");
                return Err(Error::AuthorizationDenied);
            }
            ERROR_EXPIRED_TOKEN => {
                warn!("Device code expired");
                return Err(Error::DeviceCodeExpired);
            }
            _ => {
                let description = error_response
                    .error_description
                    .unwrap_or_else(|| error_response.error.clone());
                return Err(Error::DeviceFlow(description));
            }
        }
    }

    // Could not parse response
    Err(Error::DeviceFlow(format!(
        "Unexpected response ({}): {}",
        status, body
    )))
}

/// Polls for token with automatic retry logic.
///
/// This is a convenience function that handles the polling loop,
/// respecting the interval and retrying on `Pending` results.
///
/// # Arguments
///
/// * `client` - HTTP client for making requests
/// * `pending` - The pending device flow state
/// * `on_pending` - Optional callback called on each pending poll
///
/// # Returns
///
/// The GitHub access token on success.
///
/// # Errors
///
/// Returns an error if:
/// - User denies authorization
/// - Device code expires
/// - Maximum poll attempts exceeded
/// - Network error occurs
pub async fn poll_until_complete(
    client: &reqwest::Client,
    pending: &DeviceFlowPending,
    mut on_pending: Option<&mut dyn FnMut(u32)>,
) -> Result<String> {
    let mut interval = pending.interval.max(DEFAULT_POLL_INTERVAL_SECS) + POLL_INTERVAL_BUFFER_SECS;
    let mut attempts = 0u32;

    loop {
        // Wait before polling (including first attempt).
        // Per RFC 8628, the client MUST wait at least `interval` seconds between requests.
        // The first poll should also wait since the user needs time to visit the URL and enter the code.
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

        attempts += 1;
        if attempts > MAX_POLL_ATTEMPTS {
            return Err(Error::DeviceFlow(
                "Maximum poll attempts exceeded".to_string(),
            ));
        }

        // Notify callback
        if let Some(ref mut callback) = on_pending {
            callback(attempts);
        }

        match poll_for_token(client, &pending.device_code).await? {
            PollResult::Pending => continue,
            PollResult::SlowDown => {
                interval += 5;
                continue;
            }
            PollResult::Complete(token) => return Ok(token),
        }
    }
}

/// Masks a token for safe logging.
///
/// Shows the first 4 characters followed by asterisks.
#[must_use]
pub fn mask_token(token: &str) -> String {
    if token.len() <= 4 {
        "*".repeat(token.len())
    } else {
        format!("{}****", &token[..4])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_token_normal() {
        assert_eq!(mask_token("gho_abc123xyz"), "gho_****");
    }

    #[test]
    fn test_mask_token_short() {
        assert_eq!(mask_token("abc"), "***");
        assert_eq!(mask_token("abcd"), "****");
    }

    #[test]
    fn test_device_flow_pending_verification_url() {
        let pending = DeviceFlowPending {
            device_code: "dc_test".to_string(),
            user_code: "ABCD-1234".to_string(),
            verification_uri: "https://github.com/login/device".to_string(),
            expires_in: 900,
            interval: 5,
        };

        assert_eq!(
            pending.verification_url_with_code(),
            "https://github.com/login/device?user_code=ABCD-1234"
        );
    }
}
