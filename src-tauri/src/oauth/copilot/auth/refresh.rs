//! Copilot token refresh logic.
//!
//! This module handles proactive token refresh to ensure API calls
//! never fail due to an expired token.
//!
//! The refresh strategy uses a buffer period (default 60 seconds) before
//! the actual expiration time, triggering a refresh when the token is
//! "about to expire" rather than waiting until it actually expires.

use tracing::{debug, info, warn};

use crate::oauth::copilot::auth::device_flow::mask_token;
use crate::oauth::copilot::auth::token_exchange::{exchange_for_copilot_token, TokenExchangeConfig};
use crate::oauth::copilot::error::{Error, Result};
use crate::oauth::copilot::models::auth::TokenInfo;

/// Default buffer time (in seconds) before expiry to trigger refresh.
pub const REFRESH_BUFFER_SECS: i64 = 60;

/// Refreshes the Copilot token if it's expired or about to expire.
///
/// This function checks if the Copilot token in `token_info` needs to be
/// refreshed (either missing or within the buffer period of expiration),
/// and if so, exchanges the GitHub token for a new Copilot token.
///
/// # Time Handling
///
/// The function uses the server-provided `expires_at` timestamp for
/// determining expiration. A buffer period (default 60 seconds) is used
/// to trigger proactive refresh before the token actually expires.
///
/// # Arguments
///
/// * `client` - HTTP client for making the exchange request
/// * `token_info` - Current token information (modified in place on refresh)
/// * `config` - Configuration for the token exchange
///
/// # Returns
///
/// - `Ok(true)` - Token was refreshed
/// - `Ok(false)` - Token was still valid, no refresh needed
/// - `Err(_)` - Refresh failed
///
/// # Example
///
/// ```no_run
/// use crate::oauth::copilot::auth::refresh::refresh_copilot_token_if_needed;
/// use crate::oauth::copilot::auth::token_exchange::TokenExchangeConfig;
/// use crate::oauth::copilot::models::TokenInfo;
///
/// # async fn example() -> crate::oauth::copilot::Result<()> {
/// let client = reqwest::Client::new();
/// let config = TokenExchangeConfig::default();
/// let mut token_info = TokenInfo::new("gho_xxxxx");
///
/// let refreshed = refresh_copilot_token_if_needed(&client, &mut token_info, &config).await?;
/// if refreshed {
///     println!("Token was refreshed");
/// }
/// # Ok(())
/// # }
/// ```
pub async fn refresh_copilot_token_if_needed(
    client: &reqwest::Client,
    token_info: &mut TokenInfo,
    config: &TokenExchangeConfig,
) -> Result<bool> {
    if !needs_refresh(token_info) {
        debug!("Copilot token is still valid, no refresh needed");
        return Ok(false);
    }

    info!(
        expires_at = ?token_info.copilot_expires_at,
        "Refreshing Copilot token"
    );

    let copilot_response =
        exchange_for_copilot_token(client, &token_info.github_token, config).await?;

    // Update the token info with the new Copilot token
    token_info.copilot_token = Some(copilot_response.token.clone());
    token_info.copilot_expires_at = Some(copilot_response.expires_at);

    info!(
        new_expires_at = copilot_response.expires_at,
        token_preview = %mask_token(&copilot_response.token),
        "Copilot token refreshed successfully"
    );

    Ok(true)
}

/// Checks if the Copilot token needs to be refreshed.
///
/// A token needs refresh if:
/// - There is no Copilot token
/// - The token expires within the buffer period
///
/// This function uses server timestamps and does not depend on local
/// clock accuracy for the expiration check.
fn needs_refresh(token_info: &TokenInfo) -> bool {
    token_info.needs_copilot_refresh()
}

/// Ensures a valid Copilot token is available.
///
/// This function is similar to [`refresh_copilot_token_if_needed`] but
/// returns the token info directly if successful, or an error if the
/// token cannot be obtained.
///
/// # Errors
///
/// - [`Error::NotAuthenticated`] - No GitHub token available
/// - [`Error::RefreshFailed`] - Token exchange failed
///
/// # Example
///
/// ```no_run
/// use crate::oauth::copilot::auth::refresh::ensure_valid_copilot_token;
/// use crate::oauth::copilot::auth::token_exchange::TokenExchangeConfig;
/// use crate::oauth::copilot::models::TokenInfo;
///
/// # async fn example() -> crate::oauth::copilot::Result<()> {
/// let client = reqwest::Client::new();
/// let config = TokenExchangeConfig::default();
/// let mut token_info = TokenInfo::new("gho_xxxxx");
///
/// ensure_valid_copilot_token(&client, &mut token_info, &config).await?;
///
/// // Now token_info.copilot_token is guaranteed to be Some
/// let copilot_token = token_info.copilot_token.as_ref().unwrap();
/// # Ok(())
/// # }
/// ```
pub async fn ensure_valid_copilot_token(
    client: &reqwest::Client,
    token_info: &mut TokenInfo,
    config: &TokenExchangeConfig,
) -> Result<()> {
    // Validate GitHub token exists
    if token_info.github_token.is_empty() {
        warn!("No GitHub token available for token exchange");
        return Err(Error::NotAuthenticated);
    }

    // Refresh if needed
    match refresh_copilot_token_if_needed(client, token_info, config).await {
        Ok(_) => {
            // Verify we have a token now
            if token_info.copilot_token.is_none() {
                return Err(Error::RefreshFailed(
                    "Token exchange succeeded but no token received".to_string(),
                ));
            }
            Ok(())
        }
        Err(e) => {
            warn!(error = %e, "Failed to ensure valid Copilot token");
            Err(Error::RefreshFailed(format!("Token refresh failed: {}", e)))
        }
    }
}

