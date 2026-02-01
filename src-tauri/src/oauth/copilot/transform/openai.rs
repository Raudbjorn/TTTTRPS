//! OpenAI format types and transformations.
//!
//! This module provides bidirectional transformations between OpenAI's Chat Completions API
//! format and Copilot's native format. The OpenAI format is largely compatible with Copilot,
//! so transformations are mostly pass-through with some field normalization.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::oauth::copilot::models::{
    ChatRequest, ChatResponse, Choice, Content, ContentPart, ImageDetail, ImageUrl, Message, Role,
    StreamChunk, StreamData,
};
#[cfg(test)]
use crate::oauth::copilot::models::Usage;

// =============================================================================
// OpenAI Request Types
// =============================================================================

/// OpenAI-format chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatRequest {
    /// The model to use for completion.
    pub model: String,

    /// The messages comprising the conversation.
    pub messages: Vec<OpenAIMessage>,

    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Sampling temperature (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Top-p sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Number of completions to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<OpenAIStop>,

    /// Presence penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Frequency penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// Modify likelihood of specific tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, f32>>,

    /// User identifier for abuse monitoring.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// Tools available for the model to call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAITool>>,

    /// Controls how the model calls tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<OpenAIToolChoice>,

    /// Seed for deterministic sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    /// Response format specification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<OpenAIResponseFormat>,

    /// Stream options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<OpenAIStreamOptions>,
}

/// Stop sequence(s) - can be a single string or array.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenAIStop {
    /// Single stop sequence.
    Single(String),
    /// Multiple stop sequences.
    Multiple(Vec<String>),
}

impl OpenAIStop {
    /// Convert to a vector of strings.
    #[must_use]
    pub fn into_vec(self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s],
            Self::Multiple(v) => v,
        }
    }
}

/// Response format specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIResponseFormat {
    /// The format type.
    #[serde(rename = "type")]
    pub format_type: String,

    /// JSON schema for structured outputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<serde_json::Value>,
}

/// Stream options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIStreamOptions {
    /// Include usage stats in stream.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

/// An OpenAI-format message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    /// The role of the message author.
    pub role: String,

    /// The message content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<OpenAIContent>,

    /// Optional name for the author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Tool calls made by the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,

    /// For tool role: the tool call ID this is responding to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// OpenAI message content - can be string or array of parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenAIContent {
    /// Simple text content.
    Text(String),
    /// Multi-part content with text and/or images.
    Parts(Vec<OpenAIContentPart>),
}

/// A part of multi-part content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OpenAIContentPart {
    /// Text content.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },
    /// Image URL content.
    #[serde(rename = "image_url")]
    ImageUrl {
        /// The image URL details.
        image_url: OpenAIImageUrl,
    },
}

/// Image URL with optional detail level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIImageUrl {
    /// The URL of the image (can be a data URL).
    pub url: String,
    /// The detail level for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// A tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITool {
    /// The type of tool (always "function").
    #[serde(rename = "type")]
    pub tool_type: String,
    /// The function definition.
    pub function: OpenAIFunction,
}

/// A function definition for tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunction {
    /// The function name.
    pub name: String,
    /// The function description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The function parameters as a JSON schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    /// Whether to enable strict mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Tool choice specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenAIToolChoice {
    /// String mode: "none", "auto", or "required".
    Mode(String),
    /// Specific function to call.
    Function {
        /// The type (always "function").
        #[serde(rename = "type")]
        choice_type: String,
        /// The function to call.
        function: OpenAIToolChoiceFunction,
    },
}

/// Specific function choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIToolChoiceFunction {
    /// The function name.
    pub name: String,
}

/// A tool call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIToolCall {
    /// The tool call ID.
    pub id: String,
    /// The type of tool (always "function").
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function call details.
    pub function: OpenAIFunctionCall,
}

/// Function call details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunctionCall {
    /// The function name.
    pub name: String,
    /// The function arguments as a JSON string.
    pub arguments: String,
}

// =============================================================================
// OpenAI Response Types
// =============================================================================

/// OpenAI-format chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatResponse {
    /// Unique identifier for the completion.
    pub id: String,
    /// Object type (always "chat.completion").
    pub object: String,
    /// Unix timestamp of creation.
    pub created: i64,
    /// The model used.
    pub model: String,
    /// The completion choices.
    pub choices: Vec<OpenAIChoice>,
    /// Token usage statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAIUsage>,
    /// System fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// A completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChoice {
    /// The choice index.
    pub index: u32,
    /// The generated message.
    pub message: OpenAIMessage,
    /// The reason generation stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OpenAIUsage {
    /// Tokens in the prompt.
    pub prompt_tokens: u32,
    /// Tokens in the completion.
    pub completion_tokens: u32,
    /// Total tokens used.
    pub total_tokens: u32,
}

// =============================================================================
// OpenAI Streaming Types
// =============================================================================

