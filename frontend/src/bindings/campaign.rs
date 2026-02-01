use serde::{Deserialize, Serialize};
use serde_json::json;
use super::core::{invoke, invoke_void, invoke_no_args};

// ============================================================================
// Campaign Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub system: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub settings: CampaignSettings,
}

pub type ThemeWeights = std::collections::HashMap<String, f32>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignStats {
    pub session_count: usize,
    pub npc_count: usize,
    pub total_playtime_minutes: i64,
    pub last_played: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignSettings {
    pub theme: String,
    pub theme_weights: std::collections::HashMap<String, f32>,
    pub voice_enabled: bool,
    pub auto_transcribe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub description: String,
    pub created_at: String,
    pub snapshot_type: String,
}

// ============================================================================
// Campaign Commands
// ============================================================================

pub async fn list_campaigns() -> Result<Vec<Campaign>, String> {
    invoke_no_args("list_campaigns").await
}

pub async fn create_campaign(name: String, system: String) -> Result<Campaign, String> {
    #[derive(Serialize)]
    struct Args {
        name: String,
        system: String,
    }
    invoke("create_campaign", &Args { name, system }).await
}

pub async fn get_campaign(id: String) -> Result<Option<Campaign>, String> {
    invoke("get_campaign", &json!({ "id": id })).await
}

pub async fn delete_campaign(id: String) -> Result<(), String> {
    invoke_void("delete_campaign", &json!({ "id": id })).await
}

pub async fn archive_campaign(id: String) -> Result<(), String> {
    invoke_void("archive_campaign", &json!({ "id": id })).await
}

pub async fn restore_campaign(id: String) -> Result<(), String> {
    invoke_void("restore_campaign", &json!({ "id": id })).await
}

pub async fn list_archived_campaigns() -> Result<Vec<Campaign>, String> {
    invoke("list_archived_campaigns", &json!({})).await
}

pub async fn get_campaign_theme(campaign_id: String) -> Result<ThemeWeights, String> {
    invoke("get_campaign_theme", &json!({ "campaign_id": campaign_id })).await
}

pub async fn set_campaign_theme(campaign_id: String, weights: ThemeWeights) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        weights: ThemeWeights,
    }
    invoke_void("set_campaign_theme", &Args { campaign_id, weights }).await
}

pub async fn get_theme_preset(system: String) -> Result<ThemeWeights, String> {
    #[derive(Serialize)]
    struct Args {
        system: String,
    }
    invoke("get_theme_preset", &Args { system }).await
}

pub async fn list_snapshots(campaign_id: String) -> Result<Vec<SnapshotSummary>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_snapshots", &Args { campaign_id }).await
}

pub async fn create_snapshot(campaign_id: String, description: String) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        description: String,
    }
    invoke("create_snapshot", &Args { campaign_id, description }).await
}

pub async fn restore_snapshot(campaign_id: String, snapshot_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        snapshot_id: String,
    }
    invoke_void("restore_snapshot", &Args { campaign_id, snapshot_id }).await
}

pub async fn get_campaign_stats(campaign_id: String) -> Result<CampaignStats, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("get_campaign_stats", &Args { campaign_id }).await
}

pub async fn generate_campaign_cover(campaign_id: String, title: String) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        title: String,
    }
    invoke("generate_campaign_cover", &Args { campaign_id, title }).await
}

// ============================================================================
// Session Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSession {
    pub id: String,
    pub campaign_id: String,
    pub session_number: u32,
    pub status: String,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub campaign_id: String,
    pub session_number: u32,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_minutes: Option<i64>,
    pub status: String,
    pub note_count: usize,
    pub had_combat: bool,
    pub order_index: i32,
}

// ============================================================================
// Session Commands
// ============================================================================

pub async fn start_session(campaign_id: String, session_number: u32) -> Result<GameSession, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        session_number: u32,
    }
    invoke("start_session", &Args { campaign_id, session_number }).await
}

pub async fn get_session(session_id: String) -> Result<Option<GameSession>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("get_session", &Args { session_id }).await
}

pub async fn get_active_session(campaign_id: String) -> Result<Option<GameSession>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("get_active_session", &Args { campaign_id }).await
}

pub async fn list_sessions(campaign_id: String) -> Result<Vec<SessionSummary>, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
    }
    invoke("list_sessions", &Args { campaign_id }).await
}

