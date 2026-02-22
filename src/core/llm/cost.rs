//! LLM Provider Cost Tracking and Estimation
//!
//! Provides cost tracking, budget management, and pricing information
//! for all supported LLM providers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Token Usage
// ============================================================================

/// Token usage for a request/response
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input/prompt tokens
    pub input_tokens: u32,
    /// Number of output/completion tokens
    pub output_tokens: u32,
}

impl TokenUsage {
    /// Create a new token usage record
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
        }
    }

    /// Total tokens used
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }

    /// Add another usage to this one
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
    }
}

// ============================================================================
// Provider Pricing
// ============================================================================

/// Pricing information for a provider/model combination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderPricing {
    /// Provider identifier (e.g., "claude", "openai")
    pub provider_id: String,
    /// Model identifier (e.g., "claude-3-5-sonnet", "gpt-4o")
    pub model_id: String,
    /// Cost per 1 million input tokens in USD
    pub input_cost_per_million: f64,
    /// Cost per 1 million output tokens in USD
    pub output_cost_per_million: f64,
    /// Maximum context window size (if known)
    pub context_window: Option<u32>,
    /// Maximum output tokens (if known)
    pub max_output_tokens: Option<u32>,
    /// Whether the model is free (e.g., local Ollama)
    pub is_free: bool,
}

