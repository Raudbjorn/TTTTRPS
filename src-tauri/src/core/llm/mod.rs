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
