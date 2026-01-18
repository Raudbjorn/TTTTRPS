//! Data models for the Claude API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Token information stored after OAuth flow.
/// Compatible with Go claude-gate format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token type (always "oauth" for this crate)
    #[serde(rename = "type")]
    pub token_type: String,

    /// OAuth access token
    #[serde(rename = "access")]
    pub access_token: String,

    /// OAuth refresh token
    #[serde(rename = "refresh")]
    pub refresh_token: String,

    /// Unix timestamp when token expires
    #[serde(rename = "expires")]
    pub expires_at: i64,
}

impl TokenInfo {
    /// Create a new token info from OAuth response.
    #[must_use]
    pub fn new(access_token: String, refresh_token: String, expires_in: i64) -> Self {
        let expires_at = Utc::now().timestamp() + expires_in;
        Self {
            token_type: "oauth".to_string(),
            access_token,
            refresh_token,
            expires_at,
        }
    }

    /// Check if the token is expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() >= self.expires_at
    }

    /// Check if the token needs refresh (expires within 5 minutes).
    #[must_use]
    pub fn needs_refresh(&self) -> bool {
        const REFRESH_BUFFER_SECS: i64 = 300; // 5 minutes
        Utc::now().timestamp() >= (self.expires_at - REFRESH_BUFFER_SECS)
    }

    /// Get expiration as DateTime.
    #[must_use]
    pub fn expires_at_datetime(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.expires_at, 0)
    }

    /// Time until expiration in seconds.
    #[must_use]
    pub fn time_until_expiry(&self) -> i64 {
        self.expires_at - Utc::now().timestamp()
    }
}

/// Message role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User message
    User,
    /// Assistant (Claude) message
    Assistant,
}

/// Content block within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text {
        /// The text content
        text: String,
    },
    /// Image content (for vision)
    Image {
        /// Image source
        source: ImageSource,
    },
    /// Document content (for PDF processing)
    Document {
        /// Document source
        source: DocumentSource,
    },
    /// Tool use request from Claude
    ToolUse {
        /// Unique ID for this tool use
        id: String,
        /// Name of the tool
        name: String,
        /// Tool input as JSON
        input: serde_json::Value,
    },
    /// Tool result from user
    ToolResult {
        /// ID of the tool use this is a result for
        tool_use_id: String,
        /// Tool output content
        content: String,
        /// Whether the tool execution failed
        #[serde(default)]
        is_error: bool,
    },
}

impl ContentBlock {
    /// Create a text content block.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create an image content block from base64 data.
    ///
    /// # Arguments
    ///
    /// * `data` - Base64-encoded image data
    /// * `media_type` - MIME type (e.g., "image/png", "image/jpeg")
    #[must_use]
    pub fn image_base64(data: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Base64 {
                media_type: media_type.into(),
                data: data.into(),
            },
        }
    }

    /// Create an image content block from a URL.
    #[must_use]
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            source: ImageSource::Url { url: url.into() },
        }
    }

    /// Create a document content block from base64 data.
    ///
    /// # Arguments
    ///
    /// * `data` - Base64-encoded document data
    /// * `media_type` - MIME type (e.g., "application/pdf")
    #[must_use]
    pub fn document_base64(data: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self::Document {
            source: DocumentSource::Base64 {
                media_type: media_type.into(),
                data: data.into(),
            },
        }
    }

    /// Create a PDF document content block from base64 data.
    ///
    /// Convenience method that sets the media type to "application/pdf".
    #[must_use]
    pub fn pdf_base64(data: impl Into<String>) -> Self {
        Self::document_base64(data, "application/pdf")
    }

    /// Get the text content if this is a text block.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// Image source for image content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64-encoded image data
    Base64 {
        /// MIME type of the image
        media_type: String,
        /// Base64-encoded image data
        data: String,
    },
    /// URL reference to an image
    Url {
        /// URL of the image
        url: String,
    },
}

impl ImageSource {
    /// Create a base64 image source.
    #[must_use]
    pub fn base64(data: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self::Base64 {
            media_type: media_type.into(),
            data: data.into(),
        }
    }

    /// Create a PNG image source from base64 data.
    #[must_use]
    pub fn png(data: impl Into<String>) -> Self {
        Self::base64(data, "image/png")
    }

    /// Create a JPEG image source from base64 data.
    #[must_use]
    pub fn jpeg(data: impl Into<String>) -> Self {
        Self::base64(data, "image/jpeg")
    }

    /// Create a URL image source.
    #[must_use]
    pub fn url(url: impl Into<String>) -> Self {
        Self::Url { url: url.into() }
    }
}