impl ProviderPricing {
    /// Create pricing for a free model (e.g., local Ollama)
    pub fn free(provider_id: &str, model_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            model_id: model_id.to_string(),
            input_cost_per_million: 0.0,
            output_cost_per_million: 0.0,
            context_window: None,
            max_output_tokens: None,
            is_free: true,
        }
    }

    /// Calculate cost for given token usage
    pub fn calculate_cost(&self, usage: &TokenUsage) -> f64 {
        if self.is_free {
            return 0.0;
        }
        let input_cost = (usage.input_tokens as f64 / 1_000_000.0) * self.input_cost_per_million;
        let output_cost = (usage.output_tokens as f64 / 1_000_000.0) * self.output_cost_per_million;
        input_cost + output_cost
    }

    /// Estimate cost for a request (before execution)
    pub fn estimate_cost(&self, estimated_input: u32, estimated_output: u32) -> f64 {
        self.calculate_cost(&TokenUsage::new(estimated_input, estimated_output))
    }

    /// Get known pricing for common models (as of early 2025)
    pub fn for_model(provider: &str, model: &str) -> Option<Self> {
        let (input, output, context, max_out) = match (provider, model) {
            // ============ Claude/Anthropic Models ============
            ("claude", m) | ("anthropic", m) if m.contains("opus-4") => {
                (15.0, 75.0, Some(200_000), Some(32_000))
            }
            ("claude", m) | ("anthropic", m) if m.contains("sonnet-4") => {
                (3.0, 15.0, Some(200_000), Some(64_000))
            }
            ("claude", m) | ("anthropic", m) if m.contains("3-5-sonnet") || m.contains("3.5-sonnet") => {
                (3.0, 15.0, Some(200_000), Some(8_192))
            }
            ("claude", m) | ("anthropic", m) if m.contains("3-5-haiku") || m.contains("3.5-haiku") => {
                (0.80, 4.0, Some(200_000), Some(8_192))
            }
            ("claude", m) | ("anthropic", m) if m.contains("haiku") => {
                (0.25, 1.25, Some(200_000), Some(4_096))
            }
            ("claude", m) | ("anthropic", m) if m.contains("opus") => {
                (15.0, 75.0, Some(200_000), Some(4_096))
            }

            // ============ OpenAI Models ============
            ("openai", m) if m.contains("gpt-4o-mini") => {
                (0.15, 0.60, Some(128_000), Some(16_384))
            }
            ("openai", m) if m.contains("gpt-4o") && !m.contains("mini") => {
                (2.50, 10.0, Some(128_000), Some(16_384))
            }
            ("openai", m) if m.contains("gpt-4-turbo") => {
                (10.0, 30.0, Some(128_000), Some(4_096))
            }
            ("openai", m) if m.contains("gpt-4") && !m.contains("turbo") && !m.contains("o") => {
                (30.0, 60.0, Some(8_192), Some(8_192))
            }
            ("openai", m) if m.contains("gpt-3.5-turbo") => {
                (0.50, 1.50, Some(16_385), Some(4_096))
            }
            ("openai", m) if m.contains("o1-preview") => {
                (15.0, 60.0, Some(128_000), Some(32_768))
            }
            ("openai", m) if m.contains("o1-mini") => {
                (3.0, 12.0, Some(128_000), Some(65_536))
            }
            ("openai", m) if m.contains("o1") && !m.contains("preview") && !m.contains("mini") => {
                (15.0, 60.0, Some(200_000), Some(100_000))
            }
            ("openai", m) if m.contains("o3-mini") => {
                (1.10, 4.40, Some(200_000), Some(100_000))
            }

            // ============ Google Gemini Models ============
            ("gemini", m) | ("google", m) if m.contains("2.0-flash") || m.contains("2.0 flash") => {
                (0.10, 0.40, Some(1_000_000), Some(8_192))
            }
            ("gemini", m) | ("google", m) if m.contains("1.5-pro") || m.contains("1.5 pro") => {
                (1.25, 5.0, Some(2_000_000), Some(8_192))
            }
            ("gemini", m) | ("google", m) if m.contains("1.5-flash") || m.contains("1.5 flash") => {
                (0.075, 0.30, Some(1_000_000), Some(8_192))
            }
            ("gemini", m) | ("google", m) if m.contains("1.0-pro") || m.contains("pro") => {
                (0.50, 1.50, Some(32_760), Some(8_192))
            }

            // ============ Groq Models (Fast Inference) ============
            ("groq", m) if m.contains("llama-3.3-70b") => {
                (0.59, 0.79, Some(128_000), Some(32_768))
            }
            ("groq", m) if m.contains("llama-3.1-70b") => {
                (0.59, 0.79, Some(128_000), Some(8_192))
            }
            ("groq", m) if m.contains("llama-3.1-8b") || m.contains("llama-3-8b") => {
                (0.05, 0.08, Some(128_000), Some(8_192))
            }
            ("groq", m) if m.contains("mixtral-8x7b") => {
                (0.24, 0.24, Some(32_768), Some(32_768))
            }
            ("groq", m) if m.contains("gemma2-9b") || m.contains("gemma-9b") => {
                (0.20, 0.20, Some(8_192), Some(8_192))
            }

            // ============ Mistral Models ============
            ("mistral", m) if m.contains("large") => {
                (2.0, 6.0, Some(128_000), Some(128_000))
            }
            ("mistral", m) if m.contains("medium") => {
                (2.7, 8.1, Some(32_000), Some(32_000))
            }
            ("mistral", m) if m.contains("small") => {
                (0.2, 0.6, Some(32_000), Some(32_000))
            }
            ("mistral", m) if m.contains("codestral") => {
                (0.2, 0.6, Some(32_000), Some(32_000))
            }
            ("mistral", m) if m.contains("nemo") => {
                (0.15, 0.15, Some(128_000), Some(128_000))
            }

            // ============ DeepSeek Models ============
            ("deepseek", m) if m.contains("chat") => {
                (0.14, 0.28, Some(64_000), Some(4_096))
            }
            ("deepseek", m) if m.contains("coder") => {
                (0.14, 0.28, Some(128_000), Some(4_096))
            }
            ("deepseek", m) if m.contains("reasoner") || m.contains("r1") => {
                (0.55, 2.19, Some(64_000), Some(8_192))
            }

            // ============ Cohere Models ============
            ("cohere", m) if m.contains("command-r-plus") || m.contains("command-r+") => {
                (2.5, 10.0, Some(128_000), Some(4_096))
            }
            ("cohere", m) if m.contains("command-r") && !m.contains("plus") && !m.contains("+") => {
                (0.5, 1.5, Some(128_000), Some(4_096))
            }
            ("cohere", m) if m.contains("command-light") => {
                (0.30, 0.60, Some(4_096), Some(4_096))
            }
            ("cohere", m) if m.contains("command") => {
                (1.0, 2.0, Some(4_096), Some(4_096))
            }

            // ============ Together AI Models ============
            ("together", m) if m.contains("405b") || m.contains("405B") => {
                (3.50, 3.50, Some(128_000), Some(4_096))
            }
            ("together", m) if m.contains("70b") || m.contains("70B") => {
                (0.90, 0.90, Some(128_000), Some(4_096))
            }
            ("together", m) if m.contains("8b") || m.contains("8B") => {
                (0.20, 0.20, Some(128_000), Some(4_096))
            }
            ("together", m) if m.contains("mixtral") || m.contains("8x22") => {
                (1.20, 1.20, Some(65_536), Some(4_096))
            }
            ("together", m) if m.contains("qwen") && (m.contains("72b") || m.contains("72B")) => {
                (0.90, 0.90, Some(128_000), Some(4_096))
            }

            // ============ OpenRouter (Various Providers) ============
            // OpenRouter pricing varies by model - these are estimates
            ("openrouter", m) if m.contains("claude") && m.contains("sonnet") => {
                (3.0, 15.0, Some(200_000), Some(8_192))
            }
            ("openrouter", m) if m.contains("gpt-4o") => {
                (2.50, 10.0, Some(128_000), Some(16_384))
            }
            ("openrouter", m) if m.contains("gemini") && m.contains("pro") => {
                (1.25, 5.0, Some(2_000_000), Some(8_192))
            }
            ("openrouter", m) if m.contains("llama") && m.contains("70b") => {
                (0.59, 0.79, Some(128_000), Some(4_096))
            }

            // ============ Ollama (Local - Free) ============
            ("ollama", _) => {
                return Some(Self::free("ollama", model));
            }

            // Unknown model - return None
            _ => return None,
        };

        Some(Self {
            provider_id: provider.to_string(),
            model_id: model.to_string(),
            input_cost_per_million: input,
            output_cost_per_million: output,
            context_window: context,
            max_output_tokens: max_out,
            is_free: false,
        })
    }

    /// Get the cost per token (average of input and output)
    pub fn avg_cost_per_token(&self) -> f64 {
        (self.input_cost_per_million + self.output_cost_per_million) / 2.0 / 1_000_000.0
    }
}

