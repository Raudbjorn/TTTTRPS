//! Streaming Response Handler
//!
//! Handles real-time streaming responses from LLM providers.

use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// A chunk of streamed content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingChunk {
    /// Content delta
    pub content: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Finish reason (if final)
    pub finish_reason: Option<String>,
    /// Token usage (if available, usually on final chunk)
    pub usage: Option<StreamingUsage>,
}

/// Token usage for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Provider type for parsing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamProvider {
    OpenAI,
    Claude,
    Gemini,
    Ollama,
}

// ============================================================================
// Stream Parser
// ============================================================================

/// Parser for provider-specific streaming formats
pub struct StreamParser {
    provider: StreamProvider,
    buffer: String,
}

impl StreamParser {
    pub fn new(provider: StreamProvider) -> Self {
        Self {
            provider,
            buffer: String::new(),
        }
    }

    /// Parse incoming data and extract chunks
    pub fn parse(&mut self, data: &str) -> Vec<StreamingChunk> {
        self.buffer.push_str(data);

        match self.provider {
            StreamProvider::OpenAI => self.parse_openai_sse(),
            StreamProvider::Claude => self.parse_claude_sse(),
            StreamProvider::Gemini => self.parse_gemini_json(),
            StreamProvider::Ollama => self.parse_ollama_ndjson(),
        }
    }

