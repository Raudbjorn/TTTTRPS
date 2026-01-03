//! Output types for parsing Claude Code JSON responses.

use serde::{Deserialize, Serialize};

/// A complete response from Claude Code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeResponse {
    /// The session ID for this conversation.
    #[serde(default)]
    pub session_id: Option<String>,

    /// The assistant's text response.
    #[serde(default)]
    pub result: String,

    /// Cost information for the request.
    #[serde(default)]
    pub cost: Option<CostInfo>,

    /// Token usage information.
    #[serde(default)]
    pub usage: Option<Usage>,

    /// Whether the response is complete.
    #[serde(default)]
    pub is_complete: bool,

    /// Tool uses that occurred during the response.
    #[serde(default)]
    pub tool_uses: Vec<ToolUse>,

    /// Any errors that occurred.
    #[serde(default)]
    pub error: Option<String>,
}

impl ClaudeResponse {
    /// Check if the response contains an error.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the text content of the response.
    pub fn text(&self) -> &str {
        &self.result
    }

    /// Get the session ID if available.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

/// Cost information for a Claude Code request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostInfo {
    /// Cost in USD.
    #[serde(default)]
    pub usd: f64,

    /// Input tokens cost.
    #[serde(default)]
    pub input_cost: f64,

    /// Output tokens cost.
    #[serde(default)]
    pub output_cost: f64,
}

/// Token usage information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of input tokens.
    #[serde(default)]
    pub input_tokens: u32,

    /// Number of output tokens.
    #[serde(default)]
    pub output_tokens: u32,

    /// Total tokens.
    #[serde(default)]
    pub total_tokens: u32,

    /// Cache creation tokens (if applicable).
    #[serde(default)]
    pub cache_creation_tokens: Option<u32>,

    /// Cache read tokens (if applicable).
    #[serde(default)]
    pub cache_read_tokens: Option<u32>,
}

/// A tool use event from Claude Code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUse {
    /// The tool name.
    pub name: String,

    /// The tool input (as JSON).
    #[serde(default)]
    pub input: serde_json::Value,

    /// The tool result (if available).
    #[serde(default)]
    pub result: Option<String>,

    /// Whether the tool call was approved.
    #[serde(default)]
    pub approved: bool,

    /// Error from the tool (if any).
    #[serde(default)]
    pub error: Option<String>,
}

/// A streaming message chunk from Claude Code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamChunk {
    /// Initial message with session info.
    #[serde(rename = "init")]
    Init {
        session_id: String,
        #[serde(default)]
        model: Option<String>,
    },

    /// Text content chunk.
    #[serde(rename = "text")]
    Text {
        content: String,
    },

    /// Tool use started.
    #[serde(rename = "tool_use_start")]
    ToolUseStart {
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },

    /// Tool use completed.
    #[serde(rename = "tool_use_end")]
    ToolUseEnd {
        name: String,
        #[serde(default)]
        result: Option<String>,
        #[serde(default)]
        error: Option<String>,
    },

    /// Response completed.
    #[serde(rename = "complete")]
    Complete {
        #[serde(default)]
        result: String,
        #[serde(default)]
        usage: Option<Usage>,
        #[serde(default)]
        cost: Option<CostInfo>,
    },

    /// Error occurred.
    #[serde(rename = "error")]
    Error {
        message: String,
    },
}

/// Parse a JSON response from Claude Code.
pub fn parse_response(output: &str) -> Result<ClaudeResponse, crate::error::ClaudeCodeError> {
    // Try to parse as a complete JSON response
    if let Ok(response) = serde_json::from_str::<ClaudeResponse>(output) {
        return Ok(response);
    }

    // Try to parse stream-json format (newline-delimited JSON)
    let mut result = String::new();
    let mut session_id = None;
    let mut usage = None;
    let mut cost = None;
    let mut tool_uses = Vec::new();
    let mut error = None;

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(line) {
            match chunk {
                StreamChunk::Init { session_id: sid, .. } => {
                    session_id = Some(sid);
                }
                StreamChunk::Text { content } => {
                    result.push_str(&content);
                }
                StreamChunk::ToolUseStart { name, input } => {
                    tool_uses.push(ToolUse {
                        name,
                        input,
                        result: None,
                        approved: true,
                        error: None,
                    });
                }
                StreamChunk::ToolUseEnd { name, result: res, error: err } => {
                    if let Some(tool) = tool_uses.iter_mut().rev().find(|t| t.name == name) {
                        tool.result = res;
                        tool.error = err;
                    }
                }
                StreamChunk::Complete { result: r, usage: u, cost: c } => {
                    if !r.is_empty() {
                        result = r;
                    }
                    usage = u;
                    cost = c;
                }
                StreamChunk::Error { message } => {
                    error = Some(message);
                }
            }
        }
    }

    // If we got nothing, return the raw output as text
    if result.is_empty() && error.is_none() {
        result = output.to_string();
    }

    Ok(ClaudeResponse {
        session_id,
        result,
        cost,
        usage,
        is_complete: true,
        tool_uses,
        error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_json() {
        let json = r#"{"result": "Hello, world!", "is_complete": true}"#;
        let response = parse_response(json).unwrap();
        assert_eq!(response.result, "Hello, world!");
        assert!(response.is_complete);
    }

    #[test]
    fn test_parse_stream_json() {
        let stream = r#"{"type": "init", "session_id": "abc123"}
{"type": "text", "content": "Hello"}
{"type": "text", "content": " world!"}
{"type": "complete", "result": "Hello world!"}"#;

        let response = parse_response(stream).unwrap();
        assert_eq!(response.session_id, Some("abc123".to_string()));
        assert_eq!(response.result, "Hello world!");
    }
}
