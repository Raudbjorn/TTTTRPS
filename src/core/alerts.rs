//! Alert System Module
//!
//! Provides alerting functionality for budget thresholds, provider issues, and anomalies.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

// ============================================================================
// Types
// ============================================================================

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Type of alert
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    /// Budget is approaching limit (threshold percentage)
    BudgetApproaching { threshold: u8 },
    /// Budget has been exceeded
    BudgetExceeded,
    /// Provider is down or unreachable
    ProviderDown { provider: String },
    /// Provider rate limit hit
    ProviderRateLimited { provider: String },
    /// Unusual spending pattern detected
    AnomalyDetected { description: String },
    /// Provider quota exceeded
    QuotaExceeded { provider: String },
    /// Custom alert
    Custom { category: String, message: String },
}

/// Alert notification channel
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertChannel {
    Log,
    SystemNotification,
    Webhook { url: String },
}

/// An alert instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert ID
    pub id: String,
    /// Alert type
    pub alert_type: AlertType,
    /// Severity level
    pub severity: AlertSeverity,
    /// Human-readable message
    pub message: String,
    /// When the alert was created
    pub created_at: DateTime<Utc>,
    /// Whether the alert has been acknowledged
    pub acknowledged: bool,
    /// When acknowledged
    pub acknowledged_at: Option<DateTime<Utc>>,
    /// Additional context
    pub context: HashMap<String, String>,
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Enabled alert channels
    pub channels: Vec<AlertChannel>,
    /// Budget thresholds that trigger alerts (percentages)
    pub budget_thresholds: Vec<u8>,
    /// Minimum time between duplicate alerts (seconds)
    pub dedup_window_secs: u64,
    /// Whether to alert on provider issues
    pub alert_on_provider_issues: bool,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            channels: vec![AlertChannel::Log, AlertChannel::SystemNotification],
            budget_thresholds: vec![50, 80, 90, 95, 100],
            dedup_window_secs: 3600, // 1 hour
            alert_on_provider_issues: true,
        }
    }
}

// ============================================================================
// Alert System
// ============================================================================

/// Manages alerts and notifications
pub struct AlertSystem {
    config: RwLock<AlertConfig>,
    alerts: RwLock<Vec<Alert>>,
    /// Track last alert time by type for deduplication
    last_alert_times: RwLock<HashMap<String, DateTime<Utc>>>,
}

