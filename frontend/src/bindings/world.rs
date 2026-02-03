use super::core::{invoke, invoke_no_args, invoke_void};
use super::mechanics::Character;
use serde::{Deserialize, Serialize};
use serde_json::json;

// ============================================================================
// World State Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InGameDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub era: Option<String>,
    pub calendar: String,
    pub time: Option<InGameTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InGameTime {
    pub hour: u8,
    pub minute: u8,
    pub period: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub campaign_id: String,
    pub current_date: InGameDate,
    pub events: Vec<WorldEvent>,
    pub locations: std::collections::HashMap<String, LocationState>,
    pub npc_relationships: Vec<NpcRelationshipState>,
    pub custom_fields: std::collections::HashMap<String, serde_json::Value>,
    pub updated_at: String,
    pub calendar_config: CalendarConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEvent {
    pub id: String,
    pub campaign_id: String,
    pub in_game_date: InGameDate,
    pub recorded_at: String,
    pub title: String,
    pub description: String,
    pub event_type: String,
    pub impact: String,
    pub location_ids: Vec<String>,
    pub npc_ids: Vec<String>,
    pub pc_ids: Vec<String>,
    pub consequences: Vec<String>,
    pub session_number: Option<u32>,
    pub is_public: bool,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationState {
    pub location_id: String,
    pub name: String,
    pub condition: String,
    pub ruler: Option<String>,
    pub controlling_faction: Option<String>,
    pub population: Option<u64>,
    pub notable_npcs: Vec<String>,
    pub active_effects: Vec<String>,
    pub resources: std::collections::HashMap<String, i32>,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    pub updated_at: String,
    pub last_accurate_date: InGameDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcRelationshipState {
    pub npc_id: String,
    pub target_id: String,
    pub target_type: String,
    pub disposition: i32,
    pub relationship_type: String,
    pub familiarity: u8,
    pub recent_interactions: Vec<InteractionRecord>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub in_game_date: InGameDate,
    pub description: String,
    pub disposition_change: i32,
    pub session_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarConfig {
    pub name: String,
    pub months_per_year: u8,
    pub days_per_month: Vec<u8>,
    pub month_names: Vec<String>,
    pub week_days: Vec<String>,
    pub eras: Vec<String>,
}

// ============================================================================
// World State Commands
// ============================================================================

pub async fn get_world_state(campaign_id: String) -> Result<WorldState, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("get_world_state", &Args { campaign_id }).await
}

pub async fn update_world_state(world_state: WorldState) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        world_state: WorldState,
    }
    invoke_void("update_world_state", &Args { world_state }).await
}

pub async fn set_in_game_date(campaign_id: String, date: InGameDate) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        date: InGameDate,
    }
    invoke_void("set_in_game_date", &Args { campaign_id, date }).await
}

pub async fn advance_in_game_date(campaign_id: String, days: i32) -> Result<InGameDate, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        days: i32,
    }
    invoke("advance_in_game_date", &Args { campaign_id, days }).await
}

pub async fn get_in_game_date(campaign_id: String) -> Result<InGameDate, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("get_in_game_date", &Args { campaign_id }).await
}

pub async fn add_world_event(
    campaign_id: String,
    title: String,
    description: String,
    date: InGameDate,
    event_type: String,
    impact: String,
) -> Result<WorldEvent, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        title: String,
        description: String,
        date: InGameDate,
        event_type: String,
        impact: String,
    }
    invoke(
        "add_world_event",
        &Args {
            campaign_id,
            title,
            description,
            date,
            event_type,
            impact,
        },
    )
    .await
}

pub async fn list_world_events(
    campaign_id: String,
    event_type: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<WorldEvent>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        event_type: Option<String>,
        limit: Option<usize>,
    }
    invoke(
        "list_world_events",
        &Args {
            campaign_id,
            event_type,
            limit,
        },
    )
    .await
}

pub async fn delete_world_event(campaign_id: String, event_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        event_id: String,
    }
    invoke_void(
        "delete_world_event",
        &Args {
            campaign_id,
            event_id,
        },
    )
    .await
}

pub async fn set_location_state(
    campaign_id: String,
    location: LocationState,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        location: LocationState,
    }
    invoke_void(
        "set_location_state",
        &Args {
            campaign_id,
            location,
        },
    )
    .await
}

pub async fn get_location_state(
    campaign_id: String,
    location_id: String,
) -> Result<Option<LocationState>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        location_id: String,
    }
    invoke(
        "get_location_state",
        &Args {
            campaign_id,
            location_id,
        },
    )
    .await
}

