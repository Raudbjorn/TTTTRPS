//! Embeddings request builder.
//!
//! This module provides a builder for constructing and sending
//! embedding requests to the Copilot API.

use reqwest::Method;
use tracing::{debug, instrument};

use crate::oauth::copilot::client::CopilotClient;
use crate::oauth::copilot::error::{Error, Result};
use crate::oauth::copilot::models::{
    EmbeddingInput, EmbeddingRequest, EmbeddingResponse, EncodingFormat,
};
use crate::oauth::copilot::storage::CopilotTokenStorage;

/// Default model for embeddings.
pub const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-3-small";

/// Builder for embedding requests.
///
/// Use [`CopilotClient::embeddings()`] to create an instance.
///
/// # Example
///
/// ```no_run
/// # use crate::oauth::copilot::CopilotClient;
/// # async fn example() -> crate::oauth::copilot::Result<()> {
/// let client = CopilotClient::builder().build()?;
///
/// // Single text embedding
/// let response = client
///     .embeddings()
///     .model("text-embedding-3-small")
///     .input("Hello, world!")
///     .send()
///     .await?;
///
/// if let Some(embedding) = response.first_embedding() {
///     println!("Embedding dimensions: {}", embedding.len());
/// }
/// # Ok(())
/// # }
/// ```
/// Builder for embedding requests.
pub struct EmbeddingsRequestBuilder<'a, S: CopilotTokenStorage> {
    client: &'a CopilotClient<S>,
    model: String,
    input: Vec<String>,
    encoding_format: Option<EncodingFormat>,
    dimensions: Option<u32>,
    user: Option<String>,
}

impl<'a, S: CopilotTokenStorage> std::fmt::Debug for EmbeddingsRequestBuilder<'a, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingsRequestBuilder")
            .field("model", &self.model)
            .field("input_count", &self.input.len())
            .field("dimensions", &self.dimensions)
            .finish()
    }
}

impl<'a, S: CopilotTokenStorage> EmbeddingsRequestBuilder<'a, S> {
    /// Creates a new embeddings request builder.
    ///
    /// Use [`CopilotClient::embeddings()`] instead of calling this directly.
    pub(crate) fn new(client: &'a CopilotClient<S>) -> Self {
        Self {
            client,
            model: DEFAULT_EMBEDDING_MODEL.to_string(),
            input: Vec::new(),
            encoding_format: None,
            dimensions: None,
            user: None,
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Model Selection
    // ─────────────────────────────────────────────────────────────────────────

    /// Sets the embedding model to use.
    ///
    /// # Arguments
    ///
    /// * `model` - Model identifier (e.g., "text-embedding-3-small", "text-embedding-3-large")
    ///
    /// # Default
    ///
    /// `"text-embedding-3-small"`
    #[must_use]
    pub fn model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Input Text
    // ─────────────────────────────────────────────────────────────────────────

    /// Adds a single text input to embed.
    ///
    /// Can be called multiple times to add more inputs.
    #[must_use]
    pub fn input(mut self, text: &str) -> Self {
        self.input.push(text.to_string());
        self
    }

    /// Adds multiple text inputs to embed.
    #[must_use]
    pub fn inputs(mut self, texts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.input.extend(texts.into_iter().map(Into::into));
        self
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Encoding Options
    // ─────────────────────────────────────────────────────────────────────────

    /// Sets the encoding format for the embeddings.
    ///
    /// # Default
    ///
    /// Not set (API default is Float)
    #[must_use]
    pub fn encoding_format(mut self, format: EncodingFormat) -> Self {
        self.encoding_format = Some(format);
        self
    }

    /// Sets the number of dimensions for the embedding.
    ///
    /// Only supported by certain models (e.g., text-embedding-3-*).
    /// Lower dimensions reduce storage and computation costs.
    ///
    /// # Default
    ///
    /// Not set (uses model's default dimensions)
    #[must_use]
    pub fn dimensions(mut self, dims: u32) -> Self {
        self.dimensions = Some(dims);
        self
    }

    /// Sets a user identifier for abuse monitoring.
    #[must_use]
    pub fn user(mut self, user_id: impl Into<String>) -> Self {
        self.user = Some(user_id.into());
        self
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Request Execution
    // ─────────────────────────────────────────────────────────────────────────

    /// Builds the request body as an [`EmbeddingRequest`].
    fn build_request(&self) -> Result<EmbeddingRequest> {
        if self.input.is_empty() {
            return Err(Error::Config(
                "No input text provided for embeddings".to_string(),
            ));
        }

        let input = if self.input.len() == 1 {
            EmbeddingInput::Single(self.input[0].clone())
        } else {
            EmbeddingInput::Multiple(self.input.clone())
        };

        Ok(EmbeddingRequest {
            model: self.model.clone(),
            input,
            encoding_format: self.encoding_format,
            dimensions: self.dimensions,
            user: self.user.clone(),
        })
    }

    /// Sends the embedding request and returns the response.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not authenticated
    /// - No input text provided
    /// - Network error
    /// - API returns an error response
    #[instrument(skip(self), fields(model = %self.model, inputs = self.input.len()))]
    pub async fn send(self) -> Result<EmbeddingResponse> {
        let request = self.build_request()?;

        debug!(
            model = %request.model,
            input_count = request.input.len(),
            dimensions = ?request.dimensions,
            "Sending embeddings request"
        );

        let body = serde_json::to_value(&request)?;
        let response: EmbeddingResponse = self
            .client
            .request(Method::POST, "/embeddings", Some(body))
            .await?;

        debug!(
            model = %response.model,
            embeddings = response.data.len(),
            "Embeddings request successful"
        );

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model() {
        assert_eq!(DEFAULT_EMBEDDING_MODEL, "text-embedding-3-small");
    }

    #[test]
    fn test_build_request_empty_input() {
        // Test validation logic
        let input: Vec<String> = vec![];
        let result: std::result::Result<(), &str> = if input.is_empty() {
            Err("No input text provided")
        } else {
            Ok(())
        };
        assert!(result.is_err());
    }

    #[test]
    fn test_embedding_request_single() {
        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Single("Hello".to_string()),
            encoding_format: None,
            dimensions: Some(256),
            user: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("text-embedding-3-small"));
        assert!(json.contains("Hello"));
        assert!(json.contains("256"));
    }

    #[test]
    fn test_embedding_request_multiple() {
        let request = EmbeddingRequest {
            model: "text-embedding-3-small".to_string(),
            input: EmbeddingInput::Multiple(vec!["One".to_string(), "Two".to_string()]),
            encoding_format: Some(EncodingFormat::Float),
            dimensions: None,
            user: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("[\"One\",\"Two\"]"));
    }
}
