//! LLM Client Module
//!
//! Provides unified interface for multiple LLM providers with:
//! - Intelligent routing and automatic fallback
//! - Health tracking and circuit breaker pattern
//! - Cost tracking and budget management
//! - Streaming support
//!
//! # Module Structure
//!
//! - `router`: Main router and `LLMProvider` trait
//! - `health`: Health tracking and circuit breaker
//! - `cost`: Cost tracking and pricing
//! - `providers`: Individual provider implementations

pub mod client;
pub mod cost;
pub mod health;
pub mod proxy;
pub mod router;
pub mod providers;

// Re-export commonly used types
pub use client::{
    get_extended_fallback_models, get_fallback_models, fetch_openrouter_models,
    fetch_litellm_models_for_provider, LLMClient, LLMConfig, ModelInfo, OllamaModel
};
pub use cost::{CostSummary, CostTracker, ProviderCosts, ProviderPricing, TokenUsage};
pub use health::{CircuitState, HealthSummary, HealthTracker, ProviderHealth};
pub use router::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LLMError, LLMProvider, LLMRouter,
    LLMRouterBuilder, MessageRole, ProviderStats, Result, RouterConfig, RoutingStrategy,
};

// Re-export provider implementations
pub use providers::*;

// Re-export proxy types
pub use proxy::LLMProxyService;

// Note: LLMManager is defined below and re-exported automatically

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create a new router with default configuration
pub fn create_router() -> LLMRouter {
    LLMRouter::with_defaults()
}

/// Create a router builder
pub fn router_builder() -> LLMRouterBuilder {
    LLMRouterBuilder::new()
}

// ============================================================================
// LLM Manager
// ============================================================================