/// Document source for document content blocks.
///
/// Used for sending PDFs and other documents to Claude for processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentSource {
    /// Base64-encoded document data
    Base64 {
        /// MIME type of the document (e.g., "application/pdf")
        media_type: String,
        /// Base64-encoded document data
        data: String,
    },
}

impl DocumentSource {
    /// Create a base64 document source.
    #[must_use]
    pub fn base64(data: impl Into<String>, media_type: impl Into<String>) -> Self {
        Self::Base64 {
            media_type: media_type.into(),
            data: data.into(),
        }
    }

    /// Create a PDF document source from base64 data.
    #[must_use]
    pub fn pdf(data: impl Into<String>) -> Self {
        Self::base64(data, "application/pdf")
    }
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message author
    pub role: Role,
    /// Content blocks
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Create a user message with text content.
    #[must_use]
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Create an assistant message with text content.
    #[must_use]
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Create a message with multiple content blocks.
    #[must_use]
    pub fn with_content(role: Role, content: Vec<ContentBlock>) -> Self {
        Self { role, content }
    }

    /// Create a user message with a PDF document and text prompt.
    ///
    /// # Arguments
    ///
    /// * `pdf_base64` - Base64-encoded PDF data
    /// * `prompt` - Text prompt describing what to do with the document
    #[must_use]
    pub fn with_pdf(pdf_base64: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![
                ContentBlock::pdf_base64(pdf_base64),
                ContentBlock::text(prompt),
            ],
        }
    }

    /// Create a user message with a document and text prompt.
    ///
    /// # Arguments
    ///
    /// * `doc_base64` - Base64-encoded document data
    /// * `media_type` - MIME type of the document
    /// * `prompt` - Text prompt describing what to do with the document
    #[must_use]
    pub fn with_document(
        doc_base64: impl Into<String>,
        media_type: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            role: Role::User,
            content: vec![
                ContentBlock::document_base64(doc_base64, media_type),
                ContentBlock::text(prompt),
            ],
        }
    }

    /// Create a user message with an image and text prompt.
    ///
    /// # Arguments
    ///
    /// * `image_base64` - Base64-encoded image data
    /// * `media_type` - MIME type of the image (e.g., "image/png")
    /// * `prompt` - Text prompt describing what to do with the image
    #[must_use]
    pub fn with_image(
        image_base64: impl Into<String>,
        media_type: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            role: Role::User,
            content: vec![
                ContentBlock::image_base64(image_base64, media_type),
                ContentBlock::text(prompt),
            ],
        }
    }

    /// Get the text content of this message (concatenated).
    #[must_use]
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(ContentBlock::as_text)
            .collect::<Vec<_>>()
            .join("")
    }
}

/// Stop reason for message completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Natural end of turn
    EndTurn,
    /// Maximum tokens reached
    MaxTokens,
    /// Stop sequence encountered
    StopSequence,
    /// Tool use requested
    ToolUse,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens consumed
    pub input_tokens: u32,
    /// Output tokens generated
    pub output_tokens: u32,
    /// Cache creation input tokens (if caching enabled)
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    /// Cache read input tokens (if caching enabled)
    #[serde(default)]
    pub cache_read_input_tokens: u32,
}

/// Response from the messages API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesResponse {
    /// Unique ID for this response
    pub id: String,
    /// Object type (always "message")
    #[serde(rename = "type")]
    pub object_type: String,
    /// Role (always "assistant")
    pub role: Role,
    /// Content blocks in the response
    pub content: Vec<ContentBlock>,
    /// Model that generated the response
    pub model: String,
    /// Reason generation stopped
    pub stop_reason: Option<StopReason>,
    /// Stop sequence that triggered stop (if applicable)
    pub stop_sequence: Option<String>,
    /// Token usage
    pub usage: Usage,
}

impl MessagesResponse {
    /// Get the text content of the response (concatenated).
    #[must_use]
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(ContentBlock::as_text)
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get tool use requests from the response.
    #[must_use]
    pub fn tool_uses(&self) -> Vec<(&str, &str, &serde_json::Value)> {
        self.content
            .iter()
            .filter_map(|c| match c {
                ContentBlock::ToolUse { id, name, input } => {
                    Some((id.as_str(), name.as_str(), input))
                }
                _ => None,
            })
            .collect()
    }
}