pub async fn list_locations(campaign_id: String) -> Result<Vec<LocationState>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_locations", &Args { campaign_id }).await
}

pub async fn update_location_condition(
    campaign_id: String,
    location_id: String,
    condition: String,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        location_id: String,
        condition: String,
    }
    invoke_void(
        "update_location_condition",
        &Args {
            campaign_id,
            location_id,
            condition,
        },
    )
    .await
}

pub async fn set_world_custom_field(
    campaign_id: String,
    key: String,
    value: serde_json::Value,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        key: String,
        value: serde_json::Value,
    }
    invoke_void(
        "set_world_custom_field",
        &Args {
            campaign_id,
            key,
            value,
        },
    )
    .await
}

pub async fn get_world_custom_field(
    campaign_id: String,
    key: String,
) -> Result<Option<serde_json::Value>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        key: String,
    }
    invoke("get_world_custom_field", &Args { campaign_id, key }).await
}

pub async fn list_world_custom_fields(
    campaign_id: String,
) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_world_custom_fields", &Args { campaign_id }).await
}

pub async fn set_calendar_config(
    campaign_id: String,
    config: CalendarConfig,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        config: CalendarConfig,
    }
    invoke_void(
        "set_calendar_config",
        &Args {
            campaign_id,
            config,
        },
    )
    .await
}

pub async fn get_calendar_config(campaign_id: String) -> Result<Option<CalendarConfig>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("get_calendar_config", &Args { campaign_id }).await
}

// ============================================================================
// NPC Conversation
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcConversation {
    pub id: String,
    pub npc_id: String,
    pub campaign_id: String,
    pub messages_json: String,
    pub unread_count: u32,
    pub last_message_at: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub parent_message_id: Option<String>,
    pub created_at: String,
}

pub async fn list_npc_conversations(campaign_id: String) -> Result<Vec<NpcConversation>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_npc_conversations", &Args { campaign_id }).await
}

pub async fn get_npc_conversation(npc_id: String) -> Result<NpcConversation, String> {
    #[derive(Serialize)]
    struct Args {
        npc_id: String,
    }
    invoke("get_npc_conversation", &Args { npc_id }).await
}

pub async fn add_npc_message(
    npc_id: String,
    content: String,
    role: String,
    parent_id: Option<String>,
) -> Result<ConversationMessage, String> {
    #[derive(Serialize)]
    struct Args {
        npc_id: String,
        content: String,
        role: String,
        parent_id: Option<String>,
    }
    invoke(
        "add_npc_message",
        &Args {
            npc_id,
            content,
            role,
            parent_id,
        },
    )
    .await
}

pub async fn mark_npc_read(npc_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        npc_id: String,
    }
    invoke_void("mark_npc_read", &Args { npc_id }).await
}

// ============================================================================
// NPC Types & Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPC {
    pub id: String,
    pub name: String,
    pub role: String, // Stringified enum
    pub appearance: AppearanceDescription,
    pub personality: NPCPersonality,
    pub voice: VoiceDescription,
    pub stats: Option<Character>,
    pub relationships: Vec<NPCRelationship>,
    pub secrets: Vec<String>,
    pub hooks: Vec<PlotHook>,
    pub notes: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceDescription {
    pub age: String,
    pub height: String,
    pub build: String,
    pub hair: String,
    pub eyes: String,
    pub skin: String,
    pub distinguishing_features: Vec<String>,
    pub clothing: String,
    pub demeanor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPCPersonality {
    pub traits: Vec<String>,
    pub ideals: Vec<String>,
    pub bonds: Vec<String>,
    pub flaws: Vec<String>,
    pub mannerisms: Vec<String>,
    pub speech_patterns: Vec<String>,
    pub motivations: Vec<String>,
    pub fears: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceDescription {
    pub pitch: String,
    pub pace: String,
    pub accent: Option<String>,
    pub vocabulary: String,
    pub sample_phrases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPCRelationship {
    pub target_id: Option<String>,
    pub target_name: String,
    pub relationship_type: String,
    pub disposition: i32,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotHook {
    pub description: String,
    pub hook_type: String, // Enum stringified
    pub urgency: String,   // Enum stringified
    pub reward_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NPCGenerationOptions {
    pub system: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub race: Option<String>,
    pub occupation: Option<String>,
    pub location: Option<String>,
    pub theme: Option<String>,
    pub generate_stats: bool,
    pub generate_backstory: bool,
    pub personality_depth: String,
    pub include_hooks: bool,
    pub include_secrets: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NpcSummary {
    pub id: String,
    pub name: String,
    pub role: String,
    pub avatar_url: String,
    pub status: String,
    pub last_message: String,
    pub unread_count: u32,
    pub last_active: String,
}

pub async fn generate_npc(
    options: NPCGenerationOptions,
    campaign_id: Option<String>,
) -> Result<NPC, String> {
    #[derive(Serialize)]
    struct Args {
        options: NPCGenerationOptions,
        campaign_id: Option<String>,
    }
    invoke(
        "generate_npc",
        &Args {
            options,
            campaign_id,
        },
    )
    .await
}

pub async fn get_npc(id: String) -> Result<Option<NPC>, String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
    }
    invoke("get_npc", &Args { id }).await
}

pub async fn list_npcs(campaign_id: Option<String>) -> Result<Vec<NPC>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: Option<String>,
    }
    invoke("list_npcs", &Args { campaign_id }).await
}

pub async fn update_npc(npc: NPC) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        npc: NPC,
    }
    invoke_void("update_npc", &Args { npc }).await
}

