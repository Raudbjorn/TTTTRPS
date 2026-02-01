//! Authentication module for GitHub Copilot API.
//!
//! This module implements the OAuth 2.0 Device Authorization Grant (RFC 8628)
//! for authenticating with GitHub, followed by token exchange to obtain a
//! Copilot API token.
//!
//! ## Authentication Flow
//!
//! 1. **Device Flow Initiation**: Request a device code from GitHub
//! 2. **User Authorization**: User visits a URL and enters a code
//! 3. **Polling**: Poll GitHub until the user completes authorization
//! 4. **Token Exchange**: Exchange the GitHub token for a Copilot token
//! 5. **Token Refresh**: Automatically refresh the Copilot token before expiry
//!
//! ## Example
//!
//! ```no_run
//! use crate::oauth::copilot::auth::device_flow::{start_device_flow, poll_until_complete};
//! use crate::oauth::copilot::auth::token_exchange::{exchange_for_copilot_token, TokenExchangeConfig};
//!
//! # async fn example() -> crate::oauth::copilot::Result<()> {
//! let client = reqwest::Client::new();
//!
//! // Start device flow
//! let pending = start_device_flow(&client).await?;
//! println!("Visit: {} and enter: {}", pending.verification_uri, pending.user_code);
//!
//! // Wait for user authorization
//! let github_token = poll_until_complete(&client, &pending, None).await?;
//!
//! // Exchange for Copilot token
//! let config = TokenExchangeConfig::default();
//! let copilot_response = exchange_for_copilot_token(&client, &github_token, &config).await?;
//!
//! println!("Authenticated! Token expires: {}", copilot_response.expires_at);
//! # Ok(())
//! # }
//! ```
//!
//! ## Token Lifecycle
//!
//! - **GitHub Token**: Long-lived (typically 8 hours) OAuth token
//! - **Copilot Token**: Short-lived (~30 minutes) derived from GitHub token
//!
//! The Copilot token should be refreshed proactively before expiration.
//! Use [`refresh::ensure_valid_copilot_token`] to handle this automatically.

pub mod constants;
pub mod device_flow;
pub mod refresh;
pub mod token_exchange;

// Re-export commonly used types
pub use device_flow::{mask_token, start_device_flow, DeviceFlowPending, PollResult};
pub use refresh::{ensure_valid_copilot_token, refresh_copilot_token_if_needed};
pub use token_exchange::{exchange_for_copilot_token, TokenExchangeConfig};
