//! Provider Statistics
//!
//! Tracks usage statistics for each LLM provider.

use crate::core::llm::cost::TokenUsage;
use serde::{Deserialize, Serialize};
use std::time::Instant;

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