pub async fn delete_npc(id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        id: String,
    }
    invoke_void("delete_npc", &Args { id }).await
}

pub async fn list_npc_summaries(campaign_id: String) -> Result<Vec<NpcSummary>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_npc_summaries", &Args { campaign_id }).await
}

pub async fn reply_as_npc(npc_id: String) -> Result<ConversationMessage, String> {
    #[derive(Serialize)]
    struct Args {
        npc_id: String,
    }
    invoke("reply_as_npc", &Args { npc_id }).await
}

// ============================================================================
// Entity Relationships & Graphs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    pub id: String,
    pub campaign_id: String,
    pub source_id: String,
    pub source_type: String,
    pub source_name: String,
    pub target_id: String,
    pub target_type: String,
    pub target_name: String,
    pub relationship_type: String,
    pub strength: String,
    pub is_active: bool,
    pub is_known: bool,
    pub description: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub tags: Vec<String>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipSummary {
    pub id: String,
    pub source_id: String,
    pub source_name: String,
    pub source_type: String,
    pub target_id: String,
    pub target_name: String,
    pub target_type: String,
    pub relationship_type: String,
    pub strength: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub stats: GraphStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub color: String,
    pub connection_count: usize,
    pub is_hub: bool,
    pub data: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
    pub strength: u8,
    pub bidirectional: bool,
    pub is_active: bool,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub entity_type_counts: std::collections::HashMap<String, usize>,
    pub relationship_type_counts: std::collections::HashMap<String, usize>,
    pub most_connected_entities: Vec<(String, usize)>,
}

pub async fn create_entity_relationship(
    campaign_id: String,
    source_id: String,
    source_type: String,
    source_name: String,
    target_id: String,
    target_type: String,
    target_name: String,
    relationship_type: String,
    strength: Option<String>,
    description: Option<String>,
) -> Result<EntityRelationship, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        source_id: String,
        source_type: String,
        source_name: String,
        target_id: String,
        target_type: String,
        target_name: String,
        relationship_type: String,
        strength: Option<String>,
        description: Option<String>,
    }
    invoke(
        "create_entity_relationship",
        &Args {
            campaign_id,
            source_id,
            source_type,
            source_name,
            target_id,
            target_type,
            target_name,
            relationship_type,
            strength,
            description,
        },
    )
    .await
}

pub async fn get_entity_relationship(
    campaign_id: String,
    relationship_id: String,
) -> Result<Option<EntityRelationship>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        relationship_id: String,
    }
    invoke(
        "get_entity_relationship",
        &Args {
            campaign_id,
            relationship_id,
        },
    )
    .await
}

pub async fn update_entity_relationship(relationship: EntityRelationship) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        relationship: EntityRelationship,
    }
    invoke_void("update_entity_relationship", &Args { relationship }).await
}

pub async fn delete_entity_relationship(
    campaign_id: String,
    relationship_id: String,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        relationship_id: String,
    }
    invoke_void(
        "delete_entity_relationship",
        &Args {
            campaign_id,
            relationship_id,
        },
    )
    .await
}

pub async fn list_entity_relationships(
    campaign_id: String,
) -> Result<Vec<RelationshipSummary>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_entity_relationships", &Args { campaign_id }).await
}

