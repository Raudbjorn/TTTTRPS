//! Error types for Claude Code interactions.

use std::process::ExitStatus;
use thiserror::Error;

/// Errors that can occur when interacting with Claude Code.
#[derive(Error, Debug)]
pub enum ClaudeCodeError {
    /// Claude Code CLI binary not found.
    #[error("Claude Code CLI not found. Install with: npm install -g @anthropic-ai/claude-code")]
    NotFound,

    /// Failed to spawn the Claude Code process.
    #[error("failed to spawn Claude Code process: {0}")]
    SpawnFailed(#[source] std::io::Error),

    /// Process exited with non-zero status.
    #[error("Claude Code exited with {status}: {stderr}")]
    ProcessFailed {
        status: ExitStatus,
        stdout: String,
        stderr: String,
    },

    /// Failed to parse JSON output.
    #[error("failed to parse Claude Code output: {message}")]
    ParseError {
        message: String,
        raw_output: String,
    },

    /// Timeout waiting for response.
    #[error("timeout after {seconds}s waiting for Claude Code response")]
    Timeout { seconds: u64 },

    /// Invalid session ID format.
    #[error("invalid session ID: {0}")]
    InvalidSessionId(String),

    /// Session not found or expired.
    #[error("session not found: {session_id}")]
    SessionNotFound { session_id: String },

    /// Claude Code returned an error response.
    #[error("Claude Code error: {0}")]
    ClaudeError(String),

    /// I/O error during communication.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Working directory does not exist or is not accessible.
    #[error("working directory not accessible: {path}")]
    WorkingDirNotAccessible { path: String },
}

/// Result type alias for Claude Code operations.
pub type Result<T> = std::result::Result<T, ClaudeCodeError>;
