//! High-level Claude API client.
//!
//! This module provides [`ClaudeClient`], a convenient interface for making
//! API calls to the Anthropic Claude API with automatic OAuth authentication.

use std::pin::Pin;
use std::sync::Arc;

use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, instrument, warn};

use super::auth::{OAuthConfig, OAuthFlow, OAuthFlowState};
use super::error::{Error, Result};
use super::models::{ContentBlock, Message, MessagesResponse, Role, StreamEvent, Tool, ToolChoice, TokenInfo};
use crate::gate::storage::TokenStorage;
use super::transform::{create_headers, create_streaming_headers, transform_request};

/// Default base URL for the Anthropic API.
pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// Default request timeout in seconds.
pub const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// Claude API client with OAuth authentication.
///
/// # Example
///
/// ```rust,no_run
/// use gate::claude::{ClaudeClient, FileTokenStorage};
///
/// # async fn example() -> gate::claude::Result<()> {
/// let storage = FileTokenStorage::default_path()?;
/// let client = ClaudeClient::builder()
///     .with_storage(storage)
///     .build()?;
///
/// // Check authentication
/// if !client.is_authenticated().await? {
///     // Start OAuth flow
///     let auth_url = client.start_oauth_flow().await?;
///     println!("Open: {}", auth_url);
///     // ... get code from user ...
/// }
///
/// // Make a request
/// let response = client.messages()
///     .model("claude-sonnet-4-20250514")
///     .max_tokens(1024)
///     .user_message("Hello!")
///     .send()
///     .await?;
///
/// println!("{}", response.text());
/// # Ok(())
/// # }
/// ```
pub struct ClaudeClient<S: TokenStorage> {
    /// OAuth flow handler (includes storage).
    oauth: Arc<RwLock<OAuthFlow<S>>>,
    /// HTTP client.
    http_client: reqwest::Client,
    /// Base URL for API requests.
    base_url: String,
    /// Cached token info (for performance and proactive refresh).
    /// Stores the full TokenInfo to enable checking `needs_refresh()`.
    cached_token: Arc<RwLock<Option<TokenInfo>>>,
}

