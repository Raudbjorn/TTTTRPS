//! Output types for parsing Gemini CLI JSON responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing;

/// A complete response from Gemini CLI.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeminiResponse {
    /// The main AI-generated response text.
    #[serde(default)]
    pub response: Option<String>,

    /// Usage and performance statistics.
    #[serde(default)]
    pub stats: Option<Stats>,

    /// Error information if the request failed.
    #[serde(default)]
    pub error: Option<GeminiError>,
}

impl GeminiResponse {
    /// Check if the response contains an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the text content of the response.
    pub fn text(&self) -> &str {
        self.response.as_deref().unwrap_or("")
    }

    /// Check if the response is successful.
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.response.is_some()
    }
}

/// Error information from Gemini CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiError {
    /// Error type (e.g., "FatalAuthenticationError").
    #[serde(rename = "type", default)]
    pub error_type: String,

    /// Human-readable error message.
    #[serde(default)]
    pub message: String,
}

/// Statistics from a Gemini CLI session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stats {
    /// Session-wide statistics.
    #[serde(default)]
    pub session: Option<SessionStats>,

    /// Per-model statistics.
    #[serde(default)]
    pub models: Option<HashMap<String, ModelStats>>,

    /// Tool usage statistics.
    #[serde(default)]
    pub tools: Option<ToolStats>,

    /// File operation statistics.
    #[serde(default)]
    pub files: Option<FileStats>,
}

/// Session-level statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionStats {
    /// Total duration in milliseconds.
    #[serde(default)]
    pub duration: Option<u64>,
}

/// Per-model statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelStats {
    /// API call statistics.
    #[serde(default)]
    pub api: Option<ApiStats>,

    /// Token usage statistics.
    #[serde(default)]
    pub tokens: Option<TokenStats>,
}

/// API call statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ApiStats {
    /// Total number of API requests.
    #[serde(default)]
    pub total_requests: u32,

    /// Total number of errors.
    #[serde(default)]
    pub total_errors: u32,

    /// Total latency in milliseconds.
    #[serde(default)]
    pub total_latency_ms: u64,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenStats {
    /// Prompt tokens.
    #[serde(default)]
    pub prompt: u32,

    /// Candidate/response tokens.
    #[serde(default)]
    pub candidates: u32,

    /// Total tokens.
    #[serde(default)]
    pub total: u32,

    /// Cached tokens (context caching).
    #[serde(default)]
    pub cached: u32,

    /// Thinking/reasoning tokens.
    #[serde(default)]
    pub thoughts: u32,

    /// Tool-related tokens.
    #[serde(default)]
    pub tool: u32,
}

/// Tool usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolStats {
    /// Total tool calls.
    #[serde(default)]
    pub total_calls: u32,

    /// Successful tool calls.
    #[serde(default)]
    pub total_success: u32,

    /// Failed tool calls.
    #[serde(default)]
    pub total_fail: u32,

    /// Total duration of tool calls in milliseconds.
    #[serde(default)]
    pub total_duration_ms: u64,

    /// Decision statistics.
    #[serde(default)]
    pub total_decisions: Option<DecisionStats>,

    /// Per-tool statistics.
    #[serde(default)]
    pub by_name: Option<HashMap<String, ToolCallStats>>,
}

/// Decision statistics for tool approval.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecisionStats {
    /// Accepted by user.
    #[serde(default)]
    pub accept: u32,

    /// Rejected by user.
    #[serde(default)]
    pub reject: u32,

    /// Modified by user.
    #[serde(default)]
    pub modify: u32,

    /// Auto-accepted (YOLO mode or safe tools).
    #[serde(default)]
    pub auto_accept: u32,
}

/// Per-tool call statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallStats {
    /// Number of calls to this tool.
    #[serde(default)]
    pub count: u32,

    /// Successful calls.
    #[serde(default)]
    pub success: u32,

    /// Failed calls.
    #[serde(default)]
    pub fail: u32,

    /// Total duration in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,

    /// Decision statistics for this tool.
    #[serde(default)]
    pub decisions: Option<DecisionStats>,
}

