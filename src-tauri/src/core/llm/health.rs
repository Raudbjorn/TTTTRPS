//! LLM Provider Health Tracking
//!
//! Provides health monitoring, circuit breaker pattern, and uptime tracking
//! for LLM providers. Enables intelligent routing based on provider availability.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// ============================================================================
// Circuit Breaker Pattern
// ============================================================================

/// State of a circuit breaker for a provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum CircuitState {
    /// Normal operation - requests allowed
    #[default]
    Closed,
    /// Provider failing - requests blocked
    Open,
    /// Testing recovery - limited requests allowed
    HalfOpen,
}


/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening circuit
    pub failure_threshold: u32,
    /// Number of consecutive successes in half-open before closing
    pub success_threshold: u32,
    /// Duration to wait before transitioning from open to half-open
    pub timeout_duration: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 2,
            timeout_duration: Duration::from_secs(30),
        }
    }
}

/// Circuit breaker for managing provider availability
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
    config: CircuitBreakerConfig,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a circuit breaker with custom configuration
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure: None,
            config,
        }
    }

    /// Check if the circuit allows execution
    pub fn can_execute(&mut self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has passed - transition to half-open
                if let Some(last) = self.last_failure {
                    if last.elapsed() >= self.config.timeout_duration {
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

    /// Record a successful execution
    pub fn record_success(&mut self) {
        self.failure_count = 0;
        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.success_threshold {
                    self.state = CircuitState::Closed;
                    self.success_count = 0;
                }
            }
            _ => {
                self.state = CircuitState::Closed;
            }
        }
    }

    /// Record a failed execution
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure = Some(Instant::now());
        self.success_count = 0;

        if self.failure_count >= self.config.failure_threshold {
            self.state = CircuitState::Open;
        }
    }

    /// Get the current state of the circuit
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Get the failure count
    pub fn failure_count(&self) -> u32 {
        self.failure_count
    }

    /// Get the time since last failure
    pub fn time_since_failure(&self) -> Option<Duration> {
        self.last_failure.map(|t| t.elapsed())
    }

    /// Manually reset the circuit breaker
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failure_count = 0;
        self.success_count = 0;
        self.last_failure = None;
    }

    /// Force the circuit open (for maintenance, etc.)
    pub fn force_open(&mut self) {
        self.state = CircuitState::Open;
        self.last_failure = Some(Instant::now());
    }
}

// ============================================================================
// Provider Health Status
// ============================================================================

/// Detailed health status for a single provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    /// Provider identifier
    pub provider_id: String,
    /// Whether the provider is currently healthy
    pub is_healthy: bool,
    /// Timestamp of last health check (Unix timestamp)
    pub last_check_timestamp: i64,
    /// Timestamp of last successful request
    pub last_success_timestamp: Option<i64>,
    /// Timestamp of last failed request
    pub last_failure_timestamp: Option<i64>,
    /// Last failure reason if applicable
    pub last_failure_reason: Option<String>,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    /// Uptime percentage (0-100)
    pub uptime_percentage: f64,
    /// Total health checks performed
    pub total_checks: u64,
    /// Successful health checks
    pub successful_checks: u64,
    /// Average response latency in milliseconds
    pub avg_latency_ms: u64,
    /// Circuit breaker state
    #[serde(skip)]
    pub circuit_state: CircuitState,
}

impl Default for ProviderHealth {
    fn default() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            provider_id: String::new(),
            is_healthy: true,
            last_check_timestamp: now,
            last_success_timestamp: Some(now),
            last_failure_timestamp: None,
            last_failure_reason: None,
            consecutive_failures: 0,
            uptime_percentage: 100.0,
            total_checks: 0,
            successful_checks: 0,
            avg_latency_ms: 0,
            circuit_state: CircuitState::Closed,
        }
    }
}

