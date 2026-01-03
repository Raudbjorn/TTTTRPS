//! Error types for Gemini CLI interactions.

use std::process::ExitStatus;
use thiserror::Error;

/// Errors that can occur when interacting with Gemini CLI.
#[derive(Error, Debug)]
pub enum GeminiCliError {
    /// Gemini CLI binary not found.
    #[error("Gemini CLI not found. Install with: npm install -g @google/gemini-cli")]
    NotFound,

    /// Failed to spawn the Gemini CLI process.
    #[error("failed to spawn Gemini CLI process: {0}")]
    SpawnFailed(#[source] std::io::Error),

    /// Process exited with non-zero status.
    #[error("Gemini CLI exited with {status}: {stderr}")]
    ProcessFailed {
        status: ExitStatus,
        stdout: String,
        stderr: String,
    },

    /// Failed to parse JSON output.
    #[error("failed to parse Gemini CLI output: {message}")]
    ParseError {
        message: String,
        raw_output: String,
    },

    /// Timeout waiting for response.
    #[error("timeout after {seconds}s waiting for Gemini CLI response")]
    Timeout { seconds: u64 },

    /// Authentication error.
    #[error("authentication failed: {message}. Run 'gemini' interactively to authenticate.")]
    AuthenticationError { message: String },

    /// Gemini CLI returned an error response.
    #[error("Gemini error ({error_type}): {message}")]
    GeminiError { error_type: String, message: String },

    /// Rate limit exceeded.
    #[error("rate limit exceeded: {message}")]
    RateLimitError { message: String },

    /// I/O error during communication.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Working directory does not exist or is not accessible.
    #[error("working directory not accessible: {path}")]
    WorkingDirNotAccessible { path: String },

    /// Tool execution failed.
    #[error("tool execution failed: {tool_name} - {message}")]
    ToolError { tool_name: String, message: String },
}

/// Result type alias for Gemini CLI operations.
pub type Result<T> = std::result::Result<T, GeminiCliError>;
