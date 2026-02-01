//! Anthropic Messages API format types and transformations.
//!
//! This module provides bidirectional transformations between Anthropic's Messages API
//! format and Copilot's native format (which follows the OpenAI convention).
//!
//! ## Key Differences Handled
//!
//! 1. **System Prompts**: Anthropic uses a separate `system` field,
//!    while Copilot/OpenAI uses system role messages.
//!
//! 2. **Content Blocks**: Anthropic uses typed content blocks,
//!    while Copilot uses OpenAI-style content parts.
//!
//! 3. **Image Format**: Anthropic uses base64 with media type,
//!    while Copilot uses data URLs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::oauth::copilot::models::{
    ChatRequest, ChatResponse, Content, ContentPart, ImageDetail, ImageUrl, Message, Role,
    StreamChunk,
};
#[cfg(test)]
use crate::oauth::copilot::models::Usage;

// =============================================================================
// Anthropic Request Types
// =============================================================================

/// Anthropic Messages API request format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessagesRequest {
    /// The model to use for completion.
    pub model: String,

    /// Maximum tokens to generate (required in Anthropic API).
    pub max_tokens: u32,

    /// The messages comprising the conversation.
    pub messages: Vec<AnthropicMessage>,

    /// System prompt (string or array of blocks).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<AnthropicSystem>,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Sampling temperature (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Top-p sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Request metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<AnthropicMetadata>,
}

/// System prompt - can be a string or array of text blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicSystem {
    /// Simple text system prompt.
    Text(String),
    /// Array of text blocks (for cache control).
    Blocks(Vec<AnthropicSystemBlock>),
}

impl AnthropicSystem {
    /// Convert to a single string.
    #[must_use]
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    AnthropicSystemBlock::Text { text, .. } => Some(text.as_str()),
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
        }
    }
}

/// A system block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicSystemBlock {
    /// Text block.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
        /// Cache control settings.
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<AnthropicCacheControl>,
    },
}

/// Cache control configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicCacheControl {
    /// Cache type.
    #[serde(rename = "type")]
    pub cache_type: String,
}

/// Request metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMetadata {
    /// User identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// An Anthropic message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessage {
    /// Role: "user" or "assistant".
    pub role: String,
    /// Message content (string or array of blocks).
    pub content: AnthropicContent,
}

/// Anthropic message content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicContent {
    /// Simple text content.
    Text(String),
    /// Array of content blocks.
    Blocks(Vec<AnthropicContentBlock>),
}

/// An Anthropic content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicContentBlock {
    /// Text block.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
        /// Cache control.
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<AnthropicCacheControl>,
    },

    /// Image block.
    #[serde(rename = "image")]
    Image {
        /// Image source.
        source: AnthropicImageSource,
        /// Cache control.
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<AnthropicCacheControl>,
    },

    /// Tool use block (assistant response).
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Tool call ID.
        id: String,
        /// Tool name.
        name: String,
        /// Tool input (JSON object).
        input: serde_json::Value,
    },

    /// Tool result block (user message).
    #[serde(rename = "tool_result")]
    ToolResult {
        /// The tool call ID this responds to.
        tool_use_id: String,
        /// The result content.
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<AnthropicToolResultContent>,
        /// Whether this is an error result.
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },

    /// Thinking block (extended thinking).
    #[serde(rename = "thinking")]
    Thinking {
        /// The thinking content.
        thinking: String,
        /// Signature for verification.
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
}

/// Tool result content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnthropicToolResultContent {
    /// Simple text result.
    Text(String),
    /// Array of content blocks.
    Blocks(Vec<AnthropicToolResultBlock>),
}

/// A tool result content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicToolResultBlock {
    /// Text block.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },
    /// Image block.
    #[serde(rename = "image")]
    Image {
        /// Image source.
        source: AnthropicImageSource,
    },
}

/// Anthropic image source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicImageSource {
    /// Source type (always "base64").
    #[serde(rename = "type")]
    pub source_type: String,
    /// Media type.
    pub media_type: String,
    /// Base64-encoded image data.
    pub data: String,
}

// =============================================================================
// Anthropic Response Types
// =============================================================================

/// Anthropic Messages API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessagesResponse {
    /// Message ID.
    pub id: String,

    /// Object type (always "message").
    #[serde(rename = "type")]
    pub message_type: String,

    /// Role (always "assistant").
    pub role: String,

    /// Content blocks.
    pub content: Vec<AnthropicResponseContentBlock>,

    /// Model used.
    pub model: String,

    /// Stop reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,

    /// Stop sequence that triggered stop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,

    /// Token usage.
    pub usage: AnthropicUsage,
}

