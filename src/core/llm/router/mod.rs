//! LLM Provider Router
//!
//! Provides intelligent routing between LLM providers with:
//! - Unified `LLMProvider` trait for all providers
//! - Health tracking and circuit breaker pattern
//! - Automatic failover when providers fail
//! - Cost tracking and budget management
//! - Multiple routing strategies
//! - Streaming support

mod builder;
mod config;
mod error;
mod provider;
mod stats;
mod types;

#[cfg(test)]
mod tests;

// Re-export public API
pub use builder::LLMRouterBuilder;
pub use config::{RouterConfig, RoutingStrategy};
pub use error::{LLMError, Result};
pub use provider::LLMProvider;
pub use stats::ProviderStats;
pub use types::{ChatChunk, ChatMessage, ChatRequest, ChatResponse, MessageRole};

use crate::core::llm::cost::{CostSummary, CostTracker, CostTrackerConfig, TokenUsage};
use crate::core::llm::health::{
    CircuitState, HealthSummary, HealthTracker, HealthTrackerConfig, ProviderHealth,
};

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;

// ============================================================================
// Active Stream State
// ============================================================================

/// State for tracking active streams.
/// Fields are used for Debug output and potential future monitoring/metrics.
#[derive(Debug)]
#[allow(dead_code)]
struct StreamState {
    stream_id: String,
    provider: String,
    model: String,
    is_canceled: bool,
    chunks_received: u32,
}

// ============================================================================
// LLM Router
// ============================================================================

/// Main router for LLM providers
#[derive(Clone)]
pub struct LLMRouter {
    /// Registered providers by ID
    providers: HashMap<String, Arc<dyn LLMProvider>>,
    /// Provider priority order
    provider_order: Vec<String>,
    /// Health tracker
    health_tracker: Arc<RwLock<HealthTracker>>,
    /// Cost tracker
    cost_tracker: Arc<RwLock<CostTracker>>,
    /// Statistics per provider
    stats: Arc<RwLock<HashMap<String, ProviderStats>>>,
    /// Active streams
    active_streams: Arc<RwLock<HashMap<String, StreamState>>>,
    /// Configuration
    config: RouterConfig,
    /// Round-robin index
    round_robin_index: Arc<RwLock<usize>>,
}

impl LLMRouter {
    /// Create a new router with configuration
    pub fn new(config: RouterConfig) -> Self {
        let mut cost_tracker = CostTracker::with_config(CostTrackerConfig {
            monthly_budget: config.monthly_budget,
            daily_budget: config.daily_budget,
            budget_alert_threshold: 0.8,
        });
        cost_tracker.monthly_budget = config.monthly_budget;
        cost_tracker.daily_budget = config.daily_budget;

        Self {
            providers: HashMap::new(),
            provider_order: Vec::new(),
            health_tracker: Arc::new(RwLock::new(HealthTracker::new(HealthTrackerConfig {
                check_interval_secs: config.health_check_interval.as_secs(),
                ..Default::default()
            }))),
            cost_tracker: Arc::new(RwLock::new(cost_tracker)),
            stats: Arc::new(RwLock::new(HashMap::new())),
            active_streams: Arc::new(RwLock::new(HashMap::new())),
            config,
            round_robin_index: Arc::new(RwLock::new(0)),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RouterConfig::default())
    }

    /// Register a provider
    pub async fn add_provider(&mut self, provider: Arc<dyn LLMProvider>) {
        let id = provider.id().to_string();

        // Add to providers map
        self.providers.insert(id.clone(), provider.clone());

        // Add to priority order if not already present
        if !self.provider_order.contains(&id) {
            self.provider_order.push(id.clone());
        }

        // Initialize tracking
        self.health_tracker.write().await.add_provider(&id);
        self.stats
            .write()
            .await
            .insert(id.clone(), ProviderStats::default());

        // Set pricing if available
        if let Some(pricing) = provider.pricing() {
            self.cost_tracker.write().await.set_pricing(pricing);
        }
    }

