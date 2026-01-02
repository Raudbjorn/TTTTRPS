//! Usage Tracking Module
//!
//! Tracks token usage and costs per LLM request with historical data storage.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

use super::costs::{get_model_pricing, BudgetLimit, BudgetStatus, CostBreakdown, ModelCostDetails, ProviderCostDetails, BudgetPeriodType};

// ============================================================================
// Types
// ============================================================================

/// Individual usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub provider: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cached_tokens: Option<u32>,
    pub cost_usd: f64,
    /// Optional session/request context
    pub context: Option<String>,
}

impl UsageRecord {
    /// Create a new usage record with automatic cost calculation
    pub fn new(
        provider: String,
        model: String,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Self {
        let pricing = get_model_pricing(&provider, &model);
        let cost_usd = pricing.calculate_cost(input_tokens, output_tokens);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            provider,
            model,
            input_tokens,
            output_tokens,
            cached_tokens: None,
            cost_usd,
            context: None,
        }
    }

    /// Create with cached tokens
    pub fn with_cache(
        provider: String,
        model: String,
        input_tokens: u32,
        output_tokens: u32,
        cached_tokens: u32,
    ) -> Self {
        let pricing = get_model_pricing(&provider, &model);
        let cost_usd = pricing.calculate_cost_with_cache(input_tokens, cached_tokens, output_tokens);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            provider,
            model,
            input_tokens,
            output_tokens,
            cached_tokens: Some(cached_tokens),
            cost_usd,
            context: None,
        }
    }
}

/// Aggregated usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cached_tokens: u64,
    pub total_requests: u32,
    pub total_cost_usd: f64,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub by_provider: HashMap<String, ProviderUsage>,
}

/// Per-provider usage breakdown
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderUsage {
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u32,
    pub cost_usd: f64,
}

/// Session-specific usage (for current session tracking)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionUsage {
    pub session_id: Option<String>,
    pub session_start: DateTime<Utc>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u32,
    pub cost_usd: f64,
}

// ============================================================================
// Usage Tracker
// ============================================================================

/// Tracks usage across providers and time periods
pub struct UsageTracker {
    /// In-memory usage records (recent)
    records: RwLock<Vec<UsageRecord>>,
    /// Maximum records to keep in memory
    max_records: usize,
    /// Session-specific tracking
    session_usage: RwLock<SessionUsage>,
    /// Budget limits
    budget_limits: RwLock<HashMap<BudgetPeriodType, BudgetLimit>>,
}

impl UsageTracker {
    pub fn new() -> Self {
        let mut budget_limits = HashMap::new();
        budget_limits.insert(BudgetPeriodType::Monthly, BudgetLimit::default());

        Self {
            records: RwLock::new(Vec::with_capacity(10000)),
            max_records: 100000,
            session_usage: RwLock::new(SessionUsage {
                session_start: Utc::now(),
                ..Default::default()
            }),
            budget_limits: RwLock::new(budget_limits),
        }
    }

    /// Record a new usage entry
    pub fn record(&self, record: UsageRecord) {
        // Update session tracking
        {
            let mut session = self.session_usage.write().unwrap();
            session.input_tokens += record.input_tokens as u64;
            session.output_tokens += record.output_tokens as u64;
            session.requests += 1;
            session.cost_usd += record.cost_usd;
        }

        // Store record
        {
            let mut records = self.records.write().unwrap();
            records.push(record);

            // Rotate if needed
            if records.len() > self.max_records {
                records.drain(0..10000);
            }
        }
    }