/// Returns the remaining validity duration of the Copilot token.
///
/// Returns `None` if:
/// - There is no Copilot token
/// - The token has already expired
/// - The expiration time is not set
///
/// This can be used for logging or debugging token state.
#[must_use]
pub fn token_remaining_validity(token_info: &TokenInfo) -> Option<std::time::Duration> {
    let expires_at = token_info.copilot_expires_at?;
    let now = chrono::Utc::now().timestamp();
    let remaining = expires_at - now;

    if remaining > 0 {
        Some(std::time::Duration::from_secs(remaining as u64))
    } else {
        None
    }
}

/// Returns seconds until the Copilot token should be refreshed.
///
/// This accounts for the buffer period, returning 0 if refresh should
/// happen now.
#[must_use]
pub fn seconds_until_refresh(token_info: &TokenInfo) -> i64 {
    match token_info.copilot_expires_at {
        Some(expires_at) => {
            let now = chrono::Utc::now().timestamp();
            let refresh_at = expires_at - REFRESH_BUFFER_SECS;
            (refresh_at - now).max(0)
        }
        None => 0, // No token, refresh immediately
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_needs_refresh_no_token() {
        let token_info = TokenInfo::new("gho_test");
        assert!(needs_refresh(&token_info));
    }

    #[test]
    fn test_needs_refresh_expired() {
        let token_info = TokenInfo::with_copilot("gho_test", "cop_test", 0);
        assert!(needs_refresh(&token_info));
    }

    #[test]
    fn test_needs_refresh_within_buffer() {
        let expires_at = Utc::now().timestamp() + 30; // 30 seconds from now
        let token_info = TokenInfo::with_copilot("gho_test", "cop_test", expires_at);
        assert!(needs_refresh(&token_info)); // Within 60-second buffer
    }

    #[test]
    fn test_needs_refresh_still_valid() {
        let expires_at = Utc::now().timestamp() + 3600; // 1 hour from now
        let token_info = TokenInfo::with_copilot("gho_test", "cop_test", expires_at);
        assert!(!needs_refresh(&token_info));
    }

    #[test]
    fn test_token_remaining_validity() {
        // No token
        let no_token = TokenInfo::new("gho_test");
        assert!(token_remaining_validity(&no_token).is_none());

        // Expired token
        let expired = TokenInfo::with_copilot("gho_test", "cop_test", 0);
        assert!(token_remaining_validity(&expired).is_none());

        // Valid token
        let expires_at = Utc::now().timestamp() + 3600;
        let valid = TokenInfo::with_copilot("gho_test", "cop_test", expires_at);
        let remaining = token_remaining_validity(&valid);
        assert!(remaining.is_some());
        assert!(remaining.unwrap().as_secs() > 3500); // At least 3500 seconds
    }

    #[test]
    fn test_seconds_until_refresh() {
        // No token - refresh now
        let no_token = TokenInfo::new("gho_test");
        assert_eq!(seconds_until_refresh(&no_token), 0);

        // Token expiring in 30 seconds - within buffer, refresh now
        let expires_at = Utc::now().timestamp() + 30;
        let expiring_soon = TokenInfo::with_copilot("gho_test", "cop_test", expires_at);
        assert_eq!(seconds_until_refresh(&expiring_soon), 0);

        // Token valid for 2 hours - refresh in ~2 hours minus buffer
        let expires_at = Utc::now().timestamp() + 7200;
        let valid = TokenInfo::with_copilot("gho_test", "cop_test", expires_at);
        let until_refresh = seconds_until_refresh(&valid);
        // Should be approximately 7200 - 60 = 7140 seconds
        assert!(until_refresh > 7000);
        assert!(until_refresh < 7200);
    }

    #[tokio::test]
    async fn test_ensure_valid_copilot_token_no_github_token() {
        let client = reqwest::Client::new();
        let config = TokenExchangeConfig::default();
        let mut token_info = TokenInfo {
            token_type: "github".to_string(),
            github_token: String::new(), // Empty GitHub token
            copilot_token: None,
            copilot_expires_at: None,
        };

        let result = ensure_valid_copilot_token(&client, &mut token_info, &config).await;
        assert!(matches!(result, Err(Error::NotAuthenticated)));
    }
}