/// Events emitted during streaming responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Message stream started
    MessageStart {
        /// The initial message object
        message: MessagesResponse,
    },
    /// Content block started
    ContentBlockStart {
        /// Index of the content block
        index: usize,
        /// The content block
        content_block: ContentBlock,
    },
    /// Ping event (keepalive)
    Ping,
    /// Content block delta (incremental update)
    ContentBlockDelta {
        /// Index of the content block
        index: usize,
        /// The delta
        delta: ContentDelta,
    },
    /// Content block finished
    ContentBlockStop {
        /// Index of the content block
        index: usize,
    },
    /// Message delta (final updates)
    MessageDelta {
        /// Delta containing stop reason and usage
        delta: MessageDeltaData,
        /// Updated usage
        usage: Usage,
    },
    /// Message stream finished
    MessageStop,
    /// Error event
    Error {
        /// Error details
        error: ApiError,
    },
}

/// Content delta for streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentDelta {
    /// Text delta
    TextDelta {
        /// The text fragment
        text: String,
    },
    /// Tool input delta (JSON fragment)
    InputJsonDelta {
        /// The partial JSON string
        partial_json: String,
    },
}

/// Message delta data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeltaData {
    /// Stop reason
    pub stop_reason: Option<StopReason>,
    /// Stop sequence (if applicable)
    pub stop_sequence: Option<String>,
}

/// API error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error message
    pub message: String,
}

/// Tool definition for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON schema for tool input
    pub input_schema: serde_json::Value,
}

impl Tool {
    /// Create a new tool definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
    }
}

/// Tool choice configuration for controlling how Claude uses tools.
///
/// # Examples
///
/// ```rust
/// use claude_gate::ToolChoice;
///
/// // Let Claude decide whether to use tools (default)
/// let choice = ToolChoice::Auto;
///
/// // Force Claude to use a tool
/// let choice = ToolChoice::Any;
///
/// // Force Claude to use a specific tool
/// let choice = ToolChoice::tool("get_weather");
/// ```
#[derive(Debug, Clone)]
pub enum ToolChoice {
    /// Let Claude decide whether to use tools (default behavior)
    Auto,
    /// Force Claude to use one of the provided tools
    Any,
    /// Force Claude to use a specific tool by name
    Tool(String),
}

impl ToolChoice {
    /// Create a tool choice that forces a specific tool.
    #[must_use]
    pub fn tool(name: impl Into<String>) -> Self {
        Self::Tool(name.into())
    }
}

impl Serialize for ToolChoice {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        match self {
            Self::Auto => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("type", "auto")?;
                map.end()
            }
            Self::Any => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("type", "any")?;
                map.end()
            }
            Self::Tool(name) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "tool")?;
                map.serialize_entry("name", name)?;
                map.end()
            }
        }
    }
}

/// Model alias mappings to canonical model IDs.
pub mod model_aliases {
    use std::collections::HashMap;
    use std::sync::LazyLock;

    /// Map of model aliases to canonical model IDs.
    /// Matches Go claude-gate transformer.go aliases.
    pub static ALIASES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
        let mut m = HashMap::new();

        // Claude 4.5 family
        m.insert("claude-opus-4-5", "claude-opus-4-5-20251101");
        m.insert("claude-opus-4-5-latest", "claude-opus-4-5-20251101");
        m.insert("claude-sonnet-4-5", "claude-sonnet-4-5-20250929");
        m.insert("claude-sonnet-4-5-latest", "claude-sonnet-4-5-20250929");
        m.insert("claude-haiku-4-5", "claude-haiku-4-5-20251001");
        m.insert("claude-haiku-4-5-latest", "claude-haiku-4-5-20251001");

        // Claude 4 family (current)
        m.insert("claude-sonnet-4", "claude-sonnet-4-20250514");
        m.insert("claude-sonnet-4-0", "claude-sonnet-4-20250514");
        m.insert("claude-opus-4", "claude-opus-4-20250514");
        m.insert("claude-opus-4-0", "claude-opus-4-20250514");

        // Claude 4.1 family
        m.insert("claude-opus-4-1", "claude-opus-4-1-20250414");
        m.insert("claude-opus-4-1-latest", "claude-opus-4-1-20250414");

        // Claude 3.7 family
        m.insert("claude-3-7-sonnet", "claude-3-7-sonnet-20250219");
        m.insert("claude-3-7-sonnet-latest", "claude-3-7-sonnet-20250219");

        // Claude 3.5 family
        m.insert("claude-3-5-sonnet", "claude-3-5-sonnet-20241022");
        m.insert("claude-3-5-sonnet-latest", "claude-3-5-sonnet-20241022");
        m.insert("claude-3-5-haiku", "claude-3-5-haiku-20241022");
        m.insert("claude-3-5-haiku-latest", "claude-3-5-haiku-20241022");

