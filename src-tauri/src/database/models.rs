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
    pub setting: Option<String>,
    pub current_in_game_date: Option<String>,
    pub house_rules: Option<String>,   // JSON
    pub world_state: Option<String>,   // JSON
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
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
    #[serde(default)]
    pub order_index: i32,
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
    pub location_id: Option<String>,
    pub voice_profile_id: Option<String>,
    pub quest_hooks: Option<String>,  // JSON array
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
            setting: None,
            current_in_game_date: None,
            house_rules: None,
            world_state: None,
            created_at: now.clone(),
            updated_at: now,
            archived_at: None,
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
            order_index: 0,
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

// ============================================================================
// New Model Structs for Extended Database Schema
// ============================================================================

/// Campaign version record for versioning/rollback support
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CampaignVersionRecord {
    pub id: String,
    pub campaign_id: String,
    pub version_number: i32,
    pub snapshot_type: String,  // "manual", "auto_save", "pre_edit", "session_start", "session_end"
    pub description: Option<String>,
    pub data: String,           // JSON snapshot of campaign state
    pub diff_data: Option<String>, // JSON diff from previous version
    pub created_at: String,
}

impl CampaignVersionRecord {
    pub fn new(
        id: String,
        campaign_id: String,
        version_number: i32,
        snapshot_type: String,
        data: String,
    ) -> Self {
        Self {
            id,
            campaign_id,
            version_number,
            snapshot_type,
            description: None,
            data,
            diff_data: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Entity type enum for relationship tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Npc,
    Character,
    Location,
    Quest,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Npc => "npc",
            EntityType::Character => "character",
            EntityType::Location => "location",
            EntityType::Quest => "quest",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "npc" => Some(EntityType::Npc),
            "character" => Some(EntityType::Character),
            "location" => Some(EntityType::Location),
            "quest" => Some(EntityType::Quest),
            _ => None,
        }
    }
}

/// Entity relationship record for tracking relationships between campaign entities
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EntityRelationshipRecord {
    pub id: String,
    pub campaign_id: String,
    pub source_entity_type: String,
    pub source_entity_id: String,
    pub target_entity_type: String,
    pub target_entity_id: String,
    pub relationship_type: String,  // "ally", "enemy", "family", "employee", "located_at", etc.
    pub description: Option<String>,
    pub strength: f64,              // 0.0 to 1.0
    pub bidirectional: bool,
    pub metadata: Option<String>,   // JSON for additional properties
    pub created_at: String,
    pub updated_at: String,
}

impl EntityRelationshipRecord {
    pub fn new(
        id: String,
        campaign_id: String,
        source_entity_type: EntityType,
        source_entity_id: String,
        target_entity_type: EntityType,
        target_entity_id: String,
        relationship_type: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            campaign_id,
            source_entity_type: source_entity_type.as_str().to_string(),
            source_entity_id,
            target_entity_type: target_entity_type.as_str().to_string(),
            target_entity_id,
            relationship_type,
            description: None,
            strength: 1.0,
            bidirectional: false,
            metadata: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

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

/// Session note record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionNoteRecord {
    pub id: String,
    pub session_id: String,
    pub campaign_id: String,
    pub content: String,
    pub tags: Option<String>,        // JSON array
    pub entity_links: Option<String>, // JSON array of {type, id}
    pub created_at: String,
    pub updated_at: String,
}

impl SessionNoteRecord {
    pub fn new(
        id: String,
        session_id: String,
        campaign_id: String,
        content: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            session_id,
            campaign_id,
            content,
            tags: None,
            entity_links: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

/// Session event record for timeline tracking
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionEventRecord {
    pub id: String,
    pub session_id: String,
    pub timestamp: String,
    pub event_type: String,  // "combat_start", "combat_end", "npc_interaction", "location_change", etc.
    pub description: Option<String>,
    pub entities: Option<String>,  // JSON array of entity references
    pub metadata: Option<String>,  // JSON for event-specific data
    pub created_at: String,
}

impl SessionEventRecord {
    pub fn new(
        id: String,
        session_id: String,
        event_type: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            session_id,
            timestamp: now.clone(),
            event_type,
            description: None,
            entities: None,
            metadata: None,
            created_at: now,
        }
    }
}

/// Combat state record for tracking active and historical combat encounters
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CombatStateRecord {
    pub id: String,
    pub session_id: String,
    pub name: Option<String>,
    pub round: i32,
    pub current_turn: i32,
    pub is_active: bool,
    pub combatants: String,    // JSON array of combatant data
    pub conditions: Option<String>,  // JSON array of active conditions
    pub environment: Option<String>, // JSON for environmental effects
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub ended_at: Option<String>,
}

impl CombatStateRecord {
    pub fn new(
        id: String,
        session_id: String,
        combatants: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            session_id,
            name: None,
            round: 1,
            current_turn: 0,
            is_active: true,
            combatants,
            conditions: None,
            environment: None,
            notes: None,
            created_at: now.clone(),
            updated_at: now,
            ended_at: None,
        }
    }
}

/// Location record (already exists in migrations v2, adding model struct)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LocationRecord {
    pub id: String,
    pub campaign_id: String,
    pub name: String,
    pub location_type: String,
    pub description: Option<String>,
    pub parent_id: Option<String>,
    pub connections_json: String,    // JSON array of connected location IDs
    pub npcs_present_json: String,   // JSON array of NPC IDs
    pub features_json: String,       // JSON array of notable features
    pub secrets_json: String,        // JSON array of hidden information
    pub attributes_json: String,     // JSON object for additional attributes
    pub tags_json: String,           // JSON array of tags
    pub created_at: String,
    pub updated_at: String,
}