pub async fn end_session(session_id: String) -> Result<SessionSummary, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("end_session", &Args { session_id }).await
}

pub async fn reorder_session(session_id: String, new_order: i32) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        new_order: i32,
    }
    invoke_void("reorder_session", &Args { session_id, new_order }).await
}

// ============================================================================
// Timeline Types & Commands
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEventType {
    SessionStart,
    SessionPause,
    SessionResume,
    SessionEnd,
    CombatStart,
    CombatEnd,
    CombatRoundStart,
    CombatTurnStart,
    CombatDamage,
    CombatHealing,
    CombatDeath,
    NoteAdded,
    NoteEdited,
    NoteDeleted,
    #[serde(rename = "npc_interaction")]
    NPCInteraction,
    #[serde(rename = "npc_dialogue")]
    NPCDialogue,
    #[serde(rename = "npc_mood")]
    NPCMood,
    LocationChange,
    SceneChange,
    PlayerAction,
    PlayerRoll,
    SkillCheck,
    SavingThrow,
    ConditionApplied,
    ConditionRemoved,
    ConditionExpired,
    ItemAcquired,
    ItemUsed,
    ItemLost,
    Custom(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum TimelineEventSeverity {
    Trace,
    Info,
    Notable,
    Important,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEntityRef {
    pub entity_type: String,
    pub entity_id: String,
    pub name: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEventData {
    pub id: String,
    pub session_id: String,
    pub event_type: TimelineEventType,
    pub timestamp: String,
    pub title: String,
    pub description: String,
    pub severity: TimelineEventSeverity,
    pub entity_refs: Vec<TimelineEntityRef>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimelineCombatSummary {
    pub encounters: usize,
    pub total_rounds: u32,
    pub damage_dealt: Option<i32>,
    pub healing_done: Option<i32>,
    pub deaths: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineKeyMoment {
    pub title: String,
    pub description: String,
    pub time_offset_minutes: i64,
    pub severity: TimelineEventSeverity,
    pub event_type: TimelineEventType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineSummaryData {
    pub session_id: String,
    pub duration_minutes: i64,
    pub total_events: usize,
    pub combat: TimelineCombatSummary,
    pub key_moments: Vec<TimelineKeyMoment>,
    pub npcs_encountered: Vec<TimelineEntityRef>,
    pub locations_visited: Vec<TimelineEntityRef>,
    pub items_acquired: Vec<String>,
    pub conditions_applied: Vec<String>,
    pub tags_used: Vec<String>,
}

pub async fn add_timeline_event(
    session_id: String,
    event_type: String,
    title: String,
    description: String,
    severity: Option<String>,
    entity_refs: Option<Vec<TimelineEntityRef>>,
    tags: Option<Vec<String>>,
    metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
) -> Result<TimelineEventData, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        event_type: String,
        title: String,
        description: String,
        severity: Option<String>,
        entity_refs: Option<Vec<TimelineEntityRef>>,
        tags: Option<Vec<String>>,
        metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    }
    invoke(
        "add_timeline_event",
        &Args {
            session_id,
            event_type,
            title,
            description,
            severity,
            entity_refs,
            tags,
            metadata,
        },
    )
    .await
}

pub async fn get_session_timeline(session_id: String) -> Result<Vec<TimelineEventData>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("get_session_timeline", &Args { session_id }).await
}

pub async fn get_timeline_summary(session_id: String) -> Result<TimelineSummaryData, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("get_timeline_summary", &Args { session_id }).await
}

pub async fn get_timeline_events_by_type(
    session_id: String,
    event_type: String,
) -> Result<Vec<TimelineEventData>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        event_type: String,
    }
    invoke("get_timeline_events_by_type", &Args { session_id, event_type }).await
}

pub async fn generate_session_summary(session_id: String) -> Result<String, String> {
    let summary = get_timeline_summary(session_id.clone()).await?;

    let mut narrative = String::new();
    narrative.push_str(&format!(
        "Session Summary (Duration: {} minutes, {} events)

",
        summary.duration_minutes, summary.total_events
    ));

    if !summary.key_moments.is_empty() {
        narrative.push_str("KEY MOMENTS:
");
        for moment in &summary.key_moments {
            narrative.push_str(&format!(
                "- [{:?}] {} - {}
",
                moment.severity, moment.title, moment.description
            ));
        }
        narrative.push('\n');
    }

    if summary.combat.encounters > 0 {
        narrative.push_str(&format!(
            "COMBAT: {} encounter(s), {} total rounds",
            summary.combat.encounters, summary.combat.total_rounds
        ));
        if let Some(damage) = summary.combat.damage_dealt {
            narrative.push_str(&format!(", {} damage dealt", damage));
        }
        if let Some(healing) = summary.combat.healing_done {
            narrative.push_str(&format!(", {} healing done", healing));
        }
        if summary.combat.deaths > 0 {
            narrative.push_str(&format!(", {} death(s)", summary.combat.deaths));
        }
        narrative.push_str("

");
    }

    if !summary.npcs_encountered.is_empty() {
        narrative.push_str("NPCs ENCOUNTERED: ");
        let names: Vec<&str> = summary.npcs_encountered.iter().map(|n| n.name.as_str()).collect();
        narrative.push_str(&names.join(", "));
        narrative.push_str("

");
    }

    if !summary.locations_visited.is_empty() {
        narrative.push_str("LOCATIONS VISITED: ");
        let names: Vec<&str> = summary.locations_visited.iter().map(|l| l.name.as_str()).collect();
        narrative.push_str(&names.join(", "));
        narrative.push_str("

");
    }

    if !summary.items_acquired.is_empty() {
        narrative.push_str("ITEMS ACQUIRED: ");
        narrative.push_str(&summary.items_acquired.join(", "));
        narrative.push_str("

");
    }

    Ok(narrative)
}

// ============================================================================
// Session Notes
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NoteCategory {
    General,
    Combat,
    Character,
    Location,
    Plot,
    Quest,
    Loot,
    Rules,
    Meta,
    Worldbuilding,
    Dialogue,
    Secret,
    #[serde(untagged)]
    Custom(String),
}

impl Default for NoteCategory {
    fn default() -> Self {
        NoteCategory::General
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NoteEntityType {
    NPC,
    Player,
    Location,
    Item,
    Quest,
    Session,
    Campaign,
    Combat,
    #[serde(untagged)]
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEntityLink {
    pub entity_type: NoteEntityType,
    pub entity_id: String,
    pub entity_name: String,
    pub linked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNote {
    pub id: String,
    pub session_id: String,
    pub campaign_id: String,
    pub title: String,
    pub content: String,
    pub category: NoteCategory,
    pub tags: Vec<String>,
    pub linked_entities: Vec<NoteEntityLink>,
    pub is_pinned: bool,
    pub is_private: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorizationResponse {
    pub suggested_category: String,
    pub suggested_tags: Vec<String>,
    pub detected_entities: Vec<DetectedEntity>,
    pub confidence: f32,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedEntity {
    pub entity_type: String,
    pub name: String,
    pub context: Option<String>,
}

pub async fn create_session_note(
    session_id: String,
    campaign_id: String,
    title: String,
    content: String,
    category: Option<String>,
    tags: Option<Vec<String>>,
    is_pinned: Option<bool>,
    is_private: Option<bool>,
) -> Result<SessionNote, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        campaign_id: String,
        title: String,
        content: String,
        category: Option<String>,
        tags: Option<Vec<String>>,
        is_pinned: Option<bool>,
        is_private: Option<bool>,
    }
    invoke("create_session_note", &Args {
        session_id,
        campaign_id,
        title,
        content,
        category,
        tags,
        is_pinned,
        is_private,
    }).await
}

pub async fn get_session_note(note_id: String) -> Result<Option<SessionNote>, String> {
    #[derive(Serialize)]
    struct Args {
        note_id: String,
    }
    invoke("get_session_note", &Args { note_id }).await
}

pub async fn update_session_note(note: SessionNote) -> Result<SessionNote, String> {
    #[derive(Serialize)]
    struct Args {
        note: SessionNote,
    }
    invoke("update_session_note", &Args { note }).await
}

pub async fn delete_session_note(note_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        note_id: String,
    }
    invoke_void("delete_session_note", &Args { note_id }).await
}

pub async fn list_session_notes(session_id: String) -> Result<Vec<SessionNote>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("list_session_notes", &Args { session_id }).await
}

pub async fn search_session_notes(
    query: String,
    session_id: Option<String>,
) -> Result<Vec<SessionNote>, String> {
    #[derive(Serialize)]
    struct Args {
        query: String,
        session_id: Option<String>,
    }
    invoke("search_session_notes", &Args { query, session_id }).await
}

pub async fn get_notes_by_category(
    category: String,
    session_id: Option<String>,
) -> Result<Vec<SessionNote>, String> {
    #[derive(Serialize)]
    struct Args {
        category: String,
        session_id: Option<String>,
    }
    invoke("get_notes_by_category", &Args { category, session_id }).await
}

pub async fn get_notes_by_tag(tag: String) -> Result<Vec<SessionNote>, String> {
    #[derive(Serialize)]
    struct Args {
        tag: String,
    }
    invoke("get_notes_by_tag", &Args { tag }).await
}

pub async fn categorize_note_ai(
    title: String,
    content: String,
) -> Result<CategorizationResponse, String> {
    #[derive(Serialize)]
    struct Args {
        title: String,
        content: String,
    }
    invoke("categorize_note_ai", &Args { title, content }).await
}

pub async fn link_entity_to_note(
    note_id: String,
    entity_type: String,
    entity_id: String,
    entity_name: String,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        note_id: String,
        entity_type: String,
        entity_id: String,
        entity_name: String,
    }
    invoke_void("link_entity_to_note", &Args {
        note_id,
        entity_type,
        entity_id,
        entity_name,
    }).await
}

pub async fn unlink_entity_from_note(
    note_id: String,
    entity_id: String,
) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        note_id: String,
        entity_id: String,
    }
    invoke_void("unlink_entity_from_note", &Args { note_id, entity_id }).await
}

// ============================================================================
// Campaign Versioning
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionSummary {
    pub id: String,
    pub campaign_id: String,
    pub version_number: u64,
    pub description: String,
    pub version_type: String,
    pub created_at: String,
    pub created_by: Option<String>,
    pub tags: Vec<String>,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignVersion {
    pub id: String,
    pub campaign_id: String,
    pub version_number: u64,
    pub description: String,
    pub version_type: String,
    pub created_at: String,
    pub created_by: Option<String>,
    pub data_snapshot: String,
    pub data_hash: String,
    pub parent_version_id: Option<String>,
    pub tags: Vec<String>,
    pub size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignDiff {
    pub from_version_id: String,
    pub to_version_id: String,
    pub from_version_number: u64,
    pub to_version_number: u64,
    pub changes: Vec<DiffEntry>,
    pub stats: DiffStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub path: String,
    pub operation: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiffStats {
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
    pub total_changes: usize,
}

pub async fn create_campaign_version(
    campaign_id: String,
    description: String,
    version_type: String,
) -> Result<VersionSummary, String> {
    #[derive(Serialize)]
    struct Args {
        campaign_id: String,
        description: String,
        version_type: String,
    }
    invoke("create_campaign_version", &Args { campaign_id, description, version_type }).await
}

pub async fn list_campaign_versions(campaign_id: String) -> Result<Vec<VersionSummary>, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String }
    invoke("list_campaign_versions", &Args { campaign_id }).await
}

pub async fn get_campaign_version(campaign_id: String, version_id: String) -> Result<CampaignVersion, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, version_id: String }
    invoke("get_campaign_version", &Args { campaign_id, version_id }).await
}

pub async fn compare_campaign_versions(
    campaign_id: String,
    from_version_id: String,
    to_version_id: String,
) -> Result<CampaignDiff, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, from_version_id: String, to_version_id: String }
    invoke("compare_campaign_versions", &Args { campaign_id, from_version_id, to_version_id }).await
}

pub async fn rollback_campaign(campaign_id: String, version_id: String) -> Result<Campaign, String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, version_id: String }
    invoke("rollback_campaign", &Args { campaign_id, version_id }).await
}

pub async fn delete_campaign_version(campaign_id: String, version_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, version_id: String }
    invoke_void("delete_campaign_version", &Args { campaign_id, version_id }).await
}

pub async fn add_version_tag(campaign_id: String, version_id: String, tag: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, version_id: String, tag: String }
    invoke_void("add_version_tag", &Args { campaign_id, version_id, tag }).await
}

pub async fn mark_version_milestone(campaign_id: String, version_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { campaign_id: String, version_id: String }
    invoke_void("mark_version_milestone", &Args { campaign_id, version_id }).await
}
