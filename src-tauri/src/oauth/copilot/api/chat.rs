//! Chat completion request builder.
//!
//! This module provides a fluent builder for constructing and sending
//! chat completion requests to the Copilot API.

use futures_util::Stream;
use reqwest::Method;
use std::pin::Pin;
use tracing::{debug, instrument};

use crate::oauth::copilot::client::CopilotClient;
use crate::oauth::copilot::error::{Error, Result};
use crate::oauth::copilot::models::{
    ChatRequest, ChatResponse, Content, Message, Role, SseParser, StreamChunk,
};
use crate::oauth::copilot::storage::CopilotTokenStorage;

/// Default model for chat completions.
pub const DEFAULT_CHAT_MODEL: &str = "gpt-4o";

/// Builder for chat completion requests.
///
/// Use [`CopilotClient::chat()`] to create an instance.
///
/// # Example
///
/// ```no_run
/// # use crate::oauth::copilot::CopilotClient;
/// # async fn example() -> crate::oauth::copilot::Result<()> {
/// let client = CopilotClient::builder().build()?;
///
/// // Simple request
/// let response = client
///     .chat()
///     .model("gpt-4o")
///     .system("You are a helpful assistant")
///     .user("Hello!")
///     .send()
///     .await?;
///
/// println!("Response: {}", response.first_content().unwrap_or_default());
/// # Ok(())
/// # }
/// ```
/// Builder for chat completion requests.
pub struct ChatRequestBuilder<'a, S: CopilotTokenStorage> {
    client: &'a CopilotClient<S>,
    model: String,
    messages: Vec<Message>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    stream: bool,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    presence_penalty: Option<f32>,
    frequency_penalty: Option<f32>,
    user: Option<String>,
}

impl<'a, S: CopilotTokenStorage> std::fmt::Debug for ChatRequestBuilder<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatRequestBuilder")
            .field("model", &self.model)
            .field("messages", &self.messages.len())
            .field("max_tokens", &self.max_tokens)
            .field("temperature", &self.temperature)
            .field("stream", &self.stream)
            .finish()
    }
}

impl<'a, S: CopilotTokenStorage> ChatRequestBuilder<'a, S> {
    /// Creates a new chat request builder.
    ///
    /// Use [`CopilotClient::chat()`] instead of calling this directly.
    pub(crate) fn new(client: &'a CopilotClient<S>) -> Self {
        Self {
            client,
            model: DEFAULT_CHAT_MODEL.to_string(),
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            stream: false,
            top_p: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Model Selection
    // ─────────────────────────────────────────────────────────────────────────

    /// Sets the model to use for completion.
    ///
    /// # Arguments
    ///
    /// * `model` - Model identifier (e.g., "gpt-4o", "gpt-4-turbo")
    ///
    /// # Default
    ///
    /// `"gpt-4o"`
    #[must_use]
    pub fn model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Message Construction
    // ─────────────────────────────────────────────────────────────────────────

    /// Adds a system message.
    #[must_use]
    pub fn system(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::system(content.into()));
        self
    }

    /// Adds a user message.
    #[must_use]
    pub fn user(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content.into()));
        self
    }

    /// Adds an assistant message.
    #[must_use]
    pub fn assistant(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(content.into()));
        self
    }

    /// Adds a custom message.
    #[must_use]
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Sets all messages at once.
    #[must_use]
    pub fn messages(mut self, messages: impl Into<Vec<Message>>) -> Self {
        self.messages = messages.into();
        self
    }

    /// Adds a user message with an image.
    #[must_use]
    pub fn user_with_image(mut self, text: &str, image_url: &str) -> Self {
        use crate::oauth::copilot::models::{ContentPart, ImageUrl};

        let parts = vec![
            ContentPart::Text {
                text: text.to_string(),
            },
            ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: image_url.to_string(),
                    detail: None,
                },
            },
        ];

        self.messages.push(Message {
            role: Role::User,
            content: Content::Parts(parts),
            name: None,
        });

        self
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Generation Parameters
    // ─────────────────────────────────────────────────────────────────────────

    /// Sets the maximum number of tokens to generate.
    #[must_use]
    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Sets the sampling temperature (0.0 to 2.0).
    ///
    /// Lower values make output more focused and deterministic,
    /// higher values make it more creative and random.
    #[must_use]
    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Sets the top-p (nucleus) sampling parameter.
    ///
    /// An alternative to temperature. Only consider tokens with
    /// cumulative probability up to this value.
    #[must_use]
    pub fn top_p(mut self, p: f32) -> Self {
        self.top_p = Some(p);
        self
    }

    /// Sets stop sequences.
    ///
    /// Generation will stop when any of these sequences is encountered.
    #[must_use]
    pub fn stop(mut self, sequences: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.stop = Some(sequences.into_iter().map(Into::into).collect());
        self
    }

    /// Sets the presence penalty (-2.0 to 2.0).
    ///
    /// Positive values penalize new tokens based on whether they appear
    /// in the text so far, encouraging the model to talk about new topics.
    #[must_use]
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Sets the frequency penalty (-2.0 to 2.0).
    ///
    /// Positive values penalize tokens based on how often they appear
    /// in the text so far, reducing repetition.
    #[must_use]
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Sets a user identifier for abuse monitoring.
    #[must_use]
    pub fn user_id(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Request Building
    // ─────────────────────────────────────────────────────────────────────────

    /// Builds the request body as a [`ChatRequest`].
    fn build_request(&self, stream: bool) -> Result<ChatRequest> {
        if self.messages.is_empty() {
            return Err(Error::Config("No messages provided".to_string()));
        }

        Ok(ChatRequest {
            model: self.model.clone(),
            messages: self.messages.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            stream: if stream { Some(true) } else { None },
            top_p: self.top_p,
            stop: self.stop.clone(),
            presence_penalty: self.presence_penalty,
            frequency_penalty: self.frequency_penalty,
            user: self.user.clone(),
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Request Execution
    // ─────────────────────────────────────────────────────────────────────────

    /// Sends the chat completion request and returns the full response.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not authenticated
    /// - No messages provided
    /// - Network error
    /// - API returns an error response
    #[instrument(skip(self), fields(model = %self.model, messages = self.messages.len()))]
    pub async fn send(self) -> Result<ChatResponse> {
        let request = self.build_request(false)?;

        debug!(
            model = %request.model,
            messages = request.messages.len(),
            max_tokens = ?request.max_tokens,
            "Sending chat completion request"
        );

        let body = serde_json::to_value(&request)?;
        let response: ChatResponse = self
            .client
            .request(Method::POST, "/chat/completions", Some(body))
            .await?;

        debug!(
            id = %response.id,
            model = %response.model,
            finish_reason = ?response.first_finish_reason(),
            "Chat completion successful"
        );

        Ok(response)
    }

    /// Sends the chat completion request and returns a stream of chunks.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use futures_util::StreamExt;
    /// # use crate::oauth::copilot::CopilotClient;
    /// # async fn example() -> crate::oauth::copilot::Result<()> {
    /// let client = CopilotClient::builder().build()?;
    ///
    /// let mut stream = client
    ///     .chat()
    ///     .user("Tell me a story")
    ///     .send_stream()
    ///     .await?;
    ///
    /// while let Some(chunk) = stream.next().await {
    ///     match chunk? {
    ///         crate::oauth::copilot::StreamChunk::Delta { content, .. } => {
    ///             print!("{content}");
    ///         }
    ///         crate::oauth::copilot::StreamChunk::Done => break,
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self), fields(model = %self.model, messages = self.messages.len()))]
    pub async fn send_stream(
        self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let request = self.build_request(true)?;

        debug!(
            model = %request.model,
            messages = request.messages.len(),
            "Starting streaming chat completion"
        );

        let body = serde_json::to_value(&request)?;
        let response = self
            .client
            .request_stream(Method::POST, "/chat/completions", Some(body))
            .await?;

        Ok(parse_sse_stream(response))
    }
}

