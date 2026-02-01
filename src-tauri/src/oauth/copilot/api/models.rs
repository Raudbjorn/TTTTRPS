//! Models listing API.
//!
//! This module provides functionality for listing available models
//! from the Copilot API, with optional caching to reduce redundant requests.

use reqwest::Method;
use tracing::{debug, instrument};

use crate::oauth::copilot::client::CopilotClient;
use crate::oauth::copilot::error::Result;
use crate::oauth::copilot::models::ModelsResponse;
use crate::oauth::copilot::storage::CopilotTokenStorage;

impl<S: CopilotTokenStorage> CopilotClient<S> {
    /// Lists all available models.
    ///
    /// This method always makes a fresh API request.
    /// For repeated calls, consider using [`models_cached()`](Self::models_cached).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not authenticated
    /// - Network error
    /// - API returns an error response
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crate::oauth::copilot::CopilotClient;
    /// # async fn example() -> crate::oauth::copilot::Result<()> {
    /// let client = CopilotClient::builder().build()?;
    /// let models = client.models().await?;
    ///
    /// // Find models that support chat
    /// let chat_models = models.chat_models();
    /// println!("Found {} chat models", chat_models.len());
    ///
    /// // Find a specific model
    /// if let Some(gpt4o) = models.find("gpt-4o") {
    ///     println!("GPT-4o max context: {}", gpt4o.max_context_tokens());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self))]
    pub async fn models(&self) -> Result<ModelsResponse> {
        debug!("Fetching models list");

        let models_response: ModelsResponse = self.request(Method::GET, "/models", None).await?;

        debug!(
            count = models_response.len(),
            "Models list fetched successfully"
        );

        // Update cache
        self.cache_models(models_response.clone()).await;

        Ok(models_response)
    }

    /// Lists all available models, using a session cache.
    ///
    /// This method returns cached results if available, otherwise
    /// makes a fresh API request and caches the result.
    ///
    /// The cache is session-scoped (cleared when the client is dropped).
    ///
    /// # Cache Behavior
    ///
    /// - First call: Makes API request, caches result, returns response
    /// - Subsequent calls: Returns cached response (no network request)
    /// - After [`clear_models_cache()`](Self::clear_models_cache): Makes fresh request
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not authenticated
    /// - Network error (on cache miss)
    /// - API returns an error response (on cache miss)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crate::oauth::copilot::CopilotClient;
    /// # async fn example() -> crate::oauth::copilot::Result<()> {
    /// let client = CopilotClient::builder().build()?;
    ///
    /// // First call fetches from API
    /// let models = client.models_cached().await?;
    ///
    /// // Second call returns cached data (no network request)
    /// let models = client.models_cached().await?;
    ///
    /// // Force refresh by clearing cache
    /// client.clear_models_cache().await;
    /// let models = client.models_cached().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self))]
    pub async fn models_cached(&self) -> Result<ModelsResponse> {
        // Check cache first
        if let Some(cached) = self.get_cached_models().await {
            debug!("Returning cached models list");
            return Ok(cached);
        }

        // Cache miss - fetch from API
        self.models().await
    }
}

#[cfg(test)]
mod tests {
    use crate::oauth::copilot::models::ModelsResponse;

    #[test]
    fn test_models_response_structure() {
        let json = r#"{
            "object": "list",
            "data": [
                {
                    "id": "gpt-4o",
                    "object": "model",
                    "created": 1700000000,
                    "owned_by": "openai",
                    "capabilities": {
                        "family": "gpt-4o",
                        "type": "chat",
                        "supports": {
                            "chat_completions": true,
                            "tool_calls": true,
                            "vision": true
                        },
                        "limits": {
                            "max_context_window_tokens": 128000,
                            "max_output_tokens": 4096
                        }
                    }
                }
            ]
        }"#;

        let response: ModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.len(), 1);
        assert!(response.find("gpt-4o").is_some());
        assert!(!response.chat_models().is_empty());
    }
}