/// Response content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicResponseContentBlock {
    /// Text block.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },

    /// Tool use block.
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Tool call ID.
        id: String,
        /// Tool name.
        name: String,
        /// Tool input.
        input: serde_json::Value,
    },

    /// Thinking block.
    #[serde(rename = "thinking")]
    Thinking {
        /// The thinking content.
        thinking: String,
        /// Signature.
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicUsage {
    /// Input tokens.
    pub input_tokens: u32,
    /// Output tokens.
    pub output_tokens: u32,
    /// Cache creation tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    /// Cache read tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

// =============================================================================
// Anthropic Streaming Types
// =============================================================================

/// Anthropic streaming event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicStreamEvent {
    /// Message started.
    #[serde(rename = "message_start")]
    MessageStart {
        /// Initial message state.
        message: AnthropicStreamMessageStart,
    },

    /// Content block started.
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        /// Block index.
        index: u32,
        /// Initial block state.
        content_block: AnthropicStreamContentBlock,
    },

    /// Content block delta.
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        /// Block index.
        index: u32,
        /// Delta content.
        delta: AnthropicStreamDelta,
    },

    /// Content block stopped.
    #[serde(rename = "content_block_stop")]
    ContentBlockStop {
        /// Block index.
        index: u32,
    },

    /// Message delta (stop reason, usage).
    #[serde(rename = "message_delta")]
    MessageDelta {
        /// Delta content.
        delta: AnthropicMessageDelta,
        /// Usage update.
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<AnthropicStreamUsage>,
    },

    /// Message stopped.
    #[serde(rename = "message_stop")]
    MessageStop,

    /// Ping event (keep-alive).
    #[serde(rename = "ping")]
    Ping,

    /// Error event.
    #[serde(rename = "error")]
    Error {
        /// Error details.
        error: AnthropicStreamError,
    },
}

/// Initial message state in stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicStreamMessageStart {
    /// Message ID.
    pub id: String,
    /// Object type.
    #[serde(rename = "type")]
    pub message_type: String,
    /// Role.
    pub role: String,
    /// Content (empty initially).
    pub content: Vec<AnthropicResponseContentBlock>,
    /// Model.
    pub model: String,
    /// Stop reason (null initially).
    pub stop_reason: Option<String>,
    /// Stop sequence.
    pub stop_sequence: Option<String>,
    /// Usage.
    pub usage: AnthropicStreamUsage,
}

/// Content block in stream start event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicStreamContentBlock {
    /// Text block.
    #[serde(rename = "text")]
    Text {
        /// Initial text (usually empty).
        text: String,
    },

    /// Tool use block.
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Tool call ID.
        id: String,
        /// Tool name.
        name: String,
        /// Initial input (empty object).
        input: serde_json::Value,
    },

    /// Thinking block.
    #[serde(rename = "thinking")]
    Thinking {
        /// Initial thinking (usually empty).
        thinking: String,
    },
}

/// Delta content in stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AnthropicStreamDelta {
    /// Text delta.
    #[serde(rename = "text_delta")]
    TextDelta {
        /// Text to append.
        text: String,
    },

    /// Tool input delta (partial JSON).
    #[serde(rename = "input_json_delta")]
    InputJsonDelta {
        /// Partial JSON to append.
        partial_json: String,
    },

    /// Thinking delta.
    #[serde(rename = "thinking_delta")]
    ThinkingDelta {
        /// Thinking to append.
        thinking: String,
    },
}

/// Message delta (stop info).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicMessageDelta {
    /// Stop reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    /// Stop sequence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
}

/// Usage in stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicStreamUsage {
    /// Input tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    /// Output tokens.
    pub output_tokens: u32,
}

/// Error in stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicStreamError {
    /// Error type.
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error message.
    pub message: String,
}

// =============================================================================
// Stream State
// =============================================================================

/// State machine for Anthropic stream event generation.
#[derive(Debug, Default)]
pub struct AnthropicStreamState {
    /// Whether message_start has been sent.
    pub message_started: bool,
    /// Current content block index.
    pub current_block_index: u32,
    /// Whether a content block is currently open.
    pub block_open: bool,
    /// Whether the current block is a text block.
    pub current_block_is_text: bool,
    /// Tool calls being streamed.
    pub tool_calls: HashMap<u32, ToolCallInfo>,
    /// Message ID.
    pub message_id: String,
    /// Model.
    pub model: String,
}