impl<S: TokenStorage + 'static> ClaudeClient<S> {
    /// Create a new client builder.
    pub fn builder() -> ClaudeClientBuilder<S> {
        ClaudeClientBuilder::new()
    }

    /// Create a new client with the given storage.
    pub fn new(storage: S) -> Result<Self> {
        Self::builder().with_storage(storage).build()
    }

    /// Check if the client has a valid authentication token.
    pub async fn is_authenticated(&self) -> Result<bool> {
        self.oauth.read().await.is_authenticated().await
    }

    /// Start the OAuth authorization flow.
    ///
    /// Returns the URL that the user should open in their browser.
    pub async fn start_oauth_flow(&self) -> Result<String> {
        let mut oauth = self.oauth.write().await;
        let (url, _state) = oauth.start_authorization()?;
        Ok(url)
    }

    /// Start OAuth flow and return both URL and state.
    ///
    /// Use this if you need the state for verification.
    pub async fn start_oauth_flow_with_state(&self) -> Result<(String, OAuthFlowState)> {
        let mut oauth = self.oauth.write().await;
        oauth.start_authorization()
    }

    /// Complete the OAuth flow by exchanging the authorization code.
    ///
    /// # Arguments
    ///
    /// * `code` - The authorization code from the OAuth callback
    /// * `state` - Optional state parameter for CSRF verification
    pub async fn complete_oauth_flow(
        &self,
        code: &str,
        state: Option<&str>,
    ) -> Result<TokenInfo> {
        let mut oauth = self.oauth.write().await;
        let token = oauth.exchange_code(code, state).await?;
        // Clear cached token to force refresh
        *self.cached_token.write().await = None;
        Ok(token.into())
    }

    /// Log out and remove stored credentials.
    pub async fn logout(&self) -> Result<()> {
        self.oauth.read().await.logout().await?;
        *self.cached_token.write().await = None;
        Ok(())
    }

    /// Get the current token info, if authenticated.
    ///
    /// Returns `None` if not authenticated. Use this to check token expiry,
    /// time remaining, etc.
    pub async fn get_token_info(&self) -> Result<Option<TokenInfo>> {
        let res = self.oauth.read().await.storage().load("anthropic").await?;
        Ok(res.map(Into::into))
    }

    /// Get a valid access token, refreshing if necessary.
    ///
    /// Proactively refreshes tokens ~5 minutes before expiry to avoid
    /// request failures due to mid-request token expiration.
    async fn get_access_token(&self) -> Result<String> {
        // Try cache first - but check if it needs refresh
        {
            let cached = self.cached_token.read().await;
            if let Some(ref token_info) = *cached {
                if !token_info.needs_refresh() {
                    debug!("Using cached token (expires in {} seconds)", token_info.time_until_expiry());
                    return Ok(token_info.access_token.clone());
                }
                debug!("Cached token needs refresh (expires in {} seconds)", token_info.time_until_expiry());
            }
        }

        // Get fresh token from OAuth flow (handles refresh automatically)
        let oauth = self.oauth.read().await;
        let access_token = oauth.get_access_token().await?;

        // Load the full token info to cache for future refresh checks
        if let Some(token_info) = oauth.storage().load("anthropic").await? {
            *self.cached_token.write().await = Some(token_info.into());
        }

        Ok(access_token)
    }

    /// Create a messages request builder.
    #[must_use]
    pub fn messages(&self) -> MessagesRequestBuilder<'_, S> {
        MessagesRequestBuilder::new(self)
    }

    /// Make a raw API request.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method
    /// * `path` - API path (e.g., "/v1/messages")
    /// * `body` - Optional request body
    #[instrument(skip(self, body))]
    pub async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<reqwest::Response> {
        let access_token = self.get_access_token().await?;
        let url = format!("{}{}", self.base_url, path);
        let headers = create_headers(&access_token);

        let mut request = self.http_client.request(method.clone(), &url).headers(headers);

        if let Some(body) = body {
            let transformed = transform_request(body);
            debug!(url = %url, "Making API request");
            request = request.json(&transformed);
        }

        let response = request.send().await?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status().as_u16();

            // Try to parse error response
            let body = response.text().await.unwrap_or_default();
            warn!(status, body = %body, "API request failed");

            // Check if it's an auth error - clear cache
            if status == 401 {
                *self.cached_token.write().await = None;
            }

            return Err(Error::api(status, body, None));
        }

        Ok(response)
    }

    /// Make a streaming API request.
    #[instrument(skip(self, body))]
    pub async fn request_stream(
        &self,
        path: &str,
        body: Value,
    ) -> Result<SseStream> {
        let access_token = self.get_access_token().await?;
        let url = format!("{}{}", self.base_url, path);
        // Use streaming-specific headers (Connection: close, Cache-Control: no-cache)
        let headers = create_streaming_headers(&access_token);

        // Ensure streaming is enabled
        let mut body = transform_request(body);
        body["stream"] = Value::Bool(true);

        debug!(url = %url, "Starting streaming request");

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::api(status, body, None));
        }

        Ok(SseStream::new(response))
    }

    /// List available models from the API.
    ///
    /// Returns a list of models that can be used with the messages API.
    #[instrument(skip(self))]
    pub async fn list_models(&self) -> Result<Vec<super::models::ApiModel>> {
        let response = self
            .request(reqwest::Method::GET, "/v1/models", None)
            .await?;

        let models_response: super::models::ModelsResponse = response.json().await?;
        Ok(models_response.data)
    }
}

/// Builder for [`ClaudeClient`].
pub struct ClaudeClientBuilder<S: TokenStorage> {
    storage: Option<S>,
    oauth_config: OAuthConfig,
    base_url: String,
    timeout_secs: u64,
}

