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
pub mod model_selector;
pub mod proxy;
pub mod router;
pub mod session;
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

// Re-export session types
pub use session::{
    ProviderSession, SessionError, SessionId, SessionInfo, SessionManager, SessionStore,
    SessionResult, SessionChatRequest, SessionChatResponse, ClaudeStreamEvent,
};

// Re-export model selector types
pub use model_selector::{
    ModelSelector, ModelSelection, SubscriptionPlan, TaskComplexity, UsageData,
    AuthType, model_selector,
};

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
    /// Currently active provider ID for the proxy (for cleanup on switch)
    current_proxy_provider: RwLock<Option<String>>,
}

impl LLMManager {
    /// Create a new LLM manager with default settings
    pub fn new() -> Self {
        Self {
            router: LLMRouter::with_defaults(),
            proxy: RwLock::new(None),
            chat_client: RwLock::new(None),
            proxy_port: std::env::var("LLM_PROXY_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(18787),
            current_proxy_provider: RwLock::new(None),
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

    /// Unregister a provider from the proxy
    pub async fn unregister_proxy_provider(&self, id: &str) {
        let proxy_guard = self.proxy.read().await;
        if let Some(ref proxy) = *proxy_guard {
            proxy.unregister_provider(id).await;
        }
    }

    /// Set the default provider for the proxy
    ///
    /// When a model name without provider prefix is used (e.g., "gpt-4" instead of "openai:gpt-4"),
    /// the proxy will route to this provider.
    pub async fn set_default_proxy_provider(&self, id: &str) -> std::result::Result<(), String> {
        // Ensure proxy is running
        self.ensure_proxy().await?;

        let proxy_guard = self.proxy.read().await;
        if let Some(ref proxy) = *proxy_guard {
            proxy.set_default_provider(id).await;
        }

        Ok(())
    }

    /// Set the embedding callback for the proxy
    ///
    /// This callback handles /v1/embeddings requests, forwarding them to the configured
    /// embedding provider (e.g., Copilot, OpenAI).
    pub async fn set_embedding_callback(
        &self,
        callback: proxy::EmbeddingCallback,
    ) -> std::result::Result<(), String> {
        // Ensure proxy is running
        self.ensure_proxy().await?;

        let proxy_guard = self.proxy.read().await;
        if let Some(ref proxy) = *proxy_guard {
            proxy.set_embedding_callback(callback).await;
        }

        Ok(())
    }

    /// Set the default embedding model for the proxy
    pub async fn set_default_embedding_model(&self, model: &str) -> std::result::Result<(), String> {
        self.ensure_proxy().await?;

        let proxy_guard = self.proxy.read().await;
        if let Some(ref proxy) = *proxy_guard {
            proxy.set_default_embedding_model(model).await;
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
    ///
    /// This method handles provider switching by:
    /// 1. Unregistering the previous provider from the proxy (if different)
    /// 2. Registering the new provider with the proxy (if it requires proxy)
    /// 3. Setting the new provider as the default for the proxy
    /// 4. Configuring the Meilisearch workspace
    pub async fn configure_for_chat(
        &self,
        config: &providers::ProviderConfig,
        custom_system_prompt: Option<&str>,
    ) -> std::result::Result<(), String> {
        let chat_client = self.chat_client.read().await;
        let client = chat_client
            .as_ref()
            .ok_or("Meilisearch chat client not configured")?;

        let new_provider_id = config.provider_id();

        // Check if we're switching providers
        let previous_provider = {
            let current = self.current_proxy_provider.read().await;
            current.clone()
        };

        // Unregister previous provider if it's different from the new one
        if let Some(ref prev_id) = previous_provider {
            if prev_id != new_provider_id {
                log::info!("Switching LLM proxy provider: {} -> {}", prev_id, new_provider_id);
                self.unregister_proxy_provider(prev_id).await;
            }
        }

        let proxy_url = if config.requires_proxy() {
            let url = self.ensure_proxy().await?;

            // Create and register the LLM provider
            let llm_provider = config.create_provider();
            self.register_proxy_provider(new_provider_id, llm_provider).await?;

            // Set as default provider for the proxy
            self.set_default_proxy_provider(new_provider_id).await?;

            // Track current provider
            {
                let mut current = self.current_proxy_provider.write().await;
                *current = Some(new_provider_id.to_string());
            }

            url
        } else {
            // Native provider - clear current proxy provider tracking
            {
                let mut current = self.current_proxy_provider.write().await;
                *current = None;
            }
            String::new()
        };

        client.configure_from_provider_config(config, &proxy_url, custom_system_prompt).await
    }

    /// Get the currently active proxy provider ID
    pub async fn current_proxy_provider(&self) -> Option<String> {
        let current = self.current_proxy_provider.read().await;
        current.clone()
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
        temperature: Option<f32>,
        max_tokens: Option<u32>,
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
            temperature: temperature.or(Some(0.7)),
            max_tokens: max_tokens.or(Some(4096)),
            tools: Some(vec![
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": "_meiliSearchProgress",
                        "description": "Reports real-time search progress to the user"
                    }
                }),
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": "_meiliSearchSources",
                        "description": "Provides sources and references for the information"
                    }
                })
            ]),
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
