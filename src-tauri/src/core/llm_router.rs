//! LLM Provider Router
//!
//! Intelligent routing between LLM providers with health checking,
//! automatic fallback, and circuit breaker pattern.

use crate::core::llm::{LLMClient, LLMConfig, LLMError, ChatRequest, ChatResponse, EmbeddingResponse};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
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

#[derive(Debug, Clone, Default)]
pub struct ProviderStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_latency_ms: u64,
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
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(120),
            enable_fallback: true,
            health_check_interval: Duration::from_secs(60),
        }
    }
}

// ============================================================================
// LLM Router
// ============================================================================

pub struct LLMRouter {
    /// Available providers in priority order
    providers: Vec<(String, LLMConfig)>,
    /// Circuit breakers per provider
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    /// Stats per provider
    stats: Arc<RwLock<HashMap<String, ProviderStats>>>,
    /// Router configuration
    config: RouterConfig,
}

impl LLMRouter {
    pub fn new(config: RouterConfig) -> Self {
        Self {
            providers: Vec::new(),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Add a provider to the router
    pub fn add_provider(&mut self, name: impl Into<String>, config: LLMConfig) {
        let name = name.into();
        self.providers.push((name.clone(), config));

        // Initialize circuit breaker and stats
        if let Ok(mut breakers) = self.circuit_breakers.write() {
            breakers.insert(name.clone(), CircuitBreaker::default());
        }
        if let Ok(mut stats) = self.stats.write() {
            stats.insert(name, ProviderStats::default());
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

    /// Check if a provider is available (circuit not open)
    fn is_provider_available(&self, name: &str) -> bool {
        if let Ok(mut breakers) = self.circuit_breakers.write() {
            if let Some(cb) = breakers.get_mut(name) {
                return cb.can_execute();
            }
        }
        false
    }

    /// Record a successful request
    fn record_success(&self, name: &str, latency_ms: u64) {
        if let Ok(mut breakers) = self.circuit_breakers.write() {
            if let Some(cb) = breakers.get_mut(name) {
                cb.record_success();
            }
        }
        if let Ok(mut stats) = self.stats.write() {
            if let Some(s) = stats.get_mut(name) {
                s.total_requests += 1;
                s.successful_requests += 1;
                s.total_latency_ms += latency_ms;
                s.last_used = Some(Instant::now());
            }
        }
    }

    /// Record a failed request
    fn record_failure(&self, name: &str) {
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
    }

    /// Get the next available provider
    fn get_available_provider(&self) -> Option<(String, LLMConfig)> {
        for (name, config) in &self.providers {
            if self.is_provider_available(name) {
                return Some((name.clone(), config.clone()));
            }
        }
        None
    }

    /// Send a chat request with automatic fallback
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let mut last_error: Option<LLMError> = None;
        let mut tried_providers = Vec::new();

        // Try providers in order
        for (name, config) in &self.providers {
            // Skip if circuit is open
            if !self.is_provider_available(name) {
                log::debug!("Skipping provider {} (circuit open)", name);
                continue;
            }

            tried_providers.push(name.clone());
            let client = LLMClient::new(config.clone());
            let start = Instant::now();

            // Execute with timeout
            let result = timeout(
                self.config.request_timeout,
                client.chat(request.clone())
            ).await;

            match result {
                Ok(Ok(response)) => {
                    let latency = start.elapsed().as_millis() as u64;
                    self.record_success(name, latency);
                    log::info!("Chat succeeded with provider {} ({}ms)", name, latency);
                    return Ok(response);
                }
                Ok(Err(e)) => {
                    self.record_failure(name);
                    log::warn!("Chat failed with provider {}: {}", name, e);
                    last_error = Some(e);

                    if !self.config.enable_fallback {
                        break;
                    }
                }
                Err(_) => {
                    self.record_failure(name);
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
}

impl LLMRouterBuilder {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            config: RouterConfig::default(),
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

    pub fn build(self) -> LLMRouter {
        let mut router = LLMRouter::new(self.config);
        for (name, config) in self.providers {
            router.add_provider(name, config);
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
}