impl<S: TokenStorage + 'static> ClaudeClientBuilder<S> {
    /// Create a new builder.
    fn new() -> Self {
        Self {
            storage: None,
            oauth_config: OAuthConfig::default(),
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }

    /// Set the token storage backend.
    #[must_use]
    pub fn with_storage(mut self, storage: S) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Set a custom OAuth configuration.
    #[must_use]
    pub fn with_oauth_config(mut self, config: OAuthConfig) -> Self {
        self.oauth_config = config;
        self
    }

    /// Set a custom base URL.
    #[must_use]
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set the request timeout in seconds.
    #[must_use]
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Build the client.
    ///
    /// # Errors
    ///
    /// Returns an error if no storage was provided.
    pub fn build(self) -> Result<ClaudeClient<S>> {
        let storage = self
            .storage
            .ok_or_else(|| Error::config("Token storage is required"))?;

        let oauth = OAuthFlow::with_config(storage, self.oauth_config);

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()?;

        Ok(ClaudeClient {
            oauth: Arc::new(RwLock::new(oauth)),
            http_client,
            base_url: self.base_url,
            cached_token: Arc::new(RwLock::new(None)),
        })
    }
}

/// Builder for messages API requests.
pub struct MessagesRequestBuilder<'a, S: TokenStorage> {
    client: &'a ClaudeClient<S>,
    model: Option<String>,
    messages: Vec<Message>,
    system: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<u32>,
    stop_sequences: Option<Vec<String>>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<ToolChoice>,
    stream: bool,
    metadata: Option<Value>,
}

