//! Core Entity Records
//!
//! Basic database records for campaigns, sessions, characters, documents,
//! locations, and entity relationships.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Campaign Record
// ============================================================================

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

// ============================================================================
// Session Record
// ============================================================================

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
            order_index: session_number, // Initialize from session_number for sensible ordering
        }
    }
}

// ============================================================================
// Character Record
// ============================================================================

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

// ============================================================================
// Document Record
// ============================================================================

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

// ============================================================================
// Snapshot Record
// ============================================================================

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

// ============================================================================
// Campaign Version Record
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

// ============================================================================
// Entity Type and Relationships
// ============================================================================

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

// ============================================================================
// Personality Record
// ============================================================================

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
// Session Note Record
// ============================================================================

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

// ============================================================================
// Session Event Record
// ============================================================================

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

// ============================================================================
// Location Record
// ============================================================================

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
// NPC Conversation Records
// ============================================================================

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
