use super::core::{invoke, invoke_no_args, invoke_void, invoke_void_no_args};
use serde::{Deserialize, Serialize};

// ============================================================================
// System Info & Health
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub version: String,
}

pub async fn get_app_version() -> Result<String, String> {
    invoke_no_args("get_app_version").await
}

pub async fn get_system_info() -> Result<SystemInfo, String> {
    invoke_no_args("get_system_info").await
}

// ============================================================================
// Credential Commands
// ============================================================================

pub async fn save_api_key(provider: String, api_key: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        provider: String,
        api_key: String,
    }
    invoke_void("save_api_key", &Args { provider, api_key }).await
}

pub async fn get_api_key(provider: String) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    struct Args {
        provider: String,
    }
    invoke("get_api_key", &Args { provider }).await
}

pub async fn list_stored_providers() -> Result<Vec<String>, String> {
    invoke_no_args("list_stored_providers").await
}

// ============================================================================
// Usage Tracking
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cached_tokens: u64,
    pub total_requests: u32,
    pub total_cost_usd: f64,
    pub period_start: Option<String>,
    pub period_end: Option<String>,
    pub by_provider: std::collections::HashMap<String, ProviderUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderUsage {
    pub provider: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u32,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostBreakdown {
    pub total_cost_usd: f64,
    pub by_provider: std::collections::HashMap<String, ProviderCostDetails>,
    pub by_model: std::collections::HashMap<String, ModelCostDetails>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetLimit {
    pub limit_usd: f64,
    pub period: String,
    pub warning_threshold: f64,
    pub critical_threshold: f64,
    pub block_on_limit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub period: String,
    pub limit_usd: f64,
    pub spent_usd: f64,
    pub remaining_usd: f64,
    pub percentage_used: f64,
    pub status: String,
    pub period_ends_at: Option<String>,
}

// Session usage types for chat component (backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUsage {
    pub session_input_tokens: u64,
    pub session_output_tokens: u64,
    pub session_requests: u32,
    pub session_cost_usd: f64,
}

impl Default for SessionUsage {
    fn default() -> Self {
        Self {
            session_input_tokens: 0,
            session_output_tokens: 0,
            session_requests: 0,
            session_cost_usd: 0.0,
        }
    }
}

pub async fn get_usage_stats() -> Result<UsageStats, String> {
    invoke_no_args("get_usage_stats").await
}

pub async fn get_usage_by_period(hours: i64) -> Result<UsageStats, String> {
    #[derive(Serialize)]
    struct Args {
        hours: i64,
    }
    invoke("get_usage_by_period", &Args { hours }).await
}

pub async fn get_cost_breakdown(hours: Option<i64>) -> Result<CostBreakdown, String> {
    #[derive(Serialize)]
    struct Args {
        hours: Option<i64>,
    }
    invoke("get_cost_breakdown", &Args { hours }).await
}

pub async fn get_budget_status() -> Result<Vec<BudgetStatus>, String> {
    invoke_no_args("get_budget_status").await
}

pub async fn set_budget_limit(limit: BudgetLimit) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        limit: BudgetLimit,
    }
    invoke_void("set_budget_limit", &Args { limit }).await
}

pub async fn get_provider_usage(provider: String) -> Result<ProviderUsage, String> {
    #[derive(Serialize)]
    struct Args {
        provider: String,
    }
    invoke("get_provider_usage", &Args { provider }).await
}

pub async fn reset_usage_session() -> Result<(), String> {
    invoke_void_no_args("reset_usage_session").await
}

pub async fn get_session_usage() -> Result<SessionUsage, String> {
    // Map from the new UsageStats format
    let stats: Result<UsageStats, String> = invoke_no_args("get_usage_stats").await;
    match stats {
        Ok(s) => Ok(SessionUsage {
            session_input_tokens: s.total_input_tokens,
            session_output_tokens: s.total_output_tokens,
            session_requests: s.total_requests,
            session_cost_usd: s.total_cost_usd,
        }),
        Err(e) => Err(e),
    }
}

// ============================================================================
// Security Audit Logs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    pub id: String,
    pub event_type: serde_json::Value, // Using Value for the complex enum
    pub severity: String,
    pub timestamp: String,
    pub context: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

pub async fn get_audit_logs(
    count: Option<usize>,
    min_severity: Option<String>,
) -> Result<Vec<SecurityAuditEvent>, String> {
    #[derive(Serialize)]
    struct Args {
        count: Option<usize>,
        min_severity: Option<String>,
    }
    invoke(
        "get_audit_logs",
        &Args {
            count,
            min_severity,
        },
    )
    .await
}

pub async fn query_audit_logs(
    from_hours: Option<i64>,
    min_severity: Option<String>,
    event_types: Option<Vec<String>>,
    search_text: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SecurityAuditEvent>, String> {
    #[derive(Serialize)]
    struct Args {
        from_hours: Option<i64>,
        min_severity: Option<String>,
        event_types: Option<Vec<String>>,
        search_text: Option<String>,
        limit: Option<usize>,
    }
    invoke(
        "query_audit_logs",
        &Args {
            from_hours,
            min_severity,
            event_types,
            search_text,
            limit,
        },
    )
    .await
}

pub async fn export_audit_logs(format: String, from_hours: Option<i64>) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        format: String,
        from_hours: Option<i64>,
    }
    invoke("export_audit_logs", &Args { format, from_hours }).await
}

pub async fn clear_old_logs(days: i64) -> Result<usize, String> {
    #[derive(Serialize)]
    struct Args {
        days: i64,
    }
    invoke("clear_old_logs", &Args { days }).await
}

pub async fn get_audit_summary() -> Result<std::collections::HashMap<String, usize>, String> {
    invoke_no_args("get_audit_summary").await
}

pub async fn get_security_events() -> Result<Vec<SecurityAuditEvent>, String> {
    invoke_no_args("get_security_events").await
}
