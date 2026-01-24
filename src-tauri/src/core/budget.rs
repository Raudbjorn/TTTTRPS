//! Budget Enforcer Module
//!
//! Manages spending limits and budget enforcement across LLM providers.

use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

// ============================================================================
// Types
// ============================================================================

/// Budget period type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BudgetPeriod {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Total,
}

/// Budget limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetLimit {
    /// Maximum amount in USD
    pub amount: f64,
    /// Period for the limit
    pub period: BudgetPeriod,
    /// Soft limit threshold (percentage, e.g., 0.8 = 80%)
    pub soft_threshold: f64,
    /// Hard limit threshold (percentage, e.g., 0.95 = 95%)
    pub hard_threshold: f64,
}

impl Default for BudgetLimit {
    fn default() -> Self {
        Self {
            amount: 50.0, // $50 default
            period: BudgetPeriod::Monthly,
            soft_threshold: 0.8,
            hard_threshold: 0.95,
        }
    }
}

/// Action to take when budget threshold is reached
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BudgetAction {
    /// Allow the request, no action needed
    Allow,
    /// Warn about approaching limit
    Warn,
    /// Throttle requests (add delay)
    Throttle,
    /// Downgrade to cheaper model
    Degrade,
    /// Reject the request
    Reject,
}

/// Budget status for a period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    /// Current spending in period
    pub spent: f64,
    /// Budget limit amount
    pub limit: f64,
    /// Percentage used (0.0 - 1.0+)
    pub percentage_used: f64,
    /// Recommended action
    pub action: BudgetAction,
    /// Period end time
    pub period_ends: DateTime<Utc>,
    /// Spending velocity (per hour)
    pub velocity_per_hour: f64,
    /// Projected spend by period end
    pub projected_spend: f64,
}

/// Spending record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingRecord {
    pub timestamp: DateTime<Utc>,
    pub amount: f64,
    pub provider: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// ============================================================================
// Budget Enforcer
// ============================================================================

/// Manages and enforces budget limits
pub struct BudgetEnforcer {
    /// Budget limits by period
    limits: RwLock<HashMap<BudgetPeriod, BudgetLimit>>,
    /// Spending history
    spending: RwLock<Vec<SpendingRecord>>,
    /// Per-provider limits (optional)
    provider_limits: RwLock<HashMap<String, f64>>,
}

impl BudgetEnforcer {
    pub fn new() -> Self {
        let mut limits = HashMap::new();
        limits.insert(BudgetPeriod::Monthly, BudgetLimit::default());

        Self {
            limits: RwLock::new(limits),
            spending: RwLock::new(Vec::new()),
            provider_limits: RwLock::new(HashMap::new()),
        }
    }

    /// Set a budget limit
    pub fn set_limit(&self, limit: BudgetLimit) {
        let mut limits = self.limits.write().unwrap();
        limits.insert(limit.period, limit);
    }

    /// Set a per-provider limit
    pub fn set_provider_limit(&self, provider: &str, amount: f64) {
        let mut limits = self.provider_limits.write().unwrap();
        limits.insert(provider.to_string(), amount);
    }

    /// Record spending
    pub fn record_spending(&self, record: SpendingRecord) {
        let mut spending = self.spending.write().unwrap();
        spending.push(record);

        // Clean up old records (keep last 90 days)
        let cutoff = Utc::now() - Duration::days(90);
        spending.retain(|r| r.timestamp > cutoff);
    }

