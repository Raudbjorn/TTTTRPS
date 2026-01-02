//! LLM Provider Router
//!
//! Intelligent routing between LLM providers with health checking,
//! automatic fallback, circuit breaker pattern, cost tracking, and streaming support.

use crate::core::llm::{LLMClient, LLMConfig, LLMError, ChatRequest, ChatResponse, EmbeddingResponse, TokenUsage};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::timeout;

// ============================================================================
// Circuit Breaker
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing, reject requests
    HalfOpen,    // Testing if recovered
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout_duration: Duration,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure: None,
            failure_threshold: 3,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(30),
        }
    }
}

impl CircuitBreaker {
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has passed
                if let Some(last) = self.last_failure {
                    if last.elapsed() >= self.timeout_duration {
                        self.state = CircuitState::HalfOpen;
                        self.success_count = 0;
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&mut self) {
        self.failure_count = 0;
        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                }
            }
            _ => {
                self.state = CircuitState::Closed;
            }
        }
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());
        self.success_count = 0;

        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
        }
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }
}

// ============================================================================
// Provider Stats
// ============================================================================

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
}

// ============================================================================
// Health Tracker
// ============================================================================

/// Tracks health status of LLM providers with detailed failure information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthTracker {
    /// Provider health status
    pub providers: HashMap<String, ProviderHealth>,
    /// Health check interval in seconds
    pub check_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_id: String,
    pub is_healthy: bool,
    pub last_check_timestamp: i64,
    pub last_success_timestamp: Option<i64>,
    pub last_failure_timestamp: Option<i64>,
    pub last_failure_reason: Option<String>,
    pub consecutive_failures: u32,
    pub uptime_percentage: f64,
    /// Health checks performed
    pub total_checks: u64,
    pub successful_checks: u64,
}

impl Default for ProviderHealth {
    fn default() -> Self {
        Self {
            provider_id: String::new(),
            is_healthy: true,
            last_check_timestamp: chrono::Utc::now().timestamp(),
            last_success_timestamp: Some(chrono::Utc::now().timestamp()),
            last_failure_timestamp: None,
            last_failure_reason: None,
            consecutive_failures: 0,
            uptime_percentage: 100.0,
            total_checks: 0,
            successful_checks: 0,
        }
    }
}

impl ProviderHealth {
    pub fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            ..Default::default()
        }
    }

    pub fn record_success(&mut self) {
        let now = chrono::Utc::now().timestamp();
        self.is_healthy = true;
        self.last_check_timestamp = now;
        self.last_success_timestamp = Some(now);
        self.consecutive_failures = 0;
        self.total_checks += 1;
        self.successful_checks += 1;
        self.update_uptime();
    }

    pub fn record_failure(&mut self, reason: &str) {
        let now = chrono::Utc::now().timestamp();
        self.last_check_timestamp = now;
        self.last_failure_timestamp = Some(now);
        self.last_failure_reason = Some(reason.to_string());
        self.consecutive_failures += 1;
        self.total_checks += 1;

        // Mark unhealthy after 3 consecutive failures
        if self.consecutive_failures >= 3 {
            self.is_healthy = false;
        }
        self.update_uptime();
    }

    fn update_uptime(&mut self) {
        if self.total_checks > 0 {
            self.uptime_percentage = (self.successful_checks as f64 / self.total_checks as f64) * 100.0;
        }
    }
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
            check_interval_secs: 60,
        }
    }
}

impl HealthTracker {
    pub fn new(check_interval_secs: u64) -> Self {
        Self {
            providers: HashMap::new(),
            check_interval_secs,
        }
    }

    pub fn add_provider(&mut self, provider_id: &str) {
        self.providers.insert(
            provider_id.to_string(),
            ProviderHealth::new(provider_id),
        );
    }

    pub fn remove_provider(&mut self, provider_id: &str) {
        self.providers.remove(provider_id);
    }

    pub fn record_success(&mut self, provider_id: &str) {
        if let Some(health) = self.providers.get_mut(provider_id) {
            health.record_success();
        }
    }

    pub fn record_failure(&mut self, provider_id: &str, reason: &str) {
        if let Some(health) = self.providers.get_mut(provider_id) {
            health.record_failure(reason);
        }
    }

    pub fn is_healthy(&self, provider_id: &str) -> bool {
        self.providers
            .get(provider_id)
            .map(|h| h.is_healthy)
            .unwrap_or(false)
    }

    pub fn get_health(&self, provider_id: &str) -> Option<&ProviderHealth> {
        self.providers.get(provider_id)
    }

    pub fn healthy_providers(&self) -> Vec<&str> {
        self.providers
            .iter()
            .filter(|(_, h)| h.is_healthy)
            .map(|(id, _)| id.as_str())
            .collect()
    }

    pub fn all_health(&self) -> &HashMap<String, ProviderHealth> {
        &self.providers
    }
}

// ============================================================================
// Cost Tracker
// ============================================================================

/// Pricing information for a provider/model combination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPricing {
    pub provider_id: String,
    pub model_id: String,
    /// Cost per 1M input tokens in USD
    pub input_cost_per_million: f64,
    /// Cost per 1M output tokens in USD
    pub output_cost_per_million: f64,
    /// Optional context window size
    pub context_window: Option<u32>,
    /// Optional max output tokens
    pub max_output_tokens: Option<u32>,
}

impl ProviderPricing {
    /// Calculate cost for a given token usage
    pub fn calculate_cost(&self, usage: &TokenUsage) -> f64 {
        let input_cost = (usage.input_tokens as f64 / 1_000_000.0) * self.input_cost_per_million;
        let output_cost = (usage.output_tokens as f64 / 1_000_000.0) * self.output_cost_per_million;
        input_cost + output_cost
    }

