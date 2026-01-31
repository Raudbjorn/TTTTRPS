//! Chat Models
//!
//! Database records for chat sessions, messages, usage tracking, and voice profiles.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Type-Safe Enums for Chat Session Status and Message Role
// ============================================================================

/// Chat session status (type-safe alternative to raw strings)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatSessionStatus {
    Active,
    Archived,
}

impl ChatSessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChatSessionStatus::Active => "active",
            ChatSessionStatus::Archived => "archived",
        }
    }
}

impl std::fmt::Display for ChatSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for ChatSessionStatus {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "active" => Ok(ChatSessionStatus::Active),
            "archived" => Ok(ChatSessionStatus::Archived),
            _ => Err(format!("Unknown chat session status: {}", s)),
        }
    }
}

/// Chat message role (type-safe alternative to raw strings)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    Error,
    System,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Error => "error",
            MessageRole::System => "system",
        }
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for MessageRole {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, <Self as TryFrom<&str>>::Error> {
        match s {
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "error" => Ok(MessageRole::Error),
            "system" => Ok(MessageRole::System),
            _ => Err(format!("Unknown message role: {}", s)),
        }
    }
}

// ============================================================================
// Usage Tracking Records
// ============================================================================

/// Usage tracking record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageRecord {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub estimated_cost_usd: f64,
    pub timestamp: String,
}

impl UsageRecord {
    pub fn new(provider: String, model: String, input_tokens: u32, output_tokens: u32) -> Self {
        // Rough cost estimation (per 1M tokens)
        let cost = estimate_cost(&provider, &model, input_tokens, output_tokens);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            provider,
            model,
            input_tokens,
            output_tokens,
            estimated_cost_usd: cost,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Estimate API cost based on provider and model
fn estimate_cost(provider: &str, model: &str, input_tokens: u32, output_tokens: u32) -> f64 {
    // Prices per 1M tokens (approximate, as of late 2024)
    let (input_price, output_price) = match provider.to_lowercase().as_str() {
        "claude" | "anthropic" => {
            match model {
                m if m.contains("opus") => (15.0, 75.0),
                m if m.contains("sonnet") => (3.0, 15.0),
                m if m.contains("haiku") => (0.25, 1.25),
                _ => (3.0, 15.0), // Default to Sonnet pricing
            }
        }
        "gemini" | "google" => {
            match model {
                m if m.contains("pro") => (1.25, 5.0),
                m if m.contains("flash") => (0.075, 0.30),
                _ => (1.25, 5.0),
            }
        }
        "openai" | "gpt" => {
            match model {
                m if m.contains("gpt-4o") => (2.5, 10.0),
                m if m.contains("gpt-4") => (30.0, 60.0),
                m if m.contains("gpt-3.5") => (0.5, 1.5),
                _ => (2.5, 10.0),
            }
        }
        "ollama" | "local" => (0.0, 0.0), // Local models are free
        _ => (1.0, 3.0), // Conservative default
    };

    let input_cost = (input_tokens as f64 / 1_000_000.0) * input_price;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * output_price;

    input_cost + output_cost
}

/// Aggregated usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_requests: u32,
    pub estimated_cost_usd: f64,
}

/// Per-provider usage statistics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProviderUsageStats {
    pub provider: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub requests: i64,
    pub estimated_cost_usd: f64,
}

// ============================================================================
// Global Chat Session Records
// ============================================================================

/// Global chat session record - persists across navigation
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GlobalChatSessionRecord {
    pub id: String,
    pub status: String,  // Use ChatSessionStatus::as_str() for type-safe values
    pub linked_game_session_id: Option<String>,
    pub linked_campaign_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl GlobalChatSessionRecord {
    pub fn new() -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            status: ChatSessionStatus::Active.to_string(),
            linked_game_session_id: None,
            linked_campaign_id: None,
            // Assign updated_at first with clone, then created_at takes ownership
            updated_at: now.clone(),
            created_at: now,
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == ChatSessionStatus::Active.as_str()
    }

    /// Get status as type-safe enum
    pub fn status_enum(&self) -> Result<ChatSessionStatus, String> {
        ChatSessionStatus::try_from(self.status.as_str())
    }
}

impl Default for GlobalChatSessionRecord {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Chat Message Record
// ============================================================================

/// Chat message record for persistent storage
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatMessageRecord {
    pub id: String,
    pub session_id: String,
    pub role: String,  // Use MessageRole::as_str() for type-safe values
    pub content: String,
    pub tokens_input: Option<i32>,
    pub tokens_output: Option<i32>,
    pub is_streaming: i32,  // SQLite doesn't have bool, use 0/1
    pub metadata: Option<String>,  // JSON for extensibility
    pub created_at: String,
}

impl ChatMessageRecord {
    pub fn new(session_id: String, role: String, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            role,
            content,
            tokens_input: None,
            tokens_output: None,
            is_streaming: 0,
            metadata: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create with type-safe role enum
    pub fn with_role(session_id: String, role: MessageRole, content: String) -> Self {
        Self::new(session_id, role.to_string(), content)
    }

    pub fn with_tokens(mut self, input: i32, output: i32) -> Self {
        self.tokens_input = Some(input);
        self.tokens_output = Some(output);
        self
    }

    pub fn with_metadata(mut self, metadata: &str) -> Self {
        self.metadata = Some(metadata.to_string());
        self
    }

    pub fn streaming(mut self) -> Self {
        self.is_streaming = 1;
        self
    }

    /// Get role as type-safe enum
    pub fn role_enum(&self) -> Result<MessageRole, String> {
        MessageRole::try_from(self.role.as_str())
    }
}

// ============================================================================
// Voice Profile Record
// ============================================================================

/// Voice profile record for NPC voice synthesis
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VoiceProfileRecord {
    pub id: String,
    pub name: String,
    pub provider: String,       // "elevenlabs", "azure", "google", etc.
    pub voice_id: String,       // Provider-specific voice ID
    pub settings: Option<String>, // JSON with provider-specific settings
    pub age_range: Option<String>, // "child", "adult", "elderly"
    pub gender: Option<String>,    // "male", "female", "neutral"
    pub personality_traits: Option<String>, // JSON array of traits
    pub is_preset: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl VoiceProfileRecord {
    pub fn new(
        id: String,
        name: String,
        provider: String,
        voice_id: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name,
            provider,
            voice_id,
            settings: None,
            age_range: None,
            gender: None,
            personality_traits: None,
            is_preset: false,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}