impl ProviderHealth {
    /// Create a new health record for a provider
    pub fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            ..Default::default()
        }
    }

    /// Record a successful health check or request
    pub fn record_success(&mut self, latency_ms: Option<u64>) {
        let now = chrono::Utc::now().timestamp();
        self.is_healthy = true;
        self.last_check_timestamp = now;
        self.last_success_timestamp = Some(now);
        self.consecutive_failures = 0;
        self.total_checks += 1;
        self.successful_checks += 1;

        // Update average latency
        if let Some(latency) = latency_ms {
            if self.avg_latency_ms == 0 {
                self.avg_latency_ms = latency;
            } else {
                // Exponential moving average
                self.avg_latency_ms = (self.avg_latency_ms * 9 + latency) / 10;
            }
        }

        self.update_uptime();
    }

    /// Record a failed health check or request
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

    /// Update uptime percentage based on check history
    fn update_uptime(&mut self) {
        if self.total_checks > 0 {
            self.uptime_percentage =
                (self.successful_checks as f64 / self.total_checks as f64) * 100.0;
        }
    }

    /// Check if the provider should be considered for requests
    pub fn is_available(&self) -> bool {
        self.is_healthy && self.circuit_state != CircuitState::Open
    }

    /// Get time since last successful request
    pub fn time_since_success(&self) -> Option<Duration> {
        self.last_success_timestamp.map(|ts| {
            let now = chrono::Utc::now().timestamp();
            Duration::from_secs((now - ts).max(0) as u64)
        })
    }

    /// Get time since last failure
    pub fn time_since_failure(&self) -> Option<Duration> {
        self.last_failure_timestamp.map(|ts| {
            let now = chrono::Utc::now().timestamp();
            Duration::from_secs((now - ts).max(0) as u64)
        })
    }
}

// ============================================================================
// Health Tracker
// ============================================================================

/// Configuration for the health tracker
#[derive(Debug, Clone)]
pub struct HealthTrackerConfig {
    /// Interval between automatic health checks in seconds
    pub check_interval_secs: u64,
    /// Number of failures before marking provider unhealthy
    pub failure_threshold: u32,
    /// Circuit breaker configuration
    pub circuit_breaker_config: CircuitBreakerConfig,
}

impl Default for HealthTrackerConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 60,
            failure_threshold: 3,
            circuit_breaker_config: CircuitBreakerConfig::default(),
        }
    }
}

/// Centralized health tracking for all LLM providers
#[derive(Debug)]
pub struct HealthTracker {
    /// Health status per provider
    providers: HashMap<String, ProviderHealth>,
    /// Circuit breakers per provider
    circuit_breakers: HashMap<String, CircuitBreaker>,
    /// Configuration
    config: HealthTrackerConfig,
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self::new(HealthTrackerConfig::default())
    }
}

impl HealthTracker {
    /// Create a new health tracker with configuration
    pub fn new(config: HealthTrackerConfig) -> Self {
        Self {
            providers: HashMap::new(),
            circuit_breakers: HashMap::new(),
            config,
        }
    }

    /// Create with a specific check interval
    pub fn with_check_interval(check_interval_secs: u64) -> Self {
        Self::new(HealthTrackerConfig {
            check_interval_secs,
            ..Default::default()
        })
    }

    /// Register a new provider for tracking
    pub fn add_provider(&mut self, provider_id: &str) {
        if !self.providers.contains_key(provider_id) {
            self.providers
                .insert(provider_id.to_string(), ProviderHealth::new(provider_id));
            self.circuit_breakers.insert(
                provider_id.to_string(),
                CircuitBreaker::with_config(self.config.circuit_breaker_config.clone()),
            );
        }
    }

    /// Remove a provider from tracking
    pub fn remove_provider(&mut self, provider_id: &str) {
        self.providers.remove(provider_id);
        self.circuit_breakers.remove(provider_id);
    }

    /// Record a successful request for a provider
    pub fn record_success(&mut self, provider_id: &str, latency_ms: Option<u64>) {
        if let Some(health) = self.providers.get_mut(provider_id) {
            health.record_success(latency_ms);
        }
        if let Some(cb) = self.circuit_breakers.get_mut(provider_id) {
            cb.record_success();
            // Update health with circuit state
            if let Some(health) = self.providers.get_mut(provider_id) {
                health.circuit_state = cb.state();
            }
        }
    }

    /// Record a failed request for a provider
    pub fn record_failure(&mut self, provider_id: &str, reason: &str) {
        if let Some(health) = self.providers.get_mut(provider_id) {
            health.record_failure(reason);
        }
        if let Some(cb) = self.circuit_breakers.get_mut(provider_id) {
            cb.record_failure();
            // Update health with circuit state
            if let Some(health) = self.providers.get_mut(provider_id) {
                health.circuit_state = cb.state();
            }
        }
    }