pub async fn get_relationships_for_entity(
    campaign_id: String,
    entity_id: String,
) -> Result<Vec<EntityRelationship>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        entity_id: String,
    }
    invoke(
        "get_relationships_for_entity",
        &Args {
            campaign_id,
            entity_id,
        },
    )
    .await
}

pub async fn get_relationships_between_entities(
    campaign_id: String,
    entity_a: String,
    entity_b: String,
) -> Result<Vec<EntityRelationship>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        entity_a: String,
        entity_b: String,
    }
    invoke(
        "get_relationships_between_entities",
        &Args {
            campaign_id,
            entity_a,
            entity_b,
        },
    )
    .await
}

pub async fn get_entity_graph(
    campaign_id: String,
    include_inactive: Option<bool>,
) -> Result<EntityGraph, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        include_inactive: Option<bool>,
    }
    invoke(
        "get_entity_graph",
        &Args {
            campaign_id,
            include_inactive,
        },
    )
    .await
}

pub async fn get_ego_graph(
    campaign_id: String,
    entity_id: String,
    depth: Option<usize>,
) -> Result<EntityGraph, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        entity_id: String,
        depth: Option<usize>,
    }
    invoke(
        "get_ego_graph",
        &Args {
            campaign_id,
            entity_id,
            depth,
        },
    )
    .await
}