    /// Remove a provider
    pub async fn remove_provider(&mut self, id: &str) {
        self.providers.remove(id);
        self.provider_order.retain(|p| p != id);
        self.health_tracker.write().await.remove_provider(id);
        self.stats.write().await.remove(id);
    }

    /// Get provider IDs in priority order
    pub fn provider_ids(&self) -> Vec<String> {
        self.provider_order.clone()
    }

    /// Get a provider by ID
    pub fn get_provider(&self, id: &str) -> Option<Arc<dyn LLMProvider>> {
        self.providers.get(id).cloned()
    }

    /// Set routing strategy
    pub fn set_routing_strategy(&mut self, strategy: RoutingStrategy) {
        self.config.routing_strategy = strategy;
    }

    /// Get current routing strategy
    pub fn routing_strategy(&self) -> RoutingStrategy {
        self.config.routing_strategy
    }

    /// Get stats for a provider
    pub async fn get_stats(&self, id: &str) -> Option<ProviderStats> {
        self.stats.read().await.get(id).cloned()
    }

    /// Get all provider stats
    pub async fn get_all_stats(&self) -> HashMap<String, ProviderStats> {
        self.stats.read().await.clone()
    }

    /// Get health status for a provider
    pub async fn get_health(&self, id: &str) -> Option<ProviderHealth> {
        self.health_tracker.read().await.get_health(id).cloned()
    }

    /// Get all health statuses
    pub async fn get_all_health(&self) -> HashMap<String, ProviderHealth> {
        self.health_tracker.read().await.all_health().clone()
    }