    /// Check if a provider is healthy and available
    pub fn is_healthy(&self, provider_id: &str) -> bool {
        self.providers
            .get(provider_id)
            .map(|h| h.is_healthy)
            .unwrap_or(false)
    }

    /// Check if a provider's circuit allows execution
    pub fn can_execute(&mut self, provider_id: &str) -> bool {
        self.circuit_breakers
            .get_mut(provider_id)
            .map(|cb| cb.can_execute())
            .unwrap_or(false)
    }

    /// Check if a provider is available (healthy + circuit allows)
    pub fn is_available(&mut self, provider_id: &str) -> bool {
        let healthy = self.is_healthy(provider_id);
        let circuit_ok = self.can_execute(provider_id);
        healthy || circuit_ok // Allow execution if circuit is in half-open state
    }

    /// Get health status for a provider
    pub fn get_health(&self, provider_id: &str) -> Option<&ProviderHealth> {
        self.providers.get(provider_id)
    }

    /// Get mutable health status for a provider
    pub fn get_health_mut(&mut self, provider_id: &str) -> Option<&mut ProviderHealth> {
        self.providers.get_mut(provider_id)
    }

    /// Get all healthy provider IDs
    pub fn healthy_providers(&self) -> Vec<&str> {
        self.providers
            .iter()
            .filter(|(_, h)| h.is_healthy)
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Get all available provider IDs (considering circuit breakers)
    pub fn available_providers(&mut self) -> Vec<String> {
        let mut available = Vec::new();
        for provider_id in self.providers.keys().cloned().collect::<Vec<_>>() {
            if self.is_available(&provider_id) {
                available.push(provider_id);
            }
        }
        available
    }

    /// Get all health statuses
    pub fn all_health(&self) -> &HashMap<String, ProviderHealth> {
        &self.providers
    }

    /// Get circuit breaker state for a provider
    pub fn get_circuit_state(&self, provider_id: &str) -> Option<CircuitState> {
        self.circuit_breakers.get(provider_id).map(|cb| cb.state())
    }

    /// Reset a provider's circuit breaker
    pub fn reset_circuit(&mut self, provider_id: &str) {
        if let Some(cb) = self.circuit_breakers.get_mut(provider_id) {
            cb.reset();
            if let Some(health) = self.providers.get_mut(provider_id) {
                health.circuit_state = CircuitState::Closed;
                health.is_healthy = true;
                health.consecutive_failures = 0;
            }
        }
    }

    /// Force a provider's circuit open
    pub fn force_circuit_open(&mut self, provider_id: &str) {
        if let Some(cb) = self.circuit_breakers.get_mut(provider_id) {
            cb.force_open();
            if let Some(health) = self.providers.get_mut(provider_id) {
                health.circuit_state = CircuitState::Open;
            }
        }
    }

    /// Get the check interval
    pub fn check_interval(&self) -> Duration {
        Duration::from_secs(self.config.check_interval_secs)
    }

    /// Get summary statistics
    pub fn summary(&self) -> HealthSummary {
        let total = self.providers.len();
        let healthy = self.providers.values().filter(|h| h.is_healthy).count();
        let avg_uptime = if total > 0 {
            self.providers.values().map(|h| h.uptime_percentage).sum::<f64>() / total as f64
        } else {
            100.0
        };

        HealthSummary {
            total_providers: total,
            healthy_providers: healthy,
            unhealthy_providers: total - healthy,
            average_uptime_percentage: avg_uptime,
        }
    }
}

/// Summary of health tracker state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub total_providers: usize,
    pub healthy_providers: usize,
    pub unhealthy_providers: usize,
    pub average_uptime_percentage: f64,
}

// ============================================================================
// Thread-Safe Health Tracker
// ============================================================================

/// Thread-safe wrapper for HealthTracker
#[derive(Debug, Clone)]
pub struct SharedHealthTracker {
    inner: Arc<RwLock<HealthTracker>>,
}

impl Default for SharedHealthTracker {
    fn default() -> Self {
        Self::new(HealthTrackerConfig::default())
    }
}