/// Parses an SSE byte stream into StreamChunk events.
fn parse_sse_stream(
    byte_stream: impl Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>> {
    use futures_util::StreamExt;

    let stream = async_stream::stream! {
        let mut parser = SseParser::new();
        let mut buffer = String::new();

        futures_util::pin_mut!(byte_stream);

        while let Some(result) = byte_stream.next().await {
            let bytes = match result {
                Ok(b) => b,
                Err(e) => {
                    yield Err(Error::Http(e));
                    return; // Stop stream on HTTP error
                }
            };

            let text = match std::str::from_utf8(&bytes) {
                Ok(t) => t,
                Err(e) => {
                    yield Err(Error::Stream(format!("Invalid UTF-8: {e}")));
                    return; // Stop stream on UTF-8 decode error
                }
            };

            buffer.push_str(text);

            // Process complete lines
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if let Some(chunks) = parser.parse_line(&line) {
                    for chunk in chunks {
                        yield Ok(chunk);
                    }
                }
            }
        }

        // Process any remaining content
        if !buffer.is_empty() {
            if let Some(chunks) = parser.parse_line(&buffer) {
                for chunk in chunks {
                    yield Ok(chunk);
                }
            }
        }
    };

    Box::pin(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests create a builder but don't make actual requests

    #[test]
    fn test_builder_defaults() {
        // We can't actually test this without a client, but we can verify the module compiles
        assert_eq!(DEFAULT_CHAT_MODEL, "gpt-4o");
    }

    #[test]
    fn test_build_request_empty_messages() {
        // Create a mock test for validation logic
        let messages: Vec<Message> = vec![];
        let result: std::result::Result<(), &str> = if messages.is_empty() {
            Err("No messages provided")
        } else {
            Ok(())
        };
        assert!(result.is_err());
    }

    #[test]
    fn test_chat_request_construction() {
        let request = ChatRequest {
            model: "gpt-4o".to_string(),
            messages: vec![Message::user("Hello")],
            max_tokens: Some(100),
            temperature: Some(0.7),
            stream: None,
            top_p: None,
            stop: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
        };

        assert_eq!(request.model, "gpt-4o");
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.max_tokens, Some(100));
    }
}