/// File operation statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileStats {
    /// Total lines added.
    #[serde(default)]
    pub total_lines_added: u32,

    /// Total lines removed.
    #[serde(default)]
    pub total_lines_removed: u32,
}

/// A streaming event from Gemini CLI (stream-json format).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    /// Session initialization.
    #[serde(rename = "init")]
    Init {
        timestamp: String,
        session_id: String,
        model: String,
    },

    /// User or assistant message.
    #[serde(rename = "message")]
    Message {
        role: String,
        content: String,
        timestamp: String,
        #[serde(default)]
        delta: bool,
    },

    /// Tool use request.
    #[serde(rename = "tool_use")]
    ToolUse {
        tool_name: String,
        tool_id: String,
        #[serde(default)]
        parameters: serde_json::Value,
        timestamp: String,
    },

    /// Tool execution result.
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_id: String,
        status: String,
        #[serde(default)]
        output: String,
        timestamp: String,
    },

    /// Final result with statistics.
    #[serde(rename = "result")]
    Result {
        status: String,
        #[serde(default)]
        stats: Option<Stats>,
        timestamp: String,
    },

    /// Error event.
    #[serde(rename = "error")]
    Error {
        message: String,
        #[serde(default)]
        error_type: Option<String>,
    },
}

/// Parse a JSON response from Gemini CLI.
pub fn parse_response(output: &str) -> Result<GeminiResponse, crate::error::GeminiCliError> {
    let trimmed = output.trim();

    // Try to parse as a complete JSON response
    if let Ok(response) = serde_json::from_str::<GeminiResponse>(trimmed) {
        return Ok(response);
    }

    // Try to parse stream-json format (newline-delimited JSON)
    let mut response_text = String::new();
    let mut stats = None;
    let mut error = None;

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(event) = serde_json::from_str::<StreamEvent>(line) {
            match event {
                StreamEvent::Message { content, role, delta, .. } => {
                    if role == "assistant" {
                        if delta {
                            response_text.push_str(&content);
                        } else {
                            response_text = content;
                        }
                    }
                }
                StreamEvent::Result { stats: s, .. } => {
                    stats = s;
                }
                StreamEvent::Error { message, error_type } => {
                    error = Some(GeminiError {
                        error_type: error_type.unwrap_or_else(|| "Error".to_string()),
                        message,
                    });
                }
                _ => {}
            }
        } else {
            // Log unparseable lines - may contain diagnostic info or stderr mixed with stdout
            tracing::warn!("Failed to parse stream event line: {}", line);
        }
    }

    // If we parsed stream events, return the accumulated response
    if !response_text.is_empty() || stats.is_some() || error.is_some() {
        return Ok(GeminiResponse {
            response: if response_text.is_empty() {
                None
            } else {
                Some(response_text)
            },
            stats,
            error,
        });
    }

    // If not JSON, treat as plain text response
    Ok(GeminiResponse {
        response: Some(output.to_string()),
        stats: None,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_response() {
        let json = r#"{
            "response": "The capital of France is Paris.",
            "stats": {
                "tools": {
                    "totalCalls": 1,
                    "totalSuccess": 1
                }
            }
        }"#;

        let response = parse_response(json).unwrap();
        assert_eq!(response.text(), "The capital of France is Paris.");
        assert!(response.is_success());
    }

    #[test]
    fn test_parse_error_response() {
        let json = r#"{
            "response": null,
            "error": {
                "type": "FatalAuthenticationError",
                "message": "Authentication failed"
            }
        }"#;

        let response = parse_response(json).unwrap();
        assert!(response.is_error());
        assert_eq!(response.error.unwrap().error_type, "FatalAuthenticationError");
    }

    #[test]
    fn test_parse_stream_json() {
        let stream = r#"{"type":"init","timestamp":"2025-01-01T00:00:00Z","session_id":"abc","model":"gemini-2.5-pro"}
{"type":"message","role":"assistant","content":"Hello","timestamp":"2025-01-01T00:00:01Z","delta":false}
{"type":"result","status":"success","timestamp":"2025-01-01T00:00:02Z"}"#;

        let response = parse_response(stream).unwrap();
        assert_eq!(response.text(), "Hello");
    }
}
