//! # Gemini CLI Rust Bridge
//!
//! A Rust library for programmatic interaction with Google's Gemini CLI.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use gemini_cli_rs::{GeminiCliClient, GeminiCliClientBuilder};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Simple usage
//!     let client = GeminiCliClient::new()?;
//!     let response = client.prompt("What is 2 + 2?").await?;
//!     println!("Response: {}", response.text());
//!
//!     // With builder pattern
//!     let client = GeminiCliClientBuilder::new()
//!         .timeout_secs(120)
//!         .model("gemini-2.5-flash")
//!         .build()?;
//!
//!     let response = client.prompt("Explain monads").await?;
//!     println!("{}", response.text());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Working Directory Context
//!
//! Gemini CLI uses the working directory for file operations and context:
//!
//! ```rust,no_run
//! use gemini_cli_rs::GeminiCliClientBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GeminiCliClientBuilder::new()
//!     .working_dir("/path/to/project")
//!     .build()?;
//!
//! // Gemini CLI will have context of this directory
//! let response = client.prompt("List the source files in this project").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## YOLO Mode
//!
//! Enable auto-approval of all tool actions (use with caution!):
//!
//! ```rust,no_run
//! use gemini_cli_rs::GeminiCliClientBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GeminiCliClientBuilder::new()
//!     .yolo_mode()
//!     .working_dir("/trusted/project")
//!     .build()?;
//!
//! // All file writes and shell commands will be auto-approved
//! let response = client.prompt("Create a hello world Python script").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Piping Input
//!
//! Send file contents or other input via stdin:
//!
//! ```rust,no_run
//! use gemini_cli_rs::GeminiCliClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GeminiCliClient::new()?;
//! let code = std::fs::read_to_string("src/main.rs")?;
//!
//! let response = client.prompt_with_stdin(
//!     "Review this code for bugs",
//!     &code
//! ).await?;
//!
//! println!("{}", response.text());
//! # Ok(())
//! # }
//! ```

mod client;
mod config;
mod error;
mod output;

pub use client::{GeminiCliClient, GeminiCliClientBuilder};
pub use config::{GeminiCliConfig, OutputFormat};
pub use error::{GeminiCliError, Result};
pub use output::{
    GeminiError, GeminiResponse, Stats, TokenStats, ToolStats, StreamEvent,
};

/// Prelude for convenient imports.
pub mod prelude {
    pub use crate::client::{GeminiCliClient, GeminiCliClientBuilder};
    pub use crate::config::{GeminiCliConfig, OutputFormat};
    pub use crate::error::{GeminiCliError, Result};
    pub use crate::output::{GeminiResponse, StreamEvent};
}
