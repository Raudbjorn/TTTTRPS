//! LLM Provider Router
//!
//! Provides intelligent routing between LLM providers with:
//! - Unified `LLMProvider` trait for all providers
//! - Health tracking and circuit breaker pattern
//! - Automatic failover when providers fail
//! - Cost tracking and budget management
//! - Multiple routing strategies
//! - Streaming support

use super::cost::{CostSummary, CostTracker, CostTrackerConfig, ProviderPricing, TokenUsage};
use super::health::{CircuitState, HealthTracker, HealthTrackerConfig, ProviderHealth, HealthSummary};
use async_trait::async_trait;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during LLM operations
#[derive(Debug, thiserror::Error)]
pub enum LLMError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limited: retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Provider not configured: {0}")]
    NotConfigured(String),

    #[error("Embedding not supported for provider: {0}")]
    EmbeddingNotSupported(String),

    #[error("Streaming not supported for provider: {0}")]
    StreamingNotSupported(String),

    #[error("Budget exceeded: {0}")]
    BudgetExceeded(String),

    #[error("No healthy providers available")]
    NoProvidersAvailable,

    #[error("Request timeout")]
    Timeout,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Stream cancelled")]
    StreamCancelled,
}

pub type Result<T> = std::result::Result<T, LLMError>;

// ============================================================================
// Message Types
// ============================================================================

/// Role of a message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// A single message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

/// Request for a chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Optional: Request specific provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl ChatRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages,
            system_prompt: None,
            temperature: None,
            max_tokens: None,
            provider: None,
        }
    }

    pub fn with_system(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn with_max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }
}

/// Response from a chat completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    pub usage: Option<TokenUsage>,
    pub finish_reason: Option<String>,
    pub latency_ms: u64,
    pub cost_usd: Option<f64>,
}

/// A chunk from a streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    pub stream_id: String,
    pub content: String,
    pub provider: String,
    pub model: String,
    pub is_final: bool,
    pub finish_reason: Option<String>,
    pub usage: Option<TokenUsage>,
    pub index: u32,
}

// ============================================================================
// LLM Provider Trait
// ============================================================================

/// Trait that all LLM providers must implement
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the provider's unique identifier
    fn id(&self) -> &str;

    /// Get the provider's display name
    fn name(&self) -> &str;

    /// Get the model being used
    fn model(&self) -> &str;

    /// Check if the provider is healthy/available
    async fn health_check(&self) -> bool;

    /// Get pricing information for this provider/model
    fn pricing(&self) -> Option<ProviderPricing>;

    /// Send a chat completion request
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;

    /// Send a streaming chat request
    /// Returns a receiver that yields ChatChunk events
    async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk>>>;

    /// Check if streaming is supported
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Check if embeddings are supported
    fn supports_embeddings(&self) -> bool {
        false
    }
}

// ============================================================================
// Provider Statistics
// ============================================================================

/// Statistics for a single provider
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_latency_ms: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    #[serde(skip)]
    pub last_used: Option<Instant>,
}

impl ProviderStats {
    pub fn avg_latency_ms(&self) -> u64 {
        if self.successful_requests == 0 {
            0
        } else {
            self.total_latency_ms / self.successful_requests
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }

    pub fn total_tokens(&self) -> u64 {
        self.total_input_tokens + self.total_output_tokens
    }

    pub fn avg_tokens_per_request(&self) -> u64 {
        if self.successful_requests == 0 {
            0
        } else {
            self.total_tokens() / self.successful_requests
        }
    }

    pub fn record_success(&mut self, latency_ms: u64, usage: Option<&TokenUsage>, cost: f64) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.total_latency_ms += latency_ms;
        self.total_cost_usd += cost;
        self.last_used = Some(Instant::now());

        if let Some(u) = usage {
            self.total_input_tokens += u.input_tokens as u64;
            self.total_output_tokens += u.output_tokens as u64;
        }
    }

    pub fn record_failure(&mut self) {
        self.total_requests += 1;
        self.failed_requests += 1;
        self.last_used = Some(Instant::now());
    }
}

// ============================================================================
// Routing Strategy
// ============================================================================

/// Strategy for selecting providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RoutingStrategy {
    /// Use providers in configured priority order
    #[default]
    Priority,
    /// Prefer the cheapest available provider
    CostOptimized,
    /// Prefer the fastest provider based on latency
    LatencyOptimized,
    /// Round-robin between healthy providers
    RoundRobin,
    /// Random selection from healthy providers
    Random,
}

