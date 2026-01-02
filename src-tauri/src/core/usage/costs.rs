//! Cost Calculation Module
//!
//! Provides accurate pricing data for LLM providers and cost calculations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Pricing information per million tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Input token price per 1M tokens (USD)
    pub input_price_per_million: f64,
    /// Output token price per 1M tokens (USD)
    pub output_price_per_million: f64,
    /// Optional context caching discount (percentage, 0.0 - 1.0)
    pub cache_discount: Option<f64>,
}

impl ModelPricing {
    pub const fn new(input: f64, output: f64) -> Self {
        Self {
            input_price_per_million: input,
            output_price_per_million: output,
            cache_discount: None,
        }
    }

    pub const fn with_cache(input: f64, output: f64, cache: f64) -> Self {
        Self {
            input_price_per_million: input,
            output_price_per_million: output,
            cache_discount: Some(cache),
        }
    }

    /// Calculate cost for given token counts
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_price_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_price_per_million;
        input_cost + output_cost
    }

    /// Calculate cost with cached input tokens
    pub fn calculate_cost_with_cache(
        &self,
        input_tokens: u32,
        cached_tokens: u32,
        output_tokens: u32,
    ) -> f64 {
        let discount = self.cache_discount.unwrap_or(0.0);
        let non_cached = input_tokens.saturating_sub(cached_tokens);

        let input_cost = (non_cached as f64 / 1_000_000.0) * self.input_price_per_million;
        let cached_cost = (cached_tokens as f64 / 1_000_000.0)
            * self.input_price_per_million
            * (1.0 - discount);
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_price_per_million;

        input_cost + cached_cost + output_cost
    }
}

// ============================================================================
// Pricing Data (as of late 2024 / early 2025)
// ============================================================================

/// Get pricing for a specific model
pub fn get_model_pricing(provider: &str, model: &str) -> ModelPricing {
    match provider.to_lowercase().as_str() {
        "claude" | "anthropic" => get_claude_pricing(model),
        "openai" | "gpt" => get_openai_pricing(model),
        "gemini" | "google" => get_gemini_pricing(model),
        "openrouter" => get_openrouter_pricing(model),
        "mistral" => get_mistral_pricing(model),
        "groq" => get_groq_pricing(model),
        "together" => get_together_pricing(model),
        "cohere" => get_cohere_pricing(model),
        "deepseek" => get_deepseek_pricing(model),
        "ollama" | "local" => ModelPricing::new(0.0, 0.0), // Local models are free
        _ => ModelPricing::new(1.0, 3.0), // Conservative default
    }
}

fn get_claude_pricing(model: &str) -> ModelPricing {
    let model_lower = model.to_lowercase();
    if model_lower.contains("opus") {
        if model_lower.contains("4") {
            ModelPricing::with_cache(15.0, 75.0, 0.9) // Claude 4 Opus with 90% cache discount
        } else {
            ModelPricing::new(15.0, 75.0) // Claude 3 Opus
        }
    } else if model_lower.contains("sonnet") {
        if model_lower.contains("3-5") || model_lower.contains("3.5") {
            ModelPricing::with_cache(3.0, 15.0, 0.9) // Claude 3.5 Sonnet
        } else {
            ModelPricing::new(3.0, 15.0) // Claude 3 Sonnet
        }
    } else if model_lower.contains("haiku") {
        if model_lower.contains("3-5") || model_lower.contains("3.5") {
            ModelPricing::with_cache(1.0, 5.0, 0.9) // Claude 3.5 Haiku
        } else {
            ModelPricing::new(0.25, 1.25) // Claude 3 Haiku
        }
    } else {
        ModelPricing::new(3.0, 15.0) // Default to Sonnet pricing
    }
}

fn get_openai_pricing(model: &str) -> ModelPricing {
    let model_lower = model.to_lowercase();
    if model_lower.contains("gpt-4o-mini") {
        ModelPricing::new(0.15, 0.60)
    } else if model_lower.contains("gpt-4o") {
        ModelPricing::new(2.5, 10.0)
    } else if model_lower.contains("gpt-4-turbo") {
        ModelPricing::new(10.0, 30.0)
    } else if model_lower.contains("gpt-4") {
        ModelPricing::new(30.0, 60.0)
    } else if model_lower.contains("gpt-3.5") {
        ModelPricing::new(0.5, 1.5)
    } else if model_lower.contains("o1-preview") {
        ModelPricing::new(15.0, 60.0)
    } else if model_lower.contains("o1-mini") {
        ModelPricing::new(3.0, 12.0)
    } else if model_lower.contains("o1") {
        ModelPricing::new(15.0, 60.0)
    } else {
        ModelPricing::new(2.5, 10.0) // Default to GPT-4o pricing
    }
}

