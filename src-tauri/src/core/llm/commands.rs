//! Tauri Commands for LLM Router Operations
//!
//! This module provides Tauri IPC commands for managing LLM providers,
//! routing, health tracking, and cost management.

use super::cost::{CostSummary, ProviderCosts, ProviderPricing, TokenUsage};
use super::health::{CircuitState, HealthSummary, ProviderHealth};
use super::router::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LLMRouter, ProviderStats,
    RouterConfig, RoutingStrategy,
};
use super::providers::ProviderConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Response Types
// ============================================================================

/// Response for router status queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterStatus {
    /// List of registered provider IDs
    pub providers: Vec<String>,
    /// Current routing strategy
    pub routing_strategy: RoutingStrategy,
    /// Health summary
    pub health_summary: HealthSummary,
    /// Cost summary
    pub cost_summary: CostSummary,
    /// Whether fallback is enabled
    pub fallback_enabled: bool,
}

/// Response for provider info queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub model: String,
    pub is_healthy: bool,
    pub circuit_state: String,
    pub stats: ProviderStats,
    pub pricing: Option<ProviderPricing>,
}

/// Request to add a new provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AddProviderRequest {
    Ollama { host: String, model: String },
    Claude { api_key: String, model: String, max_tokens: Option<u32> },
    OpenAI { api_key: String, model: String, max_tokens: Option<u32>, organization_id: Option<String>, base_url: Option<String> },
    Gemini { api_key: String, model: String },
    OpenRouter { api_key: String, model: String },
    Mistral { api_key: String, model: String },
    Groq { api_key: String, model: String },
    Together { api_key: String, model: String },
    Cohere { api_key: String, model: String },
    DeepSeek { api_key: String, model: String },
}

impl AddProviderRequest {
    pub fn to_config(&self) -> ProviderConfig {
        match self {
            AddProviderRequest::Ollama { host, model } => {
                ProviderConfig::Ollama { host: host.clone(), model: model.clone() }
            }
            AddProviderRequest::Claude { api_key, model, max_tokens } => {
                ProviderConfig::Claude {
                    api_key: api_key.clone(),
                    model: model.clone(),
                    max_tokens: max_tokens.unwrap_or(8192),
                }
            }
            AddProviderRequest::OpenAI { api_key, model, max_tokens, organization_id, base_url } => {
                ProviderConfig::OpenAI {
                    api_key: api_key.clone(),
                    model: model.clone(),
                    max_tokens: max_tokens.unwrap_or(4096),
                    organization_id: organization_id.clone(),
                    base_url: base_url.clone(),
                }
            }
            AddProviderRequest::Gemini { api_key, model } => {
                ProviderConfig::Gemini { api_key: api_key.clone(), model: model.clone() }
            }
            AddProviderRequest::OpenRouter { api_key, model } => {
                ProviderConfig::OpenRouter { api_key: api_key.clone(), model: model.clone() }
            }
            AddProviderRequest::Mistral { api_key, model } => {
                ProviderConfig::Mistral { api_key: api_key.clone(), model: model.clone() }
            }
            AddProviderRequest::Groq { api_key, model } => {
                ProviderConfig::Groq { api_key: api_key.clone(), model: model.clone() }
            }
            AddProviderRequest::Together { api_key, model } => {
                ProviderConfig::Together { api_key: api_key.clone(), model: model.clone() }
            }
            AddProviderRequest::Cohere { api_key, model } => {
                ProviderConfig::Cohere { api_key: api_key.clone(), model: model.clone() }
            }
            AddProviderRequest::DeepSeek { api_key, model } => {
                ProviderConfig::DeepSeek { api_key: api_key.clone(), model: model.clone() }
            }
        }
    }
}

/// Cost estimate response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub provider: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub estimated_cost_usd: f64,
    pub pricing: Option<ProviderPricing>,
}

// ============================================================================
// Router State for Tauri
// ============================================================================

/// Thread-safe router state for Tauri
#[derive(Clone)]
pub struct RouterState {
    pub router: Arc<RwLock<LLMRouter>>,
}

