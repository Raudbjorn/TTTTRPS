//! Chat-related data models.
//!
//! This module contains the data structures for chat completion requests
//! and responses used by the Copilot API.

use serde::{Deserialize, Serialize};

// =============================================================================
// Chat Request Types
// =============================================================================

/// A chat completion request to the Copilot API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The model to use for completion.
    pub model: String,

    /// The messages comprising the conversation.
    pub messages: Vec<Message>,

    /// Maximum number of tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// Sampling temperature (0.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Top-p (nucleus) sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// Stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// Presence penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    /// Frequency penalty (-2.0 to 2.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    /// User identifier for abuse monitoring.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl Default for ChatRequest {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            stream: None,
            top_p: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
        }
    }
}

impl ChatRequest {
    /// Creates a new chat request with the specified model.
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    /// Adds a message to the conversation.
    #[must_use]
    pub fn with_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Sets the messages for the conversation.
    #[must_use]
    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    /// Sets the maximum tokens.
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Sets the temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Enables streaming.
    #[must_use]
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Returns true if streaming is enabled.
    #[must_use]
    pub fn is_streaming(&self) -> bool {
        self.stream.unwrap_or(false)
    }
}

// =============================================================================
// Message Types
// =============================================================================

/// Role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message (instructions).
    System,
    /// User message.
    User,
    /// Assistant message.
    Assistant,
    /// Tool response message.
    Tool,
}

impl Role {
    /// Returns the string representation of the role.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author.
    pub role: Role,

    /// The message content.
    pub content: Content,

    /// Optional name for the author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    /// Creates a new message.
    #[must_use]
    pub fn new(role: Role, content: impl Into<Content>) -> Self {
        Self {
            role,
            content: content.into(),
            name: None,
        }
    }

    /// Creates a system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, Content::Text(content.into()))
    }

    /// Creates a user message.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, Content::Text(content.into()))
    }

    /// Creates an assistant message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, Content::Text(content.into()))
    }

    /// Adds a name to the message.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Returns true if the message contains images.
    #[must_use]
    pub fn has_images(&self) -> bool {
        match &self.content {
            Content::Text(_) => false,
            Content::Parts(parts) => parts
                .iter()
                .any(|p| matches!(p, ContentPart::ImageUrl { .. })),
        }
    }
}

// =============================================================================
// Content Types
// =============================================================================

/// Message content - either text or a list of parts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    /// Simple text content.
    Text(String),
    /// Multi-part content (text and/or images).
    Parts(Vec<ContentPart>),
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content::Text(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Content::Text(s.to_string())
    }
}

impl Content {
    /// Returns the text content, or joins text parts if multi-part.
    #[must_use]
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

/// A part of multi-part content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    /// Text content.
    #[serde(rename = "text")]
    Text {
        /// The text.
        text: String,
    },
    /// Image URL content.
    #[serde(rename = "image_url")]
    ImageUrl {
        /// The image URL details.
        image_url: ImageUrl,
    },
}

/// Image URL with optional detail level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUrl {
    /// The URL (can be a data URL).
    pub url: String,
    /// The detail level for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetail>,
}

/// Image processing detail level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    /// Automatic detail selection.
    Auto,
    /// Low detail (faster, less tokens).
    Low,
    /// High detail (slower, more tokens).
    High,
}

// =============================================================================
// Chat Response Types
// =============================================================================

/// A chat completion response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Unique identifier for the completion.
    pub id: String,

    /// Object type (always "chat.completion").
    pub object: String,

    /// Unix timestamp of creation.
    pub created: i64,

    /// The model used.
    pub model: String,

    /// The completion choices.
    pub choices: Vec<Choice>,

    /// Token usage statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,

    /// System fingerprint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

impl ChatResponse {
    /// Returns the first choice's message content as text.
    #[must_use]
    pub fn first_content(&self) -> Option<String> {
        self.choices.first().map(|c| c.message.content.as_text())
    }