    /// Check if a request should be allowed
    pub fn check_request(&self, estimated_cost: f64, provider: &str) -> BudgetAction {
        let statuses = self.get_all_statuses();

        // Check each period's status
        let mut worst_action = BudgetAction::Allow;

        for status in statuses.values() {
            // Would this request push us over?
            let new_percentage = (status.spent + estimated_cost) / status.limit;

            let action = if new_percentage >= 1.0 {
                BudgetAction::Reject
            } else if new_percentage >= status.percentage_used
                && status.percentage_used >= 0.95
            {
                BudgetAction::Degrade
            } else if new_percentage >= 0.9 {
                BudgetAction::Throttle
            } else if new_percentage >= 0.8 {
                BudgetAction::Warn
            } else {
                BudgetAction::Allow
            };

            // Keep the most restrictive action
            if action_severity(action) > action_severity(worst_action) {
                worst_action = action;
            }
        }

        // Also check provider-specific limits
        let provider_limits = self.provider_limits.read().unwrap();
        if let Some(&limit) = provider_limits.get(provider) {
            let provider_spent = self.get_provider_spending(provider, BudgetPeriod::Monthly);
            if provider_spent + estimated_cost >= limit
                && action_severity(BudgetAction::Reject) > action_severity(worst_action) {
                    worst_action = BudgetAction::Reject;
                }
        }

        worst_action
    }

    /// Get budget status for all periods
    pub fn get_all_statuses(&self) -> HashMap<BudgetPeriod, BudgetStatus> {
        let limits = self.limits.read().unwrap();
        let mut statuses = HashMap::new();

        for (period, limit) in limits.iter() {
            statuses.insert(*period, self.get_status(*period, limit));
        }

        statuses
    }

    /// Get status for a specific period
    fn get_status(&self, period: BudgetPeriod, limit: &BudgetLimit) -> BudgetStatus {
        let (period_start, period_end) = get_period_bounds(period);
        let spent = self.get_spending_in_period(period_start, Utc::now());
        let percentage_used = spent / limit.amount;

        // Calculate velocity
        let hours_elapsed = (Utc::now() - period_start).num_hours().max(1) as f64;
        let velocity_per_hour = spent / hours_elapsed;

        // Project spend
        let hours_remaining = (period_end - Utc::now()).num_hours().max(0) as f64;
        let projected_spend = spent + (velocity_per_hour * hours_remaining);

        // Determine action
        let action = if percentage_used >= 1.0 {
            BudgetAction::Reject
        } else if percentage_used >= limit.hard_threshold {
            BudgetAction::Degrade
        } else if percentage_used >= limit.soft_threshold {
            BudgetAction::Warn
        } else {
            BudgetAction::Allow
        };

        BudgetStatus {
            spent,
            limit: limit.amount,
            percentage_used,
            action,
            period_ends: period_end,
            velocity_per_hour,
            projected_spend,
        }
    }

    /// Get total spending in a time period
    fn get_spending_in_period(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> f64 {
        let spending = self.spending.read().unwrap();
        spending
            .iter()
            .filter(|r| r.timestamp >= start && r.timestamp <= end)
            .map(|r| r.amount)
            .sum()
    }

    /// Get spending for a specific provider
    fn get_provider_spending(&self, provider: &str, period: BudgetPeriod) -> f64 {
        let (period_start, _) = get_period_bounds(period);
        let spending = self.spending.read().unwrap();
        spending
            .iter()
            .filter(|r| r.provider == provider && r.timestamp >= period_start)
            .map(|r| r.amount)
            .sum()
    }

    /// Get recommended model based on budget status
    pub fn recommend_model(&self, preferred_model: &str, provider: &str) -> String {
        let action = self.check_request(0.0001, provider); // Check with small amount

        match action {
            BudgetAction::Degrade => {
                // Downgrade to cheaper models
                match preferred_model {
                    "gpt-4o" | "gpt-4-turbo" => "gpt-4o-mini".to_string(),
                    "claude-3-opus" | "claude-3-5-sonnet" => "claude-3-haiku".to_string(),
                    "gemini-1.5-pro" => "gemini-1.5-flash".to_string(),
                    _ => preferred_model.to_string(),
                }
            }
            _ => preferred_model.to_string(),
        }
    }

    /// Get spending summary
    pub fn get_spending_summary(&self) -> SpendingSummary {
        let spending = self.spending.read().unwrap();
        let now = Utc::now();

        let today_start = now - Duration::hours(now.hour() as i64);
        let week_start = now - Duration::days(now.weekday().num_days_from_monday() as i64);
        let month_start = now - Duration::days(now.day() as i64 - 1);

        let today: f64 = spending
            .iter()
            .filter(|r| r.timestamp >= today_start)
            .map(|r| r.amount)
            .sum();

        let this_week: f64 = spending
            .iter()
            .filter(|r| r.timestamp >= week_start)
            .map(|r| r.amount)
            .sum();

        let this_month: f64 = spending
            .iter()
            .filter(|r| r.timestamp >= month_start)
            .map(|r| r.amount)
            .sum();

        let mut by_provider: HashMap<String, f64> = HashMap::new();
        for record in spending.iter().filter(|r| r.timestamp >= month_start) {
            *by_provider.entry(record.provider.clone()).or_default() += record.amount;
        }

        SpendingSummary {
            today,
            this_week,
            this_month,
            by_provider,
        }
    }
}

impl Default for BudgetEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

/// Spending summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingSummary {
    pub today: f64,
    pub this_week: f64,
    pub this_month: f64,
    pub by_provider: HashMap<String, f64>,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn action_severity(action: BudgetAction) -> u8 {
    match action {
        BudgetAction::Allow => 0,
        BudgetAction::Warn => 1,
        BudgetAction::Throttle => 2,
        BudgetAction::Degrade => 3,
        BudgetAction::Reject => 4,
    }
}

fn get_period_bounds(period: BudgetPeriod) -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Utc::now();