    /// Track usage from LLM response
    pub fn track(
        &self,
        provider: &str,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> UsageRecord {
        let record = UsageRecord::new(
            provider.to_string(),
            model.to_string(),
            input_tokens,
            output_tokens,
        );
        self.record(record.clone());
        record
    }

    /// Track usage with cached tokens
    pub fn track_with_cache(
        &self,
        provider: &str,
        model: &str,
        input_tokens: u32,
        output_tokens: u32,
        cached_tokens: u32,
    ) -> UsageRecord {
        let record = UsageRecord::with_cache(
            provider.to_string(),
            model.to_string(),
            input_tokens,
            output_tokens,
            cached_tokens,
        );
        self.record(record.clone());
        record
    }

    /// Get total usage statistics
    pub fn get_total_stats(&self) -> UsageStats {
        let records = self.records.read().unwrap();
        self.aggregate_stats(&records, None, None)
    }

    /// Get usage statistics for a time period
    pub fn get_stats_by_period(&self, hours: i64) -> UsageStats {
        let cutoff = Utc::now() - Duration::hours(hours);
        let records = self.records.read().unwrap();
        let filtered: Vec<&UsageRecord> = records
            .iter()
            .filter(|r| r.timestamp >= cutoff)
            .collect();

        self.aggregate_stats_refs(&filtered, Some(cutoff), Some(Utc::now()))
    }

    /// Get usage for a specific provider
    pub fn get_provider_stats(&self, provider: &str) -> ProviderUsage {
        let records = self.records.read().unwrap();

        let mut stats = ProviderUsage {
            provider: provider.to_string(),
            ..Default::default()
        };

        for record in records.iter() {
            if record.provider.eq_ignore_ascii_case(provider) {
                stats.input_tokens += record.input_tokens as u64;
                stats.output_tokens += record.output_tokens as u64;
                stats.requests += 1;
                stats.cost_usd += record.cost_usd;
            }
        }

        stats
    }

    /// Get current session usage
    pub fn get_session_usage(&self) -> SessionUsage {
        self.session_usage.read().unwrap().clone()
    }

    /// Reset session usage (start new session)
    pub fn reset_session(&self) {
        let mut session = self.session_usage.write().unwrap();
        *session = SessionUsage {
            session_start: Utc::now(),
            ..Default::default()
        };
    }

    /// Get detailed cost breakdown
    pub fn get_cost_breakdown(&self, hours: Option<i64>) -> CostBreakdown {
        let records = self.records.read().unwrap();

        let filtered: Vec<&UsageRecord> = if let Some(h) = hours {
            let cutoff = Utc::now() - Duration::hours(h);
            records.iter().filter(|r| r.timestamp >= cutoff).collect()
        } else {
            records.iter().collect()
        };

        let mut breakdown = CostBreakdown::default();
        let mut by_provider: HashMap<String, ProviderCostDetails> = HashMap::new();
        let mut by_model: HashMap<String, ModelCostDetails> = HashMap::new();

        for record in filtered.iter() {
            breakdown.total_cost_usd += record.cost_usd;

            // Provider breakdown
            let provider_entry = by_provider
                .entry(record.provider.clone())
                .or_insert_with(|| ProviderCostDetails {
                    provider: record.provider.clone(),
                    ..Default::default()
                });
            provider_entry.total_cost_usd += record.cost_usd;
            provider_entry.input_tokens += record.input_tokens as u64;
            provider_entry.output_tokens += record.output_tokens as u64;
            provider_entry.requests += 1;

            // Model breakdown
            let model_key = format!("{}:{}", record.provider, record.model);
            let pricing = get_model_pricing(&record.provider, &record.model);
            let model_entry = by_model.entry(model_key).or_insert_with(|| ModelCostDetails {
                model: record.model.clone(),
                provider: record.provider.clone(),
                input_price_per_million: pricing.input_price_per_million,
                output_price_per_million: pricing.output_price_per_million,
                ..Default::default()
            });
            model_entry.total_cost_usd += record.cost_usd;
            model_entry.input_tokens += record.input_tokens as u64;
            model_entry.output_tokens += record.output_tokens as u64;
            model_entry.requests += 1;
        }

        // Calculate averages
        for provider in by_provider.values_mut() {
            if provider.requests > 0 {
                provider.avg_cost_per_request = provider.total_cost_usd / provider.requests as f64;
            }
        }

        breakdown.by_provider = by_provider;
        breakdown.by_model = by_model;

        if let Some(h) = hours {
            let cutoff = Utc::now() - Duration::hours(h);
            breakdown.period_start = Some(cutoff.to_rfc3339());
            breakdown.period_end = Some(Utc::now().to_rfc3339());
        }

        breakdown
    }

    /// Set a budget limit
    pub fn set_budget_limit(&self, limit: BudgetLimit) {
        let mut limits = self.budget_limits.write().unwrap();
        limits.insert(limit.period, limit);
    }

    /// Get all budget limits
    pub fn get_budget_limits(&self) -> HashMap<BudgetPeriodType, BudgetLimit> {
        self.budget_limits.read().unwrap().clone()
    }

    /// Check budget status for all configured limits
    pub fn check_budget_status(&self) -> Vec<BudgetStatus> {
        let limits = self.budget_limits.read().unwrap();
        let mut statuses = Vec::new();

        for (period, limit) in limits.iter() {
            let hours = match period {
                BudgetPeriodType::Hourly => 1,
                BudgetPeriodType::Daily => 24,
                BudgetPeriodType::Weekly => 168,
                BudgetPeriodType::Monthly => 720,
                BudgetPeriodType::Total => 0,
            };

            let spent = if hours > 0 {
                self.get_stats_by_period(hours).total_cost_usd
            } else {
                self.get_total_stats().total_cost_usd
            };

            let mut status = limit.check(spent);

            // Add period end time
            if hours > 0 {
                let period_end = Utc::now() + Duration::hours(hours);
                status.period_ends_at = Some(period_end.to_rfc3339());
            }

            statuses.push(status);
        }

        statuses
    }

    /// Check if a request should be allowed based on budget
    pub fn should_allow_request(&self, estimated_cost: f64) -> (bool, Option<BudgetStatus>) {
        let limits = self.budget_limits.read().unwrap();

        for (period, limit) in limits.iter() {
            if !limit.block_on_limit {
                continue;
            }

            let hours = match period {
                BudgetPeriodType::Hourly => 1,
                BudgetPeriodType::Daily => 24,
                BudgetPeriodType::Weekly => 168,
                BudgetPeriodType::Monthly => 720,
                BudgetPeriodType::Total => 0,
            };

            let spent = if hours > 0 {
                self.get_stats_by_period(hours).total_cost_usd
            } else {
                self.get_total_stats().total_cost_usd
            };

            if spent + estimated_cost > limit.limit_usd {
                return (false, Some(limit.check(spent)));
            }
        }

        (true, None)
    }

    /// Clear all usage records
    pub fn clear(&self) {
        let mut records = self.records.write().unwrap();
        records.clear();
    }

    /// Get recent records
    pub fn get_recent_records(&self, limit: usize) -> Vec<UsageRecord> {
        let records = self.records.read().unwrap();
        records.iter().rev().take(limit).cloned().collect()
    }

    // Helper to aggregate stats
    fn aggregate_stats(
        &self,
        records: &[UsageRecord],
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> UsageStats {
        let refs: Vec<&UsageRecord> = records.iter().collect();
        self.aggregate_stats_refs(&refs, start, end)
    }

    fn aggregate_stats_refs(
        &self,
        records: &[&UsageRecord],
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> UsageStats {
        let mut stats = UsageStats {
            period_start: start,
            period_end: end,
            ..Default::default()
        };

        let mut by_provider: HashMap<String, ProviderUsage> = HashMap::new();

        for record in records.iter() {
            stats.total_input_tokens += record.input_tokens as u64;
            stats.total_output_tokens += record.output_tokens as u64;
            stats.total_cached_tokens += record.cached_tokens.unwrap_or(0) as u64;
            stats.total_requests += 1;
            stats.total_cost_usd += record.cost_usd;

            let provider_entry = by_provider
                .entry(record.provider.clone())
                .or_insert_with(|| ProviderUsage {
                    provider: record.provider.clone(),
                    ..Default::default()
                });
            provider_entry.input_tokens += record.input_tokens as u64;
            provider_entry.output_tokens += record.output_tokens as u64;
            provider_entry.requests += 1;
            provider_entry.cost_usd += record.cost_usd;
        }

        stats.by_provider = by_provider;
        stats
    }
}

impl Default for UsageTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_record_creation() {
        let record = UsageRecord::new(
            "claude".to_string(),
            "claude-3-5-sonnet".to_string(),
            1000,
            500,
        );
        assert!(record.cost_usd > 0.0);
        assert_eq!(record.input_tokens, 1000);
        assert_eq!(record.output_tokens, 500);
    }

    #[test]
    fn test_usage_tracking() {
        let tracker = UsageTracker::new();

        tracker.track("claude", "claude-3-5-sonnet", 1000, 500);
        tracker.track("openai", "gpt-4o", 2000, 1000);

        let stats = tracker.get_total_stats();
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.total_input_tokens, 3000);
        assert_eq!(stats.total_output_tokens, 1500);
        assert!(stats.total_cost_usd > 0.0);
    }

    #[test]
    fn test_session_usage() {
        let tracker = UsageTracker::new();

        tracker.track("claude", "claude-3-5-sonnet", 1000, 500);

        let session = tracker.get_session_usage();
        assert_eq!(session.requests, 1);
        assert!(session.cost_usd > 0.0);

        tracker.reset_session();

        let session = tracker.get_session_usage();
        assert_eq!(session.requests, 0);
        assert_eq!(session.cost_usd, 0.0);
    }

    #[test]
    fn test_cost_breakdown() {
        let tracker = UsageTracker::new();

        tracker.track("claude", "claude-3-5-sonnet", 1000, 500);
        tracker.track("claude", "claude-3-5-sonnet", 2000, 1000);
        tracker.track("openai", "gpt-4o", 1000, 500);

        let breakdown = tracker.get_cost_breakdown(None);
        assert_eq!(breakdown.by_provider.len(), 2);
        assert!(breakdown.total_cost_usd > 0.0);
    }

    #[test]
    fn test_budget_check() {
        let tracker = UsageTracker::new();

        tracker.set_budget_limit(BudgetLimit {
            limit_usd: 1.0,
            period: BudgetPeriodType::Daily,
            warning_threshold: 0.8,
            critical_threshold: 0.95,
            block_on_limit: true,
        });

        // Initial state should allow
        let (allowed, _) = tracker.should_allow_request(0.5);
        assert!(allowed);
    }
}
