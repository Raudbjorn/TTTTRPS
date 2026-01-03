//! Claude Desktop CDP Bridge
//!
//! Enables communication with Claude Desktop via Chrome DevTools Protocol (CDP).
//! This provides an alternative to API-based Claude access that uses the existing
//! Claude Desktop authentication.
//!
//! ## Usage
//!
//! First, start Claude Desktop with remote debugging enabled:
//!
//! ```bash
//! claude --remote-debugging-port=9333
//! ```
//!
//! Or use the manager to auto-launch:
//!
//! ```rust,no_run
//! use crate::core::claude_cdp::ClaudeDesktopManager;
//!
//! let manager = ClaudeDesktopManager::new();
//! manager.connect_or_launch().await?;
//! let response = manager.send_message("Hello!").await?;
//! ```
//!
//! ## Limitations
//!
//! - No streaming (CDP gives full responses only)
//! - No token counting (subscription model)
//! - Depends on UI selectors that may break with updates
//! - Slower than direct API access

mod client;
mod config;
mod error;
mod manager;

pub use client::{ClaudeClient, ConnectionState, Message};
pub use config::{ClaudeConfig, DEFAULT_CDP_PORT, DEFAULT_TIMEOUT_SECS, CLAUDE_BINARY_PATHS};
pub use error::{ClaudeCdpError, Result};
pub use manager::{ClaudeDesktopManager, ClaudeDesktopStatus};
