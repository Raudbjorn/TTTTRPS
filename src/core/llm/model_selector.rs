//! Dynamic Model Selection for Claude Code
//!
//! Intelligently selects between Claude models (Opus vs Sonnet) based on:
//! - Subscription plan (Pro, Max, Max20, Free, Team, Enterprise, API)
//! - Usage utilization (5-hour and 7-day windows)
//! - Task complexity (auto-detected from prompt)
//!
//! ## Usage
//!
//! ```ignore
//! let selector = ModelSelector::new();
//! let selection = selector.get_selection_for_prompt("refactor the auth module").await?;
//! println!("Using model: {} ({})", selection.model, selection.selection_reason);
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

// ============================================================================
// Constants
// ============================================================================

const USAGE_API_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const CACHE_TTL_SECS: u64 = 300; // 5 minutes
const API_TIMEOUT_SECS: u64 = 10;

// Model IDs
const OPUS_MODEL_ID: &str = "claude-opus-4-20250514";
const SONNET_MODEL_ID: &str = "claude-sonnet-4-20250514";

// ============================================================================
// Types
// ============================================================================

/// Subscription plan type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionPlan {
    Free,
    Pro,
    Max,
    Max20,
    Team,
    Enterprise,
    Api,
    Unknown,
}

impl std::fmt::Display for SubscriptionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Free => write!(f, "free"),
            Self::Pro => write!(f, "pro"),
            Self::Max => write!(f, "max"),
            Self::Max20 => write!(f, "max_20"),
            Self::Team => write!(f, "team"),
            Self::Enterprise => write!(f, "enterprise"),
            Self::Api => write!(f, "api"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&str> for SubscriptionPlan {
    /// Parse plan from string (liberal input acceptance).
    /// Canonical output format via Display uses "max" and "max_20".
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "free" => Self::Free,
            "pro" => Self::Pro,
            // Accept variants: "max", "max_5", "max5" all map to Max (5x Pro limits)
            "max" | "max_5" | "max5" => Self::Max,
            "max_20" | "max20" => Self::Max20,
            "team" => Self::Team,
            "enterprise" => Self::Enterprise,
            "api" => Self::Api,
            _ => Self::Unknown,
        }
    }
}

/// Authentication type detected
#[derive(Debug, Clone)]
pub enum AuthType {
    /// OAuth with subscription plan and access token
    OAuth {
        plan: SubscriptionPlan,
        token: String,
    },
    /// API key (pay-per-use)
    ApiKey,
    /// No authentication found
    None,
}

impl AuthType {
    pub fn type_string(&self) -> &'static str {
        match self {
            Self::OAuth { .. } => "oauth",
            Self::ApiKey => "api",
            Self::None => "none",
        }
    }
}

/// Task complexity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskComplexity {
    Light,
    Medium,
    Heavy,
}

impl std::fmt::Display for TaskComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Light => write!(f, "light"),
            Self::Medium => write!(f, "medium"),
            Self::Heavy => write!(f, "heavy"),
        }
    }
}

/// Usage data from Anthropic API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    pub five_hour_util: f64,
    pub seven_day_util: f64,
    #[serde(default)]
    pub five_hour_resets_at: Option<String>,
    #[serde(default)]
    pub seven_day_resets_at: Option<String>,
    pub cached_at: u64,
}

impl Default for UsageData {
    fn default() -> Self {
        Self {
            five_hour_util: 0.5, // Conservative default
            seven_day_util: 0.5,
            five_hour_resets_at: None,
            seven_day_resets_at: None,
            cached_at: 0,
        }
    }
}

impl UsageData {
    /// Check if cached data is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs();
        now - self.cached_at < CACHE_TTL_SECS
    }
}

/// Model selection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection {
    /// Full model ID (e.g., "claude-opus-4-20250514")
    pub model: String,
    /// Short model name (e.g., "opus", "sonnet")
    pub model_short: String,
    /// Subscription plan
    pub plan: String,
    /// Auth type ("oauth", "api", "none")
    pub auth_type: String,
    /// Current usage data
    pub usage: UsageData,
    /// Detected task complexity
    pub complexity: String,
    /// Human-readable selection reason
    pub selection_reason: String,
    /// Whether a manual override is active
    pub override_active: bool,
}