impl RouterState {
    pub fn new(router: LLMRouter) -> Self {
        Self {
            router: Arc::new(RwLock::new(router)),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(LLMRouter::with_defaults())
    }
}

// ============================================================================
// Command Implementations
// ============================================================================

/// Get current router status
pub async fn get_router_status(router: &RwLock<LLMRouter>) -> Result<RouterStatus, String> {
    let r = router.read().await;
    Ok(RouterStatus {
        providers: r.provider_ids(),
        routing_strategy: r.routing_strategy(),
        health_summary: r.get_health_summary().await,
        cost_summary: r.get_cost_summary().await,
        fallback_enabled: r.config().enable_fallback,
    })
}

/// Get all provider information
pub async fn get_all_providers(router: &RwLock<LLMRouter>) -> Result<Vec<ProviderInfo>, String> {
    let r = router.read().await;
    let mut providers = Vec::new();

    for id in r.provider_ids() {
        if let Some(provider) = r.get_provider(&id) {
            let stats = r.get_stats(&id).await.unwrap_or_default();
            let health = r.get_health(&id).await;
            let circuit_state = r.get_circuit_state(&id).await;

            providers.push(ProviderInfo {
                id: id.clone(),
                name: provider.name().to_string(),
                model: provider.model().to_string(),
                is_healthy: health.as_ref().map(|h| h.is_healthy).unwrap_or(false),
                circuit_state: match circuit_state {
                    Some(CircuitState::Closed) => "closed".to_string(),
                    Some(CircuitState::Open) => "open".to_string(),
                    Some(CircuitState::HalfOpen) => "half_open".to_string(),
                    None => "unknown".to_string(),
                },
                stats,
                pricing: provider.pricing(),
            });
        }
    }

    Ok(providers)
}

/// Get provider by ID
pub async fn get_provider(router: &RwLock<LLMRouter>, id: &str) -> Result<Option<ProviderInfo>, String> {
    let r = router.read().await;

    if let Some(provider) = r.get_provider(id) {
        let stats = r.get_stats(id).await.unwrap_or_default();
        let health = r.get_health(id).await;
        let circuit_state = r.get_circuit_state(id).await;

        Ok(Some(ProviderInfo {
            id: id.to_string(),
            name: provider.name().to_string(),
            model: provider.model().to_string(),
            is_healthy: health.as_ref().map(|h| h.is_healthy).unwrap_or(false),
            circuit_state: match circuit_state {
                Some(CircuitState::Closed) => "closed".to_string(),
                Some(CircuitState::Open) => "open".to_string(),
                Some(CircuitState::HalfOpen) => "half_open".to_string(),
                None => "unknown".to_string(),
            },
            stats,
            pricing: provider.pricing(),
        }))
    } else {
        Ok(None)
    }
}

/// Add a new provider
pub async fn add_provider(
    router: &RwLock<LLMRouter>,
    request: AddProviderRequest,
) -> Result<String, String> {
    let config = request.to_config();
    let id = config.provider_id().to_string();
    let provider = config.create_provider();

    let mut r = router.write().await;
    r.add_provider(provider).await;

    Ok(id)
}

/// Remove a provider
pub async fn remove_provider(router: &RwLock<LLMRouter>, id: &str) -> Result<(), String> {
    let mut r = router.write().await;
    r.remove_provider(id).await;
    Ok(())
}

/// Set routing strategy
pub async fn set_routing_strategy(
    router: &RwLock<LLMRouter>,
    strategy: RoutingStrategy,
) -> Result<(), String> {
    let mut r = router.write().await;
    r.set_routing_strategy(strategy);
    Ok(())
}

/// Get health status for all providers
pub async fn get_all_health(
    router: &RwLock<LLMRouter>,
) -> Result<HashMap<String, ProviderHealth>, String> {
    let r = router.read().await;
    Ok(r.get_all_health().await)
}

/// Get health for a specific provider
pub async fn get_provider_health(
    router: &RwLock<LLMRouter>,
    id: &str,
) -> Result<Option<ProviderHealth>, String> {
    let r = router.read().await;
    Ok(r.get_health(id).await)
}

/// Get healthy provider IDs
pub async fn get_healthy_providers(router: &RwLock<LLMRouter>) -> Result<Vec<String>, String> {
    let r = router.read().await;
    Ok(r.healthy_providers().await)
}

/// Run health checks on all providers
pub async fn run_health_checks(
    router: &RwLock<LLMRouter>,
) -> Result<HashMap<String, bool>, String> {
    let r = router.read().await;
    Ok(r.health_check_all().await)
}

/// Reset circuit breaker for a provider
pub async fn reset_circuit(router: &RwLock<LLMRouter>, id: &str) -> Result<(), String> {
    let r = router.read().await;
    r.reset_circuit(id).await;
    Ok(())
}

/// Get cost summary
pub async fn get_cost_summary(router: &RwLock<LLMRouter>) -> Result<CostSummary, String> {
    let r = router.read().await;
    Ok(r.get_cost_summary().await)
}

/// Estimate cost for a request
pub async fn estimate_cost(
    router: &RwLock<LLMRouter>,
    provider: &str,
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
) -> Result<CostEstimate, String> {
    let r = router.read().await;
    let estimated_cost = r.estimate_cost(provider, model, input_tokens, output_tokens).await;

    // Get pricing info if available
    let pricing = if let Some(p) = r.get_provider(provider) {
        p.pricing()
    } else {
        ProviderPricing::for_model(provider, model)
    };

    Ok(CostEstimate {
        provider: provider.to_string(),
        model: model.to_string(),
        input_tokens,
        output_tokens,
        estimated_cost_usd: estimated_cost,
        pricing,
    })
}

/// Set monthly budget
pub async fn set_monthly_budget(router: &RwLock<LLMRouter>, budget: Option<f64>) -> Result<(), String> {
    let r = router.read().await;
    r.set_monthly_budget(budget).await;
    Ok(())
}

/// Set daily budget
pub async fn set_daily_budget(router: &RwLock<LLMRouter>, budget: Option<f64>) -> Result<(), String> {
    let r = router.read().await;
    r.set_daily_budget(budget).await;
    Ok(())
}

/// Reset all cost tracking
pub async fn reset_costs(router: &RwLock<LLMRouter>) -> Result<(), String> {
    let r = router.read().await;
    r.reset_costs().await;
    Ok(())
}

/// Get all provider stats
pub async fn get_all_stats(
    router: &RwLock<LLMRouter>,
) -> Result<HashMap<String, ProviderStats>, String> {
    let r = router.read().await;
    Ok(r.get_all_stats().await)
}

/// Send a chat request
pub async fn chat(
    router: &RwLock<LLMRouter>,
    request: ChatRequest,
) -> Result<ChatResponse, String> {
    let r = router.read().await;
    r.chat(request).await.map_err(|e| e.to_string())
}

/// Send a streaming chat request
/// Returns a stream ID that can be used to receive chunks via events
pub async fn stream_chat(
    router: &RwLock<LLMRouter>,
    request: ChatRequest,
) -> Result<String, String> {
    let r = router.read().await;
    let mut rx = r.stream_chat(request).await.map_err(|e| e.to_string())?;

    // Get stream ID from first chunk or generate one
    let stream_id = uuid::Uuid::new_v4().to_string();

    // Note: In a real Tauri command, you would emit events here.
    // This is a placeholder showing how the stream would be used.
    // The actual implementation would be in the commands.rs file with
    // access to tauri::AppHandle for event emission.

    Ok(stream_id)
}

/// Cancel an active stream
pub async fn cancel_stream(router: &RwLock<LLMRouter>, stream_id: &str) -> Result<bool, String> {
    let r = router.read().await;
    Ok(r.cancel_stream(stream_id).await)
}

/// Get active stream IDs
pub async fn get_active_streams(router: &RwLock<LLMRouter>) -> Result<Vec<String>, String> {
    let r = router.read().await;
    Ok(r.active_stream_ids().await)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_provider_request_serialization() {
        let request = AddProviderRequest::Ollama {
            host: "http://localhost:11434".to_string(),
            model: "llama3".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("ollama"));
        assert!(json.contains("localhost"));
    }

    #[test]
    fn test_add_provider_to_config() {
        let request = AddProviderRequest::Claude {
            api_key: "sk-ant-test".to_string(),
            model: "claude-3-5-sonnet".to_string(),
            max_tokens: Some(4096),
        };

        let config = request.to_config();
        assert_eq!(config.provider_id(), "claude");
    }
}