// ============================================================================
// Provider Costs
// ============================================================================

/// Accumulated costs for a single provider
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderCosts {
    /// Provider identifier
    pub provider_id: String,
    /// Total cost accumulated in USD
    pub total_cost_usd: f64,
    /// Total input tokens used
    pub input_tokens: u64,
    /// Total output tokens used
    pub output_tokens: u64,
    /// Number of requests made
    pub request_count: u64,
}

impl ProviderCosts {
    /// Create a new cost record for a provider
    pub fn new(provider_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            ..Default::default()
        }
    }

    /// Total tokens used
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Average cost per request
    pub fn avg_cost_per_request(&self) -> f64 {
        if self.request_count == 0 {
            0.0
        } else {
            self.total_cost_usd / self.request_count as f64
        }
    }

    /// Average tokens per request
    pub fn avg_tokens_per_request(&self) -> u64 {
        if self.request_count == 0 {
            0
        } else {
            self.total_tokens() / self.request_count
        }
    }

    /// Record a request with cost
    pub fn record(&mut self, usage: &TokenUsage, cost: f64) {
        self.input_tokens += usage.input_tokens as u64;
        self.output_tokens += usage.output_tokens as u64;
        self.total_cost_usd += cost;
        self.request_count += 1;
    }
}

// ============================================================================
// Cost Tracker
// ============================================================================

/// Configuration for cost tracking
#[derive(Debug, Clone, Default)]
pub struct CostTrackerConfig {
    /// Optional monthly budget in USD
    pub monthly_budget: Option<f64>,
    /// Optional daily budget in USD
    pub daily_budget: Option<f64>,
    /// Whether to alert when approaching budget (at 80%)
    pub budget_alert_threshold: f64,
}

impl CostTrackerConfig {
    /// Create a new config with monthly budget
    pub fn with_monthly_budget(budget: f64) -> Self {
        Self {
            monthly_budget: Some(budget),
            budget_alert_threshold: 0.8,
            ..Default::default()
        }
    }
}