impl AlertSystem {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(AlertConfig::default()),
            alerts: RwLock::new(Vec::new()),
            last_alert_times: RwLock::new(HashMap::new()),
        }
    }

    pub fn with_config(config: AlertConfig) -> Self {
        Self {
            config: RwLock::new(config),
            alerts: RwLock::new(Vec::new()),
            last_alert_times: RwLock::new(HashMap::new()),
        }
    }

    /// Update configuration
    pub fn set_config(&self, config: AlertConfig) {
        let mut cfg = self.config.write().unwrap();
        *cfg = config;
    }

    /// Trigger an alert
    pub fn trigger(&self, alert_type: AlertType, message: &str, severity: AlertSeverity) -> Option<String> {
        // Check for deduplication
        let dedup_key = format!("{:?}", alert_type);
        if self.is_duplicate(&dedup_key) {
            return None;
        }

        let alert_id = uuid::Uuid::new_v4().to_string();
        let alert = Alert {
            id: alert_id.clone(),
            alert_type,
            severity,
            message: message.to_string(),
            created_at: Utc::now(),
            acknowledged: false,
            acknowledged_at: None,
            context: HashMap::new(),
        };

        // Store alert
        {
            let mut alerts = self.alerts.write().unwrap();
            alerts.push(alert.clone());

            // Keep only last 1000 alerts
            if alerts.len() > 1000 {
                alerts.drain(0..100);
            }
        }

        // Update last alert time
        {
            let mut times = self.last_alert_times.write().unwrap();
            times.insert(dedup_key, Utc::now());
        }

        // Deliver via channels
        self.deliver_alert(&alert);

        Some(alert_id)
    }

    /// Check if this is a duplicate alert
    fn is_duplicate(&self, key: &str) -> bool {
        let config = self.config.read().unwrap();
        let times = self.last_alert_times.read().unwrap();

        if let Some(last_time) = times.get(key) {
            let elapsed = Utc::now() - *last_time;
            return elapsed < Duration::seconds(config.dedup_window_secs as i64);
        }

        false
    }

    /// Deliver alert to configured channels
    fn deliver_alert(&self, alert: &Alert) {
        let config = self.config.read().unwrap();

        for channel in &config.channels {
            match channel {
                AlertChannel::Log => {
                    self.log_alert(alert);
                }
                AlertChannel::SystemNotification => {
                    self.system_notify(alert);
                }
                AlertChannel::Webhook { url } => {
                    self.webhook_notify(alert, url);
                }
            }
        }
    }

    fn log_alert(&self, alert: &Alert) {
        match alert.severity {
            AlertSeverity::Info => {
                tracing::info!(
                    alert_id = %alert.id,
                    alert_type = ?alert.alert_type,
                    "Alert: {}",
                    alert.message
                );
            }
            AlertSeverity::Warning => {
                tracing::warn!(
                    alert_id = %alert.id,
                    alert_type = ?alert.alert_type,
                    "Alert: {}",
                    alert.message
                );
            }
            AlertSeverity::Critical => {
                tracing::error!(
                    alert_id = %alert.id,
                    alert_type = ?alert.alert_type,
                    "CRITICAL Alert: {}",
                    alert.message
                );
            }
        }
    }

    fn system_notify(&self, alert: &Alert) {
        // In a real implementation, this would use tauri's notification API
        // For now, just log that we would notify
        tracing::debug!(
            "Would send system notification: {} - {}",
            alert.severity_str(),
            alert.message
        );
    }

    fn webhook_notify(&self, alert: &Alert, _url: &str) {
        // In a real implementation, this would POST to the webhook URL
        // For now, just log
        tracing::debug!(
            "Would send webhook notification for alert: {}",
            alert.id
        );
    }

    /// Trigger a budget alert based on percentage used
    pub fn trigger_budget_alert(&self, percentage: f64, limit: f64, spent: f64) {
        let config = self.config.read().unwrap();
        let percentage_int = (percentage * 100.0) as u8;

        for threshold in &config.budget_thresholds {
            if percentage_int >= *threshold {
                let (alert_type, severity) = if percentage_int >= 100 {
                    (AlertType::BudgetExceeded, AlertSeverity::Critical)
                } else {
                    (
                        AlertType::BudgetApproaching { threshold: *threshold },
                        if *threshold >= 90 {
                            AlertSeverity::Warning
                        } else {
                            AlertSeverity::Info
                        },
                    )
                };

                let message = format!(
                    "Budget {}% used: ${:.2} of ${:.2}",
                    percentage_int, spent, limit
                );

                self.trigger(alert_type, &message, severity);
                break; // Only trigger highest threshold
            }
        }
    }

    /// Trigger a provider down alert
    pub fn trigger_provider_down(&self, provider: &str, error: &str) {
        let config = self.config.read().unwrap();
        if !config.alert_on_provider_issues {
            return;
        }

        self.trigger(
            AlertType::ProviderDown {
                provider: provider.to_string(),
            },
            &format!("Provider {} is down: {}", provider, error),
            AlertSeverity::Critical,
        );
    }

    /// Trigger a rate limit alert
    pub fn trigger_rate_limit(&self, provider: &str, retry_after_secs: u64) {
        let config = self.config.read().unwrap();
        if !config.alert_on_provider_issues {
            return;
        }

        self.trigger(
            AlertType::ProviderRateLimited {
                provider: provider.to_string(),
            },
            &format!(
                "Provider {} rate limited. Retry after {} seconds",
                provider, retry_after_secs
            ),
            AlertSeverity::Warning,
        );
    }

    /// Acknowledge an alert
    pub fn acknowledge(&self, alert_id: &str) -> bool {
        let mut alerts = self.alerts.write().unwrap();
        if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
            alert.acknowledged_at = Some(Utc::now());
            return true;
        }
        false
    }

    /// Get all unacknowledged alerts
    pub fn get_unacknowledged(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().unwrap();
        alerts
            .iter()
            .filter(|a| !a.acknowledged)
            .cloned()
            .collect()
    }

    /// Get alerts by severity
    pub fn get_by_severity(&self, severity: AlertSeverity) -> Vec<Alert> {
        let alerts = self.alerts.read().unwrap();
        alerts
            .iter()
            .filter(|a| a.severity == severity)
            .cloned()
            .collect()
    }

    /// Get recent alerts (last N)
    pub fn get_recent(&self, count: usize) -> Vec<Alert> {
        let alerts = self.alerts.read().unwrap();
        alerts.iter().rev().take(count).cloned().collect()
    }

    /// Clear old alerts (older than days)
    pub fn cleanup_old_alerts(&self, days: i64) {
        let cutoff = Utc::now() - Duration::days(days);
        let mut alerts = self.alerts.write().unwrap();
        alerts.retain(|a| a.created_at > cutoff);
    }
}

impl Default for AlertSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Alert {
    fn severity_str(&self) -> &str {
        match self.severity {
            AlertSeverity::Info => "INFO",
            AlertSeverity::Warning => "WARNING",
            AlertSeverity::Critical => "CRITICAL",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_creation() {
        let system = AlertSystem::new();

        let id = system.trigger(
            AlertType::Custom {
                category: "test".to_string(),
                message: "Test alert".to_string(),
            },
            "This is a test alert",
            AlertSeverity::Info,
        );

        assert!(id.is_some());

        let alerts = system.get_recent(10);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].message, "This is a test alert");
    }

    #[test]
    fn test_deduplication() {
        let config = AlertConfig {
            dedup_window_secs: 3600,
            ..Default::default()
        };
        let system = AlertSystem::with_config(config);

        let id1 = system.trigger(
            AlertType::BudgetExceeded,
            "Budget exceeded",
            AlertSeverity::Critical,
        );

        let id2 = system.trigger(
            AlertType::BudgetExceeded,
            "Budget exceeded again",
            AlertSeverity::Critical,
        );

        assert!(id1.is_some());
        assert!(id2.is_none()); // Should be deduplicated
    }

    #[test]
    fn test_acknowledge() {
        let system = AlertSystem::new();

        let id = system
            .trigger(
                AlertType::BudgetExceeded,
                "Test",
                AlertSeverity::Warning,
            )
            .unwrap();

        assert_eq!(system.get_unacknowledged().len(), 1);

        system.acknowledge(&id);

        assert_eq!(system.get_unacknowledged().len(), 0);
    }
}