fn get_gemini_pricing(model: &str) -> ModelPricing {
    let model_lower = model.to_lowercase();
    if model_lower.contains("flash-2") {
        ModelPricing::new(0.075, 0.30)
    } else if model_lower.contains("flash") {
        ModelPricing::new(0.075, 0.30)
    } else if model_lower.contains("pro-2") {
        ModelPricing::new(1.25, 5.0)
    } else if model_lower.contains("pro") {
        ModelPricing::new(1.25, 5.0)
    } else if model_lower.contains("ultra") {
        ModelPricing::new(3.0, 12.0)
    } else {
        ModelPricing::new(1.25, 5.0) // Default to Pro pricing
    }
}

fn get_openrouter_pricing(model: &str) -> ModelPricing {
    // OpenRouter uses the underlying model's pricing plus a small markup
    // For simplicity, we'll use the base model pricing
    let model_lower = model.to_lowercase();

    // Claude models via OpenRouter
    if model_lower.contains("claude") {
        return get_claude_pricing(model);
    }

    // OpenAI models via OpenRouter
    if model_lower.contains("gpt") || model_lower.contains("openai") {
        return get_openai_pricing(model);
    }

    // Gemini models via OpenRouter
    if model_lower.contains("gemini") || model_lower.contains("google") {
        return get_gemini_pricing(model);
    }

    // Mistral models
    if model_lower.contains("mistral") {
        return get_mistral_pricing(model);
    }

    // Meta Llama models
    if model_lower.contains("llama") {
        if model_lower.contains("405b") {
            return ModelPricing::new(2.7, 2.7);
        } else if model_lower.contains("70b") {
            return ModelPricing::new(0.59, 0.79);
        } else if model_lower.contains("8b") {
            return ModelPricing::new(0.055, 0.055);
        }
    }

    // Default for unknown OpenRouter models
    ModelPricing::new(1.0, 3.0)
}

fn get_mistral_pricing(model: &str) -> ModelPricing {
    let model_lower = model.to_lowercase();
    if model_lower.contains("large") {
        ModelPricing::new(2.0, 6.0)
    } else if model_lower.contains("medium") {
        ModelPricing::new(2.7, 8.1)
    } else if model_lower.contains("small") {
        ModelPricing::new(0.2, 0.6)
    } else if model_lower.contains("tiny") || model_lower.contains("7b") {
        ModelPricing::new(0.14, 0.14)
    } else if model_lower.contains("codestral") {
        ModelPricing::new(0.3, 0.9)
    } else {
        ModelPricing::new(2.0, 6.0) // Default to Large pricing
    }
}

fn get_groq_pricing(model: &str) -> ModelPricing {
    // Groq offers very competitive pricing
    let model_lower = model.to_lowercase();
    if model_lower.contains("llama-3.3-70b") {
        ModelPricing::new(0.59, 0.79)
    } else if model_lower.contains("llama-3.1-70b") {
        ModelPricing::new(0.59, 0.79)
    } else if model_lower.contains("llama-3.1-8b") {
        ModelPricing::new(0.05, 0.08)
    } else if model_lower.contains("mixtral") {
        ModelPricing::new(0.24, 0.24)
    } else if model_lower.contains("gemma") {
        ModelPricing::new(0.07, 0.07)
    } else {
        ModelPricing::new(0.27, 0.27) // Conservative default
    }
}

fn get_together_pricing(model: &str) -> ModelPricing {
    let model_lower = model.to_lowercase();
    if model_lower.contains("llama-3.1-405b") {
        ModelPricing::new(3.5, 3.5)
    } else if model_lower.contains("llama-3.1-70b") {
        ModelPricing::new(0.88, 0.88)
    } else if model_lower.contains("llama-3.1-8b") {
        ModelPricing::new(0.18, 0.18)
    } else if model_lower.contains("mistral") || model_lower.contains("mixtral") {
        ModelPricing::new(0.6, 0.6)
    } else if model_lower.contains("qwen") {
        ModelPricing::new(0.9, 0.9)
    } else {
        ModelPricing::new(0.9, 0.9)
    }
}