use crate::core::meilisearch_chat::{
    ChatPrompts, ChatProviderConfig, MeilisearchChatClient,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manages LLM providers, proxy service, and Meilisearch chat integration.
///
/// The LLMManager provides a unified interface for:
/// - Routing requests to LLM providers
/// - Managing the OpenAI-compatible proxy for Meilisearch
/// - Configuring chat workspaces with any provider
pub struct LLMManager {
    /// The LLM router for direct provider access
    router: LLMRouter,
    /// The proxy service (lazily initialized)
    proxy: RwLock<Option<LLMProxyService>>,
    /// Meilisearch chat client (optional, set when Meilisearch is configured)
    chat_client: RwLock<Option<MeilisearchChatClient>>,
    /// Default proxy port
    proxy_port: u16,
}

impl LLMManager {
    /// Create a new LLM manager with default settings
    pub fn new() -> Self {
        Self {
            router: LLMRouter::with_defaults(),
            proxy: RwLock::new(None),
            chat_client: RwLock::new(None),
            proxy_port: 8787,
        }
    }

    /// Create with a custom proxy port
    pub fn with_proxy_port(mut self, port: u16) -> Self {
        self.proxy_port = port;
        self
    }

    /// Set the Meilisearch chat client
    pub async fn set_chat_client(&self, host: &str, api_key: Option<&str>) {
        let client = MeilisearchChatClient::new(host, api_key);
        let mut chat_client = self.chat_client.write().await;
        *chat_client = Some(client);
        log::info!("Meilisearch chat client configured: {}", host);
    }

    /// Get the LLM router
    pub fn router(&self) -> &LLMRouter {
        &self.router
    }

    /// Get the proxy URL
    pub fn proxy_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.proxy_port)
    }

    /// Ensure the proxy service is running
    pub async fn ensure_proxy(&self) -> std::result::Result<String, String> {
        let mut proxy_guard = self.proxy.write().await;

        if proxy_guard.is_none() {
            let mut proxy = LLMProxyService::new(self.proxy_port);
            proxy.start().await?;
            *proxy_guard = Some(proxy);
            log::info!("LLM proxy started on port {}", self.proxy_port);
        }

        Ok(self.proxy_url())
    }

    /// Stop the proxy service
    pub async fn stop_proxy(&self) {
        let mut proxy_guard = self.proxy.write().await;
        if let Some(ref mut proxy) = *proxy_guard {
            proxy.stop().await;
        }
        *proxy_guard = None;
    }

    /// Register a provider with the proxy
    pub async fn register_proxy_provider(
        &self,
        id: &str,
        provider: Arc<dyn LLMProvider>,
    ) -> std::result::Result<(), String> {
        // Ensure proxy is running
        self.ensure_proxy().await?;

        let proxy_guard = self.proxy.read().await;
        if let Some(ref proxy) = *proxy_guard {
            proxy.register_provider(id, provider).await;
        }

        Ok(())
    }

    /// Configure a chat workspace with a specific provider
    ///
    /// This method:
    /// 1. Ensures the proxy is running (if needed)
    /// 2. Registers the provider with the proxy
    /// 3. Configures the Meilisearch workspace
    pub async fn configure_chat_workspace(
        &self,
        workspace_id: &str,
        provider: ChatProviderConfig,
        custom_prompts: Option<ChatPrompts>,
    ) -> std::result::Result<(), String> {
        let chat_client = self.chat_client.read().await;
        let client = chat_client
            .as_ref()
            .ok_or("Meilisearch chat client not configured")?;

        let proxy_url = if provider.requires_proxy() {
            // Start proxy and register provider
            let url = self.ensure_proxy().await?;

            // Create and register the LLM provider
            let llm_provider = provider.to_provider_config().create_provider();
            self.register_proxy_provider(provider.provider_id(), llm_provider)
                .await?;

            url
        } else {
            // Native providers don't need proxy
            String::new()
        };

        // Configure the workspace
        client
            .configure_workspace_with_provider(workspace_id, &provider, &proxy_url, custom_prompts)
            .await?;

        Ok(())
    }

    /// Get workspace settings
    pub async fn get_workspace_settings(
        &self,
        workspace_id: &str,
    ) -> std::result::Result<Option<crate::core::meilisearch_chat::ChatWorkspaceSettings>, String> {
        let chat_client = self.chat_client.read().await;
        let client = chat_client
            .as_ref()
            .ok_or("Meilisearch chat client not configured")?;

        client.get_workspace_settings(workspace_id).await
    }

    /// Check if proxy is running
    pub async fn is_proxy_running(&self) -> bool {
        let proxy_guard = self.proxy.read().await;
        proxy_guard
            .as_ref()
            .map(|p| p.is_running())
            .unwrap_or(false)
    }

    /// List providers registered with the proxy
    pub async fn list_proxy_providers(&self) -> Vec<String> {
        let proxy_guard = self.proxy.read().await;
        match proxy_guard.as_ref() {
            Some(proxy) => proxy.list_providers().await,
            None => Vec::new(),
        }
    }

    /// Configure for chat (convenience method using default workspace)
    pub async fn configure_for_chat(
        &self,
        config: &super::providers::ProviderConfig,
        custom_system_prompt: Option<&str>,
    ) -> std::result::Result<(), String> {
        let chat_client = self.chat_client.read().await;
        let client = chat_client
            .as_ref()
            .ok_or("Meilisearch chat client not configured")?;

        let proxy_url = if config.requires_proxy() {
             let url = self.ensure_proxy().await?;
             let llm_provider = config.create_provider();
             self.register_proxy_provider(config.provider_id(), llm_provider).await?;
             url
        } else {
            String::new()
        };

        client.configure_from_provider_config(config, &proxy_url, custom_system_prompt).await
    }

    /// Send a chat message (using default DM workspace)
    /// Send a chat message (using default DM workspace)
    pub async fn chat(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> std::result::Result<String, String> {
        let chat_client = self.chat_client.read().await;
        let client = chat_client
            .as_ref()
            .ok_or("Meilisearch chat client not configured")?;

        // Convert router messages to Meilisearch messages
        let meili_messages: Vec<crate::core::meilisearch_chat::ChatMessage> = messages.into_iter().map(|m| {
            crate::core::meilisearch_chat::ChatMessage {
                role: m.role.to_string().to_lowercase(),
                content: m.content,
            }
        }).collect();

        // Use default workspace "dm-assistant"
        client.chat_completion("dm-assistant", meili_messages, model).await
    }

    /// Stream chat response (using default DM workspace)
    /// Stream chat response (using default DM workspace)
    pub async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> std::result::Result<tokio::sync::mpsc::Receiver<std::result::Result<String, String>>, String> {
        let chat_client = self.chat_client.read().await;
        let client = chat_client
            .as_ref()
            .ok_or("Meilisearch chat client not configured")?;

        // Convert router messages to Meilisearch messages
        let meili_messages: Vec<crate::core::meilisearch_chat::ChatMessage> = messages.into_iter().map(|m| {
            crate::core::meilisearch_chat::ChatMessage {
                role: m.role.to_string().to_lowercase(),
                content: m.content,
            }
        }).collect();

        let request = crate::core::meilisearch_chat::ChatCompletionRequest {
            model: model.to_string(),
            messages: meili_messages,
            stream: true,
            temperature: Some(0.7),
            max_tokens: Some(4096),
        };

        client.chat_completion_stream("dm-assistant", request).await
    }
}

impl Default for LLMManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all main types are exported
        let _: TokenUsage = TokenUsage::default();
        let _: ProviderStats = ProviderStats::default();
        let _: RouterConfig = RouterConfig::default();
        let _: RoutingStrategy = RoutingStrategy::Priority;
    }

    #[test]
    fn test_create_router() {
        let router = create_router();
        assert_eq!(router.routing_strategy(), RoutingStrategy::Priority);
    }
}
