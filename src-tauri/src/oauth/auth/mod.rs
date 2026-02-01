//! OAuth 2.0 authentication module with PKCE support.
//!
//! This module provides a unified OAuth implementation for multiple providers
//! (Claude/Anthropic, Gemini/Google) with the following features:
//!
//! - **PKCE Support**: Secure authorization code flow with S256 challenge
//! - **Provider Configuration**: Pre-configured settings for Claude and Gemini
//! - **Token Management**: Automatic refresh of expired tokens
//! - **State Validation**: CSRF protection via state parameter
//! - **Composite Tokens**: Project IDs embedded in refresh tokens
//!
//! # Architecture
//!
//! ```text
//! +----------------+     +----------------+     +----------------+
//! |   OAuthFlow    | --> |  OAuthConfig   | --> |  TokenStorage  |
//! +----------------+     +----------------+     +----------------+
//!        |                      |
//!        v                      v
//! +----------------+     +----------------+
//! | OAuthFlowState | --> |     Pkce       |
//! +----------------+     +----------------+
//! ```
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use gate::auth::{OAuthFlow, OAuthConfig};
//! use gate::storage::MemoryTokenStorage;
//!
//! # async fn example() -> gate::Result<()> {
//! // Create a flow for Claude
//! let storage = MemoryTokenStorage::new();
//! let flow = OAuthFlow::new(storage, OAuthConfig::claude(), "anthropic");
//!
//! // Start authorization
//! let (auth_url, state) = flow.start_authorization_async().await?;
//! println!("Open: {}", auth_url);
//!
//! // After user authorizes, exchange the code
//! // let token = flow.exchange_code(&code, Some(&state.state)).await?;
//!
//! // Get access token (auto-refreshes if needed)
//! let access_token = flow.get_access_token().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Provider Configuration
//!
//! Pre-configured settings are available for supported providers:
//!
//! ```rust,ignore
//! use gate::auth::OAuthConfig;
//!
//! // Claude (Anthropic)
//! let claude_config = OAuthConfig::claude();
//! assert!(claude_config.client_secret.is_none()); // PKCE-only
//!
//! // Gemini (Google Cloud Code)
//! let gemini_config = OAuthConfig::gemini();
//! assert!(gemini_config.client_secret.is_some()); // Requires secret
//! ```
//!
//! # Custom Configuration
//!
//! You can also build custom configurations:
//!
//! ```rust,ignore
//! use gate::auth::OAuthConfig;
//!
//! let config = OAuthConfig::builder()
//!     .client_id("my-client-id")
//!     .client_secret("my-secret")
//!     .auth_url("https://example.com/oauth/authorize")
//!     .token_url("https://example.com/oauth/token")
//!     .redirect_uri("http://localhost:8080/callback")
//!     .scopes(vec!["openid", "profile"])
//!     .callback_port(8080)
//!     .build();
//! ```
//!
//! # Security Considerations
//!
//! - **PKCE**: Always use PKCE for public clients. Never expose client secrets in
//!   native applications unless absolutely required by the provider.
//! - **State Validation**: Always validate the state parameter in callbacks to
//!   prevent CSRF attacks.
//! - **Token Storage**: Use secure storage (keyring, encrypted files) in production.
//! - **Logging**: Never log access or refresh tokens, even at debug level.

pub mod config;
pub mod flow;
pub mod pkce;
pub mod state;

// Re-export main types at the auth level
pub use config::{OAuthConfig, OAuthConfigBuilder};
pub use flow::OAuthFlow;
pub use pkce::Pkce;
pub use state::{generate_state, OAuthFlowState};
