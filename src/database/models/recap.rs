//! Session Recap Models
//!
//! Database records for session recaps, arc recaps, and PC knowledge filtering.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Recap Status Enum
// ============================================================================

/// Recap generation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecapStatus {
    Pending,
    Generating,
    Complete,
    Failed,
    Edited,
}

impl RecapStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecapStatus::Pending => "pending",
            RecapStatus::Generating => "generating",
            RecapStatus::Complete => "complete",
            RecapStatus::Failed => "failed",
            RecapStatus::Edited => "edited",
        }
    }
}

impl std::fmt::Display for RecapStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for RecapStatus {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "pending" => Ok(RecapStatus::Pending),
            "generating" => Ok(RecapStatus::Generating),
            "complete" => Ok(RecapStatus::Complete),
            "failed" => Ok(RecapStatus::Failed),
            "edited" => Ok(RecapStatus::Edited),
            _ => Err(format!("Unknown recap status: {}", s)),
        }
    }
}

impl Default for RecapStatus {
    fn default() -> Self {
        RecapStatus::Pending
    }
}

// ============================================================================
// Recap Type Enum
// ============================================================================

/// Recap type for different granularities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecapType {
    Session,
    Arc,
    Campaign,
}

impl RecapType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecapType::Session => "session",
            RecapType::Arc => "arc",
            RecapType::Campaign => "campaign",
        }
    }
}

impl std::fmt::Display for RecapType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for RecapType {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "session" => Ok(RecapType::Session),
            "arc" => Ok(RecapType::Arc),
            "campaign" => Ok(RecapType::Campaign),
            _ => Err(format!("Unknown recap type: {}", s)),
        }
    }
}

impl Default for RecapType {
    fn default() -> Self {
        RecapType::Session
    }
}

// ============================================================================
// Session Recap Record
// ============================================================================

/// Session recap database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SessionRecapRecord {
    pub id: String,
    pub session_id: String,
    pub campaign_id: String,
    pub prose_text: Option<String>,
    pub bullet_summary: Option<String>,  // JSON array
    pub cliffhanger: Option<String>,
    pub key_npcs: String,      // JSON array of NPC IDs
    pub key_locations: String, // JSON array of location IDs
    pub key_events: String,    // JSON array of event summaries
    pub player_knowledge: Option<String>,  // JSON map of character_id -> knowledge
    pub arc_id: Option<String>,
    pub recap_type: String,
    pub generation_status: String,
    pub generated_at: Option<String>,
    pub edited_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl SessionRecapRecord {
    pub fn new(session_id: String, campaign_id: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            session_id,
            campaign_id,
            prose_text: None,
            bullet_summary: None,
            cliffhanger: None,
            key_npcs: "[]".to_string(),
            key_locations: "[]".to_string(),
            key_events: "[]".to_string(),
            player_knowledge: None,
            arc_id: None,
            recap_type: RecapType::Session.to_string(),
            generation_status: RecapStatus::Pending.to_string(),
            generated_at: None,
            edited_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn status_enum(&self) -> Result<RecapStatus, String> {
        RecapStatus::try_from(self.generation_status.as_str())
    }

    pub fn recap_type_enum(&self) -> Result<RecapType, String> {
        RecapType::try_from(self.recap_type.as_str())
    }

    pub fn with_arc(mut self, arc_id: String) -> Self {
        self.arc_id = Some(arc_id);
        self
    }

    pub fn with_prose(mut self, prose: String) -> Self {
        self.prose_text = Some(prose);
        self
    }

    pub fn with_bullets(mut self, bullets: &[String]) -> Self {
        self.bullet_summary = Some(serde_json::to_string(bullets).unwrap_or_default());
        self
    }

    pub fn with_cliffhanger(mut self, cliffhanger: String) -> Self {
        self.cliffhanger = Some(cliffhanger);
        self
    }

    pub fn with_key_npcs(mut self, npc_ids: &[String]) -> Self {
        self.key_npcs = serde_json::to_string(npc_ids).unwrap_or_default();
        self
    }

    pub fn with_key_locations(mut self, location_ids: &[String]) -> Self {
        self.key_locations = serde_json::to_string(location_ids).unwrap_or_default();
        self
    }

    pub fn with_key_events(mut self, events: &[String]) -> Self {
        self.key_events = serde_json::to_string(events).unwrap_or_default();
        self
    }

    pub fn mark_generating(&mut self) {
        self.generation_status = RecapStatus::Generating.to_string();
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn mark_complete(&mut self) {
        self.generation_status = RecapStatus::Complete.to_string();
        self.generated_at = Some(chrono::Utc::now().to_rfc3339());
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn mark_edited(&mut self) {
        self.generation_status = RecapStatus::Edited.to_string();
        self.edited_at = Some(chrono::Utc::now().to_rfc3339());
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn mark_failed(&mut self) {
        self.generation_status = RecapStatus::Failed.to_string();
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn key_npcs_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.key_npcs).unwrap_or_default()
    }

    pub fn key_locations_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.key_locations).unwrap_or_default()
    }

    pub fn key_events_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.key_events).unwrap_or_default()
    }

    pub fn bullet_summary_vec(&self) -> Vec<String> {
        self.bullet_summary
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }
}

