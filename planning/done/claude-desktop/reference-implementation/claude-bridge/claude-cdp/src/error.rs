//! Error types for the Claude CDP bridge.

use thiserror::Error;

/// Errors that can occur when communicating with Claude Desktop via CDP.
#[derive(Error, Debug)]
pub enum ClaudeCdpError {
    /// Failed to connect to Claude Desktop.
    #[error("failed to connect to Claude Desktop at {url}: {source}")]
    ConnectionFailed {
        url: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Claude Desktop is not running or CDP is not enabled.
    #[error("Claude Desktop not reachable. Ensure it's running with --remote-debugging-port={port}")]
    NotReachable { port: u16 },

    /// No Claude page/tab found in the browser.
    #[error("no Claude conversation page found in browser contexts")]
    NoClaudePageFound,

    /// Failed to find the message input element.
    #[error("could not locate message input element: {details}")]
    InputElementNotFound { details: String },

    /// Failed to send a message.
    #[error("failed to send message: {reason}")]
    SendFailed { reason: String },

    /// Timeout waiting for Claude's response.
    #[error("timeout after {seconds}s waiting for Claude's response")]
    ResponseTimeout { seconds: u64 },

    /// Failed to extract response text.
    #[error("failed to extract response from page: {details}")]
    ResponseExtractionFailed { details: String },

    /// JavaScript execution failed.
    #[error("JavaScript execution failed: {script_hint} - {error}")]
    JsExecutionFailed { script_hint: String, error: String },

    /// The connection was closed unexpectedly.
    #[error("CDP connection closed unexpectedly")]
    ConnectionClosed,

    /// Generic CDP protocol error.
    #[error("CDP protocol error: {0}")]
    ProtocolError(String),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type alias for Claude CDP operations.
pub type Result<T> = std::result::Result<T, ClaudeCdpError>;