// ============================================================================
// Claude Credentials JSON Structure
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeCredentials {
    claude_ai_oauth: Option<ClaudeOAuth>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeOAuth {
    access_token: String,
    #[serde(default)]
    subscription_type: Option<String>,
}

// ============================================================================
// Usage API Response
// ============================================================================

#[derive(Debug, Deserialize)]
struct UsageApiResponse {
    five_hour: UsageWindow,
    seven_day: UsageWindow,
}

#[derive(Debug, Deserialize)]
struct UsageWindow {
    utilization: f64,
    #[serde(default)]
    resets_at: Option<String>,
}

// ============================================================================
// Model Selector
// ============================================================================

/// Intelligent model selector for Claude Code
pub struct ModelSelector {
    /// Cached usage data
    usage_cache: RwLock<Option<UsageData>>,
    /// Manual model override (bypasses heuristics)
    model_override: RwLock<Option<String>>,
    /// HTTP client for API calls
    client: reqwest::Client,
}

impl ModelSelector {
    /// Create a new model selector
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(API_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();

        Self {
            usage_cache: RwLock::new(None),
            model_override: RwLock::new(None),
            client,
        }
    }

    /// Get cache directory path
    fn cache_dir() -> Option<PathBuf> {
        dirs::cache_dir().map(|d| d.join("claude-model-select"))
    }

    /// Get cache file path
    fn cache_file() -> Option<PathBuf> {
        Self::cache_dir().map(|d| d.join("usage.json"))
    }

    /// Read Claude credentials from ~/.claude/credentials.json
    fn read_claude_credentials() -> Option<ClaudeCredentials> {
        let home = dirs::home_dir()?;
        let creds_path = home.join(".claude").join("credentials.json");

        if creds_path.exists() {
            let content = std::fs::read_to_string(&creds_path).ok()?;
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }

    /// Detect authentication type
    pub fn detect_auth() -> AuthType {
        // Priority 1: ANTHROPIC_API_KEY environment variable
        if std::env::var("ANTHROPIC_API_KEY").is_ok() {
            log::debug!("Model selector: ANTHROPIC_API_KEY env var detected");
            return AuthType::ApiKey;
        }

        // Priority 2: Read ~/.claude/credentials.json for OAuth
        if let Some(creds) = Self::read_claude_credentials() {
            if let Some(oauth) = creds.claude_ai_oauth {
                let plan = oauth
                    .subscription_type
                    .as_deref()
                    .map(SubscriptionPlan::from)
                    .unwrap_or(SubscriptionPlan::Unknown);

                log::debug!("Model selector: OAuth credentials found, plan: {:?}", plan);
                return AuthType::OAuth {
                    plan,
                    token: oauth.access_token,
                };
            }
        }

        log::debug!("Model selector: No authentication found");
        AuthType::None
    }

    /// Load cached usage data from disk
    async fn load_cached_usage(&self) -> Option<UsageData> {
        // Check in-memory cache first
        {
            let cache = self.usage_cache.read().await;
            if let Some(ref data) = *cache {
                if data.is_valid() {
                    return Some(data.clone());
                }
            }
        }

        // Try disk cache
        if let Some(cache_path) = Self::cache_file() {
            if cache_path.exists() {
                if let Ok(content) = tokio::fs::read_to_string(&cache_path).await {
                    if let Ok(data) = serde_json::from_str::<UsageData>(&content) {
                        if data.is_valid() {
                            // Update in-memory cache
                            let mut cache = self.usage_cache.write().await;
                            *cache = Some(data.clone());
                            return Some(data);
                        }
                    }
                }
            }
        }

        None
    }

    /// Save usage data to cache
    async fn save_usage_cache(&self, data: &UsageData) {
        // Update in-memory cache
        {
            let mut cache = self.usage_cache.write().await;
            *cache = Some(data.clone());
        }

        // Write to disk
        if let Some(cache_dir) = Self::cache_dir() {
            let _ = tokio::fs::create_dir_all(&cache_dir).await;
            if let Some(cache_path) = Self::cache_file() {
                if let Ok(content) = serde_json::to_string_pretty(data) {
                    let _ = tokio::fs::write(cache_path, content).await;
                }
            }
        }
    }

    /// Fetch usage data from Anthropic API
    pub async fn fetch_usage(&self, token: &str) -> Result<UsageData, String> {
        // Check cache first
        if let Some(cached) = self.load_cached_usage().await {
            log::debug!("Model selector: Using cached usage data");
            return Ok(cached);
        }

        // Fetch from API
        log::debug!("Model selector: Fetching usage from API");
        let response = self
            .client
            .get(USAGE_API_URL)
            .header("Authorization", format!("Bearer {}", token))
            .header("anthropic-beta", "oauth-2025-04-20")
            .header("User-Agent", "claude-code/2.0.32")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch usage: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            log::warn!("Model selector: Usage API returned {}", status);

            // Fall back to stale cache or defaults
            if let Some(cache_path) = Self::cache_file() {
                if cache_path.exists() {
                    if let Ok(content) = tokio::fs::read_to_string(&cache_path).await {
                        if let Ok(data) = serde_json::from_str::<UsageData>(&content) {
                            log::warn!("Model selector: Using stale cache due to API error");
                            return Ok(data);
                        }
                    }
                }
            }

            return Ok(UsageData::default());
        }

        let api_data: UsageApiResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse usage response: {}", e))?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs();

        let data = UsageData {
            five_hour_util: api_data.five_hour.utilization,
            seven_day_util: api_data.seven_day.utilization,
            five_hour_resets_at: api_data.five_hour.resets_at,
            seven_day_resets_at: api_data.seven_day.resets_at,
            cached_at: now,
        };

        // Cache the result
        self.save_usage_cache(&data).await;

        Ok(data)
    }

    /// Detect task complexity from prompt
    pub fn detect_complexity(prompt: &str) -> TaskComplexity {
        let word_count = prompt.split_whitespace().count();
        let prompt_lower = prompt.to_lowercase();

        // Heavy indicators - complex operations
        let heavy_keywords = [
            "refactor",
            "implement",
            "rewrite",
            "migrate",
            "architect",
            "redesign",
            "entire",
            "comprehensive",
            "full rewrite",
            "major changes",
            "from scratch",
            "overhaul",
        ];
        let has_heavy = heavy_keywords.iter().any(|k| prompt_lower.contains(k));

        // Light indicators - simple operations
        let light_keywords = [
            "fix typo",
            "rename",
            "update comment",
            "simple",
            "quick",
            "small change",
            "one line",
            "minor",
            "trivial",
            "just change",
        ];
        let has_light = light_keywords.iter().any(|k| prompt_lower.contains(k));

        if has_heavy || word_count > 500 {
            TaskComplexity::Heavy
        } else if has_light || word_count < 10 {
            TaskComplexity::Light
        } else {
            TaskComplexity::Medium
        }
    }

    /// Select model based on plan, usage, and complexity
    pub fn select_model(
        plan: SubscriptionPlan,
        usage: &UsageData,
        complexity: TaskComplexity,
    ) -> (&'static str, &'static str, String) {
        // Thresholds based on complexity
        let (opus_5h, opus_7d) = match complexity {
            TaskComplexity::Light => (0.70, 0.80),
            TaskComplexity::Medium => (0.55, 0.65),
            TaskComplexity::Heavy => (0.40, 0.50),
        };

        let (model_short, model_id, reason) = match plan {
            SubscriptionPlan::Max20 => {
                // Max20 ($200/mo) - 20x Pro limits, generous Opus access
                if usage.five_hour_util < 0.85 {
                    (
                        "opus",
                        OPUS_MODEL_ID,
                        format!(
                            "Max20 plan with {}% 5h usage - plenty of headroom for Opus",
                            (usage.five_hour_util * 100.0) as u32
                        ),
                    )
                } else {
                    (
                        "sonnet",
                        SONNET_MODEL_ID,
                        format!(
                            "Max20 plan but 5h usage at {}% - conserving with Sonnet",
                            (usage.five_hour_util * 100.0) as u32
                        ),
                    )
                }
            }

            SubscriptionPlan::Max => {
                // Max5 ($100/mo) - 5x Pro limits, moderate Opus access
                if usage.five_hour_util < opus_5h && usage.seven_day_util < opus_7d {
                    (
                        "opus",
                        OPUS_MODEL_ID,
                        format!(
                            "Max plan with {}% 5h / {}% 7d usage - using Opus",
                            (usage.five_hour_util * 100.0) as u32,
                            (usage.seven_day_util * 100.0) as u32
                        ),
                    )
                } else {
                    (
                        "sonnet",
                        SONNET_MODEL_ID,
                        format!(
                            "Max plan with {}% 5h / {}% 7d usage - conserving with Sonnet",
                            (usage.five_hour_util * 100.0) as u32,
                            (usage.seven_day_util * 100.0) as u32
                        ),
                    )
                }
            }

            SubscriptionPlan::Pro => {
                // Pro ($20/mo) - Conservative with Opus, Sonnet-first
                if usage.five_hour_util < 0.25 && usage.seven_day_util < 0.35 {
                    (
                        "opus",
                        OPUS_MODEL_ID,
                        format!(
                            "Pro plan with low usage ({}% 5h) - treating yourself to Opus",
                            (usage.five_hour_util * 100.0) as u32
                        ),
                    )
                } else {
                    (
                        "sonnet",
                        SONNET_MODEL_ID,
                        format!(
                            "Pro plan at {}% 5h usage - Sonnet for efficiency",
                            (usage.five_hour_util * 100.0) as u32
                        ),
                    )
                }
            }

            SubscriptionPlan::Api => {
                // API - Cost-based decision
                if matches!(complexity, TaskComplexity::Heavy) {
                    (
                        "opus",
                        OPUS_MODEL_ID,
                        "API key with heavy task - using Opus for quality".to_string(),
                    )
                } else {
                    (
                        "sonnet",
                        SONNET_MODEL_ID,
                        "API key - Sonnet for cost efficiency".to_string(),
                    )
                }
            }

            SubscriptionPlan::Team | SubscriptionPlan::Enterprise => {
                // Team/Enterprise - Similar to Max
                if usage.five_hour_util < opus_5h {
                    (
                        "opus",
                        OPUS_MODEL_ID,
                        format!(
                            "{:?} plan with {}% usage - using Opus",
                            plan,
                            (usage.five_hour_util * 100.0) as u32
                        ),
                    )
                } else {
                    (
                        "sonnet",
                        SONNET_MODEL_ID,
                        format!(
                            "{:?} plan at {}% usage - conserving with Sonnet",
                            plan,
                            (usage.five_hour_util * 100.0) as u32
                        ),
                    )
                }
            }

            SubscriptionPlan::Free | SubscriptionPlan::Unknown => {
                // Free tier or unknown - Sonnet only
                (
                    "sonnet",
                    SONNET_MODEL_ID,
                    "Free/unknown plan - using Sonnet".to_string(),
                )
            }
        };

        (model_short, model_id, reason)
    }

    /// Set a manual model override
    pub async fn set_override(&self, model: Option<String>) {
        let mut override_guard = self.model_override.write().await;
        *override_guard = model;
    }

    /// Get current model override
    pub async fn get_override(&self) -> Option<String> {
        let override_guard = self.model_override.read().await;
        override_guard.clone()
    }

    /// Get model selection with default complexity
    pub async fn get_selection(&self, complexity: TaskComplexity) -> Result<ModelSelection, String> {
        // Check for manual override first
        if let Some(override_model) = self.get_override().await {
            let model_short = if override_model.contains("opus") {
                "opus"
            } else {
                "sonnet"
            };

            return Ok(ModelSelection {
                model: override_model.clone(),
                model_short: model_short.to_string(),
                plan: "n/a".to_string(),
                auth_type: "override".to_string(),
                usage: UsageData::default(),
                complexity: complexity.to_string(),
                selection_reason: format!("Manual override: {}", override_model),
                override_active: true,
            });
        }

        // Detect auth type
        let auth = Self::detect_auth();

        let (plan, usage) = match &auth {
            AuthType::OAuth { plan, token } => {
                let usage = self.fetch_usage(token).await.unwrap_or_default();
                (*plan, usage)
            }
            AuthType::ApiKey => (SubscriptionPlan::Api, UsageData::default()),
            AuthType::None => (SubscriptionPlan::Unknown, UsageData::default()),
        };

        let (model_short, model_id, reason) = Self::select_model(plan, &usage, complexity);

        Ok(ModelSelection {
            model: model_id.to_string(),
            model_short: model_short.to_string(),
            plan: plan.to_string(),
            auth_type: auth.type_string().to_string(),
            usage,
            complexity: complexity.to_string(),
            selection_reason: reason,
            override_active: false,
        })
    }

    /// Get model selection with complexity auto-detected from prompt
    pub async fn get_selection_for_prompt(&self, prompt: &str) -> Result<ModelSelection, String> {
        let complexity = Self::detect_complexity(prompt);
        self.get_selection(complexity).await
    }

    /// Synchronous version for contexts where async isn't available
    /// Returns just the model ID with default complexity
    /// Note: Checks override via try_read (non-blocking)
    pub fn select_model_sync(&self) -> String {
        // Check for manual override first (non-blocking)
        if let Ok(guard) = self.model_override.try_read() {
            if let Some(ref override_model) = *guard {
                return override_model.clone();
            }
        }

        let auth = Self::detect_auth();

        let (plan, usage) = match auth {
            AuthType::OAuth { plan, .. } => (plan, UsageData::default()),
            AuthType::ApiKey => (SubscriptionPlan::Api, UsageData::default()),
            AuthType::None => (SubscriptionPlan::Unknown, UsageData::default()),
        };

        let (_, model_id, _) = Self::select_model(plan, &usage, TaskComplexity::Medium);
        model_id.to_string()
    }
}

impl Default for ModelSelector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Global Instance
// ============================================================================

use std::sync::OnceLock;

static MODEL_SELECTOR: OnceLock<ModelSelector> = OnceLock::new();

/// Get the global model selector instance
pub fn model_selector() -> &'static ModelSelector {
    MODEL_SELECTOR.get_or_init(ModelSelector::new)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_plan_parsing() {
        assert_eq!(SubscriptionPlan::from("pro"), SubscriptionPlan::Pro);
        assert_eq!(SubscriptionPlan::from("max"), SubscriptionPlan::Max);
        assert_eq!(SubscriptionPlan::from("max_5"), SubscriptionPlan::Max);
        assert_eq!(SubscriptionPlan::from("max_20"), SubscriptionPlan::Max20);
        assert_eq!(SubscriptionPlan::from("free"), SubscriptionPlan::Free);
        assert_eq!(SubscriptionPlan::from("unknown"), SubscriptionPlan::Unknown);
    }

    #[test]
    fn test_complexity_detection() {
        // Light tasks
        assert_eq!(
            ModelSelector::detect_complexity("fix typo in readme"),
            TaskComplexity::Light
        );
        assert_eq!(
            ModelSelector::detect_complexity("rename variable x to y"),
            TaskComplexity::Light
        );

        // Heavy tasks
        assert_eq!(
            ModelSelector::detect_complexity("refactor the entire authentication system"),
            TaskComplexity::Heavy
        );
        assert_eq!(
            ModelSelector::detect_complexity("implement a new feature from scratch"),
            TaskComplexity::Heavy
        );

        // Medium (default)
        assert_eq!(
            ModelSelector::detect_complexity("add a button to the settings page that opens a modal"),
            TaskComplexity::Medium
        );
    }

    #[test]
    fn test_model_selection_max20() {
        let usage = UsageData {
            five_hour_util: 0.3,
            seven_day_util: 0.2,
            ..Default::default()
        };

        let (model, _, _) =
            ModelSelector::select_model(SubscriptionPlan::Max20, &usage, TaskComplexity::Medium);
        assert_eq!(model, "opus");

        let high_usage = UsageData {
            five_hour_util: 0.9,
            seven_day_util: 0.5,
            ..Default::default()
        };

        let (model, _, _) =
            ModelSelector::select_model(SubscriptionPlan::Max20, &high_usage, TaskComplexity::Medium);
        assert_eq!(model, "sonnet");
    }

    #[test]
    fn test_model_selection_pro() {
        // Pro is conservative - only Opus at very low usage
        let low_usage = UsageData {
            five_hour_util: 0.1,
            seven_day_util: 0.1,
            ..Default::default()
        };

        let (model, _, _) =
            ModelSelector::select_model(SubscriptionPlan::Pro, &low_usage, TaskComplexity::Medium);
        assert_eq!(model, "opus");

        let medium_usage = UsageData {
            five_hour_util: 0.4,
            seven_day_util: 0.3,
            ..Default::default()
        };

        let (model, _, _) =
            ModelSelector::select_model(SubscriptionPlan::Pro, &medium_usage, TaskComplexity::Medium);
        assert_eq!(model, "sonnet");
    }

    #[test]
    fn test_model_selection_api() {
        let usage = UsageData::default();

        // API with light task = Sonnet
        let (model, _, _) =
            ModelSelector::select_model(SubscriptionPlan::Api, &usage, TaskComplexity::Light);
        assert_eq!(model, "sonnet");

        // API with heavy task = Opus
        let (model, _, _) =
            ModelSelector::select_model(SubscriptionPlan::Api, &usage, TaskComplexity::Heavy);
        assert_eq!(model, "opus");
    }

    #[test]
    fn test_model_selection_free() {
        // Free tier always gets Sonnet
        let usage = UsageData {
            five_hour_util: 0.0,
            seven_day_util: 0.0,
            ..Default::default()
        };

        let (model, _, _) =
            ModelSelector::select_model(SubscriptionPlan::Free, &usage, TaskComplexity::Heavy);
        assert_eq!(model, "sonnet");
    }

    #[test]
    fn test_usage_data_validity() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs();

        let valid = UsageData {
            cached_at: now,
            ..Default::default()
        };
        assert!(valid.is_valid());

        let expired = UsageData {
            cached_at: now - 600, // 10 minutes ago
            ..Default::default()
        };
        assert!(!expired.is_valid());
    }
}
