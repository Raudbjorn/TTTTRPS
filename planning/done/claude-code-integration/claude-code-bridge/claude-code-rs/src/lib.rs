//! # Claude Code Rust Bridge
//!
//! A Rust library for programmatic interaction with Claude Code CLI.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use claude_code_rs::{ClaudeCodeClient, ClaudeCodeClientBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Simple usage
//!     let client = ClaudeCodeClient::new()?;
//!     let response = client.prompt("What is 2 + 2?").await?;
//!     println!("Response: {}", response.text());
//!
//!     // With builder pattern
//!     let client = ClaudeCodeClientBuilder::new()
//!         .timeout_secs(120)
//!         .model("claude-sonnet-4-20250514")
//!         .build()?;
//!
//!     let response = client.prompt("Explain monads").await?;
//!     println!("{}", response.text());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Conversation Management
//!
//! ```rust,no_run
//! use claude_code_rs::ClaudeCodeClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ClaudeCodeClient::new()?;
//!
//! // First message
//! let response = client.prompt("Let's discuss Rust ownership").await?;
//! let session_id = response.session_id().unwrap();
//!
//! // Continue the conversation
//! let response = client.continue_conversation("Give me an example").await?;
//!
//! // Resume a specific session later
//! let response = client.resume("What were we discussing?", session_id).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Working Directory Context
//!
//! Claude Code uses the working directory for file operations and context:
//!
//! ```rust,no_run
//! use claude_code_rs::ClaudeCodeClientBuilder;
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ClaudeCodeClientBuilder::new()
//!     .working_dir("/path/to/project")
//!     .build()?;
//!
//! // Claude Code will have context of this directory
//! let response = client.prompt("List the source files in this project").await?;
//! # Ok(())
//! # }
//! ```

mod client;
mod config;
mod error;
mod output;

pub use client::{ClaudeCodeClient, ClaudeCodeClientBuilder};
pub use config::{ClaudeCodeConfig, OutputFormat, PermissionMode};
pub use error::{ClaudeCodeError, Result};
pub use output::{ClaudeResponse, CostInfo, StreamChunk, ToolUse, Usage};

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::client::{ClaudeCodeClient, ClaudeCodeClientBuilder};
    pub use crate::config::{ClaudeCodeConfig, OutputFormat, PermissionMode};
    pub use crate::error::{ClaudeCodeError, Result};
    pub use crate::output::{ClaudeResponse, ToolUse};
}