    match period {
        BudgetPeriod::Hourly => {
            let start = now - Duration::minutes(now.minute() as i64);
            let end = start + Duration::hours(1);
            (start, end)
        }
        BudgetPeriod::Daily => {
            let start = now - Duration::hours(now.hour() as i64);
            let end = start + Duration::days(1);
            (start, end)
        }
        BudgetPeriod::Weekly => {
            let days_since_monday = now.weekday().num_days_from_monday() as i64;
            let start = now - Duration::days(days_since_monday);
            let end = start + Duration::weeks(1);
            (start, end)
        }
        BudgetPeriod::Monthly => {
            let start = now - Duration::days(now.day() as i64 - 1);
            let end = start + Duration::days(30); // Approximation
            (start, end)
        }
        BudgetPeriod::Total => {
            let start = DateTime::from_timestamp(0, 0).unwrap();
            let end = now + Duration::days(365 * 100); // Far future
            (start, end)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_enforcer_basic() {
        let enforcer = BudgetEnforcer::new();

        enforcer.set_limit(BudgetLimit {
            amount: 10.0,
            period: BudgetPeriod::Daily,
            soft_threshold: 0.8,
            hard_threshold: 0.95,
        });

        // Should allow initially
        let action = enforcer.check_request(1.0, "openai");
        assert_eq!(action, BudgetAction::Allow);
    }

    #[test]
    fn test_spending_record() {
        let enforcer = BudgetEnforcer::new();

        enforcer.record_spending(SpendingRecord {
            timestamp: Utc::now(),
            amount: 0.05,
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            input_tokens: 100,
            output_tokens: 50,
        });

        let summary = enforcer.get_spending_summary();
        assert!(summary.today > 0.0);
    }

    #[test]
    fn test_model_recommendation() {
        let enforcer = BudgetEnforcer::new();

        // Set a very low limit
        enforcer.set_limit(BudgetLimit {
            amount: 0.01,
            period: BudgetPeriod::Daily,
            soft_threshold: 0.8,
            hard_threshold: 0.95,
        });

        // Record spending that exceeds hard threshold
        enforcer.record_spending(SpendingRecord {
            timestamp: Utc::now(),
            amount: 0.0096, // 96% of limit
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            input_tokens: 100,
            output_tokens: 50,
        });

        let recommended = enforcer.recommend_model("gpt-4o", "openai");
        assert_eq!(recommended, "gpt-4o-mini");
    }
}