// ============================================================================
// Personality System
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneMood {
    pub tone: String,
    pub intensity: u8,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonalitySettings {
    pub tone: String,
    pub vocabulary: String,
    pub narrative_style: String,
    pub verbosity: String,
    pub genre: String,
    pub custom_patterns: Vec<String>,
    pub use_dialect: bool,
    pub dialect: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePersonalityContext {
    pub campaign_id: String,
    pub session_id: Option<String>,
    pub narrator_personality_id: Option<String>,
    pub npc_personalities: std::collections::HashMap<String, String>,
    pub location_personalities: std::collections::HashMap<String, String>,
    pub scene_mood: Option<SceneMood>,
    pub active: bool,
    pub settings: PersonalitySettings,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityPreview {
    pub personality_id: String,
    pub personality_name: String,
    pub sample_greetings: Vec<String>,
    pub sample_responses: std::collections::HashMap<String, String>,
    pub characteristics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedPersonalityPreview {
    pub basic: PersonalityPreview,
    pub system_prompt_preview: String,
    pub example_phrases: Vec<String>,
    pub knowledge_areas: Vec<String>,
    pub tags: Vec<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewResponse {
    pub personality_id: String,
    pub personality_name: String,
    pub sample_greeting: String,
    pub formality_level: u8,
    pub key_traits: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyledContent {
    pub content: String,
    pub personality_id: Option<String>,
    pub style_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetActivePersonalityRequest {
    pub session_id: String,
    pub personality_id: Option<String>,
    pub campaign_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalitySettingsRequest {
    pub campaign_id: String,
    pub tone: Option<String>,
    pub vocabulary: Option<String>,
    pub narrative_style: Option<String>,
    pub verbosity: Option<String>,
    pub genre: Option<String>,
    pub custom_patterns: Option<Vec<String>>,
    pub use_dialect: Option<bool>,
    pub dialect: Option<String>,
}

pub async fn set_active_personality(request: SetActivePersonalityRequest) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        request: SetActivePersonalityRequest,
    }
    invoke_void("set_active_personality", &Args { request }).await
}

pub async fn get_active_personality(
    session_id: String,
    campaign_id: String,
) -> Result<Option<String>, String> {
    invoke(
        "get_active_personality",
        &json!({
            "session_id": session_id,
            "campaign_id": campaign_id
        }),
    )
    .await
}

pub async fn get_personality_prompt(personality_id: String) -> Result<String, String> {
    invoke(
        "get_personality_prompt",
        &json!({
            "personality_id": personality_id
        }),
    )
    .await
}

pub async fn apply_personality_to_text(
    text: String,
    personality_id: String,
) -> Result<String, String> {
    invoke(
        "apply_personality_to_text",
        &json!({
            "text": text,
            "personality_id": personality_id
        }),
    )
    .await
}

pub async fn get_personality_context(
    campaign_id: String,
) -> Result<ActivePersonalityContext, String> {
    invoke(
        "get_personality_context",
        &json!({
            "campaign_id": campaign_id
        }),
    )
    .await
}

pub async fn get_session_personality_context(
    session_id: String,
    campaign_id: String,
) -> Result<ActivePersonalityContext, String> {
    invoke(
        "get_session_personality_context",
        &json!({
            "session_id": session_id,
            "campaign_id": campaign_id
        }),
    )
    .await
}

pub async fn set_personality_context(context: ActivePersonalityContext) -> Result<(), String> {
    invoke_void(
        "set_personality_context",
        &json!({
            "context": context
        }),
    )
    .await
}

pub async fn set_narrator_personality(
    campaign_id: String,
    personality_id: Option<String>,
) -> Result<(), String> {
    invoke_void(
        "set_narrator_personality",
        &json!({
            "campaign_id": campaign_id,
            "personality_id": personality_id
        }),
    )
    .await
}

pub async fn assign_npc_personality(
    campaign_id: String,
    npc_id: String,
    personality_id: String,
) -> Result<(), String> {
    invoke_void(
        "assign_npc_personality",
        &json!({
            "campaign_id": campaign_id,
            "npc_id": npc_id,
            "personality_id": personality_id
        }),
    )
    .await
}

pub async fn unassign_npc_personality(campaign_id: String, npc_id: String) -> Result<(), String> {
    invoke_void(
        "unassign_npc_personality",
        &json!({
            "campaign_id": campaign_id,
            "npc_id": npc_id
        }),
    )
    .await
}

pub async fn set_scene_mood(campaign_id: String, mood: Option<SceneMood>) -> Result<(), String> {
    invoke_void(
        "set_scene_mood",
        &json!({
            "campaign_id": campaign_id,
            "mood": mood
        }),
    )
    .await
}

pub async fn set_personality_settings(request: PersonalitySettingsRequest) -> Result<(), String> {
    invoke_void(
        "set_personality_settings",
        &json!({
            "request": request
        }),
    )
    .await
}

pub async fn set_personality_active(campaign_id: String, active: bool) -> Result<(), String> {
    invoke_void(
        "set_personality_active",
        &json!({
            "campaign_id": campaign_id,
            "active": active
        }),
    )
    .await
}

pub async fn preview_personality(personality_id: String) -> Result<PersonalityPreview, String> {
    invoke(
        "preview_personality",
        &json!({
            "personality_id": personality_id
        }),
    )
    .await
}

pub async fn preview_personality_extended(
    personality_id: String,
) -> Result<ExtendedPersonalityPreview, String> {
    invoke(
        "preview_personality_extended",
        &json!({
            "personality_id": personality_id
        }),
    )
    .await
}

pub async fn generate_personality_preview(
    personality_id: String,
) -> Result<PreviewResponse, String> {
    invoke(
        "generate_personality_preview",
        &json!({
            "personality_id": personality_id
        }),
    )
    .await
}

pub async fn test_personality(
    personality_id: String,
    test_prompt: String,
) -> Result<String, String> {
    invoke(
        "test_personality",
        &json!({
            "personality_id": personality_id,
            "test_prompt": test_prompt
        }),
    )
    .await
}

pub async fn get_session_system_prompt(
    session_id: String,
    campaign_id: String,
    content_type: String,
) -> Result<String, String> {
    invoke(
        "get_session_system_prompt",
        &json!({
            "session_id": session_id,
            "campaign_id": campaign_id,
            "content_type": content_type
        }),
    )
    .await
}

pub async fn style_npc_dialogue(
    npc_id: String,
    campaign_id: String,
    raw_dialogue: String,
) -> Result<StyledContent, String> {
    invoke(
        "style_npc_dialogue",
        &json!({
            "npc_id": npc_id,
            "campaign_id": campaign_id,
            "raw_dialogue": raw_dialogue
        }),
    )
    .await
}

pub async fn build_npc_system_prompt(
    npc_id: String,
    campaign_id: String,
    additional_context: Option<String>,
) -> Result<String, String> {
    invoke(
        "build_npc_system_prompt",
        &json!({
            "npc_id": npc_id,
            "campaign_id": campaign_id,
            "additional_context": additional_context
        }),
    )
    .await
}

pub async fn build_narration_prompt(
    campaign_id: String,
    narration_type: String,
) -> Result<String, String> {
    invoke(
        "build_narration_prompt",
        &json!({
            "campaign_id": campaign_id,
            "narration_type": narration_type
        }),
    )
    .await
}

pub async fn list_personalities() -> Result<Vec<PersonalityPreview>, String> {
    invoke_no_args("list_personalities").await
}

pub async fn clear_session_personality_context(session_id: String) -> Result<(), String> {
    invoke_void(
        "clear_session_personality_context",
        &json!({
            "session_id": session_id
        }),
    )
    .await
}
