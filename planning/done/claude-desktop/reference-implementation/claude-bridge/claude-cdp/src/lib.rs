//! # Claude CDP Bridge
//!
//! A Rust library for communicating with Claude Desktop via Chrome DevTools Protocol (CDP).
//!
//! This enables programmatic interaction with Claude Desktop from processes that
//! cannot or prefer not to handle API authentication directly.
//!
//! ## Usage
//!
//! First, start Claude Desktop with remote debugging enabled:
//!
//! ```bash
//! # Linux
//! claude-desktop --remote-debugging-port=9222
//!
//! # Or find your binary
//! /opt/claude-desktop/claude --remote-debugging-port=9222
//! ```
//!
//! Then connect and send messages:
//!
//! ```rust,no_run
//! use claude_cdp::{ClaudeClient, ClaudeConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut client = ClaudeClient::new();
//!     client.connect().await?;
//!
//!     let response = client.send_message("What is 2 + 2?").await?;
//!     println!("Claude: {}", response);
//!
//!     client.disconnect().await;
//!     Ok(())
//! }
//! ```
//!
//! ## Custom Configuration
//!
//! ```rust
//! use claude_cdp::ClaudeConfig;
//!
//! let config = ClaudeConfig::default()
//!     .with_port(9333)
//!     .with_timeout(60);
//! ```

mod client;
mod error;

pub use client::{ClaudeClient, ClaudeConfig, ConnectionState, Message};
pub use error::{ClaudeCdpError, Result};

/// Re-export for convenience.
pub mod prelude {
    pub use crate::client::{ClaudeClient, ClaudeConfig, ConnectionState, Message};
    pub use crate::error::{ClaudeCdpError, Result};
}