/// Info about a tool call being streamed.
#[derive(Debug, Clone)]
pub struct ToolCallInfo {
    /// Tool call ID.
    pub id: String,
    /// Tool name.
    pub name: String,
    /// Anthropic block index for this tool.
    pub anthropic_block_index: u32,
}

impl AnthropicStreamState {
    /// Creates a new stream state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if a tool block is currently open.
    fn is_tool_block_open(&self) -> bool {
        if !self.block_open {
            return false;
        }
        self.tool_calls
            .values()
            .any(|tc| tc.anthropic_block_index == self.current_block_index)
    }
}

// =============================================================================
// Transformation Functions
// =============================================================================

/// Converts an Anthropic request to Copilot format.
#[must_use]
pub fn request_to_copilot(req: AnthropicMessagesRequest) -> ChatRequest {
    let mut messages = Vec::new();

    // Handle system prompt
    if let Some(system) = req.system {
        messages.push(Message::system(system.as_text()));
    }

    // Convert messages
    for msg in req.messages {
        messages.extend(anthropic_message_to_copilot(msg));
    }

    ChatRequest {
        model: normalize_model_name(&req.model),
        messages,
        max_tokens: Some(req.max_tokens),
        temperature: req.temperature,
        stream: req.stream,
        top_p: req.top_p,
        stop: req.stop_sequences,
        presence_penalty: None,
        frequency_penalty: None,
        user: req.metadata.and_then(|m| m.user_id),
    }
}

/// Normalizes Anthropic model names to Copilot-compatible names.
fn normalize_model_name(model: &str) -> String {
    if model.starts_with("claude-sonnet-4-") {
        return "claude-sonnet-4".to_string();
    }
    if model.starts_with("claude-opus-4-") {
        return "claude-opus-4".to_string();
    }
    model.to_string()
}

/// Converts an Anthropic message to Copilot message(s).
fn anthropic_message_to_copilot(msg: AnthropicMessage) -> Vec<Message> {
    match msg.role.as_str() {
        "user" => convert_user_message(msg.content),
        "assistant" => convert_assistant_message(msg.content),
        role => {
            tracing::warn!(
                role = %role,
                "Unknown Anthropic message role, skipping message"
            );
            vec![]
        }
    }
}

/// Converts user message content to Copilot messages.
fn convert_user_message(content: AnthropicContent) -> Vec<Message> {
    let mut messages = Vec::new();

    match content {
        AnthropicContent::Text(text) => {
            messages.push(Message::user(text));
        }
        AnthropicContent::Blocks(blocks) => {
            // Separate tool results from other content
            let mut tool_results = Vec::new();
            let mut other_blocks = Vec::new();

            for block in blocks {
                match block {
                    AnthropicContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } => {
                        tool_results.push((tool_use_id, content));
                    }
                    _ => {
                        other_blocks.push(block);
                    }
                }
            }

            // Tool results become user messages
            for (tool_use_id, content) in tool_results {
                let text = match content {
                    Some(AnthropicToolResultContent::Text(t)) => t,
                    Some(AnthropicToolResultContent::Blocks(blocks)) => blocks
                        .into_iter()
                        .filter_map(|b| match b {
                            AnthropicToolResultBlock::Text { text } => Some(text),
                            AnthropicToolResultBlock::Image { .. } => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    None => String::new(),
                };
                messages.push(Message::user(format!(
                    "[Tool result for {tool_use_id}]: {text}"
                )));
            }

            // Other content becomes a user message
            if !other_blocks.is_empty() {
                let parts = blocks_to_content_parts(other_blocks);
                if !parts.is_empty() {
                    messages.push(Message::new(Role::User, Content::Parts(parts)));
                }
            }
        }
    }

    messages
}

/// Converts assistant message content to Copilot messages.
fn convert_assistant_message(content: AnthropicContent) -> Vec<Message> {
    match content {
        AnthropicContent::Text(text) => vec![Message::assistant(text)],
        AnthropicContent::Blocks(blocks) => {
            let mut text_parts = Vec::new();

            for block in &blocks {
                match block {
                    AnthropicContentBlock::Text { text, .. } => {
                        text_parts.push(text.clone());
                    }
                    AnthropicContentBlock::Thinking { thinking, .. } => {
                        text_parts.push(thinking.clone());
                    }
                    AnthropicContentBlock::ToolUse { .. } => {
                        // Tool use blocks need special handling
                    }
                    _ => {}
                }
            }

            if text_parts.is_empty() {
                vec![Message::assistant("")]
            } else {
                vec![Message::assistant(text_parts.join("\n\n"))]
            }
        }
    }
}

/// Converts Anthropic content blocks to Copilot content parts.
fn blocks_to_content_parts(blocks: Vec<AnthropicContentBlock>) -> Vec<ContentPart> {
    blocks
        .into_iter()
        .filter_map(|block| match block {
            AnthropicContentBlock::Text { text, .. } => Some(ContentPart::Text { text }),
            AnthropicContentBlock::Image { source, .. } => {
                let url = format!("data:{};base64,{}", source.media_type, source.data);
                Some(ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url,
                        detail: Some(ImageDetail::Auto),
                    },
                })
            }
            _ => None,
        })
        .collect()
}