/// OpenAI-format streaming chunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIStreamChunk {
    /// Unique identifier for the completion.
    pub id: String,
    /// Object type (always "chat.completion.chunk").
    pub object: String,
    /// Unix timestamp of creation.
    pub created: i64,
    /// The model used.
    pub model: String,
    /// The streaming choices.
    pub choices: Vec<OpenAIStreamChoice>,
    /// Usage (only in final chunk if stream_options.include_usage is true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAIUsage>,
    /// System fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// A streaming choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIStreamChoice {
    /// The choice index.
    pub index: u32,
    /// The delta (incremental content).
    pub delta: OpenAIStreamDelta,
    /// Finish reason (only in final chunk for this choice).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Delta content in a streaming response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAIStreamDelta {
    /// The role (only in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// The content to append.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls (streaming).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIStreamToolCall>>,
}

/// Streaming tool call delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIStreamToolCall {
    /// The index of this tool call.
    pub index: u32,
    /// The tool call ID (only in first chunk for this tool call).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The type (only in first chunk).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub call_type: Option<String>,
    /// The function call details (may be partial).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<OpenAIStreamFunctionCall>,
}

/// Streaming function call delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIStreamFunctionCall {
    /// The function name (only in first chunk).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Partial arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

// =============================================================================
// Transformation Functions
// =============================================================================

/// Converts an OpenAI-format request to Copilot format.
#[must_use]
pub fn request_to_copilot(req: OpenAIChatRequest) -> ChatRequest {
    let messages = req
        .messages
        .into_iter()
        .filter_map(openai_message_to_copilot)
        .collect();

    ChatRequest {
        model: req.model,
        messages,
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        stream: req.stream,
        top_p: req.top_p,
        stop: req.stop.map(OpenAIStop::into_vec),
        presence_penalty: req.presence_penalty,
        frequency_penalty: req.frequency_penalty,
        user: req.user,
    }
}

/// Converts an OpenAI message to Copilot format.
fn openai_message_to_copilot(msg: OpenAIMessage) -> Option<Message> {
    let role = match msg.role.as_str() {
        "system" => Role::System,
        "user" => Role::User,
        "assistant" => Role::Assistant,
        "tool" => {
            // TODO: Extend Message struct with tool_call_id field to preserve tool context.
            // Currently tool_call_id is not preserved, which may affect multi-turn tool
            // conversations. The Copilot API accepts Role::Tool messages without tool_call_id.
            if msg.tool_call_id.is_some() {
                tracing::debug!(
                    tool_call_id = ?msg.tool_call_id,
                    "Tool message tool_call_id not preserved in Message struct"
                );
            }
            Role::Tool
        }
        role => {
            // Unknown roles are dropped rather than converted to a default to avoid
            // sending semantically incorrect messages. This is logged as a warning.
            tracing::warn!(
                role = %role,
                "Unknown OpenAI message role, skipping message"
            );
            return None;
        }
    };

    let content = match msg.content {
        Some(OpenAIContent::Text(text)) => Content::Text(text),
        Some(OpenAIContent::Parts(parts)) => {
            let copilot_parts: Vec<ContentPart> = parts
                .into_iter()
                .map(openai_content_part_to_copilot)
                .collect();
            Content::Parts(copilot_parts)
        }
        None => Content::Text(String::new()),
    };

    let mut message = Message::new(role, content);
    if let Some(name) = msg.name {
        message = message.with_name(name);
    }

    Some(message)
}

/// Converts an OpenAI content part to Copilot format.
fn openai_content_part_to_copilot(part: OpenAIContentPart) -> ContentPart {
    match part {
        OpenAIContentPart::Text { text } => ContentPart::Text { text },
        OpenAIContentPart::ImageUrl { image_url } => ContentPart::ImageUrl {
            image_url: ImageUrl {
                url: image_url.url,
                detail: image_url.detail.and_then(|d| match d.as_str() {
                    "auto" => Some(ImageDetail::Auto),
                    "low" => Some(ImageDetail::Low),
                    "high" => Some(ImageDetail::High),
                    _ => None,
                }),
            },
        },
    }
}

