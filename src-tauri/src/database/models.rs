//! Database Models
//!
//! SQLite record types for structured data storage.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Campaign database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CampaignRecord {
    pub id: String,
    pub name: String,
    pub system: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Session database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionRecord {
    pub id: String,
    pub campaign_id: String,
    pub session_number: i32,
    pub title: Option<String>,
    pub status: String, // "active", "completed", "paused", "planned"
    pub started_at: String,
    pub ended_at: Option<String>,
    pub notes: Option<String>,
}

/// Character database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CharacterRecord {
    pub id: String,
    pub campaign_id: Option<String>,
    pub name: String,
    pub system: String,
    pub character_type: String, // "player", "npc", "monster"
    pub level: Option<i32>,
    pub data_json: String, // Full character data as JSON
    pub created_at: String,
    pub updated_at: String,
}

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

/// Document/source record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DocumentRecord {
    pub id: String,
    pub name: String,
    pub source_type: String, // "pdf", "epub", "markdown"
    pub file_path: Option<String>,
    pub page_count: i32,
    pub chunk_count: i32,
    pub status: String, // "pending", "processing", "ready", "error"
    pub ingested_at: String,
}

/// Campaign snapshot record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SnapshotRecord {
    pub id: String,
    pub campaign_id: String,
    pub description: String,
    pub snapshot_type: String, // "manual", "auto", "milestone"
    pub data_json: String,
    pub created_at: String,
}

/// Combat encounter record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CombatRecord {
    pub id: String,
    pub session_id: String,
    pub round: i32,
    pub current_turn: i32,
    pub is_active: bool,
    pub combatants_json: String,
    pub started_at: String,
    pub ended_at: Option<String>,
}

/// NPC database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NpcRecord {
    pub id: String,
    pub campaign_id: Option<String>,
    pub name: String,
    pub role: String,
    pub personality_id: Option<String>,
    pub personality_json: String,
    pub data_json: Option<String>,
    pub stats_json: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

impl CampaignRecord {
    pub fn new(id: String, name: String, system: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            name,
            system,
            description: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl SessionRecord {
    pub fn new(id: String, campaign_id: String, session_number: i32) -> Self {
        Self {
            id,
            campaign_id,
            session_number,
            title: None,
            status: "active".to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            ended_at: None,
            notes: None,
        }
    }
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

/// NPC Conversation record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NpcConversation {
    pub id: String,
    pub npc_id: String,
    pub campaign_id: String,
    pub messages_json: String, // Vec<ConversationMessage>
    pub unread_count: u32,
    pub last_message_at: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Message within an NPC conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: String, // "user" or "npc"
    pub content: String,
    pub parent_message_id: Option<String>, // For threading (B3)
    pub created_at: String,
}

impl NpcConversation {
    pub fn new(id: String, npc_id: String, campaign_id: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            npc_id,
            campaign_id,
            messages_json: "[]".to_string(),
            unread_count: 0,
            last_message_at: now.clone(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}


/// Personality database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PersonalityRecord {
    pub id: String,
    pub name: String,
    pub source: Option<String>,
    pub data_json: String,
    pub created_at: String,
    pub updated_at: String,
}
