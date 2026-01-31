//! Router Configuration
//!
//! Configuration types for the LLM router.

use serde::{Deserialize, Serialize};
use std::time::Duration;

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