// ============================================================================
// Arc Recap Record
// ============================================================================

/// Arc recap database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ArcRecapRecord {
    pub id: String,
    pub arc_id: String,
    pub campaign_id: String,
    pub title: String,
    pub summary: Option<String>,
    pub key_moments: String,    // JSON array
    pub character_arcs: String, // JSON array
    pub resolved_plots: String, // JSON array
    pub open_threads: String,   // JSON array
    pub session_ids: String,    // JSON array
    pub generation_status: String,
    pub generated_at: Option<String>,
    pub edited_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ArcRecapRecord {
    pub fn new(arc_id: String, campaign_id: String, title: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            arc_id,
            campaign_id,
            title,
            summary: None,
            key_moments: "[]".to_string(),
            character_arcs: "[]".to_string(),
            resolved_plots: "[]".to_string(),
            open_threads: "[]".to_string(),
            session_ids: "[]".to_string(),
            generation_status: RecapStatus::Pending.to_string(),
            generated_at: None,
            edited_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn status_enum(&self) -> Result<RecapStatus, String> {
        RecapStatus::try_from(self.generation_status.as_str())
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = Some(summary);
        self
    }

    pub fn with_sessions(mut self, session_ids: &[String]) -> Self {
        self.session_ids = serde_json::to_string(session_ids).unwrap_or_default();
        self
    }

    pub fn with_key_moments(mut self, moments: &[String]) -> Self {
        self.key_moments = serde_json::to_string(moments).unwrap_or_default();
        self
    }

    pub fn with_character_arcs(mut self, arcs: &[serde_json::Value]) -> Self {
        self.character_arcs = serde_json::to_string(arcs).unwrap_or_default();
        self
    }

    pub fn with_resolved_plots(mut self, plots: &[String]) -> Self {
        self.resolved_plots = serde_json::to_string(plots).unwrap_or_default();
        self
    }

    pub fn with_open_threads(mut self, threads: &[String]) -> Self {
        self.open_threads = serde_json::to_string(threads).unwrap_or_default();
        self
    }

    pub fn mark_complete(&mut self) {
        self.generation_status = RecapStatus::Complete.to_string();
        self.generated_at = Some(chrono::Utc::now().to_rfc3339());
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn session_ids_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.session_ids).unwrap_or_default()
    }
}

// ============================================================================
// PC Knowledge Filter Record
// ============================================================================

/// PC knowledge filter database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PCKnowledgeFilterRecord {
    pub id: String,
    pub recap_id: String,
    pub character_id: String,
    pub knows_npc_ids: String,       // JSON array
    pub knows_location_ids: String,  // JSON array
    pub knows_event_ids: String,     // JSON array
    pub private_notes: Option<String>,
    pub created_at: String,
}

impl PCKnowledgeFilterRecord {
    pub fn new(recap_id: String, character_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            recap_id,
            character_id,
            knows_npc_ids: "[]".to_string(),
            knows_location_ids: "[]".to_string(),
            knows_event_ids: "[]".to_string(),
            private_notes: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn with_known_npcs(mut self, npc_ids: &[String]) -> Self {
        self.knows_npc_ids = serde_json::to_string(npc_ids).unwrap_or_default();
        self
    }

    pub fn with_known_locations(mut self, location_ids: &[String]) -> Self {
        self.knows_location_ids = serde_json::to_string(location_ids).unwrap_or_default();
        self
    }

    pub fn with_known_events(mut self, event_ids: &[String]) -> Self {
        self.knows_event_ids = serde_json::to_string(event_ids).unwrap_or_default();
        self
    }

    pub fn with_private_notes(mut self, notes: String) -> Self {
        self.private_notes = Some(notes);
        self
    }

    pub fn knows_npc_ids_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.knows_npc_ids).unwrap_or_default()
    }

    pub fn knows_location_ids_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.knows_location_ids).unwrap_or_default()
    }

    pub fn knows_event_ids_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.knows_event_ids).unwrap_or_default()
    }
}
