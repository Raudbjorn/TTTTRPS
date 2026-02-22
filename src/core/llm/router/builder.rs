//! LLM Router Builder
//!
//! Builder pattern for constructing an LLMRouter.

use std::sync::Arc;
use std::time::Duration;

use super::config::{RouterConfig, RoutingStrategy};
use super::provider::LLMProvider;
use super::LLMRouter;

/// Builder for constructing an LLMRouter
pub struct LLMRouterBuilder {
    config: RouterConfig,
    providers: Vec<Arc<dyn LLMProvider>>,
}

impl LLMRouterBuilder {
    pub fn new() -> Self {
        Self {
            config: RouterConfig::default(),
            providers: Vec::new(),
        }
    }

    pub fn with_config(mut self, config: RouterConfig) -> Self {
        self.config = config;
        self
    }

    pub fn add_provider(mut self, provider: Arc<dyn LLMProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.request_timeout = timeout;
        self
    }

    pub fn with_fallback(mut self, enabled: bool) -> Self {
        self.config.enable_fallback = enabled;
        self
    }

    pub fn with_routing_strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.config.routing_strategy = strategy;
        self
    }

    pub fn with_monthly_budget(mut self, budget: f64) -> Self {
        self.config.monthly_budget = Some(budget);
        self
    }

    pub fn with_daily_budget(mut self, budget: f64) -> Self {
        self.config.daily_budget = Some(budget);
        self
    }

    pub async fn build(self) -> LLMRouter {
        let mut router = LLMRouter::new(self.config);
        for provider in self.providers {
            router.add_provider(provider).await;
        }
        router
    }
}

impl Default for LLMRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}