fn get_cohere_pricing(model: &str) -> ModelPricing {
    let model_lower = model.to_lowercase();
    if model_lower.contains("command-r-plus") {
        ModelPricing::new(2.5, 10.0)
    } else if model_lower.contains("command-r") {
        ModelPricing::new(0.15, 0.6)
    } else if model_lower.contains("command") {
        ModelPricing::new(0.5, 1.5)
    } else {
        ModelPricing::new(2.5, 10.0) // Default to Command-R+ pricing
    }
}

fn get_deepseek_pricing(model: &str) -> ModelPricing {
    // DeepSeek offers very competitive pricing
    let model_lower = model.to_lowercase();
    if model_lower.contains("coder") {
        ModelPricing::new(0.14, 0.28)
    } else if model_lower.contains("chat") || model_lower.contains("v3") {
        ModelPricing::new(0.14, 0.28)
    } else {
        ModelPricing::new(0.14, 0.28)
    }
}

// ============================================================================
// Cost Breakdown
// ============================================================================

/// Detailed cost breakdown by provider
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostBreakdown {
    pub total_cost_usd: f64,
    pub by_provider: HashMap<String, ProviderCostDetails>,
    pub by_model: HashMap<String, ModelCostDetails>,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderCostDetails {
    pub provider: String,
    pub total_cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u32,
    pub avg_cost_per_request: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelCostDetails {
    pub model: String,
    pub provider: String,
    pub total_cost_usd: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u32,
    pub input_price_per_million: f64,
    pub output_price_per_million: f64,
}

// ============================================================================
// Budget Limits
// ============================================================================

/// Budget limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetLimit {
    /// Maximum spending amount in USD
    pub limit_usd: f64,
    /// Period type
    pub period: BudgetPeriodType,
    /// Warning threshold (0.0 - 1.0)
    pub warning_threshold: f64,
    /// Critical threshold (0.0 - 1.0)
    pub critical_threshold: f64,
    /// Whether to block requests when limit is reached
    pub block_on_limit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BudgetPeriodType {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Total,
}

impl Default for BudgetLimit {
    fn default() -> Self {
        Self {
            limit_usd: 50.0,
            period: BudgetPeriodType::Monthly,
            warning_threshold: 0.8,
            critical_threshold: 0.95,
            block_on_limit: false,
        }
    }
}

/// Budget status check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub period: BudgetPeriodType,
    pub limit_usd: f64,
    pub spent_usd: f64,
    pub remaining_usd: f64,
    pub percentage_used: f64,
    pub status: BudgetAlertLevel,
    pub period_ends_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BudgetAlertLevel {
    Normal,
    Warning,
    Critical,
    Exceeded,
}

impl BudgetLimit {
    /// Check current status against this limit
    pub fn check(&self, spent_usd: f64) -> BudgetStatus {
        let percentage_used = spent_usd / self.limit_usd;
        let status = if percentage_used >= 1.0 {
            BudgetAlertLevel::Exceeded
        } else if percentage_used >= self.critical_threshold {
            BudgetAlertLevel::Critical
        } else if percentage_used >= self.warning_threshold {
            BudgetAlertLevel::Warning
        } else {
            BudgetAlertLevel::Normal
        };

        BudgetStatus {
            period: self.period,
            limit_usd: self.limit_usd,
            spent_usd,
            remaining_usd: (self.limit_usd - spent_usd).max(0.0),
            percentage_used,
            status,
            period_ends_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_calculation() {
        let pricing = ModelPricing::new(3.0, 15.0);
        let cost = pricing.calculate_cost(1000, 500);
        // 1000 input = 0.003, 500 output = 0.0075
        assert!((cost - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_claude_pricing() {
        let pricing = get_model_pricing("claude", "claude-3-5-sonnet-20241022");
        assert_eq!(pricing.input_price_per_million, 3.0);
        assert_eq!(pricing.output_price_per_million, 15.0);
    }

    #[test]
    fn test_budget_check() {
        let limit = BudgetLimit {
            limit_usd: 100.0,
            period: BudgetPeriodType::Monthly,
            warning_threshold: 0.8,
            critical_threshold: 0.95,
            block_on_limit: false,
        };

        let status = limit.check(75.0);
        assert_eq!(status.status, BudgetAlertLevel::Normal);

        let status = limit.check(85.0);
        assert_eq!(status.status, BudgetAlertLevel::Warning);

        let status = limit.check(96.0);
        assert_eq!(status.status, BudgetAlertLevel::Critical);

        let status = limit.check(105.0);
        assert_eq!(status.status, BudgetAlertLevel::Exceeded);
    }
}