/// Converts a Copilot response to Anthropic format.
#[must_use]
pub fn response_from_copilot(resp: ChatResponse) -> AnthropicMessagesResponse {
    let mut content_blocks = Vec::new();

    for choice in &resp.choices {
        let text_blocks = get_text_blocks(&choice.message.content);
        content_blocks.extend(text_blocks);
    }

    let stop_reason = resp
        .choices
        .first()
        .and_then(|c| c.finish_reason.as_ref())
        .map(|r| map_finish_reason_to_anthropic(r));

    AnthropicMessagesResponse {
        id: resp.id,
        message_type: "message".to_string(),
        role: "assistant".to_string(),
        content: content_blocks,
        model: resp.model,
        stop_reason,
        stop_sequence: None,
        usage: AnthropicUsage {
            input_tokens: resp.usage.as_ref().map_or(0, |u| u.prompt_tokens),
            output_tokens: resp.usage.as_ref().map_or(0, |u| u.completion_tokens),
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
        },
    }
}

/// Extracts text blocks from message content.
fn get_text_blocks(content: &Content) -> Vec<AnthropicResponseContentBlock> {
    match content {
        Content::Text(text) => {
            if text.is_empty() {
                vec![]
            } else {
                vec![AnthropicResponseContentBlock::Text { text: text.clone() }]
            }
        }
        Content::Parts(parts) => parts
            .iter()
            .filter_map(|p| match p {
                ContentPart::Text { text } => {
                    Some(AnthropicResponseContentBlock::Text { text: text.clone() })
                }
                ContentPart::ImageUrl { .. } => None,
            })
            .collect(),
    }
}

/// Maps OpenAI finish reasons to Anthropic stop reasons.
fn map_finish_reason_to_anthropic(reason: &str) -> String {
    match reason {
        "stop" => "end_turn".to_string(),
        "length" => "max_tokens".to_string(),
        "tool_calls" => "tool_use".to_string(),
        "content_filter" => "end_turn".to_string(),
        other => other.to_string(),
    }
}

/// Generates Anthropic stream events from a Copilot stream chunk.
#[must_use]
pub fn stream_events_from_copilot(
    chunk: StreamChunk,
    state: &mut AnthropicStreamState,
    message_id: &str,
    model: &str,
) -> Vec<AnthropicStreamEvent> {
    let mut events = Vec::new();

    if state.message_id.is_empty() {
        state.message_id = message_id.to_string();
        state.model = model.to_string();
    }

    match chunk {
        StreamChunk::Delta { content, index: _ } => {
            // Send message_start if not yet sent
            if !state.message_started {
                events.push(create_message_start(message_id, model));
                state.message_started = true;
            }

            // Close any open tool block before starting text
            if state.is_tool_block_open() {
                events.push(AnthropicStreamEvent::ContentBlockStop {
                    index: state.current_block_index,
                });
                state.current_block_index += 1;
                state.block_open = false;
            }

            // Start a text block if not open
            if !state.block_open {
                events.push(AnthropicStreamEvent::ContentBlockStart {
                    index: state.current_block_index,
                    content_block: AnthropicStreamContentBlock::Text {
                        text: String::new(),
                    },
                });
                state.block_open = true;
                state.current_block_is_text = true;
            }

            // Send text delta
            events.push(AnthropicStreamEvent::ContentBlockDelta {
                index: state.current_block_index,
                delta: AnthropicStreamDelta::TextDelta { text: content },
            });
        }

        StreamChunk::FinishReason { reason, index: _ } => {
            // Close any open block
            if state.block_open {
                events.push(AnthropicStreamEvent::ContentBlockStop {
                    index: state.current_block_index,
                });
                state.block_open = false;
            }

            // Send message_delta with stop reason
            events.push(AnthropicStreamEvent::MessageDelta {
                delta: AnthropicMessageDelta {
                    stop_reason: Some(map_finish_reason_to_anthropic(&reason)),
                    stop_sequence: None,
                },
                usage: None,
            });
        }

        StreamChunk::Usage(usage) => {
            events.push(AnthropicStreamEvent::MessageDelta {
                delta: AnthropicMessageDelta {
                    stop_reason: None,
                    stop_sequence: None,
                },
                usage: Some(AnthropicStreamUsage {
                    input_tokens: Some(usage.prompt_tokens),
                    output_tokens: usage.completion_tokens,
                }),
            });
        }

        StreamChunk::Done => {
            events.push(AnthropicStreamEvent::MessageStop);
        }
    }

    events
}