/// Centralized cost tracking for all LLM providers
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostTracker {
    /// Custom pricing overrides per provider/model
    #[serde(skip)]
    pub pricing: HashMap<String, ProviderPricing>,
    /// Accumulated costs per provider
    pub costs: HashMap<String, ProviderCosts>,
    /// Total cost across all providers
    pub total_cost_usd: f64,
    /// Optional monthly budget in USD
    pub monthly_budget: Option<f64>,
    /// Cost this month
    pub monthly_cost: f64,
    /// Optional daily budget
    pub daily_budget: Option<f64>,
    /// Cost today
    pub daily_cost: f64,
    /// Current month being tracked (YYYY-MM)
    pub current_month: String,
    /// Current day being tracked (YYYY-MM-DD)
    pub current_day: String,
}

impl CostTracker {
    /// Create a new cost tracker
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            current_month: now.format("%Y-%m").to_string(),
            current_day: now.format("%Y-%m-%d").to_string(),
            ..Default::default()
        }
    }

    /// Create with configuration
    pub fn with_config(config: CostTrackerConfig) -> Self {
        let mut tracker = Self::new();
        tracker.monthly_budget = config.monthly_budget;
        tracker.daily_budget = config.daily_budget;
        tracker
    }

    /// Set custom pricing for a provider/model
    pub fn set_pricing(&mut self, pricing: ProviderPricing) {
        let key = format!("{}:{}", pricing.provider_id, pricing.model_id);
        self.pricing.insert(key, pricing);
    }

    /// Get pricing for a provider/model (custom or default)
    pub fn get_pricing(&self, provider: &str, model: &str) -> Option<ProviderPricing> {
        let key = format!("{}:{}", provider, model);
        self.pricing
            .get(&key)
            .cloned()
            .or_else(|| ProviderPricing::for_model(provider, model))
    }

    /// Record usage and calculate cost
    pub fn record_usage(&mut self, provider: &str, model: &str, usage: &TokenUsage) -> f64 {
        self.maybe_reset_periods();

        let pricing = self.get_pricing(provider, model);
        let cost = pricing.as_ref().map(|p| p.calculate_cost(usage)).unwrap_or(0.0);

        // Update provider costs
        let provider_costs = self
            .costs
            .entry(provider.to_string())
            .or_insert_with(|| ProviderCosts::new(provider));
        provider_costs.record(usage, cost);

        // Update totals
        self.total_cost_usd += cost;
        self.monthly_cost += cost;
        self.daily_cost += cost;

        cost
    }

    /// Maybe reset monthly/daily counters if period changed
    fn maybe_reset_periods(&mut self) {
        let now = chrono::Utc::now();
        let current_month = now.format("%Y-%m").to_string();
        let current_day = now.format("%Y-%m-%d").to_string();

        if current_month != self.current_month {
            self.monthly_cost = 0.0;
            self.current_month = current_month;
        }

        if current_day != self.current_day {
            self.daily_cost = 0.0;
            self.current_day = current_day;
        }
    }

    /// Check if within monthly budget
    pub fn is_within_monthly_budget(&self) -> bool {
        match self.monthly_budget {
            Some(budget) => self.monthly_cost <= budget,
            None => true,
        }
    }

    /// Check if within daily budget
    pub fn is_within_daily_budget(&self) -> bool {
        match self.daily_budget {
            Some(budget) => self.daily_cost <= budget,
            None => true,
        }
    }

    /// Check if within all budgets
    pub fn is_within_budget(&self) -> bool {
        self.is_within_monthly_budget() && self.is_within_daily_budget()
    }

    /// Get remaining monthly budget
    pub fn remaining_monthly_budget(&self) -> Option<f64> {
        self.monthly_budget.map(|b| (b - self.monthly_cost).max(0.0))
    }

    /// Get remaining daily budget
    pub fn remaining_daily_budget(&self) -> Option<f64> {
        self.daily_budget.map(|b| (b - self.daily_cost).max(0.0))
    }

    /// Estimate cost for a request (before execution)
    pub fn estimate_cost(
        &self,
        provider: &str,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> f64 {
        let usage = TokenUsage::new(input_tokens, output_tokens);
        let pricing = self.get_pricing(provider, model);
        pricing.map(|p| p.calculate_cost(&usage)).unwrap_or(0.0)
    }

    /// Get costs grouped by provider
    pub fn costs_by_provider(&self) -> &HashMap<String, ProviderCosts> {
        &self.costs
    }

    /// Get cost summary
    pub fn summary(&self) -> CostSummary {
        CostSummary {
            total_cost_usd: self.total_cost_usd,
            monthly_cost: self.monthly_cost,
            daily_cost: self.daily_cost,
            monthly_budget: self.monthly_budget,
            daily_budget: self.daily_budget,
            remaining_monthly_budget: self.remaining_monthly_budget(),
            remaining_daily_budget: self.remaining_daily_budget(),
            is_within_budget: self.is_within_budget(),
            costs_by_provider: self.costs.clone(),
            current_month: self.current_month.clone(),
            current_day: self.current_day.clone(),
        }
    }

    /// Reset all cost tracking
    pub fn reset(&mut self) {
        self.costs.clear();
        self.total_cost_usd = 0.0;
        self.monthly_cost = 0.0;
        self.daily_cost = 0.0;
    }

    /// Set monthly budget
    pub fn set_monthly_budget(&mut self, budget: Option<f64>) {
        self.monthly_budget = budget;
    }

    /// Set daily budget
    pub fn set_daily_budget(&mut self, budget: Option<f64>) {
        self.daily_budget = budget;
    }

    /// Get the cheapest provider for a request
    pub fn cheapest_provider(&self, providers: &[&str], input_tokens: u32, output_tokens: u32) -> Option<String> {
        providers
            .iter()
            .filter_map(|&p| {
                // Try to find pricing - if none exists, use a high default
                let cost = self.estimate_cost(p, "", input_tokens, output_tokens);
                Some((p.to_string(), cost))
            })
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(p, _)| p)
    }
}

