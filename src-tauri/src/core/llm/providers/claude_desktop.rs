//! Claude Desktop Provider via CDP
//!
//! Provides LLM access through Claude Desktop's Chrome DevTools Protocol (CDP).
//! This is useful for development/testing without API costs, using your existing
//! Claude Desktop subscription.
//!
//! ## Limitations
//!
//! - No streaming support (CDP gives full responses only)
//! - No token counting (can't track costs)
//! - Depends on UI selectors that may break with updates
//! - Slower than direct API access
//!
//! ## Usage
//!
//! ```rust,no_run
//! use crate::core::llm::providers::ClaudeDesktopProvider;
//!
//! // Create provider with default config
//! let provider = ClaudeDesktopProvider::new();
//!
//! // Or with custom port
//! let provider = ClaudeDesktopProvider::with_port(9333);
//! ```

use crate::core::claude_cdp::{ClaudeConfig, ClaudeDesktopManager};
use crate::core::llm::cost::ProviderPricing;
use crate::core::llm::router::{
    ChatChunk, ChatRequest, ChatResponse, LLMError, LLMProvider, Result,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};

/// Claude Desktop provider using CDP.
pub struct ClaudeDesktopProvider {
    manager: Arc<Mutex<ClaudeDesktopManager>>,
    port: u16,
    timeout_secs: u64,
}

impl ClaudeDesktopProvider {
    /// Create a new provider with default configuration.
    pub fn new() -> Self {
        Self::with_config(9333, 120)
    }

    /// Create a new provider with custom port.
    pub fn with_port(port: u16) -> Self {
        Self::with_config(port, 120)
    }

    /// Create a new provider with custom port and timeout.
    pub fn with_config(port: u16, timeout_secs: u64) -> Self {
        let config = ClaudeConfig::default()
            .with_port(port)
            .with_timeout(timeout_secs);

        Self {
            manager: Arc::new(Mutex::new(ClaudeDesktopManager::with_config(config))),
            port,
            timeout_secs,
        }
    }

    /// Get the manager for direct access.
    pub fn manager(&self) -> Arc<Mutex<ClaudeDesktopManager>> {
        self.manager.clone()
    }

    /// Connect to Claude Desktop.
    pub async fn connect(&self) -> std::result::Result<(), crate::core::claude_cdp::ClaudeCdpError> {
        let manager = self.manager.lock().await;
        manager.connect().await
    }

    /// Connect or launch Claude Desktop.
    pub async fn connect_or_launch(&self) -> std::result::Result<(), crate::core::claude_cdp::ClaudeCdpError> {
        let manager = self.manager.lock().await;
        manager.connect_or_launch().await
    }

    /// Build the message content from the request.
    fn build_message(&self, request: &ChatRequest) -> String {
        let mut parts = Vec::new();

        // Add system prompt if present
        if let Some(ref system) = request.system_prompt {
            parts.push(format!("[System: {}]\n", system));
        }

        // Add conversation context (last few messages for context)
        for msg in request.messages.iter() {
            match msg.role {
                crate::core::llm::router::MessageRole::User => {
                    parts.push(format!("User: {}", msg.content));
                }
                crate::core::llm::router::MessageRole::Assistant => {
                    parts.push(format!("Assistant: {}", msg.content));
                }
                crate::core::llm::router::MessageRole::System => {
                    // Skip system messages in conversation (already added above)
                }
            }
        }

        // The final user message is what we actually send
        if let Some(last_user_msg) = request.messages.iter().rev().find(|m| {
            matches!(m.role, crate::core::llm::router::MessageRole::User)
        }) {
            // If there's conversation context, just return the last user message
            // Claude Desktop will maintain its own context
            last_user_msg.content.clone()
        } else {
            parts.join("\n\n")
        }
    }
}

impl Default for ClaudeDesktopProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProvider for ClaudeDesktopProvider {
    fn id(&self) -> &str {
        "claude-desktop"
    }

    fn name(&self) -> &str {
        "Claude Desktop (CDP)"
    }

    fn model(&self) -> &str {
        // We don't know which model the user has selected in Claude Desktop
        "claude-desktop"
    }

    async fn health_check(&self) -> bool {
        let manager = self.manager.lock().await;
        manager.is_connected().await && manager.health_check().await
    }

    fn pricing(&self) -> Option<ProviderPricing> {
        // No per-token pricing - subscription based
        None
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let manager = self.manager.lock().await;

        if !manager.is_connected().await {
            return Err(LLMError::NotConfigured(
                "Claude Desktop not connected. Call connect() or connect_or_launch() first.".to_string()
            ));
        }

        let message = self.build_message(&request);
        debug!(message_len = message.len(), "sending message to Claude Desktop");

        let start = Instant::now();

        let response_text = manager.send_message(&message).await.map_err(|e| {
            LLMError::ApiError {
                status: 0,
                message: e.to_string(),
            }
        })?;

        let latency_ms = start.elapsed().as_millis() as u64;

        info!(
            response_len = response_text.len(),
            latency_ms,
            "received response from Claude Desktop"
        );

        Ok(ChatResponse {
            content: response_text,
            model: "claude-desktop".to_string(),
            provider: "claude-desktop".to_string(),
            usage: None, // Can't track tokens via CDP
            finish_reason: Some("stop".to_string()),
            latency_ms,
            cost_usd: None, // Subscription-based
            tool_calls: None,
        })
    }

    async fn stream_chat(
        &self,
        _request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        // CDP doesn't support streaming - we get the full response after Claude finishes
        warn!("streaming not supported for Claude Desktop provider");
        Err(LLMError::StreamingNotSupported("claude-desktop".to_string()))
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    fn supports_embeddings(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_id() {
        let provider = ClaudeDesktopProvider::new();
        assert_eq!(provider.id(), "claude-desktop");
        assert_eq!(provider.name(), "Claude Desktop (CDP)");
    }

    #[test]
    fn test_no_pricing() {
        let provider = ClaudeDesktopProvider::new();
        assert!(provider.pricing().is_none());
    }

    #[test]
    fn test_no_streaming() {
        let provider = ClaudeDesktopProvider::new();
        assert!(!provider.supports_streaming());
        assert!(!provider.supports_embeddings());
    }

    #[test]
    fn test_custom_config() {
        let provider = ClaudeDesktopProvider::with_config(9444, 60);
        assert_eq!(provider.port, 9444);
        assert_eq!(provider.timeout_secs, 60);
    }
}