    /// Get known pricing for common models
    pub fn for_model(provider: &str, model: &str) -> Option<Self> {
        // Common model pricing (as of late 2024/early 2025)
        let (input, output) = match (provider, model) {
            // Claude models
            ("claude", m) if m.contains("opus") => (15.0, 75.0),
            ("claude", m) if m.contains("sonnet-4") || m.contains("3-5-sonnet") => (3.0, 15.0),
            ("claude", m) if m.contains("haiku") => (0.25, 1.25),

            // OpenAI models
            ("openai", m) if m.contains("gpt-4o-mini") => (0.15, 0.60),
            ("openai", m) if m.contains("gpt-4o") => (2.50, 10.0),
            ("openai", m) if m.contains("gpt-4-turbo") => (10.0, 30.0),
            ("openai", m) if m.contains("o1-preview") => (15.0, 60.0),
            ("openai", m) if m.contains("o1-mini") => (3.0, 12.0),

            // Gemini models
            ("gemini", m) if m.contains("1.5-pro") => (1.25, 5.0),
            ("gemini", m) if m.contains("1.5-flash") => (0.075, 0.30),
            ("gemini", m) if m.contains("2.0-flash") => (0.10, 0.40),

            // Groq (faster, cheaper inference)
            ("groq", m) if m.contains("llama-3.3-70b") => (0.59, 0.79),
            ("groq", m) if m.contains("llama-3.1-8b") => (0.05, 0.08),
            ("groq", m) if m.contains("mixtral") => (0.24, 0.24),

            // Mistral
            ("mistral", m) if m.contains("large") => (2.0, 6.0),
            ("mistral", m) if m.contains("medium") => (2.7, 8.1),
            ("mistral", m) if m.contains("small") => (0.2, 0.6),

            // DeepSeek (very cheap)
            ("deepseek", _) => (0.14, 0.28),

            // Cohere
            ("cohere", m) if m.contains("command-r-plus") => (2.5, 10.0),
            ("cohere", m) if m.contains("command-r") => (0.5, 1.5),

            // Ollama (local, free)
            ("ollama", _) => (0.0, 0.0),

            _ => return None,
        };

        Some(ProviderPricing {
            provider_id: provider.to_string(),
            model_id: model.to_string(),
            input_cost_per_million: input,
            output_cost_per_million: output,
            context_window: None,
            max_output_tokens: None,
        })
    }
}

/// Tracks costs across all providers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostTracker {
    /// Pricing information per provider/model
    pub pricing: HashMap<String, ProviderPricing>,
    /// Accumulated costs per provider
    pub costs: HashMap<String, ProviderCosts>,
    /// Total cost across all providers
    pub total_cost_usd: f64,
    /// Optional monthly budget in USD
    pub monthly_budget: Option<f64>,
    /// Cost this month
    pub monthly_cost: f64,
    /// Month being tracked (YYYY-MM format)
    pub current_month: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderCosts {
    pub provider_id: String,
    pub total_cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub request_count: u64,
}

impl ProviderCosts {
    pub fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            ..Default::default()
        }
    }
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            current_month: chrono::Utc::now().format("%Y-%m").to_string(),
            ..Default::default()
        }
    }

    /// Set pricing for a provider/model
    pub fn set_pricing(&mut self, pricing: ProviderPricing) {
        let key = format!("{}:{}", pricing.provider_id, pricing.model_id);
        self.pricing.insert(key, pricing);
    }

    /// Get pricing for a provider/model
    pub fn get_pricing(&self, provider: &str, model: &str) -> Option<&ProviderPricing> {
        let key = format!("{}:{}", provider, model);
        self.pricing.get(&key)
    }

    /// Record usage and calculate cost
    pub fn record_usage(&mut self, provider: &str, model: &str, usage: &TokenUsage) -> f64 {
        // Check if we need to reset monthly tracking
        let current_month = chrono::Utc::now().format("%Y-%m").to_string();
        if current_month != self.current_month {
            self.monthly_cost = 0.0;
            self.current_month = current_month;
        }

        // Get or create pricing
        let pricing = self.get_pricing(provider, model)
            .cloned()
            .or_else(|| ProviderPricing::for_model(provider, model));

        let cost = pricing.as_ref().map(|p| p.calculate_cost(usage)).unwrap_or(0.0);

        // Update provider costs
        let provider_costs = self.costs
            .entry(provider.to_string())
            .or_insert_with(|| ProviderCosts::new(provider));

        provider_costs.total_cost_usd += cost;
        provider_costs.input_tokens += usage.input_tokens as u64;
        provider_costs.output_tokens += usage.output_tokens as u64;
        provider_costs.request_count += 1;

        // Update totals
        self.total_cost_usd += cost;
        self.monthly_cost += cost;

        cost
    }

    /// Check if within budget
    pub fn is_within_budget(&self) -> bool {
        match self.monthly_budget {
            Some(budget) => self.monthly_cost <= budget,
            None => true,
        }
    }

    /// Get remaining budget
    pub fn remaining_budget(&self) -> Option<f64> {
        self.monthly_budget.map(|b| (b - self.monthly_cost).max(0.0))
    }

    /// Estimate cost for a request
    pub fn estimate_cost(&self, provider: &str, model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
        let usage = TokenUsage { input_tokens, output_tokens };
        let pricing = self.get_pricing(provider, model)
            .cloned()
            .or_else(|| ProviderPricing::for_model(provider, model));

        pricing.map(|p| p.calculate_cost(&usage)).unwrap_or(0.0)
    }

    /// Get all costs by provider
    pub fn costs_by_provider(&self) -> &HashMap<String, ProviderCosts> {
        &self.costs
    }
}

// ============================================================================
// Streaming Types
// ============================================================================

/// A single chunk from a streaming response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    /// Unique ID for this stream
    pub stream_id: String,
    /// The content delta
    pub content: String,
    /// Provider that generated this chunk
    pub provider: String,
    /// Model used
    pub model: String,
    /// Whether this is the final chunk
    pub is_final: bool,
    /// Finish reason if final (stop, length, etc.)
    pub finish_reason: Option<String>,
    /// Token usage (only present in final chunk)
    pub usage: Option<TokenUsage>,
    /// Chunk index in stream
    pub index: u32,
}

/// Stream state for managing active streams
#[derive(Debug)]
pub struct StreamState {
    pub stream_id: String,
    pub provider: String,
    pub model: String,
    pub is_cancelled: bool,
    pub chunks_received: u32,
    pub total_content: String,
}

/// Cost summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    pub total_cost_usd: f64,
    pub monthly_cost: f64,
    pub monthly_budget: Option<f64>,
    pub remaining_budget: Option<f64>,
    pub is_within_budget: bool,
    pub costs_by_provider: HashMap<String, ProviderCosts>,
}

// ============================================================================
// Router Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Request timeout
    pub request_timeout: Duration,
    /// Whether to enable automatic fallback
    pub enable_fallback: bool,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Whether to prefer cheaper providers when available
    pub prefer_cost_optimization: bool,
    /// Optional monthly budget in USD
    pub monthly_budget: Option<f64>,
    /// Maximum retries per provider
    pub max_retries_per_provider: u32,
    /// Streaming chunk timeout
    pub stream_chunk_timeout: Duration,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(120),
            enable_fallback: true,
            health_check_interval: Duration::from_secs(60),
            prefer_cost_optimization: false,
            monthly_budget: None,
            max_retries_per_provider: 1,
            stream_chunk_timeout: Duration::from_secs(30),
        }
    }
}

/// Routing strategy for selecting providers
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Use providers in configured priority order
    Priority,
    /// Prefer the cheapest available provider
    CostOptimized,
    /// Prefer the fastest provider based on latency
    LatencyOptimized,
    /// Round-robin between healthy providers
    RoundRobin,
}

// ============================================================================
// LLM Router
// ============================================================================