impl<'a, S: TokenStorage + 'static> MessagesRequestBuilder<'a, S> {
    fn new(client: &'a ClaudeClient<S>) -> Self {
        Self {
            client,
            model: None,
            messages: Vec::new(),
            system: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            stream: false,
            metadata: None,
        }
    }

    /// Set the model to use.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a user message.
    #[must_use]
    pub fn user_message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content));
        self
    }

    /// Add an assistant message.
    #[must_use]
    pub fn assistant_message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(content));
        self
    }

    /// Add a message with custom content blocks.
    #[must_use]
    pub fn message(mut self, role: Role, content: Vec<ContentBlock>) -> Self {
        self.messages.push(Message::with_content(role, content));
        self
    }

    /// Add multiple messages.
    #[must_use]
    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Add a user message with a PDF document and text prompt.
    ///
    /// # Arguments
    ///
    /// * `pdf_base64` - Base64-encoded PDF data
    /// * `prompt` - Text prompt describing what to do with the document
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use gate::claude::{ClaudeClient, FileTokenStorage};
    /// # async fn example() -> gate::claude::Result<()> {
    /// # let storage = FileTokenStorage::default_path()?;
    /// # let client = ClaudeClient::builder().with_storage(storage).build()?;
    /// use base64::{Engine, engine::general_purpose::STANDARD};
    ///
    /// let pdf_bytes = std::fs::read("document.pdf")?;
    /// let pdf_base64 = STANDARD.encode(&pdf_bytes);
    ///
    /// let response = client.messages()
    ///     .model("claude-sonnet-4-20250514")
    ///     .max_tokens(8192)
    ///     .pdf_message(&pdf_base64, "Extract the text from this PDF")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn pdf_message(mut self, pdf_base64: impl Into<String>, prompt: impl Into<String>) -> Self {
        self.messages.push(Message::with_pdf(pdf_base64, prompt));
        self
    }

    /// Add a user message with a document and text prompt.
    ///
    /// # Arguments
    ///
    /// * `doc_base64` - Base64-encoded document data
    /// * `media_type` - MIME type of the document (e.g., "application/pdf")
    /// * `prompt` - Text prompt describing what to do with the document
    #[must_use]
    pub fn document_message(
        mut self,
        doc_base64: impl Into<String>,
        media_type: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        self.messages
            .push(Message::with_document(doc_base64, media_type, prompt));
        self
    }

    /// Add a user message with an image and text prompt (vision).
    ///
    /// # Arguments
    ///
    /// * `image_base64` - Base64-encoded image data
    /// * `media_type` - MIME type of the image (e.g., "image/png", "image/jpeg")
    /// * `prompt` - Text prompt describing what to do with the image
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use gate::claude::{ClaudeClient, FileTokenStorage};
    /// # async fn example() -> gate::claude::Result<()> {
    /// # let storage = FileTokenStorage::default_path()?;
    /// # let client = ClaudeClient::builder().with_storage(storage).build()?;
    /// use base64::{Engine, engine::general_purpose::STANDARD};
    ///
    /// let image_bytes = std::fs::read("image.png")?;
    /// let image_base64 = STANDARD.encode(&image_bytes);
    ///
    /// let response = client.messages()
    ///     .model("claude-sonnet-4-20250514")
    ///     .max_tokens(1024)
    ///     .image_message(&image_base64, "image/png", "What's in this image?")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn image_message(
        mut self,
        image_base64: impl Into<String>,
        media_type: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        self.messages
            .push(Message::with_image(image_base64, media_type, prompt));
        self
    }

    /// Set the system prompt.
    #[must_use]
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the maximum number of tokens to generate.
    #[must_use]
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the sampling temperature (0.0 to 1.0).
    #[must_use]
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top_p for nucleus sampling.
    #[must_use]
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set top_k for sampling.
    #[must_use]
    pub fn top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Set stop sequences.
    #[must_use]
    pub fn stop_sequences(mut self, sequences: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.stop_sequences = Some(sequences.into_iter().map(Into::into).collect());
        self
    }

    /// Set available tools for function calling.
    #[must_use]
    pub fn tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        self.tools = Some(tools.into_iter().collect());
        self
    }

    /// Set the tool choice strategy.
    ///
    /// Controls how Claude uses the provided tools:
    /// - `ToolChoice::Auto` - Claude decides whether to use tools (default)
    /// - `ToolChoice::Any` - Force Claude to use one of the tools
    /// - `ToolChoice::Tool(name)` - Force Claude to use a specific tool
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use gate::claude::{ClaudeClient, FileTokenStorage, ToolChoice};
    /// # async fn example() -> gate::claude::Result<()> {
    /// # let storage = FileTokenStorage::default_path()?;
    /// # let client = ClaudeClient::builder().with_storage(storage).build()?;
    /// let response = client.messages()
    ///     .model("claude-sonnet-4-20250514")
    ///     .max_tokens(1024)
    ///     .tools([/* ... */])
    ///     .tool_choice(ToolChoice::Any)  // Force tool use
    ///     .user_message("What's the weather?")
    ///     .send()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Enable streaming mode.
    #[must_use]
    pub fn stream(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Set request metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Build the request body.
    fn build_body(&self) -> Result<Value> {
        let model = self
            .model
            .as_ref()
            .ok_or_else(|| Error::config("Model is required"))?;

        let max_tokens = self.max_tokens.unwrap_or(4096);

        let mut body = serde_json::json!({
            "model": model,
            "messages": self.messages,
            "max_tokens": max_tokens,
        });

        if let Some(ref system) = self.system {
            body["system"] = Value::String(system.clone());
        }

        if let Some(temp) = self.temperature {
            body["temperature"] = Value::from(temp);
        }

        if let Some(top_p) = self.top_p {
            body["top_p"] = Value::from(top_p);
        }

        if let Some(top_k) = self.top_k {
            body["top_k"] = Value::from(top_k);
        }

        if let Some(ref stop) = self.stop_sequences {
            body["stop_sequences"] = serde_json::to_value(stop)?;
        }

        if let Some(ref tools) = self.tools {
            body["tools"] = serde_json::to_value(tools)?;
        }

        if let Some(ref tool_choice) = self.tool_choice {
            body["tool_choice"] = serde_json::to_value(tool_choice)?;
        }

        if self.stream {
            body["stream"] = Value::Bool(true);
        }

        if let Some(ref metadata) = self.metadata {
            body["metadata"] = metadata.clone();
        }

        Ok(body)
    }

    /// Send the request and get the response.
    pub async fn send(self) -> Result<MessagesResponse> {
        if self.stream {
            return Err(Error::config(
                "Use send_stream() for streaming requests",
            ));
        }

        let body = self.build_body()?;
        let response = self
            .client
            .request(reqwest::Method::POST, "/v1/messages", Some(body))
            .await?;

        let response: MessagesResponse = response.json().await?;
        Ok(response)
    }

    /// Send the request and get a streaming response.
    pub async fn send_stream(mut self) -> Result<impl Stream<Item = Result<StreamEvent>>> {
        self.stream = true;
        let body = self.build_body()?;
        self.client.request_stream("/v1/messages", body).await
    }
}