    /// Returns the finish reason for the first choice.
    #[must_use]
    pub fn first_finish_reason(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_deref())
    }
}

/// A completion choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// The choice index.
    pub index: u32,

    /// The generated message.
    pub message: Message,

    /// The reason generation stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Usage {
    /// Tokens in the prompt.
    pub prompt_tokens: u32,

    /// Tokens in the completion.
    pub completion_tokens: u32,

    /// Total tokens used.
    pub total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_builder() {
        let request = ChatRequest::new("gpt-4o")
            .with_message(Message::system("You are helpful"))
            .with_message(Message::user("Hello"))
            .with_max_tokens(1000)
            .with_temperature(0.7)
            .with_stream(true);

        assert_eq!(request.model, "gpt-4o");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.max_tokens, Some(1000));
        assert_eq!(request.temperature, Some(0.7));
        assert!(request.is_streaming());
    }

    #[test]
    fn test_message_constructors() {
        let sys = Message::system("system");
        assert_eq!(sys.role, Role::System);
        assert_eq!(sys.content.as_text(), "system");

        let user = Message::user("user");
        assert_eq!(user.role, Role::User);

        let asst = Message::assistant("assistant");
        assert_eq!(asst.role, Role::Assistant);
    }

    #[test]
    fn test_message_with_name() {
        let msg = Message::user("Hello").with_name("Alice");
        assert_eq!(msg.name, Some("Alice".to_string()));
    }

    #[test]
    fn test_message_has_images() {
        let text_msg = Message::user("Hello");
        assert!(!text_msg.has_images());

        let image_msg = Message {
            role: Role::User,
            content: Content::Parts(vec![ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "https://example.com/image.png".to_string(),
                    detail: Some(ImageDetail::Auto),
                },
            }]),
            name: None,
        };
        assert!(image_msg.has_images());
    }

    #[test]
    fn test_content_as_text() {
        let text = Content::Text("hello".to_string());
        assert_eq!(text.as_text(), "hello");

        let parts = Content::Parts(vec![
            ContentPart::Text {
                text: "part1".to_string(),
            },
            ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "...".to_string(),
                    detail: None,
                },
            },
            ContentPart::Text {
                text: "part2".to_string(),
            },
        ]);
        assert_eq!(parts.as_text(), "part1\npart2");
    }

    #[test]
    fn test_role_serialization() {
        assert_eq!(
            serde_json::to_string(&Role::System).unwrap(),
            "\"system\""
        );
        assert_eq!(serde_json::to_string(&Role::User).unwrap(), "\"user\"");
        assert_eq!(
            serde_json::to_string(&Role::Assistant).unwrap(),
            "\"assistant\""
        );
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest::new("gpt-4o")
            .with_message(Message::user("Hello"))
            .with_max_tokens(100);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"gpt-4o\""));
        assert!(json.contains("\"max_tokens\":100"));
        assert!(json.contains("\"role\":\"user\""));
    }

    #[test]
    fn test_chat_response_deserialization() {
        let json = r#"{
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#;

        let response: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "chatcmpl-123");
        assert_eq!(response.model, "gpt-4o");
        assert_eq!(response.first_content(), Some("Hello!".to_string()));
        assert_eq!(response.first_finish_reason(), Some("stop"));
        assert_eq!(response.usage.unwrap().total_tokens, 15);
    }

    #[test]
    fn test_multi_part_content_deserialization() {
        let json = r#"{
            "role": "user",
            "content": [
                {"type": "text", "text": "What is this?"},
                {"type": "image_url", "image_url": {"url": "data:image/png;base64,abc", "detail": "high"}}
            ]
        }"#;

        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, Role::User);
        assert!(msg.has_images());

        if let Content::Parts(parts) = &msg.content {
            assert_eq!(parts.len(), 2);
        } else {
            panic!("Expected Parts content");
        }
    }
}