impl LocationRecord {
    pub fn new(
        id: String,
        campaign_id: String,
        name: String,
        location_type: String,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            campaign_id,
            name,
            location_type,
            description: None,
            parent_id: None,
            connections_json: "[]".to_string(),
            npcs_present_json: "[]".to_string(),
            features_json: "[]".to_string(),
            secrets_json: "[]".to_string(),
            attributes_json: "{}".to_string(),
            tags_json: "[]".to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

// ============================================================================
// Search Analytics Records (TASK-023)
// ============================================================================

/// Search analytics record for tracking individual searches
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SearchAnalyticsRecord {
    pub id: String,
    pub query: String,
    pub results_count: i32,
    pub selected_result_id: Option<String>,
    pub selected_result_index: Option<i32>,
    pub response_time_ms: i32,
    pub cache_hit: bool,
    pub search_type: String,
    pub source_filter: Option<String>,
    pub campaign_id: Option<String>,
    pub created_at: String,
}

impl SearchAnalyticsRecord {
    pub fn new(
        query: String,
        results_count: i32,
        response_time_ms: i32,
        search_type: String,
        cache_hit: bool,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            query,
            results_count,
            selected_result_id: None,
            selected_result_index: None,
            response_time_ms,
            cache_hit,
            search_type,
            source_filter: None,
            campaign_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Search selection record for tracking which results users click
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SearchSelectionRecord {
    pub id: String,
    pub search_id: String,
    pub query: String,
    pub result_index: i32,
    pub source: String,
    pub was_helpful: Option<bool>,
    pub selection_delay_ms: i64,
    pub created_at: String,
}

impl SearchSelectionRecord {
    pub fn new(
        search_id: String,
        query: String,
        result_index: i32,
        source: String,
        selection_delay_ms: i64,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            search_id,
            query,
            result_index,
            source,
            was_helpful: None,
            selection_delay_ms,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

/// Aggregated query statistics record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SearchQueryStatsRecord {
    pub query_normalized: String,
    pub total_count: i32,
    pub total_clicks: i32,
    pub avg_results: f64,
    pub avg_time_ms: f64,
    pub last_searched_at: String,
    pub click_positions_json: String,
}

impl SearchQueryStatsRecord {
    pub fn new(query_normalized: String) -> Self {
        Self {
            query_normalized,
            total_count: 0,
            total_clicks: 0,
            avg_results: 0.0,
            avg_time_ms: 0.0,
            last_searched_at: chrono::Utc::now().to_rfc3339(),
            click_positions_json: "{}".to_string(),
        }
    }
}