    /// Parse OpenAI SSE format
    fn parse_openai_sse(&mut self) -> Vec<StreamingChunk> {
        let mut chunks = Vec::new();

        while let Some(line_end) = self.buffer.find('\n') {
            let line = self.buffer[..line_end].trim().to_string();
            self.buffer = self.buffer[line_end + 1..].to_string();

            if line.is_empty() || !line.starts_with("data: ") {
                continue;
            }

            let data = &line[6..];

            if data == "[DONE]" {
                chunks.push(StreamingChunk {
                    content: String::new(),
                    is_final: true,
                    finish_reason: Some("stop".to_string()),
                    usage: None,
                });
                continue;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some(choice) = json["choices"].as_array().and_then(|a| a.first()) {
                    let content = choice["delta"]["content"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();

                    let finish_reason = choice["finish_reason"]
                        .as_str()
                        .map(|s| s.to_string());

                    let is_final = finish_reason.is_some();

                    if !content.is_empty() || is_final {
                        chunks.push(StreamingChunk {
                            content,
                            is_final,
                            finish_reason,
                            usage: None,
                        });
                    }
                }
            }
        }

        chunks
    }

    /// Parse Claude SSE format
    fn parse_claude_sse(&mut self) -> Vec<StreamingChunk> {
        let mut chunks = Vec::new();

        while let Some(line_end) = self.buffer.find('\n') {
            let line = self.buffer[..line_end].trim().to_string();
            self.buffer = self.buffer[line_end + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            // Claude sends "event: " and "data: " lines
            if !line.starts_with("data: ") {
                continue;
            }

            let data = &line[6..];

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                let event_type = json["type"].as_str().unwrap_or("");

                match event_type {
                    "content_block_delta" => {
                        let content = json["delta"]["text"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();

                        if !content.is_empty() {
                            chunks.push(StreamingChunk {
                                content,
                                is_final: false,
                                finish_reason: None,
                                usage: None,
                            });
                        }
                    }
                    "message_stop" => {
                        chunks.push(StreamingChunk {
                            content: String::new(),
                            is_final: true,
                            finish_reason: Some("end_turn".to_string()),
                            usage: None,
                        });
                    }
                    "message_delta" => {
                        if let Some(stop_reason) = json["delta"]["stop_reason"].as_str() {
                            let usage = json["usage"].as_object().map(|u| StreamingUsage {
                                input_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
                                output_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
                            });

                            chunks.push(StreamingChunk {
                                content: String::new(),
                                is_final: true,
                                finish_reason: Some(stop_reason.to_string()),
                                usage,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        chunks
    }

    /// Parse Gemini JSON stream format
    fn parse_gemini_json(&mut self) -> Vec<StreamingChunk> {
        let mut chunks = Vec::new();

        // Gemini sends complete JSON objects
        while let Some(bracket_pos) = self.buffer.find('}') {
            let potential_json = &self.buffer[..=bracket_pos];

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(potential_json) {
                self.buffer = self.buffer[bracket_pos + 1..].to_string();

                if let Some(candidates) = json["candidates"].as_array() {
                    for candidate in candidates {
                        let content = candidate["content"]["parts"]
                            .as_array()
                            .and_then(|p| p.first())
                            .and_then(|p| p["text"].as_str())
                            .unwrap_or("")
                            .to_string();

                        let finish_reason = candidate["finishReason"]
                            .as_str()
                            .map(|s| s.to_string());

                        let is_final = finish_reason.is_some();

                        if !content.is_empty() || is_final {
                            chunks.push(StreamingChunk {
                                content,
                                is_final,
                                finish_reason,
                                usage: None,
                            });
                        }
                    }
                }
            } else {
                break;
            }
        }

        chunks
    }

    /// Parse Ollama newline-delimited JSON format
    fn parse_ollama_ndjson(&mut self) -> Vec<StreamingChunk> {
        let mut chunks = Vec::new();

        while let Some(line_end) = self.buffer.find('\n') {
            let line = self.buffer[..line_end].trim().to_string();
            self.buffer = self.buffer[line_end + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                let content = json["message"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let is_final = json["done"].as_bool().unwrap_or(false);

                let usage = if is_final {
                    Some(StreamingUsage {
                        input_tokens: json["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
                        output_tokens: json["eval_count"].as_u64().unwrap_or(0) as u32,
                    })
                } else {
                    None
                };

                chunks.push(StreamingChunk {
                    content,
                    is_final,
                    finish_reason: if is_final { Some("stop".to_string()) } else { None },
                    usage,
                });
            }
        }

        chunks
    }
}

// ============================================================================
// Streaming Manager
// ============================================================================

/// Manages streaming responses and aggregation
pub struct StreamingManager {
    /// Accumulated content
    accumulated_content: String,
    /// Total input tokens
    total_input_tokens: u32,
    /// Total output tokens
    total_output_tokens: u32,
    /// Whether the stream is complete
    is_complete: bool,
    /// Final finish reason
    finish_reason: Option<String>,
}

impl StreamingManager {
    pub fn new() -> Self {
        Self {
            accumulated_content: String::new(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            is_complete: false,
            finish_reason: None,
        }
    }

    /// Process a streaming chunk
    pub fn process_chunk(&mut self, chunk: StreamingChunk) {
        self.accumulated_content.push_str(&chunk.content);

        if let Some(usage) = chunk.usage {
            self.total_input_tokens = usage.input_tokens;
            self.total_output_tokens = usage.output_tokens;
        }

        if chunk.is_final {
            self.is_complete = true;
            self.finish_reason = chunk.finish_reason;
        }
    }

    /// Get the accumulated content
    pub fn content(&self) -> &str {
        &self.accumulated_content
    }

    /// Check if streaming is complete
    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// Get the final response (consumes the manager)
    pub fn into_response(self) -> StreamingResponse {
        StreamingResponse {
            content: self.accumulated_content,
            input_tokens: self.total_input_tokens,
            output_tokens: self.total_output_tokens,
            finish_reason: self.finish_reason,
        }
    }
}

impl Default for StreamingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Final aggregated streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResponse {
    pub content: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub finish_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_sse_parsing() {
        let mut parser = StreamParser::new(StreamProvider::OpenAI);

        let data = r#"data: {"choices":[{"delta":{"content":"Hello"}}]}

data: {"choices":[{"delta":{"content":" world"}}]}

data: [DONE]

"#;

        let chunks = parser.parse(data);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "Hello");
        assert_eq!(chunks[1].content, " world");
        assert!(chunks[2].is_final);
    }

    #[test]
    fn test_streaming_manager() {
        let mut manager = StreamingManager::new();

        manager.process_chunk(StreamingChunk {
            content: "Hello".to_string(),
            is_final: false,
            finish_reason: None,
            usage: None,
        });

        manager.process_chunk(StreamingChunk {
            content: " world".to_string(),
            is_final: true,
            finish_reason: Some("stop".to_string()),
            usage: Some(StreamingUsage {
                input_tokens: 10,
                output_tokens: 5,
            }),
        });

        assert!(manager.is_complete());

        let response = manager.into_response();
        assert_eq!(response.content, "Hello world");
        assert_eq!(response.input_tokens, 10);
        assert_eq!(response.output_tokens, 5);
    }
}