    /// Get healthy provider IDs
    pub async fn healthy_providers(&self) -> Vec<String> {
        self.health_tracker
            .read()
            .await
            .healthy_providers()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get circuit state for a provider
    pub async fn get_circuit_state(&self, id: &str) -> Option<CircuitState> {
        self.health_tracker.read().await.get_circuit_state(id)
    }

    /// Get cost summary
    pub async fn get_cost_summary(&self) -> CostSummary {
        self.cost_tracker.read().await.summary()
    }

    /// Estimate cost for a request
    pub async fn estimate_cost(
        &self,
        provider: &str,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> f64 {
        self.cost_tracker
            .read()
            .await
            .estimate_cost(provider, model, input_tokens, output_tokens)
    }

    /// Get health summary
    pub async fn get_health_summary(&self) -> HealthSummary {
        self.health_tracker.read().await.summary()
    }

    /// Set monthly budget
    pub async fn set_monthly_budget(&self, budget: Option<f64>) {
        self.cost_tracker.write().await.set_monthly_budget(budget);
    }

    /// Set daily budget
    pub async fn set_daily_budget(&self, budget: Option<f64>) {
        self.cost_tracker.write().await.set_daily_budget(budget);
    }

    /// Check if provider is available (healthy + circuit allows)
    async fn is_provider_available(&self, id: &str) -> bool {
        self.health_tracker.write().await.check_availability(id)
    }

    /// Record successful request
    async fn record_success(
        &self,
        id: &str,
        latency_ms: u64,
        usage: Option<&TokenUsage>,
        model: &str,
    ) {
        // Update health tracker
        self.health_tracker
            .write()
            .await
            .record_success(id, Some(latency_ms));

        // Calculate and record cost
        let cost = if let Some(u) = usage {
            self.cost_tracker.write().await.record_usage(id, model, u)
        } else {
            0.0
        };

        // Update stats
        if let Some(stats) = self.stats.write().await.get_mut(id) {
            stats.record_success(latency_ms, usage, cost);
        }
    }

    /// Record failed request
    async fn record_failure(&self, id: &str, reason: &str) {
        self.health_tracker.write().await.record_failure(id, reason);
        if let Some(stats) = self.stats.write().await.get_mut(id) {
            stats.record_failure();
        }
    }

    /// Get the next provider based on routing strategy
    async fn get_next_provider(&self, request: &ChatRequest) -> Option<Arc<dyn LLMProvider>> {
        // If specific provider requested, try that first
        if let Some(ref requested) = request.provider {
            if let Some(provider) = self.providers.get(requested) {
                if self.is_provider_available(requested).await {
                    return Some(provider.clone());
                }
            }
        }

        // Get available providers
        let mut available: Vec<&String> = Vec::new();
        for id in &self.provider_order {
            if self.is_provider_available(id).await {
                available.push(id);
            }
        }

        if available.is_empty() {
            return None;
        }

        let selected_id = match self.config.routing_strategy {
            RoutingStrategy::Priority => available.first().map(|s| s.as_str()),

            RoutingStrategy::CostOptimized => {
                let stats = self.stats.read().await;
                available
                    .into_iter()
                    .min_by(|a, b| {
                        let cost_a = stats
                            .get(a.as_str())
                            .map(|s| s.total_cost_usd / s.successful_requests.max(1) as f64)
                            .unwrap_or(f64::MAX);
                        let cost_b = stats
                            .get(b.as_str())
                            .map(|s| s.total_cost_usd / s.successful_requests.max(1) as f64)
                            .unwrap_or(f64::MAX);
                        cost_a
                            .partial_cmp(&cost_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|s| s.as_str())
            }

            RoutingStrategy::LatencyOptimized => {
                let stats = self.stats.read().await;
                available
                    .into_iter()
                    .min_by_key(|id| {
                        stats
                            .get(id.as_str())
                            .map(|s| s.avg_latency_ms())
                            .unwrap_or(u64::MAX)
                    })
                    .map(|s| s.as_str())
            }

            RoutingStrategy::RoundRobin => {
                let mut index = self.round_robin_index.write().await;
                let selected = available.get(*index % available.len()).map(|s| s.as_str());
                *index = (*index + 1) % available.len().max(1);
                selected
            }

            RoutingStrategy::Random => {
                use rand::Rng;
                let idx = rand::thread_rng().gen_range(0..available.len());
                available.get(idx).map(|s| s.as_str())
            }
        };

        selected_id.and_then(|id| self.providers.get(id).cloned())
    }

    /// Get providers ordered according to the routing strategy
    async fn get_ordered_providers(&self) -> Vec<Arc<dyn LLMProvider>> {
        // Get available providers
        let mut available: Vec<&String> = Vec::new();
        for id in &self.provider_order {
            if self.is_provider_available(id).await {
                available.push(id);
            }
        }

        if available.is_empty() {
            return Vec::new();
        }

        let ordered_ids: Vec<String> = match self.config.routing_strategy {
            RoutingStrategy::Priority => available.into_iter().cloned().collect(),

            RoutingStrategy::CostOptimized => {
                let stats = self.stats.read().await;
                let mut ids: Vec<String> = available.into_iter().cloned().collect();
                ids.sort_by(|a, b| {
                    let cost_a = stats
                        .get(a.as_str())
                        .map(|s| s.total_cost_usd / s.successful_requests.max(1) as f64)
                        .unwrap_or(f64::MAX);
                    let cost_b = stats
                        .get(b.as_str())
                        .map(|s| s.total_cost_usd / s.successful_requests.max(1) as f64)
                        .unwrap_or(f64::MAX);
                    cost_a
                        .partial_cmp(&cost_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                ids
            }

            RoutingStrategy::LatencyOptimized => {
                let stats = self.stats.read().await;
                let mut ids: Vec<String> = available.into_iter().cloned().collect();
                ids.sort_by_key(|id| {
                    stats
                        .get(id.as_str())
                        .map(|s| s.avg_latency_ms())
                        .unwrap_or(u64::MAX)
                });
                ids
            }

            RoutingStrategy::RoundRobin => {
                let mut index = self.round_robin_index.write().await;
                let start = *index % available.len().max(1);
                *index = (*index + 1) % available.len().max(1);
                // Rotate the list to start from the round-robin position
                let mut ids: Vec<String> = available.into_iter().cloned().collect();
                ids.rotate_left(start);
                ids
            }

            RoutingStrategy::Random => {
                use rand::seq::SliceRandom;
                let mut ids: Vec<String> = available.into_iter().cloned().collect();
                ids.shuffle(&mut rand::thread_rng());
                ids
            }
        };

        ordered_ids
            .into_iter()
            .filter_map(|id| self.providers.get(&id).cloned())
            .collect()
    }

    /// Send a chat request with automatic routing and fallback
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Check budget
        if !self.cost_tracker.read().await.is_within_budget() {
            return Err(LLMError::BudgetExceeded(
                "Monthly or daily budget exceeded".to_string(),
            ));
        }

        let mut last_error: Option<LLMError> = None;
        let mut tried_providers = Vec::new();

        // Build list of providers to try
        let providers_to_try: Vec<Arc<dyn LLMProvider>> = if let Some(ref requested) =
            request.provider
        {
            // Try requested provider first, then others as fallback
            let mut providers = Vec::new();
            if let Some(p) = self.providers.get(requested) {
                providers.push(p.clone());
            }
            for id in &self.provider_order {
                if id != requested {
                    if let Some(p) = self.providers.get(id) {
                        providers.push(p.clone());
                    }
                }
            }
            providers
        } else {
            // Use routing strategy to order providers
            self.get_ordered_providers().await
        };

        for provider in providers_to_try {
            let id = provider.id().to_string();

            // Check availability
            if !self.is_provider_available(&id).await {
                log::debug!("Skipping provider {} (not available)", id);
                continue;
            }

            tried_providers.push(id.clone());
            let start = Instant::now();

            // Execute with timeout
            let result = timeout(self.config.request_timeout, provider.chat(request.clone())).await;

            match result {
                Ok(Ok(response)) => {
                    let latency = start.elapsed().as_millis() as u64;
                    self.record_success(&id, latency, response.usage.as_ref(), &response.model)
                        .await;
                    log::info!("Chat succeeded with provider {} ({}ms)", id, latency);
                    return Ok(response);
                }
                Ok(Err(e)) => {
                    let error_msg = e.to_string();
                    self.record_failure(&id, &error_msg).await;
                    log::warn!("Chat failed with provider {}: {}", id, e);
                    last_error = Some(e);

                    if !self.config.enable_fallback {
                        break;
                    }
                }
                Err(_) => {
                    self.record_failure(&id, "Request timed out").await;
                    log::warn!("Chat timed out with provider {}", id);
                    last_error = Some(LLMError::Timeout);

                    if !self.config.enable_fallback {
                        break;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            if tried_providers.is_empty() {
                LLMError::NoProvidersAvailable
            } else {
                LLMError::ApiError {
                    status: 503,
                    message: format!("All providers failed: {:?}", tried_providers),
                }
            }
        }))
    }

    /// Send a streaming chat request
    pub async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>> {
        // Check budget
        if !self.cost_tracker.read().await.is_within_budget() {
            return Err(LLMError::BudgetExceeded(
                "Monthly or daily budget exceeded".to_string(),
            ));
        }

        // Get the best available provider
        let provider = self
            .get_next_provider(&request)
            .await
            .ok_or(LLMError::NoProvidersAvailable)?;

        let id = provider.id().to_string();

        if !provider.supports_streaming() {
            return Err(LLMError::StreamingNotSupported(id));
        }

        let stream_id = uuid::Uuid::new_v4().to_string();
        let model = provider.model().to_string();

        // Create stream state
        {
            let mut streams = self.active_streams.write().await;
            streams.insert(
                stream_id.clone(),
                StreamState {
                    stream_id: stream_id.clone(),
                    provider: id.clone(),
                    model: model.clone(),
                    is_canceled: false,
                    chunks_received: 0,
                },
            );
        }

        // Create channel
        let (tx, rx) = mpsc::channel::<Result<ChatChunk>>(100);

        // Clone what we need for the async task
        let router_health = Arc::clone(&self.health_tracker);
        let router_cost = Arc::clone(&self.cost_tracker);
        let router_stats = Arc::clone(&self.stats);
        let router_streams = Arc::clone(&self.active_streams);
        let stream_id_clone = stream_id.clone();
        let id_clone = id.clone();
        let model_clone = model.clone();
        let request_timeout = self.config.request_timeout;

        // Spawn streaming task
        tokio::spawn(async move {
            let start = Instant::now();

            let result = timeout(request_timeout, provider.stream_chat(request)).await;

            match result {
                Ok(Ok(mut stream_rx)) => {
                    let mut total_usage: Option<TokenUsage> = None;

                    while let Some(chunk_result) = stream_rx.recv().await {
                        // Check if canceled
                        {
                            let streams = router_streams.read().await;
                            if let Some(state) = streams.get(&stream_id_clone) {
                                if state.is_canceled {
                                    let _ = tx.send(Err(LLMError::StreamCanceled)).await;
                                    break;
                                }
                            }
                        }

                        match chunk_result {
                            Ok(chunk) => {
                                if chunk.is_final {
                                    total_usage = chunk.usage.clone();
                                }
                                if tx.send(Ok(chunk)).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(Err(e)).await;
                                break;
                            }
                        }
                    }

                    // Record success
                    let latency = start.elapsed().as_millis() as u64;
                    router_health
                        .write()
                        .await
                        .record_success(&id_clone, Some(latency));

                    if let Some(ref usage) = total_usage {
                        let cost = router_cost
                            .write()
                            .await
                            .record_usage(&id_clone, &model_clone, usage);
                        if let Some(stats) = router_stats.write().await.get_mut(&id_clone) {
                            stats.record_success(latency, Some(usage), cost);
                        }
                    }
                }
                Ok(Err(e)) => {
                    router_health
                        .write()
                        .await
                        .record_failure(&id_clone, &e.to_string());
                    if let Some(stats) = router_stats.write().await.get_mut(&id_clone) {
                        stats.record_failure();
                    }
                    let _ = tx.send(Err(e)).await;
                }
                Err(_) => {
                    router_health
                        .write()
                        .await
                        .record_failure(&id_clone, "Stream timeout");
                    if let Some(stats) = router_stats.write().await.get_mut(&id_clone) {
                        stats.record_failure();
                    }
                    let _ = tx.send(Err(LLMError::Timeout)).await;
                }
            }

            // Cleanup stream state
            router_streams.write().await.remove(&stream_id_clone);
        });

        Ok(rx)
    }

    /// Cancel an active stream
    pub async fn cancel_stream(&self, stream_id: &str) -> bool {
        let mut streams = self.active_streams.write().await;
        if let Some(state) = streams.get_mut(stream_id) {
            state.is_canceled = true;
            true
        } else {
            false
        }
    }

    /// Get active stream IDs
    pub async fn active_stream_ids(&self) -> Vec<String> {
        self.active_streams
            .read()
            .await
            .keys()
            .cloned()
            .collect()
    }

    /// Run health checks on all providers
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();

        for (id, provider) in &self.providers {
            let healthy = provider.health_check().await;
            results.insert(id.clone(), healthy);

            if healthy {
                self.health_tracker.write().await.record_success(id, None);
            } else {
                self.health_tracker
                    .write()
                    .await
                    .record_failure(id, "Health check failed");
            }
        }

        results
    }

    /// Reset circuit breaker for a provider
    pub async fn reset_circuit(&self, id: &str) {
        self.health_tracker.write().await.reset_circuit(id);
    }

    /// Reset all cost tracking
    pub async fn reset_costs(&self) {
        self.cost_tracker.write().await.reset();
    }

    /// Get router configuration
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    /// Builder for creating a router
    pub fn builder() -> LLMRouterBuilder {
        LLMRouterBuilder::new()
    }
}