/// Request body for the messages API (for direct use).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesRequest {
    /// Model to use
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// System prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Sampling temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p nucleus sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Enable streaming
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Available tools
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Request metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Server-Sent Events stream for handling streaming responses.
///
/// This stream yields [`StreamEvent`] items parsed from the SSE response.
pub struct SseStream {
    inner: Pin<Box<dyn Stream<Item = Result<StreamEvent>> + Send>>,
}

impl SseStream {
    fn new(response: reqwest::Response) -> Self {
        let stream = async_stream::stream! {
            let mut buffer = String::new();
            let mut byte_buffer = Vec::new();
            let mut byte_stream = response.bytes_stream();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        byte_buffer.extend_from_slice(&chunk);

                        let valid_len = match std::str::from_utf8(&byte_buffer) {
                            Ok(_) => byte_buffer.len(),
                            Err(e) => e.valid_up_to(),
                        };

                        // Safety: we just verified these bytes are valid UTF-8
                        let text = unsafe { std::str::from_utf8_unchecked(&byte_buffer[..valid_len]) };
                        buffer.push_str(text);

                        // Remove processed bytes, keeping any incomplete suffix
                        byte_buffer.drain(..valid_len);

                        // Parse events from buffer
                        while let Some((event_type, data)) = extract_sse_event(&mut buffer) {
                            if event_type == "message_stop" {
                                yield Ok(StreamEvent::MessageStop);
                                return;
                            }

                            if let Some(data) = data {
                                match serde_json::from_str::<StreamEvent>(&data) {
                                    Ok(event) => yield Ok(event),
                                    Err(e) => {
                                        warn!(error = %e, "Failed to parse SSE event");
                                        yield Err(Error::Api { status: 0, message: format!("SSE parse error: {}", e), error_type: None });
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(Error::Http(e));
                        return;
                    }
                }
            }
        };

        Self {
            inner: Box::pin(stream),
        }
    }

    /// Collect all text deltas from the stream into a single string.
    ///
    /// This is a convenience method for when you just want the final text.
    pub async fn collect_text(mut self) -> Result<String> {
        let mut text = String::new();

        while let Some(event) = self.next().await {
            if let StreamEvent::ContentBlockDelta { delta, .. } = event? {
                if let crate::gate::claude::models::ContentDelta::TextDelta { text: t } = delta {
                    text.push_str(&t);
                }
            }
        }

        Ok(text)
    }
}

impl Stream for SseStream {
    type Item = Result<StreamEvent>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

/// Extract a single SSE event from the buffer.
fn extract_sse_event(buffer: &mut String) -> Option<(String, Option<String>)> {
    // Find the end of an event (double newline)
    let event_end = buffer.find("\n\n")?;
    let event_text = buffer[..event_end].to_string();
    buffer.drain(..event_end + 2);

    let mut event_type = String::new();
    let mut data = None;

    for line in event_text.lines() {
        if let Some(value) = line.strip_prefix("event: ") {
            event_type = value.to_string();
        } else if let Some(value) = line.strip_prefix("data: ") {
            data = Some(value.to_string());
        }
    }

    Some((event_type, data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::storage::MemoryTokenStorage;

    #[test]
    fn test_client_builder() {
        let storage = MemoryTokenStorage::new();
        let client = ClaudeClient::builder()
            .with_storage(storage)
            .with_base_url("https://custom.api.com")
            .with_timeout(300)
            .build()
            .unwrap();

        assert_eq!(client.base_url, "https://custom.api.com");
    }

    #[test]
    fn test_messages_builder() {
        let storage = MemoryTokenStorage::new();
        let client = ClaudeClient::builder()
            .with_storage(storage)
            .build()
            .unwrap();

        let builder = client
            .messages()
            .model("claude-sonnet-4-20250514")
            .max_tokens(1024)
            .user_message("Hello!")
            .system("Be helpful.");

        let body = builder.build_body().unwrap();
        assert_eq!(body["model"], "claude-sonnet-4-20250514");
        assert_eq!(body["max_tokens"], 1024);
    }
}