#[derive(Clone)]
pub struct LLMRouter {
    /// Available providers in priority order
    providers: Vec<(String, LLMConfig)>,
    /// Circuit breakers per provider
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    /// Stats per provider
    stats: Arc<RwLock<HashMap<String, ProviderStats>>>,
    /// Health tracker
    health_tracker: Arc<RwLock<HealthTracker>>,
    /// Cost tracker
    cost_tracker: Arc<RwLock<CostTracker>>,
    /// Active streams
    active_streams: Arc<RwLock<HashMap<String, StreamState>>>,
    /// Router configuration
    config: RouterConfig,
    /// Routing strategy
    routing_strategy: RoutingStrategy,
    /// Round-robin index
    round_robin_index: Arc<RwLock<usize>>,
}

impl LLMRouter {
    pub fn new(config: RouterConfig) -> Self {
        let mut cost_tracker = CostTracker::new();
        cost_tracker.monthly_budget = config.monthly_budget;

        Self {
            providers: Vec::new(),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            health_tracker: Arc::new(RwLock::new(HealthTracker::new(config.health_check_interval.as_secs()))),
            cost_tracker: Arc::new(RwLock::new(cost_tracker)),
            active_streams: Arc::new(RwLock::new(HashMap::new())),
            config,
            routing_strategy: RoutingStrategy::Priority,
            round_robin_index: Arc::new(RwLock::new(0)),
        }
    }

    /// Set the routing strategy
    pub fn set_routing_strategy(&mut self, strategy: RoutingStrategy) {
        self.routing_strategy = strategy;
    }

    /// Get the current routing strategy
    pub fn routing_strategy(&self) -> RoutingStrategy {
        self.routing_strategy
    }

    /// Add a provider to the router
    pub fn add_provider(&mut self, name: impl Into<String>, config: LLMConfig) {
        let name = name.into();

        // Get model name from config for cost tracking
        let model_name = Self::get_model_from_config(&config);

        self.providers.push((name.clone(), config));

        // Initialize circuit breaker and stats
        if let Ok(mut breakers) = self.circuit_breakers.write() {
            breakers.insert(name.clone(), CircuitBreaker::default());
        }
        if let Ok(mut stats) = self.stats.write() {
            stats.insert(name.clone(), ProviderStats::default());
        }
        // Initialize health tracker
        if let Ok(mut health) = self.health_tracker.write() {
            health.add_provider(&name);
        }
        // Set up pricing if known
        if let Some(pricing) = ProviderPricing::for_model(&name, &model_name) {
            if let Ok(mut costs) = self.cost_tracker.write() {
                costs.set_pricing(pricing);
            }
        }
    }

    /// Remove a provider
    pub fn remove_provider(&mut self, name: &str) {
        self.providers.retain(|(n, _)| n != name);
        if let Ok(mut breakers) = self.circuit_breakers.write() {
            breakers.remove(name);
        }
        if let Ok(mut stats) = self.stats.write() {
            stats.remove(name);
        }
        if let Ok(mut health) = self.health_tracker.write() {
            health.remove_provider(name);
        }
    }

    /// Get provider names in priority order
    pub fn provider_names(&self) -> Vec<String> {
        self.providers.iter().map(|(n, _)| n.clone()).collect()
    }

    /// Get stats for a provider
    pub fn get_stats(&self, name: &str) -> Option<ProviderStats> {
        self.stats.read().ok()?.get(name).cloned()
    }

    /// Get all provider stats
    pub fn get_all_stats(&self) -> HashMap<String, ProviderStats> {
        self.stats.read().ok().map(|s| s.clone()).unwrap_or_default()
    }

    /// Get circuit breaker state for a provider
    pub fn get_circuit_state(&self, name: &str) -> Option<CircuitState> {
        self.circuit_breakers.read().ok()?.get(name).map(|cb| cb.state())
    }

    /// Get health tracker (read-only)
    pub fn get_health_tracker(&self) -> Arc<RwLock<HealthTracker>> {
        Arc::clone(&self.health_tracker)
    }

    /// Get cost tracker (read-only)
    pub fn get_cost_tracker(&self) -> Arc<RwLock<CostTracker>> {
        Arc::clone(&self.cost_tracker)
    }

    /// Get all health statuses
    pub fn get_all_health(&self) -> HashMap<String, ProviderHealth> {
        self.health_tracker.read().ok()
            .map(|h| h.providers.clone())
            .unwrap_or_default()
    }