/// Cost summary for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostSummary {
    /// Total cost all time
    pub total_cost_usd: f64,
    /// Cost this month
    pub monthly_cost: f64,
    /// Cost today
    pub daily_cost: f64,
    /// Monthly budget if set
    pub monthly_budget: Option<f64>,
    /// Daily budget if set
    pub daily_budget: Option<f64>,
    /// Remaining monthly budget
    pub remaining_monthly_budget: Option<f64>,
    /// Remaining daily budget
    pub remaining_daily_budget: Option<f64>,
    /// Whether within all budgets
    pub is_within_budget: bool,
    /// Costs by provider
    pub costs_by_provider: HashMap<String, ProviderCosts>,
    /// Current tracking month
    pub current_month: String,
    /// Current tracking day
    pub current_day: String,
}

// ============================================================================
// Thread-Safe Cost Tracker
// ============================================================================

/// Thread-safe wrapper for CostTracker
#[derive(Debug, Clone)]
pub struct SharedCostTracker {
    inner: Arc<RwLock<CostTracker>>,
}

impl Default for SharedCostTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SharedCostTracker {
    /// Create a new shared cost tracker
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(CostTracker::new())),
        }
    }

    /// Create with configuration
    pub fn with_config(config: CostTrackerConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CostTracker::with_config(config))),
        }
    }

    /// Record usage
    pub async fn record_usage(&self, provider: &str, model: &str, usage: &TokenUsage) -> f64 {
        self.inner.write().await.record_usage(provider, model, usage)
    }

    /// Estimate cost
    pub async fn estimate_cost(
        &self,
        provider: &str,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> f64 {
        self.inner
            .read()
            .await
            .estimate_cost(provider, model, input_tokens, output_tokens)
    }

    /// Check if within budget
    pub async fn is_within_budget(&self) -> bool {
        self.inner.read().await.is_within_budget()
    }

    /// Get cost summary
    pub async fn summary(&self) -> CostSummary {
        self.inner.read().await.summary()
    }

    /// Set monthly budget
    pub async fn set_monthly_budget(&self, budget: Option<f64>) {
        self.inner.write().await.set_monthly_budget(budget);
    }

    /// Set daily budget
    pub async fn set_daily_budget(&self, budget: Option<f64>) {
        self.inner.write().await.set_daily_budget(budget);
    }

    /// Set custom pricing
    pub async fn set_pricing(&self, pricing: ProviderPricing) {
        self.inner.write().await.set_pricing(pricing);
    }

    /// Get pricing for a provider/model
    pub async fn get_pricing(&self, provider: &str, model: &str) -> Option<ProviderPricing> {
        self.inner.read().await.get_pricing(provider, model)
    }

    /// Reset all tracking
    pub async fn reset(&self) {
        self.inner.write().await.reset();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage() {
        let mut usage = TokenUsage::new(100, 50);
        assert_eq!(usage.total(), 150);

        let other = TokenUsage::new(50, 25);
        usage.add(&other);
        assert_eq!(usage.input_tokens, 150);
        assert_eq!(usage.output_tokens, 75);
    }

    #[test]
    fn test_provider_pricing_claude() {
        let pricing = ProviderPricing::for_model("claude", "claude-3-5-sonnet").unwrap();
        assert_eq!(pricing.input_cost_per_million, 3.0);
        assert_eq!(pricing.output_cost_per_million, 15.0);
        assert!(!pricing.is_free);

        let usage = TokenUsage::new(1000, 500);
        let cost = pricing.calculate_cost(&usage);
        // (1000/1M * 3.0) + (500/1M * 15.0) = 0.003 + 0.0075 = 0.0105
        assert!((cost - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_provider_pricing_openai() {
        let pricing = ProviderPricing::for_model("openai", "gpt-4o").unwrap();
        assert_eq!(pricing.input_cost_per_million, 2.5);
        assert_eq!(pricing.output_cost_per_million, 10.0);

        let usage = TokenUsage::new(1000, 500);
        let cost = pricing.calculate_cost(&usage);
        // (1000/1M * 2.5) + (500/1M * 10.0) = 0.0025 + 0.005 = 0.0075
        assert!((cost - 0.0075).abs() < 0.0001);
    }

    #[test]
    fn test_provider_pricing_ollama_free() {
        let pricing = ProviderPricing::for_model("ollama", "llama3").unwrap();
        assert!(pricing.is_free);
        assert_eq!(pricing.input_cost_per_million, 0.0);

        let usage = TokenUsage::new(10000, 5000);
        let cost = pricing.calculate_cost(&usage);
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_cost_tracker_record() {
        let mut tracker = CostTracker::new();
        let usage = TokenUsage::new(1000, 500);

        let cost = tracker.record_usage("claude", "claude-3-5-sonnet", &usage);
        assert!(cost > 0.0);
        assert_eq!(tracker.costs.get("claude").unwrap().request_count, 1);
    }

    #[test]
    fn test_cost_tracker_budget() {
        let mut tracker = CostTracker::new();
        tracker.monthly_budget = Some(0.001); // Very small budget

        let usage = TokenUsage::new(100000, 50000);
        tracker.record_usage("openai", "gpt-4o", &usage);

        assert!(!tracker.is_within_monthly_budget());
        assert_eq!(tracker.remaining_monthly_budget(), Some(0.0));
    }

    #[test]
    fn test_cost_tracker_estimate() {
        let tracker = CostTracker::new();
        let estimate = tracker.estimate_cost("claude", "claude-3-5-sonnet", 1000, 500);
        assert!((estimate - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_provider_costs() {
        let mut costs = ProviderCosts::new("test");
        let usage = TokenUsage::new(100, 50);

        costs.record(&usage, 0.01);
        assert_eq!(costs.request_count, 1);
        assert_eq!(costs.total_tokens(), 150);
        assert_eq!(costs.avg_cost_per_request(), 0.01);

        costs.record(&usage, 0.02);
        assert_eq!(costs.request_count, 2);
        assert_eq!(costs.total_tokens(), 300);
        assert_eq!(costs.avg_cost_per_request(), 0.015);
    }

    #[test]
    fn test_unknown_model_returns_none() {
        let pricing = ProviderPricing::for_model("unknown_provider", "unknown_model");
        assert!(pricing.is_none());
    }

    #[test]
    fn test_groq_pricing() {
        let pricing = ProviderPricing::for_model("groq", "llama-3.3-70b-versatile").unwrap();
        assert_eq!(pricing.input_cost_per_million, 0.59);
        assert_eq!(pricing.output_cost_per_million, 0.79);
    }

    #[test]
    fn test_deepseek_pricing() {
        let pricing = ProviderPricing::for_model("deepseek", "deepseek-chat").unwrap();
        assert_eq!(pricing.input_cost_per_million, 0.14);
        assert_eq!(pricing.output_cost_per_million, 0.28);
    }
}