// ============================================================================
// Router Configuration
// ============================================================================

/// Configuration for the LLM router
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Request timeout
    pub request_timeout: Duration,
    /// Whether to enable automatic fallback on failure
    pub enable_fallback: bool,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Routing strategy
    pub routing_strategy: RoutingStrategy,
    /// Maximum retries per provider
    pub max_retries: u32,
    /// Optional monthly budget in USD
    pub monthly_budget: Option<f64>,
    /// Optional daily budget in USD
    pub daily_budget: Option<f64>,
    /// Streaming chunk timeout
    pub stream_chunk_timeout: Duration,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(120),
            enable_fallback: true,
            health_check_interval: Duration::from_secs(60),
            routing_strategy: RoutingStrategy::Priority,
            max_retries: 1,
            monthly_budget: None,
            daily_budget: None,
            stream_chunk_timeout: Duration::from_secs(30),
        }
    }
}

// ============================================================================
// Active Stream State
// ============================================================================

/// State for tracking active streams
#[derive(Debug)]
struct StreamState {
    stream_id: String,
    provider: String,
    model: String,
    is_cancelled: bool,
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
        self.stats.write().await.insert(id.clone(), ProviderStats::default());

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
        self.health_tracker.write().await.is_available(id)
    }

    /// Record successful request
    async fn record_success(&self, id: &str, latency_ms: u64, usage: Option<&TokenUsage>, model: &str) {
        // Update health tracker
        self.health_tracker.write().await.record_success(id, Some(latency_ms));

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
                        cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|s| s.as_str())
            }

            RoutingStrategy::LatencyOptimized => {
                let stats = self.stats.read().await;
                available
                    .into_iter()
                    .min_by_key(|id| {
                        stats.get(id.as_str()).map(|s| s.avg_latency_ms()).unwrap_or(u64::MAX)
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
        let providers_to_try: Vec<Arc<dyn LLMProvider>> = if let Some(ref requested) = request.provider {
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
            // Use routing strategy to get ordered list
            self.provider_order
                .iter()
                .filter_map(|id| self.providers.get(id).cloned())
                .collect()
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
                    log::info!(
                        "Chat succeeded with provider {} ({}ms)",
                        id,
                        latency
                    );
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
                    is_cancelled: false,
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
                        // Check if cancelled
                        {
                            let streams = router_streams.read().await;
                            if let Some(state) = streams.get(&stream_id_clone) {
                                if state.is_cancelled {
                                    let _ = tx.send(Err(LLMError::StreamCancelled)).await;
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
            state.is_cancelled = true;
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

// ============================================================================
// Router Builder
// ============================================================================

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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_builders() {
        let system = ChatMessage::system("You are helpful");
        assert_eq!(system.role, MessageRole::System);

        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, MessageRole::User);

        let assistant = ChatMessage::assistant("Hi there");
        assert_eq!(assistant.role, MessageRole::Assistant);
    }

    #[test]
    fn test_chat_request_builder() {
        let request = ChatRequest::new(vec![ChatMessage::user("Hi")])
            .with_system("Be helpful")
            .with_temperature(0.7)
            .with_max_tokens(1000)
            .with_provider("openai");

        assert_eq!(request.system_prompt, Some("Be helpful".to_string()));
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(1000));
        assert_eq!(request.provider, Some("openai".to_string()));
    }

    #[test]
    fn test_provider_stats() {
        let mut stats = ProviderStats::default();

        stats.record_success(100, Some(&TokenUsage::new(100, 50)), 0.01);
        assert_eq!(stats.successful_requests, 1);
        assert_eq!(stats.total_latency_ms, 100);
        assert_eq!(stats.total_input_tokens, 100);
        assert_eq!(stats.total_output_tokens, 50);

        stats.record_failure();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.failed_requests, 1);

        assert_eq!(stats.success_rate(), 0.5);
        assert_eq!(stats.avg_latency_ms(), 100);
    }

    #[test]
    fn test_routing_strategy() {
        assert_eq!(RoutingStrategy::default(), RoutingStrategy::Priority);
        assert_ne!(RoutingStrategy::Priority, RoutingStrategy::CostOptimized);
    }

    #[test]
    fn test_router_config_defaults() {
        let config = RouterConfig::default();
        assert_eq!(config.request_timeout, Duration::from_secs(120));
        assert!(config.enable_fallback);
        assert_eq!(config.routing_strategy, RoutingStrategy::Priority);
    }
}