    /// Get healthy providers
    pub fn healthy_providers(&self) -> Vec<String> {
        self.health_tracker.read().ok()
            .map(|h| h.healthy_providers().into_iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }

    /// Get cost summary
    pub fn get_cost_summary(&self) -> Option<CostSummary> {
        self.cost_tracker.read().ok().map(|c| CostSummary {
            total_cost_usd: c.total_cost_usd,
            monthly_cost: c.monthly_cost,
            monthly_budget: c.monthly_budget,
            remaining_budget: c.remaining_budget(),
            is_within_budget: c.is_within_budget(),
            costs_by_provider: c.costs.clone(),
        })
    }

    /// Estimate cost for a request
    pub fn estimate_cost(&self, provider: &str, model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
        self.cost_tracker.read().ok()
            .map(|c| c.estimate_cost(provider, model, input_tokens, output_tokens))
            .unwrap_or(0.0)
    }

    /// Check if a provider is available (circuit not open)
    fn is_provider_available(&self, name: &str) -> bool {
        if let Ok(mut breakers) = self.circuit_breakers.write() {
            if let Some(cb) = breakers.get_mut(name) {
                return cb.can_execute();
            }
        }
        false
    }

    /// Record a successful request with token usage
    fn record_success_with_usage(&self, name: &str, latency_ms: u64, usage: Option<&TokenUsage>, model: &str) {
        // Update circuit breaker
        if let Ok(mut breakers) = self.circuit_breakers.write() {
            if let Some(cb) = breakers.get_mut(name) {
                cb.record_success();
            }
        }
        // Update stats
        if let Ok(mut stats) = self.stats.write() {
            if let Some(s) = stats.get_mut(name) {
                s.total_requests += 1;
                s.successful_requests += 1;
                s.total_latency_ms += latency_ms;
                s.last_used = Some(Instant::now());

                if let Some(u) = usage {
                    s.total_input_tokens += u.input_tokens as u64;
                    s.total_output_tokens += u.output_tokens as u64;
                }
            }
        }
        // Update health tracker
        if let Ok(mut health) = self.health_tracker.write() {
            health.record_success(name);
        }
        // Update cost tracker
        if let Some(u) = usage {
            if let Ok(mut costs) = self.cost_tracker.write() {
                let cost = costs.record_usage(name, model, u);
                // Also update stats with cost
                if let Ok(mut stats) = self.stats.write() {
                    if let Some(s) = stats.get_mut(name) {
                        s.total_cost_usd += cost;
                    }
                }
            }
        }
    }

    /// Record a successful request (backward compatible)
    fn record_success(&self, name: &str, latency_ms: u64) {
        self.record_success_with_usage(name, latency_ms, None, "");
    }

    /// Record a failed request with reason
    fn record_failure_with_reason(&self, name: &str, reason: &str) {
        if let Ok(mut breakers) = self.circuit_breakers.write() {
            if let Some(cb) = breakers.get_mut(name) {
                cb.record_failure();
            }
        }
        if let Ok(mut stats) = self.stats.write() {
            if let Some(s) = stats.get_mut(name) {
                s.total_requests += 1;
                s.failed_requests += 1;
                s.last_used = Some(Instant::now());
            }
        }
        if let Ok(mut health) = self.health_tracker.write() {
            health.record_failure(name, reason);
        }
    }

    /// Record a failed request (backward compatible)
    fn record_failure(&self, name: &str) {
        self.record_failure_with_reason(name, "Unknown error");
    }

    /// Get the next available provider based on routing strategy
    fn get_next_provider(&self) -> Option<(String, LLMConfig)> {
        let available: Vec<_> = self.providers.iter()
            .filter(|(name, _)| self.is_provider_available(name))
            .collect();

        if available.is_empty() {
            return None;
        }

        match self.routing_strategy {
            RoutingStrategy::Priority => {
                available.first().map(|(n, c)| (n.clone(), c.clone()))
            }
            RoutingStrategy::CostOptimized => {
                // Find cheapest provider
                let stats = self.stats.read().ok()?;
                available.into_iter()
                    .min_by(|(a, _), (b, _)| {
                        let cost_a = stats.get(a.as_str()).map(|s| s.total_cost_usd / s.successful_requests.max(1) as f64).unwrap_or(f64::MAX);
                        let cost_b = stats.get(b.as_str()).map(|s| s.total_cost_usd / s.successful_requests.max(1) as f64).unwrap_or(f64::MAX);
                        cost_a.partial_cmp(&cost_b).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(n, c)| (n.clone(), c.clone()))
            }
            RoutingStrategy::LatencyOptimized => {
                // Find fastest provider
                let stats = self.stats.read().ok()?;
                available.into_iter()
                    .min_by_key(|(name, _)| {
                        stats.get(name.as_str()).map(|s| s.avg_latency_ms()).unwrap_or(u64::MAX)
                    })
                    .map(|(n, c)| (n.clone(), c.clone()))
            }
            RoutingStrategy::RoundRobin => {
                let mut index = self.round_robin_index.write().ok()?;
                let provider = available.get(*index % available.len());
                *index = (*index + 1) % available.len().max(1);
                provider.map(|(n, c)| (n.clone(), c.clone()))
            }
        }
    }

    /// Get the next available provider (backward compatible)
    #[allow(dead_code)]
    fn get_available_provider(&self) -> Option<(String, LLMConfig)> {
        self.get_next_provider()
    }

    /// Helper to extract model name from config
    fn get_model_from_config(config: &LLMConfig) -> String {
        match config {
            LLMConfig::Ollama { model, .. } => model.clone(),
            LLMConfig::Claude { model, .. } => model.clone(),
            LLMConfig::Gemini { model, .. } => model.clone(),
            LLMConfig::OpenAI { model, .. } => model.clone(),
            LLMConfig::OpenRouter { model, .. } => model.clone(),
            LLMConfig::Mistral { model, .. } => model.clone(),
            LLMConfig::Groq { model, .. } => model.clone(),
            LLMConfig::Together { model, .. } => model.clone(),
            LLMConfig::Cohere { model, .. } => model.clone(),
            LLMConfig::DeepSeek { model, .. } => model.clone(),
        }
    }

    /// Send a chat request with automatic fallback
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let mut last_error: Option<LLMError> = None;
        let mut tried_providers = Vec::new();

        // Check budget before proceeding
        if let Ok(costs) = self.cost_tracker.read() {
            if !costs.is_within_budget() {
                return Err(LLMError::ApiError {
                    status: 402,
                    message: "Monthly budget exceeded".to_string(),
                });
            }
        }

        // Try providers based on routing strategy
        let providers_to_try: Vec<_> = if self.routing_strategy == RoutingStrategy::Priority {
            self.providers.clone()
        } else if let Some((name, config)) = self.get_next_provider() {
            // For non-priority strategies, get the recommended provider first
            let mut ordered = vec![(name, config)];
            // Add remaining providers as fallbacks
            for (n, c) in &self.providers {
                if !ordered.iter().any(|(on, _)| on == n) {
                    ordered.push((n.clone(), c.clone()));
                }
            }
            ordered
        } else {
            self.providers.clone()
        };

        for (name, config) in &providers_to_try {
            // Skip if circuit is open
            if !self.is_provider_available(name) {
                log::debug!("Skipping provider {} (circuit open)", name);
                continue;
            }

            tried_providers.push(name.clone());
            let client = LLMClient::new(config.clone());
            let model = Self::get_model_from_config(config);
            let start = Instant::now();

            // Execute with timeout
            let result = timeout(
                self.config.request_timeout,
                client.chat(request.clone())
            ).await;

            match result {
                Ok(Ok(response)) => {
                    let latency = start.elapsed().as_millis() as u64;
                    self.record_success_with_usage(name, latency, response.usage.as_ref(), &model);
                    log::info!("Chat succeeded with provider {} ({}ms)", name, latency);
                    return Ok(response);
                }
                Ok(Err(e)) => {
                    let error_msg = e.to_string();
                    self.record_failure_with_reason(name, &error_msg);
                    log::warn!("Chat failed with provider {}: {}", name, e);
                    last_error = Some(e);

                    if !self.config.enable_fallback {
                        break;
                    }
                }
                Err(_) => {
                    self.record_failure_with_reason(name, "Request timed out");
                    log::warn!("Chat timed out with provider {}", name);
                    last_error = Some(LLMError::ApiError {
                        status: 504,
                        message: "Request timed out".to_string(),
                    });

                    if !self.config.enable_fallback {
                        break;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            if tried_providers.is_empty() {
                LLMError::NotConfigured("No providers available (all circuits open)".to_string())
            } else {
                LLMError::ApiError {
                    status: 503,
                    message: format!("All providers failed: {:?}", tried_providers),
                }
            }
        }))
    }

    /// Send a streaming chat request
    /// Returns a channel receiver that yields ChatChunk events
    pub async fn stream_chat(
        &self,
        request: ChatRequest,
    ) -> Result<mpsc::Receiver<Result<ChatChunk, LLMError>>, LLMError> {
        // Check budget before proceeding
        if let Ok(costs) = self.cost_tracker.read() {
            if !costs.is_within_budget() {
                return Err(LLMError::ApiError {
                    status: 402,
                    message: "Monthly budget exceeded".to_string(),
                });
            }
        }

        // Get the best available provider
        let (name, config) = self.get_next_provider()
            .ok_or_else(|| LLMError::NotConfigured("No providers available".to_string()))?;

        let stream_id = uuid::Uuid::new_v4().to_string();
        let model = Self::get_model_from_config(&config);

        // Create stream state
        {
            let mut streams = self.active_streams.write()
                .map_err(|_| LLMError::InvalidResponse("Lock error".to_string()))?;
            streams.insert(stream_id.clone(), StreamState {
                stream_id: stream_id.clone(),
                provider: name.clone(),
                model: model.clone(),
                is_cancelled: false,
                chunks_received: 0,
                total_content: String::new(),
            });
        }

        // Create the channel for streaming chunks
        let (tx, rx) = mpsc::channel::<Result<ChatChunk, LLMError>>(100);

        // Clone what we need for the async task
        let router_clone = self.clone();
        let stream_id_clone = stream_id.clone();
        let name_clone = name.clone();
        let model_clone = model.clone();
        let config_clone = config.clone();
        let request_clone = request.clone();
        let stream_timeout = self.config.stream_chunk_timeout;

        // Spawn the streaming task
        tokio::spawn(async move {
            let start = Instant::now();
            let result = router_clone.execute_stream(
                &name_clone,
                &model_clone,
                &config_clone,
                request_clone,
                &stream_id_clone,
                tx.clone(),
                stream_timeout,
            ).await;

            let latency = start.elapsed().as_millis() as u64;

            // Update stats based on result
            match result {
                Ok(usage) => {
                    router_clone.record_success_with_usage(&name_clone, latency, usage.as_ref(), &model_clone);
                }
                Err(ref e) => {
                    router_clone.record_failure_with_reason(&name_clone, &e.to_string());
                }
            }

            // Clean up stream state
            if let Ok(mut streams) = router_clone.active_streams.write() {
                streams.remove(&stream_id_clone);
            }
        });

        Ok(rx)
    }

    /// Execute a streaming request for a specific provider
    async fn execute_stream(
        &self,
        provider_name: &str,
        model: &str,
        config: &LLMConfig,
        request: ChatRequest,
        stream_id: &str,
        tx: mpsc::Sender<Result<ChatChunk, LLMError>>,
        chunk_timeout: Duration,
    ) -> Result<Option<TokenUsage>, LLMError> {
        // Build the streaming request based on provider
        let result = match config {
            LLMConfig::OpenAI { api_key, model, max_tokens, organization_id, base_url } => {
                self.stream_openai_compatible(
                    api_key, model, *max_tokens, organization_id.as_deref(), base_url,
                    &request, stream_id, provider_name, &tx, chunk_timeout
                ).await
            }
            LLMConfig::Claude { api_key, model, max_tokens } => {
                self.stream_claude(api_key, model, *max_tokens, &request, stream_id, provider_name, &tx, chunk_timeout).await
            }
            LLMConfig::Gemini { api_key, model } => {
                self.stream_gemini(api_key, model, &request, stream_id, provider_name, &tx, chunk_timeout).await
            }
            LLMConfig::Ollama { host, model, .. } => {
                self.stream_ollama(host, model, &request, stream_id, provider_name, &tx, chunk_timeout).await
            }
            // OpenAI-compatible providers
            LLMConfig::OpenRouter { api_key, model } => {
                self.stream_openai_compatible(
                    api_key, model, 4096, None, "https://openrouter.ai/api/v1",
                    &request, stream_id, provider_name, &tx, chunk_timeout
                ).await
            }
            LLMConfig::Mistral { api_key, model } => {
                self.stream_openai_compatible(
                    api_key, model, 4096, None, "https://api.mistral.ai/v1",
                    &request, stream_id, provider_name, &tx, chunk_timeout
                ).await
            }
            LLMConfig::Groq { api_key, model } => {
                self.stream_openai_compatible(
                    api_key, model, 4096, None, "https://api.groq.com/openai/v1",
                    &request, stream_id, provider_name, &tx, chunk_timeout
                ).await
            }
            LLMConfig::Together { api_key, model } => {
                self.stream_openai_compatible(
                    api_key, model, 4096, None, "https://api.together.xyz/v1",
                    &request, stream_id, provider_name, &tx, chunk_timeout
                ).await
            }
            LLMConfig::DeepSeek { api_key, model } => {
                self.stream_openai_compatible(
                    api_key, model, 4096, None, "https://api.deepseek.com/v1",
                    &request, stream_id, provider_name, &tx, chunk_timeout
                ).await
            }
            LLMConfig::Cohere { .. } => {
                // Cohere streaming not implemented - fall back to non-streaming
                Err(LLMError::InvalidResponse("Streaming not supported for Cohere".to_string()))
            }
        };

        result
    }

    /// Stream from OpenAI-compatible API (OpenAI, OpenRouter, Mistral, Groq, Together, DeepSeek)
    async fn stream_openai_compatible(
        &self,
        api_key: &str,
        model: &str,
        max_tokens: u32,
        organization_id: Option<&str>,
        base_url: &str,
        request: &ChatRequest,
        stream_id: &str,
        provider_name: &str,
        tx: &mpsc::Sender<Result<ChatChunk, LLMError>>,
        chunk_timeout: Duration,
    ) -> Result<Option<TokenUsage>, LLMError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| LLMError::HttpError(e))?;

        let url = format!("{}/chat/completions", base_url);

        // Build messages
        let mut messages: Vec<serde_json::Value> = Vec::new();
        if let Some(system) = &request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }
        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": match msg.role {
                    crate::core::llm::MessageRole::System => "system",
                    crate::core::llm::MessageRole::User => "user",
                    crate::core::llm::MessageRole::Assistant => "assistant",
                },
                "content": msg.content
            }));
        }

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(max_tokens),
            "stream": true
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let mut req_builder = client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json");

        if let Some(org_id) = organization_id {
            req_builder = req_builder.header("OpenAI-Organization", org_id);
        }

        let response = req_builder.json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        // Process SSE stream
        let mut chunk_index = 0u32;
        let mut total_content = String::new();
        let mut final_usage: Option<TokenUsage> = None;

        let mut stream = response.bytes_stream();

        loop {
            // Check if stream is cancelled
            if let Ok(streams) = self.active_streams.read() {
                if let Some(state) = streams.get(stream_id) {
                    if state.is_cancelled {
                        break;
                    }
                }
            }

            let chunk_result = timeout(chunk_timeout, stream.next()).await;

            match chunk_result {
                Ok(Some(Ok(bytes))) => {
                    let text = String::from_utf8_lossy(&bytes);

                    // Parse SSE events
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                // Send final chunk
                                let final_chunk = ChatChunk {
                                    stream_id: stream_id.to_string(),
                                    content: String::new(),
                                    provider: provider_name.to_string(),
                                    model: model.to_string(),
                                    is_final: true,
                                    finish_reason: Some("stop".to_string()),
                                    usage: final_usage.clone(),
                                    index: chunk_index,
                                };
                                let _ = tx.send(Ok(final_chunk)).await;
                                return Ok(final_usage);
                            }

                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                // Extract content delta
                                if let Some(delta) = json["choices"][0]["delta"]["content"].as_str() {
                                    if !delta.is_empty() {
                                        total_content.push_str(delta);
                                        chunk_index += 1;

                                        let chunk = ChatChunk {
                                            stream_id: stream_id.to_string(),
                                            content: delta.to_string(),
                                            provider: provider_name.to_string(),
                                            model: model.to_string(),
                                            is_final: false,
                                            finish_reason: None,
                                            usage: None,
                                            index: chunk_index,
                                        };
                                        let _ = tx.send(Ok(chunk)).await;
                                    }
                                }

                                // Check for finish reason
                                if let Some(reason) = json["choices"][0]["finish_reason"].as_str() {
                                    if reason != "null" {
                                        // Extract usage if present
                                        if let Some(usage) = json["usage"].as_object() {
                                            final_usage = Some(TokenUsage {
                                                input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                                                output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    let _ = tx.send(Err(LLMError::HttpError(e))).await;
                    break;
                }
                Ok(None) => break, // Stream ended
                Err(_) => {
                    // Timeout
                    let _ = tx.send(Err(LLMError::ApiError {
                        status: 504,
                        message: "Stream chunk timeout".to_string(),
                    })).await;
                    break;
                }
            }
        }

        Ok(final_usage)
    }

    /// Stream from Claude/Anthropic API
    async fn stream_claude(
        &self,
        api_key: &str,
        model: &str,
        max_tokens: u32,
        request: &ChatRequest,
        stream_id: &str,
        provider_name: &str,
        tx: &mpsc::Sender<Result<ChatChunk, LLMError>>,
        chunk_timeout: Duration,
    ) -> Result<Option<TokenUsage>, LLMError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| LLMError::HttpError(e))?;

        // Build messages (Claude has separate system parameter)
        let messages: Vec<serde_json::Value> = request.messages.iter()
            .filter(|m| m.role != crate::core::llm::MessageRole::System)
            .map(|m| serde_json::json!({
                "role": match m.role {
                    crate::core::llm::MessageRole::User => "user",
                    crate::core::llm::MessageRole::Assistant => "assistant",
                    crate::core::llm::MessageRole::System => "user",
                },
                "content": m.content
            }))
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(max_tokens),
            "stream": true
        });

        if let Some(system) = &request.system_prompt {
            body["system"] = serde_json::Value::String(system.clone());
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let response = client.post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        // Process SSE stream
        let mut chunk_index = 0u32;
        let mut final_usage: Option<TokenUsage> = None;
        let mut input_tokens = 0u32;

        let mut stream = response.bytes_stream();

        loop {
            if let Ok(streams) = self.active_streams.read() {
                if let Some(state) = streams.get(stream_id) {
                    if state.is_cancelled {
                        break;
                    }
                }
            }

            let chunk_result = timeout(chunk_timeout, stream.next()).await;

            match chunk_result {
                Ok(Some(Ok(bytes))) => {
                    let text = String::from_utf8_lossy(&bytes);

                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                let event_type = json["type"].as_str().unwrap_or("");

                                match event_type {
                                    "message_start" => {
                                        if let Some(usage) = json["message"]["usage"].as_object() {
                                            input_tokens = usage["input_tokens"].as_u64().unwrap_or(0) as u32;
                                        }
                                    }
                                    "content_block_delta" => {
                                        if let Some(delta) = json["delta"]["text"].as_str() {
                                            if !delta.is_empty() {
                                                chunk_index += 1;
                                                let chunk = ChatChunk {
                                                    stream_id: stream_id.to_string(),
                                                    content: delta.to_string(),
                                                    provider: provider_name.to_string(),
                                                    model: model.to_string(),
                                                    is_final: false,
                                                    finish_reason: None,
                                                    usage: None,
                                                    index: chunk_index,
                                                };
                                                let _ = tx.send(Ok(chunk)).await;
                                            }
                                        }
                                    }
                                    "message_delta" => {
                                        if let Some(usage) = json["usage"].as_object() {
                                            let output_tokens = usage["output_tokens"].as_u64().unwrap_or(0) as u32;
                                            final_usage = Some(TokenUsage { input_tokens, output_tokens });
                                        }
                                    }
                                    "message_stop" => {
                                        let final_chunk = ChatChunk {
                                            stream_id: stream_id.to_string(),
                                            content: String::new(),
                                            provider: provider_name.to_string(),
                                            model: model.to_string(),
                                            is_final: true,
                                            finish_reason: Some("stop".to_string()),
                                            usage: final_usage.clone(),
                                            index: chunk_index + 1,
                                        };
                                        let _ = tx.send(Ok(final_chunk)).await;
                                        return Ok(final_usage);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    let _ = tx.send(Err(LLMError::HttpError(e))).await;
                    break;
                }
                Ok(None) => break,
                Err(_) => {
                    let _ = tx.send(Err(LLMError::ApiError {
                        status: 504,
                        message: "Stream chunk timeout".to_string(),
                    })).await;
                    break;
                }
            }
        }

        Ok(final_usage)
    }

    /// Stream from Gemini API
    async fn stream_gemini(
        &self,
        api_key: &str,
        model: &str,
        request: &ChatRequest,
        stream_id: &str,
        provider_name: &str,
        tx: &mpsc::Sender<Result<ChatChunk, LLMError>>,
        chunk_timeout: Duration,
    ) -> Result<Option<TokenUsage>, LLMError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| LLMError::HttpError(e))?;

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
            model, api_key
        );

        // Build contents
        let mut contents: Vec<serde_json::Value> = Vec::new();
        for msg in &request.messages {
            let role = match msg.role {
                crate::core::llm::MessageRole::User => "user",
                crate::core::llm::MessageRole::Assistant => "model",
                crate::core::llm::MessageRole::System => continue,
            };
            contents.push(serde_json::json!({
                "role": role,
                "parts": [{ "text": msg.content }]
            }));
        }

        let mut body = serde_json::json!({ "contents": contents });

        if let Some(system) = &request.system_prompt {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{ "text": system }]
            });
        }

        if request.temperature.is_some() || request.max_tokens.is_some() {
            let mut gen_config = serde_json::Map::new();
            if let Some(temp) = request.temperature {
                gen_config.insert("temperature".to_string(), serde_json::json!(temp));
            }
            if let Some(max) = request.max_tokens {
                gen_config.insert("maxOutputTokens".to_string(), serde_json::json!(max));
            }
            body["generationConfig"] = serde_json::Value::Object(gen_config);
        }

        let response = client.post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let mut chunk_index = 0u32;
        let mut final_usage: Option<TokenUsage> = None;

        let mut stream = response.bytes_stream();

        loop {
            if let Ok(streams) = self.active_streams.read() {
                if let Some(state) = streams.get(stream_id) {
                    if state.is_cancelled {
                        break;
                    }
                }
            }

            let chunk_result = timeout(chunk_timeout, stream.next()).await;

            match chunk_result {
                Ok(Some(Ok(bytes))) => {
                    let text = String::from_utf8_lossy(&bytes);

                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                // Extract text from candidates
                                if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                                    if !text.is_empty() {
                                        chunk_index += 1;
                                        let chunk = ChatChunk {
                                            stream_id: stream_id.to_string(),
                                            content: text.to_string(),
                                            provider: provider_name.to_string(),
                                            model: model.to_string(),
                                            is_final: false,
                                            finish_reason: None,
                                            usage: None,
                                            index: chunk_index,
                                        };
                                        let _ = tx.send(Ok(chunk)).await;
                                    }
                                }

                                // Check for usage metadata
                                if let Some(usage) = json["usageMetadata"].as_object() {
                                    final_usage = Some(TokenUsage {
                                        input_tokens: usage["promptTokenCount"].as_u64().unwrap_or(0) as u32,
                                        output_tokens: usage["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
                                    });
                                }

                                // Check for finish reason
                                if let Some(reason) = json["candidates"][0]["finishReason"].as_str() {
                                    if reason != "STOP" && reason != "null" {
                                        let final_chunk = ChatChunk {
                                            stream_id: stream_id.to_string(),
                                            content: String::new(),
                                            provider: provider_name.to_string(),
                                            model: model.to_string(),
                                            is_final: true,
                                            finish_reason: Some(reason.to_string()),
                                            usage: final_usage.clone(),
                                            index: chunk_index + 1,
                                        };
                                        let _ = tx.send(Ok(final_chunk)).await;
                                        return Ok(final_usage);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    let _ = tx.send(Err(LLMError::HttpError(e))).await;
                    break;
                }
                Ok(None) => {
                    // Stream ended, send final chunk
                    let final_chunk = ChatChunk {
                        stream_id: stream_id.to_string(),
                        content: String::new(),
                        provider: provider_name.to_string(),
                        model: model.to_string(),
                        is_final: true,
                        finish_reason: Some("stop".to_string()),
                        usage: final_usage.clone(),
                        index: chunk_index + 1,
                    };
                    let _ = tx.send(Ok(final_chunk)).await;
                    break;
                }
                Err(_) => {
                    let _ = tx.send(Err(LLMError::ApiError {
                        status: 504,
                        message: "Stream chunk timeout".to_string(),
                    })).await;
                    break;
                }
            }
        }

        Ok(final_usage)
    }

    /// Stream from Ollama API
    async fn stream_ollama(
        &self,
        host: &str,
        model: &str,
        request: &ChatRequest,
        stream_id: &str,
        provider_name: &str,
        tx: &mpsc::Sender<Result<ChatChunk, LLMError>>,
        chunk_timeout: Duration,
    ) -> Result<Option<TokenUsage>, LLMError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| LLMError::HttpError(e))?;

        let url = format!("{}/api/chat", host);

        // Build messages
        let mut messages: Vec<serde_json::Value> = Vec::new();
        if let Some(system) = &request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }
        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": match msg.role {
                    crate::core::llm::MessageRole::System => "system",
                    crate::core::llm::MessageRole::User => "user",
                    crate::core::llm::MessageRole::Assistant => "assistant",
                },
                "content": msg.content
            }));
        }

        let mut options = serde_json::Map::new();
        if let Some(temp) = request.temperature {
            options.insert("temperature".to_string(), serde_json::json!(temp));
        }

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "options": options
        });

        let response = client.post(&url)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError { status, message: text });
        }

        let mut chunk_index = 0u32;
        let mut final_usage: Option<TokenUsage> = None;

        let mut stream = response.bytes_stream();

        loop {
            if let Ok(streams) = self.active_streams.read() {
                if let Some(state) = streams.get(stream_id) {
                    if state.is_cancelled {
                        break;
                    }
                }
            }

            let chunk_result = timeout(chunk_timeout, stream.next()).await;

            match chunk_result {
                Ok(Some(Ok(bytes))) => {
                    let text = String::from_utf8_lossy(&bytes);

                    // Ollama returns JSON lines, not SSE
                    for line in text.lines() {
                        if line.is_empty() {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                            if let Some(content) = json["message"]["content"].as_str() {
                                if !content.is_empty() {
                                    chunk_index += 1;
                                    let chunk = ChatChunk {
                                        stream_id: stream_id.to_string(),
                                        content: content.to_string(),
                                        provider: provider_name.to_string(),
                                        model: model.to_string(),
                                        is_final: false,
                                        finish_reason: None,
                                        usage: None,
                                        index: chunk_index,
                                    };
                                    let _ = tx.send(Ok(chunk)).await;
                                }
                            }

                            // Check if done
                            if json["done"].as_bool().unwrap_or(false) {
                                // Extract usage
                                let input_tokens = json["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
                                let output_tokens = json["eval_count"].as_u64().unwrap_or(0) as u32;
                                final_usage = Some(TokenUsage { input_tokens, output_tokens });

                                let final_chunk = ChatChunk {
                                    stream_id: stream_id.to_string(),
                                    content: String::new(),
                                    provider: provider_name.to_string(),
                                    model: model.to_string(),
                                    is_final: true,
                                    finish_reason: Some("stop".to_string()),
                                    usage: final_usage.clone(),
                                    index: chunk_index + 1,
                                };
                                let _ = tx.send(Ok(final_chunk)).await;
                                return Ok(final_usage);
                            }
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    let _ = tx.send(Err(LLMError::HttpError(e))).await;
                    break;
                }
                Ok(None) => break,
                Err(_) => {
                    let _ = tx.send(Err(LLMError::ApiError {
                        status: 504,
                        message: "Stream chunk timeout".to_string(),
                    })).await;
                    break;
                }
            }
        }

        Ok(final_usage)
    }

    /// Cancel an active stream
    pub fn cancel_stream(&self, stream_id: &str) -> bool {
        if let Ok(mut streams) = self.active_streams.write() {
            if let Some(state) = streams.get_mut(stream_id) {
                state.is_cancelled = true;
                return true;
            }
        }
        false
    }

    /// Get active stream IDs
    pub fn active_stream_ids(&self) -> Vec<String> {
        self.active_streams.read().ok()
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Generate embeddings with automatic fallback
    pub async fn embed(&self, text: &str) -> Result<EmbeddingResponse, LLMError> {
        let mut last_error: Option<LLMError> = None;

        for (name, config) in &self.providers {
            if !self.is_provider_available(name) {
                continue;
            }

            let client = LLMClient::new(config.clone());
            let start = Instant::now();

            let result = timeout(
                self.config.request_timeout,
                client.embed(text)
            ).await;

            match result {
                Ok(Ok(response)) => {
                    let latency = start.elapsed().as_millis() as u64;
                    self.record_success(name, latency);
                    return Ok(response);
                }
                Ok(Err(e)) => {
                    // Skip providers that don't support embeddings
                    if matches!(e, LLMError::EmbeddingNotSupported(_)) {
                        continue;
                    }
                    self.record_failure(name);
                    last_error = Some(e);

                    if !self.config.enable_fallback {
                        break;
                    }
                }
                Err(_) => {
                    self.record_failure(name);
                    last_error = Some(LLMError::ApiError {
                        status: 504,
                        message: "Request timed out".to_string(),
                    });

                    if !self.config.enable_fallback {
                        break;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            LLMError::EmbeddingNotSupported("No providers support embeddings".to_string())
        }))
    }

    /// Run health checks on all providers
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();

        for (name, config) in &self.providers {
            let client = LLMClient::new(config.clone());
            let healthy = client.health_check().await.unwrap_or(false);
            results.insert(name.clone(), healthy);

            // Update circuit breaker based on health
            if healthy {
                self.record_success(name, 0);
            }
        }

        results
    }
}

// ============================================================================
// Builder Pattern
// ============================================================================

impl LLMRouter {
    pub fn builder() -> LLMRouterBuilder {
        LLMRouterBuilder::new()
    }
}

pub struct LLMRouterBuilder {
    providers: Vec<(String, LLMConfig)>,
    config: RouterConfig,
    routing_strategy: Option<RoutingStrategy>,
}

impl LLMRouterBuilder {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            config: RouterConfig::default(),
            routing_strategy: None,
        }
    }

    pub fn add_provider(mut self, name: impl Into<String>, config: LLMConfig) -> Self {
        self.providers.push((name.into(), config));
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

    pub fn with_monthly_budget(mut self, budget_usd: f64) -> Self {
        self.config.monthly_budget = Some(budget_usd);
        self
    }

    pub fn with_routing_strategy(mut self, strategy: RoutingStrategy) -> Self {
        // Store strategy to apply after build
        self.routing_strategy = Some(strategy);
        self
    }

    pub fn build(self) -> LLMRouter {
        let mut router = LLMRouter::new(self.config);
        for (name, config) in self.providers {
            router.add_provider(name, config);
        }
        if let Some(strategy) = self.routing_strategy {
            router.set_routing_strategy(strategy);
        }
        router
    }
}

impl Default for LLMRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_default_closed() {
        let mut cb = CircuitBreaker::default();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_opens_after_failures() {
        let mut cb = CircuitBreaker::default();

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Closed);

        cb.record_failure(); // Third failure
        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let mut cb = CircuitBreaker::default();

        cb.record_failure();
        cb.record_failure();
        cb.record_success();

        assert_eq!(cb.state(), CircuitState::Closed);
        assert_eq!(cb.failure_count, 0);
    }

    #[test]
    fn test_provider_stats() {
        let mut stats = ProviderStats::default();

        stats.total_requests = 10;
        stats.successful_requests = 8;
        stats.failed_requests = 2;
        stats.total_latency_ms = 1600;

        assert_eq!(stats.success_rate(), 0.8);
        assert_eq!(stats.avg_latency_ms(), 200);
    }

    #[test]
    fn test_provider_stats_with_tokens() {
        let mut stats = ProviderStats::default();

        stats.total_requests = 5;
        stats.successful_requests = 5;
        stats.total_input_tokens = 1000;
        stats.total_output_tokens = 500;

        assert_eq!(stats.total_tokens(), 1500);
        assert_eq!(stats.avg_tokens_per_request(), 300);
    }

    #[test]
    fn test_health_tracker() {
        let mut tracker = HealthTracker::new(60);

        tracker.add_provider("openai");
        tracker.add_provider("claude");

        assert!(tracker.is_healthy("openai"));
        assert!(tracker.is_healthy("claude"));

        // Record failures
        tracker.record_failure("openai", "API error");
        tracker.record_failure("openai", "API error");

        // Should still be healthy after 2 failures
        assert!(tracker.is_healthy("openai"));

        // Third failure marks unhealthy
        tracker.record_failure("openai", "API error");
        assert!(!tracker.is_healthy("openai"));

        // Success resets
        tracker.record_success("openai");
        assert!(tracker.is_healthy("openai"));

        let health = tracker.get_health("openai").unwrap();
        assert_eq!(health.consecutive_failures, 0);
    }

    #[test]
    fn test_provider_pricing() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
        };

        // Test Claude pricing lookup
        let pricing = ProviderPricing::for_model("claude", "claude-3-5-sonnet").unwrap();
        let cost = pricing.calculate_cost(&usage);
        // (1000/1M * 3.0) + (500/1M * 15.0) = 0.003 + 0.0075 = 0.0105
        assert!((cost - 0.0105).abs() < 0.0001);

        // Test OpenAI pricing
        let pricing = ProviderPricing::for_model("openai", "gpt-4o").unwrap();
        let cost = pricing.calculate_cost(&usage);
        // (1000/1M * 2.5) + (500/1M * 10.0) = 0.0025 + 0.005 = 0.0075
        assert!((cost - 0.0075).abs() < 0.0001);

        // Test Ollama (free)
        let pricing = ProviderPricing::for_model("ollama", "llama3").unwrap();
        let cost = pricing.calculate_cost(&usage);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_cost_tracker() {
        let mut tracker = CostTracker::new();
        tracker.monthly_budget = Some(10.0);

        let usage = TokenUsage {
            input_tokens: 10000,
            output_tokens: 5000,
        };

        // Record usage for Claude
        let cost = tracker.record_usage("claude", "claude-3-5-sonnet", &usage);
        assert!(cost > 0.0);
        assert!(tracker.is_within_budget());

        // Check provider costs were tracked
        let provider_costs = tracker.costs.get("claude").unwrap();
        assert_eq!(provider_costs.request_count, 1);
        assert_eq!(provider_costs.input_tokens, 10000);
        assert_eq!(provider_costs.output_tokens, 5000);
    }

    #[test]
    fn test_cost_tracker_budget() {
        let mut tracker = CostTracker::new();
        tracker.monthly_budget = Some(0.001); // Very small budget

        let usage = TokenUsage {
            input_tokens: 100000,
            output_tokens: 50000,
        };

        // This should exceed the tiny budget
        tracker.record_usage("openai", "gpt-4o", &usage);
        assert!(!tracker.is_within_budget());
        assert_eq!(tracker.remaining_budget(), Some(0.0));
    }

    #[test]
    fn test_chat_chunk_serialization() {
        let chunk = ChatChunk {
            stream_id: "test-123".to_string(),
            content: "Hello".to_string(),
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            is_final: false,
            finish_reason: None,
            usage: None,
            index: 1,
        };

        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("Hello"));

        let parsed: ChatChunk = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.stream_id, "test-123");
        assert_eq!(parsed.content, "Hello");
    }

    #[test]
    fn test_routing_strategy() {
        assert_eq!(RoutingStrategy::Priority, RoutingStrategy::Priority);
        assert_ne!(RoutingStrategy::Priority, RoutingStrategy::CostOptimized);
    }
}
