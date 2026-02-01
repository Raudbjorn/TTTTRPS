//! Streaming response types.
//!
//! This module contains data structures for parsing Server-Sent Events (SSE)
//! streaming responses from the Copilot API.

use serde::{Deserialize, Serialize};

use crate::oauth::copilot::models::chat::Usage;

// =============================================================================
// Internal Stream Types
// =============================================================================

/// Parsed streaming event (internal representation).
///
/// This enum represents the different types of events that can be received
/// in a streaming response, normalized from the raw SSE data.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Content delta - new text content.
    Delta {
        /// The text content to append.
        content: String,
        /// The choice index (usually 0).
        index: u32,
    },

    /// Generation finished.
    FinishReason {
        /// The reason generation stopped (e.g., "stop", "length").
        reason: String,
        /// The choice index.
        index: u32,
    },

    /// Token usage statistics (sent at end).
    Usage(Usage),

    /// Stream complete signal.
    Done,
}

// =============================================================================
// Raw SSE Types
// =============================================================================

/// Raw streaming data from the API (SSE format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamData {
    /// Unique identifier for the completion.
    pub id: String,

    /// Object type (always "chat.completion.chunk").
    pub object: String,

    /// Unix timestamp of creation.
    pub created: i64,

    /// The model used.
    pub model: String,

    /// The streaming choices.
    pub choices: Vec<StreamChoice>,

    /// Token usage (only in final chunk if requested).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

impl StreamData {
    /// Converts raw stream data to internal StreamChunk representation.
    #[must_use]
    pub fn into_chunks(self) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();

        for choice in &self.choices {
            // Content delta
            if let Some(content) = &choice.delta.content {
                if !content.is_empty() {
                    chunks.push(StreamChunk::Delta {
                        content: content.clone(),
                        index: choice.index,
                    });
                }
            }

            // Finish reason
            if let Some(reason) = &choice.finish_reason {
                chunks.push(StreamChunk::FinishReason {
                    reason: reason.clone(),
                    index: choice.index,
                });
            }
        }

        // Usage
        if let Some(usage) = self.usage {
            chunks.push(StreamChunk::Usage(usage));
        }

        chunks
    }
}

/// A streaming choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChoice {
    /// The choice index.
    pub index: u32,

    /// The delta content.
    pub delta: StreamDelta,

    /// Finish reason (only in final chunk for this choice).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Delta content in a streaming response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamDelta {
    /// The role (only in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Content to append.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

// =============================================================================
// SSE Parser
// =============================================================================

/// Parses Server-Sent Events (SSE) lines.
///
/// SSE format:
/// ```text
/// data: {"id": "...", ...}
///
/// data: {"id": "...", ...}
///
/// data: [DONE]
/// ```
#[derive(Debug, Default)]
pub struct SseParser {
    /// Accumulated data for multi-line data events.
    buffer: String,
}

impl SseParser {
    /// Creates a new SSE parser.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses a single line of SSE input.
    ///
    /// Returns `Some(Vec<StreamChunk>)` when complete chunks are parsed,
    /// `None` if the line doesn't produce chunks (comment, empty, partial).
    ///
    /// # Arguments
    ///
    /// * `line` - A single line from the SSE stream
    ///
    /// # Example
    ///
    /// ```
    /// use crate::oauth::copilot::models::streaming::{SseParser, StreamChunk};
    ///
    /// let mut parser = SseParser::new();
    ///
    /// let result = parser.parse_line(r#"data: {"id":"1","object":"chat.completion.chunk","created":0,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hello"}}]}"#);
    /// assert!(result.is_some());
    /// ```
    pub fn parse_line(&mut self, line: &str) -> Option<Vec<StreamChunk>> {
        let line = line.trim();

        // Empty line - flush buffer
        if line.is_empty() {
            if !self.buffer.is_empty() {
                let data = std::mem::take(&mut self.buffer);
                return self.parse_data(&data);
            }
            return None;
        }

        // Comment - ignore
        if line.starts_with(':') {
            return None;
        }

        // Event type - we only handle "data"
        if line.starts_with("event:") {
            return None;
        }

        // Data line
        if let Some(data) = line.strip_prefix("data:") {
            let data = data.trim_start();

            // Check for [DONE] signal
            if data == "[DONE]" {
                return Some(vec![StreamChunk::Done]);
            }

            // Accumulate data (handles multi-line data)
            if !self.buffer.is_empty() {
                self.buffer.push('\n');
            }
            self.buffer.push_str(data);

            // Try to parse as JSON
            if let Ok(stream_data) = serde_json::from_str::<StreamData>(&self.buffer) {
                self.buffer.clear();
                let chunks = stream_data.into_chunks();
                if !chunks.is_empty() {
                    return Some(chunks);
                }
            }
        }

        None
    }