/// Creates a message_start event.
fn create_message_start(message_id: &str, model: &str) -> AnthropicStreamEvent {
    AnthropicStreamEvent::MessageStart {
        message: AnthropicStreamMessageStart {
            id: message_id.to_string(),
            message_type: "message".to_string(),
            role: "assistant".to_string(),
            content: vec![],
            model: model.to_string(),
            stop_reason: None,
            stop_sequence: None,
            usage: AnthropicStreamUsage {
                input_tokens: Some(0),
                output_tokens: 0,
            },
        },
    }
}

/// Creates an error event.
#[must_use]
pub fn create_error_event(message: &str) -> AnthropicStreamEvent {
    AnthropicStreamEvent::Error {
        error: AnthropicStreamError {
            error_type: "api_error".to_string(),
            message: message.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oauth::copilot::models::Choice;

    #[test]
    fn test_anthropic_request_to_copilot() {
        let json = r#"{
            "model": "claude-3-opus-20240229",
            "max_tokens": 1000,
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        }"#;

        let req: AnthropicMessagesRequest = serde_json::from_str(json).unwrap();
        let copilot_req = request_to_copilot(req);

        assert_eq!(copilot_req.max_tokens, Some(1000));
        assert_eq!(copilot_req.messages.len(), 1);
        assert_eq!(copilot_req.messages[0].role, Role::User);
    }

    #[test]
    fn test_anthropic_system_prompt() {
        let json = r#"{
            "model": "claude-3-opus-20240229",
            "max_tokens": 1000,
            "system": "You are helpful",
            "messages": [{"role": "user", "content": "Hi"}]
        }"#;

        let req: AnthropicMessagesRequest = serde_json::from_str(json).unwrap();
        let copilot_req = request_to_copilot(req);

        assert_eq!(copilot_req.messages.len(), 2);
        assert_eq!(copilot_req.messages[0].role, Role::System);
    }

    #[test]
    fn test_response_from_copilot() {
        let copilot_resp = ChatResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1700000000,
            model: "claude-3-opus".to_string(),
            choices: vec![Choice {
                index: 0,
                message: Message::assistant("Hello!"),
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
            system_fingerprint: None,
        };

        let anthropic_resp = response_from_copilot(copilot_resp);

        assert_eq!(anthropic_resp.message_type, "message");
        assert_eq!(anthropic_resp.stop_reason, Some("end_turn".to_string()));
        assert_eq!(anthropic_resp.usage.input_tokens, 10);
    }

    #[test]
    fn test_stop_reason_mapping() {
        assert_eq!(map_finish_reason_to_anthropic("stop"), "end_turn");
        assert_eq!(map_finish_reason_to_anthropic("length"), "max_tokens");
        assert_eq!(map_finish_reason_to_anthropic("tool_calls"), "tool_use");
    }

    #[test]
    fn test_model_name_normalization() {
        assert_eq!(
            normalize_model_name("claude-sonnet-4-20250514"),
            "claude-sonnet-4"
        );
        assert_eq!(
            normalize_model_name("claude-opus-4-20250514"),
            "claude-opus-4"
        );
        assert_eq!(
            normalize_model_name("claude-3-opus-20240229"),
            "claude-3-opus-20240229"
        );
    }

    #[test]
    fn test_stream_state() {
        let mut state = AnthropicStreamState::new();
        assert!(!state.message_started);
        assert!(!state.block_open);

        let events = stream_events_from_copilot(
            StreamChunk::Delta {
                content: "Hello".to_string(),
                index: 0,
            },
            &mut state,
            "msg_123",
            "claude-3",
        );

        assert!(state.message_started);
        assert!(state.block_open);
        assert_eq!(events.len(), 3); // message_start, block_start, delta
    }
}
