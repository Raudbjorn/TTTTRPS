//! Usage and quota API.
//!
//! This module provides functionality for fetching Copilot usage information
//! and quota status from the GitHub API.

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};

use crate::oauth::copilot::auth::constants::{API_VERSION, GITHUB_API_BASE_URL};
use crate::oauth::copilot::client::CopilotClient;
use crate::oauth::copilot::error::{Error, Result};
use crate::oauth::copilot::storage::CopilotTokenStorage;

/// Response from the Copilot usage/quota endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageResponse {
    /// The user's Copilot plan type.
    pub copilot_plan: String,

    /// The date when quotas reset (ISO 8601 format).
    pub quota_reset_date: String,

    /// Quota snapshots for different usage categories.
    #[serde(default)]
    pub quota_snapshots: QuotaSnapshots,
}

/// Quota snapshots for different Copilot features.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuotaSnapshots {
    /// Chat completions quota.
    #[serde(default)]
    pub completions: Option<QuotaInfo>,

    /// Premium requests quota (for advanced models).
    #[serde(default)]
    pub premium_requests: Option<QuotaInfo>,

    /// Code completions quota.
    #[serde(default)]
    pub code_completions: Option<QuotaInfo>,
}

/// Information about a single quota category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaInfo {
    /// Number of units used in the current period.
    pub used: u64,

    /// Maximum units allowed in the current period.
    pub limit: u64,

    /// Whether the quota is unlimited.
    #[serde(default)]
    pub unlimited: bool,

    /// Overage units (usage beyond limit).
    #[serde(default)]
    pub overage: u64,
}

impl QuotaInfo {
    /// Returns the remaining quota.
    #[must_use]
    pub fn remaining(&self) -> u64 {
        if self.unlimited {
            u64::MAX
        } else {
            self.limit.saturating_sub(self.used)
        }
    }

    /// Returns true if the quota is exhausted.
    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        !self.unlimited && self.used >= self.limit
    }

    /// Returns the usage percentage (0.0 to 100.0+).
    #[must_use]
    pub fn usage_percent(&self) -> f64 {
        if self.unlimited || self.limit == 0 {
            0.0
        } else {
            (self.used as f64 / self.limit as f64) * 100.0
        }
    }
}

impl<S: CopilotTokenStorage> CopilotClient<S> {
    /// Fetches usage and quota information.
    ///
    /// This retrieves the user's Copilot plan, quota reset date,
    /// and current usage for various quota categories.
    ///
    /// Note: This endpoint uses the GitHub API directly with the GitHub token,
    /// not the Copilot token.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Not authenticated
    /// - Network error
    /// - API returns an error response
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use crate::oauth::copilot::CopilotClient;
    /// # async fn example() -> crate::oauth::copilot::Result<()> {
    /// let client = CopilotClient::builder().build()?;
    /// let usage = client.usage().await?;
    ///
    /// println!("Copilot Plan: {}", usage.copilot_plan);
    /// println!("Quota resets: {}", usage.quota_reset_date);
    ///
    /// // Check chat completions quota
    /// if let Some(completions) = &usage.quota_snapshots.completions {
    ///     println!("Chat usage: {:.1}%", completions.usage_percent());
    ///     if completions.is_exhausted() {
    ///         println!("Warning: Chat quota exhausted!");
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[instrument(skip(self))]
    pub async fn usage(&self) -> Result<UsageResponse> {
        debug!("Fetching usage information");

        // Get token from storage
        let token = self
            .storage()
            .load()
            .await?
            .ok_or(Error::NotAuthenticated)?;

        if token.github_token.is_empty() {
            return Err(Error::NotAuthenticated);
        }

        // Build headers for GitHub API (uses GitHub token, not Copilot token)
        let mut headers = HeaderMap::new();

        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("token {}", token.github_token))
                .map_err(|e| Error::Config(format!("Invalid token header: {e}")))?,
        );

        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(crate::oauth::copilot::auth::constants::USER_AGENT),
        );

        headers.insert(
            "editor-version",
            HeaderValue::from_str(&format!("vscode/{}", self.config().vs_code_version))
                .map_err(|e| Error::Config(format!("Invalid editor version: {e}")))?,
        );

        headers.insert(
            "x-github-api-version",
            HeaderValue::from_static(API_VERSION),
        );

        // Make the request to GitHub API
        // Note: Uses internal endpoint matching VS Code Copilot implementation.
        // The documented /orgs/{org}/members/{username}/copilot endpoint requires
        // organization context which individual Copilot users don't have.
        let response = self
            .http_client()
            .get(format!("{GITHUB_API_BASE_URL}/copilot_internal/user"))
            .headers(headers)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Api {
                status: status.as_u16(),
                message,
            });
        }

        let usage_response: UsageResponse = response.json().await?;
        debug!(
            plan = %usage_response.copilot_plan,
            reset_date = %usage_response.quota_reset_date,
            "Usage information fetched successfully"
        );

        Ok(usage_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quota_info_remaining() {
        let quota = QuotaInfo {
            used: 30,
            limit: 100,
            unlimited: false,
            overage: 0,
        };
        assert_eq!(quota.remaining(), 70);

        let exhausted = QuotaInfo {
            used: 120,
            limit: 100,
            unlimited: false,
            overage: 20,
        };
        assert_eq!(exhausted.remaining(), 0);

        let unlimited = QuotaInfo {
            used: 1000,
            limit: 0,
            unlimited: true,
            overage: 0,
        };
        assert_eq!(unlimited.remaining(), u64::MAX);
    }

    #[test]
    fn test_quota_info_is_exhausted() {
        let active = QuotaInfo {
            used: 50,
            limit: 100,
            unlimited: false,
            overage: 0,
        };
        assert!(!active.is_exhausted());

        let exhausted = QuotaInfo {
            used: 100,
            limit: 100,
            unlimited: false,
            overage: 0,
        };
        assert!(exhausted.is_exhausted());

        let unlimited = QuotaInfo {
            used: 1000000,
            limit: 0,
            unlimited: true,
            overage: 0,
        };
        assert!(!unlimited.is_exhausted());
    }

    #[test]
    fn test_quota_info_usage_percent() {
        let half = QuotaInfo {
            used: 50,
            limit: 100,
            unlimited: false,
            overage: 0,
        };
        assert!((half.usage_percent() - 50.0).abs() < f64::EPSILON);

        let over = QuotaInfo {
            used: 150,
            limit: 100,
            unlimited: false,
            overage: 50,
        };
        assert!((over.usage_percent() - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_usage_response_deserialization() {
        let json = r#"{
            "copilot_plan": "individual",
            "quota_reset_date": "2024-02-01T00:00:00Z",
            "quota_snapshots": {
                "completions": {
                    "used": 100,
                    "limit": 500,
                    "unlimited": false,
                    "overage": 0
                }
            }
        }"#;

        let response: UsageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.copilot_plan, "individual");
        assert!(response.quota_snapshots.completions.is_some());
    }

    #[test]
    fn test_usage_response_minimal() {
        let json = r#"{
            "copilot_plan": "business",
            "quota_reset_date": "2024-03-01T00:00:00Z"
        }"#;

        let response: UsageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.copilot_plan, "business");
        assert!(response.quota_snapshots.completions.is_none());
    }
}