        // Claude 3 family
        m.insert("claude-3-opus", "claude-3-opus-20240229");
        m.insert("claude-3-opus-latest", "claude-3-opus-20240229");
        m.insert("claude-3-sonnet", "claude-3-sonnet-20240229");
        m.insert("claude-3-haiku", "claude-3-haiku-20240307");

        m
    });

    /// Resolve a model alias to its canonical ID.
    #[must_use]
    pub fn resolve(model: &str) -> &str {
        ALIASES.get(model).copied().unwrap_or(model)
    }
}

// ============================================================================
// Models API types
// ============================================================================

/// A model available via the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiModel {
    /// Unique identifier for the model
    pub id: String,
    /// Display name for the model
    #[serde(default)]
    pub display_name: String,
    /// Object type (always "model")
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unix timestamp when model was created
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Response from the /v1/models endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    /// List of available models
    pub data: Vec<ApiModel>,
    /// Whether there are more models
    #[serde(default)]
    pub has_more: bool,
    /// First model ID (for pagination)
    #[serde(default)]
    pub first_id: Option<String>,
    /// Last model ID (for pagination)
    #[serde(default)]
    pub last_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_info_expiry() {
        let token = TokenInfo::new("access".into(), "refresh".into(), 3600);
        assert!(!token.is_expired());
        assert!(!token.needs_refresh());

        let expired_token = TokenInfo {
            token_type: "oauth".into(),
            access_token: "access".into(),
            refresh_token: "refresh".into(),
            expires_at: Utc::now().timestamp() - 100,
        };
        assert!(expired_token.is_expired());
        assert!(expired_token.needs_refresh());
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello, Claude!");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.text(), "Hello, Claude!");
    }

    #[test]
    fn test_model_alias_resolution() {
        assert_eq!(
            model_aliases::resolve("claude-opus-4-5"),
            "claude-opus-4-5-20251101"
        );
        assert_eq!(
            model_aliases::resolve("claude-3-5-sonnet-20241022"),
            "claude-3-5-sonnet-20241022"
        );
    }

    #[test]
    fn test_tool_choice_serialization() {
        // Auto
        let auto = ToolChoice::Auto;
        let json = serde_json::to_value(&auto).unwrap();
        assert_eq!(json, serde_json::json!({"type": "auto"}));

        // Any
        let any = ToolChoice::Any;
        let json = serde_json::to_value(&any).unwrap();
        assert_eq!(json, serde_json::json!({"type": "any"}));

        // Specific tool
        let tool = ToolChoice::tool("get_weather");
        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json, serde_json::json!({"type": "tool", "name": "get_weather"}));
    }

    #[test]
    fn test_document_source_serialization() {
        let source = DocumentSource::pdf("dGVzdCBwZGYgZGF0YQ==");
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "base64",
                "media_type": "application/pdf",
                "data": "dGVzdCBwZGYgZGF0YQ=="
            })
        );
    }

    #[test]
    fn test_image_source_serialization() {
        let source = ImageSource::png("aW1hZ2UgZGF0YQ==");
        let json = serde_json::to_value(&source).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "base64",
                "media_type": "image/png",
                "data": "aW1hZ2UgZGF0YQ=="
            })
        );

        let url_source = ImageSource::url("https://example.com/image.png");
        let json = serde_json::to_value(&url_source).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "url",
                "url": "https://example.com/image.png"
            })
        );
    }

    #[test]
    fn test_content_block_document() {
        let block = ContentBlock::pdf_base64("cGRmIGRhdGE=");
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "document",
                "source": {
                    "type": "base64",
                    "media_type": "application/pdf",
                    "data": "cGRmIGRhdGE="
                }
            })
        );
    }

    #[test]
    fn test_content_block_image() {
        let block = ContentBlock::image_base64("aW1hZ2UgZGF0YQ==", "image/jpeg");
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": "image/jpeg",
                    "data": "aW1hZ2UgZGF0YQ=="
                }
            })
        );
    }

    #[test]
    fn test_message_with_pdf() {
        let msg = Message::with_pdf("cGRmIGRhdGE=", "Extract the text");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content.len(), 2);

        // First block should be document
        let json = serde_json::to_value(&msg.content[0]).unwrap();
        assert_eq!(json["type"], "document");

        // Second block should be text
        assert_eq!(msg.content[1].as_text(), Some("Extract the text"));
    }
}
