//! Analytics Components Module
//!
//! Dashboard components for usage tracking, search analytics, and audit logs.

pub mod usage_dashboard;
pub mod search_analytics;
pub mod audit_log_viewer;

pub use usage_dashboard::{UsageDashboard, UsageDashboardPage};
pub use search_analytics::{SearchAnalyticsDashboard, SearchAnalyticsPage};
pub use audit_log_viewer::{AuditLogViewer, AuditLogsPage};
