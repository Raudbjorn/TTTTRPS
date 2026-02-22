//! Property-based tests for the Cost Calculator module
//!
//! Tests invariants:
//! - Cost is non-negative
//! - Cost increases monotonically with tokens
//! - Zero tokens yields zero cost

use proptest::prelude::*;

use crate::core::llm::cost::{CostTracker, ProviderCosts, ProviderPricing, TokenUsage};

// ============================================================================
// Strategies for generating test inputs
// ============================================================================

/// Generate an arbitrary TokenUsage with reasonable bounds
fn arb_token_usage() -> impl Strategy<Value = TokenUsage> {
    (0u32..1_000_000, 0u32..1_000_000)
        .prop_map(|(input, output)| TokenUsage::new(input, output))
}

/// Generate a known provider/model pair for pricing
fn arb_provider_model() -> impl Strategy<Value = (String, String)> {
    prop_oneof![
        Just(("claude".to_string(), "claude-3-5-sonnet".to_string())),
        Just(("claude".to_string(), "claude-3-5-haiku".to_string())),
        Just(("openai".to_string(), "gpt-4o".to_string())),
        Just(("openai".to_string(), "gpt-4o-mini".to_string())),
        Just(("gemini".to_string(), "gemini-1.5-pro".to_string())),
        Just(("groq".to_string(), "llama-3.3-70b".to_string())),
        Just(("ollama".to_string(), "llama3".to_string())),
        Just(("mistral".to_string(), "mistral-large".to_string())),
        Just(("deepseek".to_string(), "deepseek-chat".to_string())),
        Just(("cohere".to_string(), "command-r-plus".to_string())),
    ]
}