    /// Parses accumulated data.
    fn parse_data(&mut self, data: &str) -> Option<Vec<StreamChunk>> {
        if data.is_empty() {
            return None;
        }

        if let Ok(stream_data) = serde_json::from_str::<StreamData>(data) {
            let chunks = stream_data.into_chunks();
            if !chunks.is_empty() {
                return Some(chunks);
            }
        }

        None
    }

    /// Resets the parser state.
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_chunk_delta() {
        let chunk = StreamChunk::Delta {
            content: "Hello".to_string(),
            index: 0,
        };

        if let StreamChunk::Delta { content, index } = chunk {
            assert_eq!(content, "Hello");
            assert_eq!(index, 0);
        } else {
            panic!("Expected Delta chunk");
        }
    }

    #[test]
    fn test_stream_data_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "created": 1700000000,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "delta": {"content": "Hello"},
                "finish_reason": null
            }]
        }"#;

        let data: StreamData = serde_json::from_str(json).unwrap();
        assert_eq!(data.id, "chatcmpl-123");
        assert_eq!(data.choices.len(), 1);
        assert_eq!(data.choices[0].delta.content, Some("Hello".to_string()));
    }

    #[test]
    fn test_stream_data_into_chunks() {
        let data = StreamData {
            id: "test".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: "gpt-4o".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta {
                    role: None,
                    content: Some("Hello".to_string()),
                },
                finish_reason: None,
            }],
            usage: None,
        };

        let chunks = data.into_chunks();
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0], StreamChunk::Delta { .. }));
    }

    #[test]
    fn test_stream_data_finish_reason() {
        let data = StreamData {
            id: "test".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: "gpt-4o".to_string(),
            choices: vec![StreamChoice {
                index: 0,
                delta: StreamDelta::default(),
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        };

        let chunks = data.into_chunks();
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0], StreamChunk::FinishReason { .. }));
    }

    #[test]
    fn test_stream_data_usage() {
        let data = StreamData {
            id: "test".to_string(),
            object: "chat.completion.chunk".to_string(),
            created: 0,
            model: "gpt-4o".to_string(),
            choices: vec![],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
        };

        let chunks = data.into_chunks();
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0], StreamChunk::Usage(_)));
    }

    #[test]
    fn test_sse_parser_basic() {
        let mut parser = SseParser::new();

        let line = r#"data: {"id":"1","object":"chat.completion.chunk","created":0,"model":"gpt-4","choices":[{"index":0,"delta":{"content":"Hi"}}]}"#;
        let result = parser.parse_line(line);

        assert!(result.is_some());
        let chunks = result.unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0], StreamChunk::Delta { .. }));
    }

    #[test]
    fn test_sse_parser_done() {
        let mut parser = SseParser::new();

        let result = parser.parse_line("data: [DONE]");

        assert!(result.is_some());
        let chunks = result.unwrap();
        assert_eq!(chunks.len(), 1);
        assert!(matches!(chunks[0], StreamChunk::Done));
    }

    #[test]
    fn test_sse_parser_comment() {
        let mut parser = SseParser::new();

        let result = parser.parse_line(": this is a comment");
        assert!(result.is_none());
    }

    #[test]
    fn test_sse_parser_empty_line() {
        let mut parser = SseParser::new();

        let result = parser.parse_line("");
        assert!(result.is_none());
    }

    #[test]
    fn test_sse_parser_event_line() {
        let mut parser = SseParser::new();

        let result = parser.parse_line("event: message");
        assert!(result.is_none());
    }

    #[test]
    fn test_sse_parser_reset() {
        let mut parser = SseParser::new();
        parser.buffer = "some data".to_string();
        parser.reset();
        assert!(parser.buffer.is_empty());
    }
}