impl SharedHealthTracker {
    /// Create a new shared health tracker
    pub fn new(config: HealthTrackerConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HealthTracker::new(config))),
        }
    }

    /// Add a provider to track
    pub async fn add_provider(&self, provider_id: &str) {
        self.inner.write().await.add_provider(provider_id);
    }

    /// Remove a provider from tracking
    pub async fn remove_provider(&self, provider_id: &str) {
        self.inner.write().await.remove_provider(provider_id);
    }

    /// Record a successful request
    pub async fn record_success(&self, provider_id: &str, latency_ms: Option<u64>) {
        self.inner
            .write()
            .await
            .record_success(provider_id, latency_ms);
    }

    /// Record a failed request
    pub async fn record_failure(&self, provider_id: &str, reason: &str) {
        self.inner
            .write()
            .await
            .record_failure(provider_id, reason);
    }

    /// Check if a provider is available
    pub async fn is_available(&self, provider_id: &str) -> bool {
        self.inner.write().await.is_available(provider_id)
    }

    /// Get health for a provider
    pub async fn get_health(&self, provider_id: &str) -> Option<ProviderHealth> {
        self.inner.read().await.get_health(provider_id).cloned()
    }

    /// Get all health statuses
    pub async fn all_health(&self) -> HashMap<String, ProviderHealth> {
        self.inner.read().await.all_health().clone()
    }

    /// Get healthy provider IDs
    pub async fn healthy_providers(&self) -> Vec<String> {
        self.inner
            .read()
            .await
            .healthy_providers()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get available provider IDs
    pub async fn available_providers(&self) -> Vec<String> {
        self.inner.write().await.available_providers()
    }

    /// Get circuit state for a provider
    pub async fn get_circuit_state(&self, provider_id: &str) -> Option<CircuitState> {
        self.inner.read().await.get_circuit_state(provider_id)
    }

    /// Reset a provider's circuit
    pub async fn reset_circuit(&self, provider_id: &str) {
        self.inner.write().await.reset_circuit(provider_id);
    }

    /// Get health summary
    pub async fn summary(&self) -> HealthSummary {
        self.inner.read().await.summary()
    }
}

// ============================================================================
// Tests
// ============================================================================

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
        assert_eq!(cb.failure_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_manual_reset() {
        let mut cb = CircuitBreaker::default();

        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);
        assert!(cb.can_execute());
    }

    #[test]
    fn test_provider_health_success() {
        let mut health = ProviderHealth::new("test-provider");

        health.record_success(Some(100));
        assert!(health.is_healthy);
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.last_success_timestamp.is_some());
    }

    #[test]
    fn test_provider_health_failure() {
        let mut health = ProviderHealth::new("test-provider");

        health.record_failure("API error");
        health.record_failure("API error");
        assert!(health.is_healthy); // Still healthy after 2 failures

        health.record_failure("API error");
        assert!(!health.is_healthy); // Unhealthy after 3 failures
        assert_eq!(health.consecutive_failures, 3);
    }

    #[test]
    fn test_health_tracker() {
        let mut tracker = HealthTracker::default();

        tracker.add_provider("openai");
        tracker.add_provider("claude");

        assert!(tracker.is_healthy("openai"));
        assert!(tracker.is_healthy("claude"));

        // Record failures
        tracker.record_failure("openai", "API error");
        tracker.record_failure("openai", "API error");
        tracker.record_failure("openai", "API error");

        assert!(!tracker.is_healthy("openai"));
        assert!(tracker.is_healthy("claude"));

        // Test healthy providers
        let healthy = tracker.healthy_providers();
        assert_eq!(healthy.len(), 1);
        assert!(healthy.contains(&"claude"));
    }

    #[test]
    fn test_health_tracker_reset() {
        let mut tracker = HealthTracker::default();

        tracker.add_provider("openai");
        tracker.record_failure("openai", "error");
        tracker.record_failure("openai", "error");
        tracker.record_failure("openai", "error");

        assert!(!tracker.is_healthy("openai"));

        tracker.reset_circuit("openai");
        assert!(tracker.is_healthy("openai"));
    }

    #[test]
    fn test_health_summary() {
        let mut tracker = HealthTracker::default();

        tracker.add_provider("openai");
        tracker.add_provider("claude");
        tracker.add_provider("gemini");

        tracker.record_failure("openai", "error");
        tracker.record_failure("openai", "error");
        tracker.record_failure("openai", "error");

        let summary = tracker.summary();
        assert_eq!(summary.total_providers, 3);
        assert_eq!(summary.healthy_providers, 2);
        assert_eq!(summary.unhealthy_providers, 1);
    }
}