/// Converts a Copilot response to OpenAI format.
#[must_use]
pub fn response_from_copilot(resp: ChatResponse) -> OpenAIChatResponse {
    let choices = resp
        .choices
        .into_iter()
        .map(copilot_choice_to_openai)
        .collect();

    OpenAIChatResponse {
        id: resp.id,
        object: resp.object,
        created: resp.created,
        model: resp.model,
        choices,
        usage: resp.usage.map(|u| OpenAIUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
        system_fingerprint: None,
    }
}

/// Converts a Copilot choice to OpenAI format.
fn copilot_choice_to_openai(choice: Choice) -> OpenAIChoice {
    OpenAIChoice {
        index: choice.index,
        message: copilot_message_to_openai(choice.message),
        finish_reason: choice.finish_reason,
    }
}

/// Converts a Copilot message to OpenAI format.
fn copilot_message_to_openai(msg: Message) -> OpenAIMessage {
    let role = msg.role.as_str().to_string();
    let content = Some(copilot_content_to_openai(msg.content));

    OpenAIMessage {
        role,
        content,
        name: msg.name,
        tool_calls: None,
        tool_call_id: None,
    }
}

/// Converts Copilot content to OpenAI format.
fn copilot_content_to_openai(content: Content) -> OpenAIContent {
    match content {
        Content::Text(text) => OpenAIContent::Text(text),
        Content::Parts(parts) => {
            let openai_parts: Vec<OpenAIContentPart> = parts
                .into_iter()
                .map(copilot_content_part_to_openai)
                .collect();
            OpenAIContent::Parts(openai_parts)
        }
    }
}

/// Converts a Copilot content part to OpenAI format.
fn copilot_content_part_to_openai(part: ContentPart) -> OpenAIContentPart {
    match part {
        ContentPart::Text { text } => OpenAIContentPart::Text { text },
        ContentPart::ImageUrl { image_url } => OpenAIContentPart::ImageUrl {
            image_url: OpenAIImageUrl {
                url: image_url.url,
                detail: image_url.detail.map(|d| match d {
                    ImageDetail::Auto => "auto".to_string(),
                    ImageDetail::Low => "low".to_string(),
                    ImageDetail::High => "high".to_string(),
                }),
            },
        },
    }
}

/// Converts a Copilot stream chunk to OpenAI format.
#[must_use]
pub fn stream_chunk_from_copilot(
    chunk: StreamChunk,
    id: &str,
    model: &str,
    created: i64,
) -> Option<OpenAIStreamChunk> {
    match chunk {
        StreamChunk::Delta { content, index } => Some(OpenAIStreamChunk {
            id: id.to_string(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.to_string(),
            choices: vec![OpenAIStreamChoice {
                index,
                delta: OpenAIStreamDelta {
                    role: None,
                    content: Some(content),
                    tool_calls: None,
                },
                finish_reason: None,
            }],
            usage: None,
            system_fingerprint: None,
        }),
        StreamChunk::FinishReason { reason, index } => Some(OpenAIStreamChunk {
            id: id.to_string(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.to_string(),
            choices: vec![OpenAIStreamChoice {
                index,
                delta: OpenAIStreamDelta::default(),
                finish_reason: Some(reason),
            }],
            usage: None,
            system_fingerprint: None,
        }),
        StreamChunk::Usage(usage) => Some(OpenAIStreamChunk {
            id: id.to_string(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.to_string(),
            choices: vec![],
            usage: Some(OpenAIUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            }),
            system_fingerprint: None,
        }),
        StreamChunk::Done => None,
    }
}

/// Converts raw stream data to an OpenAI stream chunk.
#[must_use]
pub fn stream_data_to_openai(data: StreamData) -> OpenAIStreamChunk {
    let choices = data
        .choices
        .into_iter()
        .map(|choice| OpenAIStreamChoice {
            index: choice.index,
            delta: OpenAIStreamDelta {
                role: choice.delta.role,
                content: choice.delta.content,
                tool_calls: None,
            },
            finish_reason: choice.finish_reason,
        })
        .collect();

    OpenAIStreamChunk {
        id: data.id,
        object: data.object,
        created: data.created,
        model: data.model,
        choices,
        usage: data.usage.map(|u| OpenAIUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
        system_fingerprint: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_request_to_copilot() {
        let json = r#"{
            "model": "gpt-4o",
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        }"#;

        let req: OpenAIChatRequest = serde_json::from_str(json).unwrap();
        let copilot_req = request_to_copilot(req);

        assert_eq!(copilot_req.model, "gpt-4o");
        assert_eq!(copilot_req.messages.len(), 1);
        assert_eq!(copilot_req.messages[0].role, Role::User);
    }

    #[test]
    fn test_response_from_copilot() {
        let copilot_resp = ChatResponse {
            id: "chatcmpl-123".to_string(),
            object: "chat.completion".to_string(),
            created: 1700000000,
            model: "gpt-4o".to_string(),
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

        let openai_resp = response_from_copilot(copilot_resp);

        assert_eq!(openai_resp.id, "chatcmpl-123");
        assert_eq!(openai_resp.choices.len(), 1);
        assert_eq!(openai_resp.choices[0].message.role, "assistant");
    }

    #[test]
    fn test_stop_into_vec() {
        let single = OpenAIStop::Single("END".to_string());
        assert_eq!(single.into_vec(), vec!["END"]);

        let multiple = OpenAIStop::Multiple(vec!["A".to_string(), "B".to_string()]);
        assert_eq!(multiple.into_vec(), vec!["A", "B"]);
    }

    #[test]
    fn test_stream_chunk_conversion() {
        let chunk = StreamChunk::Delta {
            content: "Hello".to_string(),
            index: 0,
        };

        let openai_chunk = stream_chunk_from_copilot(chunk, "id-123", "gpt-4o", 1700000000);

        assert!(openai_chunk.is_some());
        let openai_chunk = openai_chunk.unwrap();
        assert_eq!(openai_chunk.object, "chat.completion.chunk");
        assert_eq!(
            openai_chunk.choices[0].delta.content,
            Some("Hello".to_string())
        );
    }

    #[test]
    fn test_stream_chunk_done_returns_none() {
        let chunk = StreamChunk::Done;
        let result = stream_chunk_from_copilot(chunk, "id", "gpt-4o", 0);
        assert!(result.is_none());
    }
}
