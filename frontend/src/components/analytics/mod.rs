//! Analytics Components Module
//!
//! Dashboard components for usage tracking, search analytics, and audit logs.

pub mod audit_log_viewer;
pub mod search_analytics;
pub mod usage_dashboard;

pub use audit_log_viewer::{AuditLogViewer, AuditLogsPage};
pub use search_analytics::{SearchAnalyticsDashboard, SearchAnalyticsPage};
pub use usage_dashboard::{UsageDashboard, UsageDashboardPage};
