//! # Claude Gate
//!
//! OAuth-based Anthropic API client for Rust with flexible token storage.
//!
//! This module provides:
//! - OAuth 2.0 PKCE flow for Anthropic's Claude API
//! - Flexible callback-based token storage (lazy loading)
//! - Direct programmatic API access (no HTTP server/IPC)
//! - Automatic token refresh
//! - Streaming support for real-time responses
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use crate::claude_gate::{ClaudeClient, FileTokenStorage};
//!
//! #[tokio::main]
//! async fn main() -> crate::claude_gate::Result<()> {
//!     // Use default storage (~/.config/cld/auth.json)
//!     let storage = FileTokenStorage::default_path()?;
//!     let client = ClaudeClient::builder()
//!         .with_storage(storage)
//!         .build()?;
//!
//!     // Check authentication and make a request
//!     if client.is_authenticated().await? {
//!         let response = client.messages()
//!             .model("claude-sonnet-4-20250514")
//!             .max_tokens(1024)
//!             .user_message("Hello, Claude!")
//!             .send()
//!             .await?;
//!         println!("{}", response.text());
//!     }
//!
//!     Ok(())
//! }
//! ```

#![allow(clippy::module_name_repetitions)]

pub mod auth;
pub mod client;
pub mod error;
pub mod models;
pub mod storage;
pub mod transform;

pub use auth::{OAuthConfig, OAuthFlow, OAuthFlowState, Pkce};
pub use client::{ClaudeClient, ClaudeClientBuilder, MessagesRequest, MessagesRequestBuilder};
pub use error::{Error, Result};
pub use models::{
    ApiModel, ContentBlock, DocumentSource, ImageSource, Message, MessagesResponse, ModelsResponse,
    Role, StopReason, StreamEvent, Tool, ToolChoice, TokenInfo, Usage,
};
pub use storage::{callbacks, CallbackStorage, FileTokenStorage, MemoryTokenStorage, TokenStorage};

#[cfg(feature = "keyring")]
pub use storage::KeyringTokenStorage;