/// Generate arbitrary non-negative pricing
fn arb_pricing() -> impl Strategy<Value = ProviderPricing> {
    (
        "[a-z]{3,10}",           // provider_id
        "[a-z0-9-]{3,20}",       // model_id
        0.0f64..100.0,           // input_cost_per_million
        0.0f64..100.0,           // output_cost_per_million
        prop::option::of(1000u32..2_000_000), // context_window
        prop::option::of(100u32..100_000),    // max_output_tokens
    )
        .prop_map(
            |(provider_id, model_id, input_cost, output_cost, context, max_out)| ProviderPricing {
                provider_id,
                model_id,
                input_cost_per_million: input_cost,
                output_cost_per_million: output_cost,
                context_window: context,
                max_output_tokens: max_out,
                is_free: false,
            },
        )
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    /// Property: Cost is always non-negative
    #[test]
    fn prop_cost_is_non_negative(
        usage in arb_token_usage(),
        (provider, model) in arb_provider_model()
    ) {
        if let Some(pricing) = ProviderPricing::for_model(&provider, &model) {
            let cost = pricing.calculate_cost(&usage);
            prop_assert!(
                cost >= 0.0,
                "Cost {} should be non-negative for {} tokens",
                cost,
                usage.total()
            );
        }
    }

    /// Property: Zero tokens yields zero cost
    #[test]
    fn prop_zero_tokens_zero_cost(
        (provider, model) in arb_provider_model()
    ) {
        if let Some(pricing) = ProviderPricing::for_model(&provider, &model) {
            let usage = TokenUsage::new(0, 0);
            let cost = pricing.calculate_cost(&usage);
            prop_assert_eq!(
                cost, 0.0,
                "Zero tokens should yield zero cost, got {} for {}/{}",
                cost, provider, model
            );
        }
    }

    /// Property: Cost increases monotonically with input tokens
    #[test]
    fn prop_cost_increases_with_input_tokens(
        base_input in 0u32..500_000,
        additional_input in 1u32..500_000,
        output in 0u32..1_000_000,
        (provider, model) in arb_provider_model()
    ) {
        if let Some(pricing) = ProviderPricing::for_model(&provider, &model) {
            let usage1 = TokenUsage::new(base_input, output);
            let usage2 = TokenUsage::new(base_input + additional_input, output);

            let cost1 = pricing.calculate_cost(&usage1);
            let cost2 = pricing.calculate_cost(&usage2);

            prop_assert!(
                cost2 >= cost1,
                "Cost should increase with more tokens: {} >= {} for {}/{}",
                cost2, cost1, provider, model
            );
        }
    }

    /// Property: Cost increases monotonically with output tokens
    #[test]
    fn prop_cost_increases_with_output_tokens(
        input in 0u32..1_000_000,
        base_output in 0u32..500_000,
        additional_output in 1u32..500_000,
        (provider, model) in arb_provider_model()
    ) {
        if let Some(pricing) = ProviderPricing::for_model(&provider, &model) {
            let usage1 = TokenUsage::new(input, base_output);
            let usage2 = TokenUsage::new(input, base_output + additional_output);

            let cost1 = pricing.calculate_cost(&usage1);
            let cost2 = pricing.calculate_cost(&usage2);

            prop_assert!(
                cost2 >= cost1,
                "Cost should increase with more output tokens: {} >= {} for {}/{}",
                cost2, cost1, provider, model
            );
        }
    }

    /// Property: Cost is additive (sum of input and output costs)
    #[test]
    fn prop_cost_is_additive(
        input in 0u32..1_000_000,
        output in 0u32..1_000_000,
        (provider, model) in arb_provider_model()
    ) {
        if let Some(pricing) = ProviderPricing::for_model(&provider, &model) {
            let combined = TokenUsage::new(input, output);
            let input_only = TokenUsage::new(input, 0);
            let output_only = TokenUsage::new(0, output);

            let combined_cost = pricing.calculate_cost(&combined);
            let input_cost = pricing.calculate_cost(&input_only);
            let output_cost = pricing.calculate_cost(&output_only);

            // Allow small floating-point tolerance
            let diff = (combined_cost - (input_cost + output_cost)).abs();
            prop_assert!(
                diff < 1e-10,
                "Combined cost {} should equal input {} + output {} for {}/{}",
                combined_cost, input_cost, output_cost, provider, model
            );
        }
    }

    /// Property: Free providers (Ollama) always have zero cost
    #[test]
    fn prop_ollama_is_free(
        usage in arb_token_usage()
    ) {
        if let Some(pricing) = ProviderPricing::for_model("ollama", "any-model") {
            prop_assert!(pricing.is_free, "Ollama should be marked as free");
            let cost = pricing.calculate_cost(&usage);
            prop_assert_eq!(
                cost, 0.0,
                "Ollama cost should be 0, got {} for {} tokens",
                cost, usage.total()
            );
        }
    }

    /// Property: TokenUsage total equals sum of input and output
    #[test]
    fn prop_token_usage_total(
        input in 0u32..u32::MAX / 2,
        output in 0u32..u32::MAX / 2
    ) {
        let usage = TokenUsage::new(input, output);
        prop_assert_eq!(
            usage.total(),
            input + output,
            "Total should be sum of input and output"
        );
    }

    /// Property: TokenUsage add is correct
    #[test]
    fn prop_token_usage_add(
        input1 in 0u32..u32::MAX / 4,
        output1 in 0u32..u32::MAX / 4,
        input2 in 0u32..u32::MAX / 4,
        output2 in 0u32..u32::MAX / 4
    ) {
        let mut usage1 = TokenUsage::new(input1, output1);
        let usage2 = TokenUsage::new(input2, output2);

        usage1.add(&usage2);

        prop_assert_eq!(usage1.input_tokens, input1 + input2);
        prop_assert_eq!(usage1.output_tokens, output1 + output2);
    }

    /// Property: CostTracker estimates match direct pricing calculation
    #[test]
    fn prop_tracker_estimate_matches_pricing(
        input in 0u32..1_000_000,
        output in 0u32..1_000_000,
        (provider, model) in arb_provider_model()
    ) {
        let tracker = CostTracker::new();

        if let Some(pricing) = ProviderPricing::for_model(&provider, &model) {
            let usage = TokenUsage::new(input, output);
            let direct_cost = pricing.calculate_cost(&usage);
            let tracker_estimate = tracker.estimate_cost(&provider, &model, input, output);

            // Allow small floating-point tolerance
            let diff = (direct_cost - tracker_estimate).abs();
            prop_assert!(
                diff < 1e-10,
                "Tracker estimate {} should match direct calculation {} for {}/{}",
                tracker_estimate, direct_cost, provider, model
            );
        }
    }

    /// Property: ProviderCosts record updates correctly
    #[test]
    fn prop_provider_costs_record(
        input in 0u32..1_000_000,
        output in 0u32..1_000_000,
        cost in 0.0f64..1000.0
    ) {
        let mut costs = ProviderCosts::new("test_provider");
        let usage = TokenUsage::new(input, output);

        costs.record(&usage, cost);

        prop_assert_eq!(costs.request_count, 1);
        prop_assert_eq!(costs.input_tokens, input as u64);
        prop_assert_eq!(costs.output_tokens, output as u64);
        prop_assert!((costs.total_cost_usd - cost).abs() < 1e-10);
    }

    /// Property: ProviderCosts average calculations are correct
    #[test]
    fn prop_provider_costs_averages(
        usages in prop::collection::vec(arb_token_usage(), 1..10),
        costs in prop::collection::vec(0.0f64..100.0, 1..10)
    ) {
        // Ensure we have matching lengths
        let len = usages.len().min(costs.len());
        if len == 0 {
            return Ok(());
        }

        let mut provider_costs = ProviderCosts::new("test");

        let mut total_cost = 0.0;
        for (usage, cost) in usages.iter().take(len).zip(costs.iter().take(len)) {
            provider_costs.record(usage, *cost);
            total_cost += cost;
        }

        let expected_avg = total_cost / len as f64;
        let actual_avg = provider_costs.avg_cost_per_request();

        prop_assert!(
            (expected_avg - actual_avg).abs() < 1e-10,
            "Average cost {} should equal {} / {} = {}",
            actual_avg, total_cost, len, expected_avg
        );
    }

    /// Property: Custom pricing overrides default
    #[test]
    fn prop_custom_pricing_overrides(
        custom_pricing in arb_pricing(),
        usage in arb_token_usage()
    ) {
        let mut tracker = CostTracker::new();

        // Set custom pricing
        tracker.set_pricing(custom_pricing.clone());

        // Get pricing back
        let retrieved = tracker.get_pricing(&custom_pricing.provider_id, &custom_pricing.model_id);

        prop_assert!(
            retrieved.is_some(),
            "Custom pricing should be retrievable"
        );

        let retrieved = retrieved.unwrap();
        prop_assert_eq!(
            retrieved.input_cost_per_million,
            custom_pricing.input_cost_per_million
        );
        prop_assert_eq!(
            retrieved.output_cost_per_million,
            custom_pricing.output_cost_per_million
        );

        // Verify the custom pricing is used for cost calculation
        let cost = retrieved.calculate_cost(&usage);
        prop_assert!(cost >= 0.0, "Custom pricing should calculate valid cost");
    }

    /// Property: Budget tracking is accurate
    #[test]
    fn prop_budget_tracking(
        budget in 0.01f64..1000.0,
        usage in arb_token_usage()
    ) {
        let mut tracker = CostTracker::new();
        tracker.set_monthly_budget(Some(budget));

        // Initially within budget
        prop_assert!(tracker.is_within_monthly_budget());

        // Record some usage with known cost
        let cost = tracker.record_usage("claude", "claude-3-5-sonnet", &usage);

        // Check if budget status is correct
        if cost <= budget {
            prop_assert!(tracker.is_within_monthly_budget());
        }

        // Remaining budget should be correct
        if let Some(remaining) = tracker.remaining_monthly_budget() {
            let expected = (budget - tracker.summary().monthly_cost).max(0.0);
            prop_assert!(
                (remaining - expected).abs() < 1e-10,
                "Remaining budget {} should equal {}",
                remaining, expected
            );
        }
    }

    /// Property: Cost estimation is proportional to tokens
    #[test]
    fn prop_cost_proportional_to_tokens(
        base_input in 1u32..100_000,
        multiplier in 2u32..10,
        output in 0u32..100_000,
        (provider, model) in arb_provider_model()
    ) {
        if let Some(pricing) = ProviderPricing::for_model(&provider, &model) {
            // Skip free providers
            if pricing.is_free {
                return Ok(());
            }

            let usage1 = TokenUsage::new(base_input, output);
            let usage2 = TokenUsage::new(base_input * multiplier, output);

            let _cost1 = pricing.calculate_cost(&usage1);
            let _cost2 = pricing.calculate_cost(&usage2);

            // For the input-only portion, cost should scale proportionally
            let input_only1 = TokenUsage::new(base_input, 0);
            let input_only2 = TokenUsage::new(base_input * multiplier, 0);

            let input_cost1 = pricing.calculate_cost(&input_only1);
            let input_cost2 = pricing.calculate_cost(&input_only2);

            // Allow small floating-point tolerance
            let expected_ratio = multiplier as f64;
            if input_cost1 > 0.0 {
                let actual_ratio = input_cost2 / input_cost1;
                prop_assert!(
                    (actual_ratio - expected_ratio).abs() < 1e-6,
                    "Cost ratio {} should be {} for {}/{}",
                    actual_ratio, expected_ratio, provider, model
                );
            }
        }
    }

    /// Property: Cost is bounded for bounded input
    ///
    /// Given a maximum token count, the cost should never exceed a calculable
    /// upper bound based on the most expensive pricing.
    #[test]
    fn prop_cost_bounded_for_bounded_input(
        input in 0u32..1_000_000,
        output in 0u32..1_000_000,
        (provider, model) in arb_provider_model()
    ) {
        if let Some(pricing) = ProviderPricing::for_model(&provider, &model) {
            let usage = TokenUsage::new(input, output);
            let cost = pricing.calculate_cost(&usage);

            // Calculate the theoretical maximum cost based on pricing
            // Using the formula: cost = (input/1M * input_price) + (output/1M * output_price)
            let max_input_cost = (input as f64 / 1_000_000.0) * pricing.input_cost_per_million;
            let max_output_cost = (output as f64 / 1_000_000.0) * pricing.output_cost_per_million;
            let theoretical_max = max_input_cost + max_output_cost;

            // Cost should equal the theoretical max (or be very close due to floating-point)
            prop_assert!(
                (cost - theoretical_max).abs() < 1e-10,
                "Cost {} should equal theoretical max {} for {}/{} with {} input and {} output tokens",
                cost, theoretical_max, provider, model, input, output
            );

            // For non-free providers, establish a reasonable upper bound
            // Assuming max $100/million tokens, max 2M tokens = max $200
            if !pricing.is_free {
                let reasonable_max = 200.0; // $200 for 2M tokens at $100/M
                prop_assert!(
                    cost <= reasonable_max,
                    "Cost {} exceeds reasonable maximum {} for {}/{} with {} total tokens",
                    cost, reasonable_max, provider, model, usage.total()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Basic sanity test that cost calculation works
    #[test]
    fn test_cost_calculation_exists() {
        let pricing = ProviderPricing::for_model("claude", "claude-3-5-sonnet");
        assert!(pricing.is_some());

        let pricing = pricing.unwrap();
        let usage = TokenUsage::new(1000, 500);
        let cost = pricing.calculate_cost(&usage);
        assert!(cost > 0.0);
    }
}
