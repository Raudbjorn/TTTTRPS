//! Session Recap Generator
//!
//! Phase 8 of the Campaign Generation Overhaul.
//!
//! Generates narrative recaps from session data including:
//! - Read-aloud prose format
//! - Bullet point summaries
//! - Cliffhanger extraction
//! - Per-PC knowledge filtering
//! - Arc and campaign-level aggregation
//!
//! ## Example
//!
//! ```rust,ignore
//! let generator = RecapGenerator::new(llm_router, pool);
//!
//! let recap = generator.generate_session_recap(GenerateRecapRequest {
//!     session_id: "session-123".to_string(),
//!     campaign_id: "campaign-456".to_string(),
//!     include_prose: true,
//!     include_bullets: true,
//!     extract_cliffhanger: true,
//! }).await?;
//! ```

use std::sync::Arc;
use sqlx::sqlite::SqlitePool;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

use crate::database::{
    SessionRecapRecord, ArcRecapRecord, PCKnowledgeFilterRecord,
    RecapStatus, SessionRecord, SessionEventRecord,
    SessionNoteRecord, NpcRecord, LocationRecord,
};

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during recap generation
#[derive(Debug, Error)]
pub enum RecapError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Recap not found: {0}")]
    RecapNotFound(String),

    #[error("Arc not found: {0}")]
    ArcNotFound(String),

    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),

    #[error("Character not found: {0}")]
    CharacterNotFound(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("Generation already in progress for {0}")]
    GenerationInProgress(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Result type for recap operations
pub type RecapResult<T> = Result<T, RecapError>;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to generate a session recap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRecapRequest {
    pub session_id: String,
    pub campaign_id: String,
    /// Generate read-aloud prose
    pub include_prose: bool,
    /// Generate bullet point summary
    pub include_bullets: bool,
    /// Extract cliffhanger from session end
    pub extract_cliffhanger: bool,
    /// Maximum bullet points
    pub max_bullets: Option<usize>,
    /// Tone for prose (dramatic, casual, epic, etc.)
    pub tone: Option<String>,
}

impl Default for GenerateRecapRequest {
    fn default() -> Self {
        Self {
            session_id: String::new(),
            campaign_id: String::new(),
            include_prose: true,
            include_bullets: true,
            extract_cliffhanger: true,
            max_bullets: Some(10),
            tone: Some("dramatic".to_string()),
        }
    }
}

/// Request to generate an arc recap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateArcRecapRequest {
    pub arc_id: String,
    pub campaign_id: String,
    /// Include character arc summaries
    pub include_character_arcs: bool,
    /// Include resolved plot threads
    pub include_resolved_plots: bool,
    /// Include open threads
    pub include_open_threads: bool,
}

/// Session recap with all generated content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecap {
    pub id: String,
    pub session_id: String,
    pub campaign_id: String,
    /// Read-aloud prose text
    pub prose: Option<String>,
    /// Bullet point summary
    pub bullets: Vec<String>,
    /// Cliffhanger hook for next session
    pub cliffhanger: Option<String>,
    /// Key NPCs encountered
    pub key_npcs: Vec<EntityReference>,
    /// Key locations visited
    pub key_locations: Vec<EntityReference>,
    /// Key events that occurred
    pub key_events: Vec<String>,
    /// Generation status
    pub status: RecapStatus,
    /// When generated
    pub generated_at: Option<String>,
    /// When last edited
    pub edited_at: Option<String>,
}

/// Arc recap with aggregated content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcRecap {
    pub id: String,
    pub arc_id: String,
    pub campaign_id: String,
    pub title: String,
    /// Summary paragraph
    pub summary: Option<String>,
    /// Key moments across sessions
    pub key_moments: Vec<String>,
    /// Character arc summaries
    pub character_arcs: Vec<CharacterArcSummary>,
    /// Resolved plot threads
    pub resolved_plots: Vec<String>,
    /// Open threads continuing
    pub open_threads: Vec<String>,
    /// Sessions included
    pub session_ids: Vec<String>,
    /// Generation status
    pub status: RecapStatus,
}

/// Entity reference for NPCs/Locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityReference {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub role: Option<String>,
}

/// Character arc summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterArcSummary {
    pub character_id: String,
    pub character_name: String,
    pub arc_summary: String,
    pub key_moments: Vec<String>,
    pub growth: Option<String>,
}

/// PC knowledge filter for what a character knows
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PCKnowledgeFilter {
    pub character_id: String,
    pub knows_npcs: Vec<String>,
    pub knows_locations: Vec<String>,
    pub knows_events: Vec<String>,
    pub private_notes: Option<String>,
}

/// Filtered recap for a specific PC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilteredRecap {
    pub original_recap_id: String,
    pub character_id: String,
    /// Filtered prose (redacted for unknown info)
    pub prose: Option<String>,
    /// Filtered bullets
    pub bullets: Vec<String>,
    /// NPCs the character knows
    pub known_npcs: Vec<EntityReference>,
    /// Locations the character knows
    pub known_locations: Vec<EntityReference>,
    /// Events the character witnessed
    pub known_events: Vec<String>,
}

// ============================================================================
// Session Context for LLM
// ============================================================================

/// Assembled context for recap generation
#[derive(Debug, Clone, Serialize)]
pub struct SessionContext {
    pub session_id: String,
    pub session_number: i32,
    pub session_title: Option<String>,
    pub campaign_name: String,
    pub events: Vec<SessionEvent>,
    pub notes: Vec<String>,
    pub npcs_present: Vec<NpcContext>,
    pub locations_visited: Vec<LocationContext>,
    pub start_time: String,
    pub end_time: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionEvent {
    pub event_type: String,
    pub description: Option<String>,
    pub timestamp: String,
    pub entities: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NpcContext {
    pub id: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocationContext {
    pub id: String,
    pub name: String,
    pub location_type: String,
}

// ============================================================================
// Recap Generator
// ============================================================================

/// Core generator for session and arc recaps
pub struct RecapGenerator {
    pool: Arc<SqlitePool>,
    // LLM router would be injected here in full implementation
    // llm_router: Arc<LlmRouter>,
}

impl RecapGenerator {
    /// Create a new recap generator
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Session Recap Operations
    // ========================================================================

    /// Generate a session recap
    pub async fn generate_session_recap(
        &self,
        request: GenerateRecapRequest,
    ) -> RecapResult<SessionRecap> {
        info!(session_id = %request.session_id, "Generating session recap");

        // Create or update recap record
        let mut recap_record = SessionRecapRecord::new(
            request.session_id.clone(),
            request.campaign_id.clone(),
        );
        recap_record.mark_generating();

        // Use a transaction to atomically check status and claim generation lock.
        // This prevents race conditions where two concurrent requests both pass
        // the status check before either marks the recap as "Generating".
        let mut tx = self.pool.begin().await?;

        // Check if recap already exists and is generating (within transaction)
        let existing: Option<SessionRecapRecord> = sqlx::query_as(
            "SELECT * FROM session_recaps WHERE session_id = ?"
        )
        .bind(&request.session_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(ref record) = existing {
            if record.status_enum().unwrap_or_default() == RecapStatus::Generating {
                // Rollback and return error - another request is already generating
                tx.rollback().await?;
                return Err(RecapError::GenerationInProgress(request.session_id.clone()));
            }
        }

        // Upsert the record to claim the generation lock
        sqlx::query(
            r#"
            INSERT INTO session_recaps (id, session_id, campaign_id, prose_text, bullet_summary,
                cliffhanger, key_npcs, key_locations, key_events, player_knowledge, arc_id,
                recap_type, generation_status, generated_at, edited_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(session_id) DO UPDATE SET
                generation_status = excluded.generation_status,
                updated_at = excluded.updated_at
            "#
        )
        .bind(&recap_record.id)
        .bind(&recap_record.session_id)
        .bind(&recap_record.campaign_id)
        .bind(&recap_record.prose_text)
        .bind(&recap_record.bullet_summary)
        .bind(&recap_record.cliffhanger)
        .bind(&recap_record.key_npcs)
        .bind(&recap_record.key_locations)
        .bind(&recap_record.key_events)
        .bind(&recap_record.player_knowledge)
        .bind(&recap_record.arc_id)
        .bind(&recap_record.recap_type)
        .bind(&recap_record.generation_status)
        .bind(&recap_record.generated_at)
        .bind(&recap_record.edited_at)
        .bind(&recap_record.created_at)
        .bind(&recap_record.updated_at)
        .execute(&mut *tx)
        .await?;

        // Commit the transaction to release the lock - generation can now proceed
        tx.commit().await?;

        // Gather session context
        let context = self.gather_session_context(&request.session_id).await?;

        // Generate content (placeholder for LLM integration)
        let prose = if request.include_prose {
            Some(self.generate_prose(&context, request.tone.as_deref()).await?)
        } else {
            None
        };

        let bullets = if request.include_bullets {
            self.generate_bullets(&context, request.max_bullets.unwrap_or(10)).await?
        } else {
            Vec::new()
        };

        let cliffhanger = if request.extract_cliffhanger {
            self.extract_cliffhanger(&context).await?
        } else {
            None
        };

        // Extract key entities
        let key_npcs = self.extract_key_npcs(&context).await?;
        let key_locations = self.extract_key_locations(&context).await?;
        let key_events: Vec<String> = context.events.iter()
            .filter_map(|e| e.description.clone())
            .collect();

        // Update recap record with generated content
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE session_recaps SET
                prose_text = ?, bullet_summary = ?, cliffhanger = ?,
                key_npcs = ?, key_locations = ?, key_events = ?,
                generation_status = ?, generated_at = ?, updated_at = ?
            WHERE session_id = ?
            "#
        )
        .bind(&prose)
        .bind(serde_json::to_string(&bullets)?)
        .bind(&cliffhanger)
        .bind(serde_json::to_string(&key_npcs.iter().map(|n| &n.id).collect::<Vec<_>>())?)
        .bind(serde_json::to_string(&key_locations.iter().map(|l| &l.id).collect::<Vec<_>>())?)
        .bind(serde_json::to_string(&key_events)?)
        .bind(RecapStatus::Complete.as_str())
        .bind(&now)
        .bind(&now)
        .bind(&request.session_id)
        .execute(self.pool.as_ref())
        .await?;

        // Fetch the updated record
        let record: SessionRecapRecord = sqlx::query_as(
            "SELECT * FROM session_recaps WHERE session_id = ?"
        )
        .bind(&request.session_id)
        .fetch_one(self.pool.as_ref())
        .await?;

        Ok(SessionRecap {
            id: record.id,
            session_id: record.session_id,
            campaign_id: record.campaign_id,
            prose,
            bullets,
            cliffhanger,
            key_npcs,
            key_locations,
            key_events,
            status: RecapStatus::Complete,
            generated_at: record.generated_at,
            edited_at: record.edited_at,
        })
    }

    /// Get an existing session recap
    pub async fn get_session_recap(&self, session_id: &str) -> RecapResult<Option<SessionRecap>> {
        let record: Option<SessionRecapRecord> = sqlx::query_as(
            "SELECT * FROM session_recaps WHERE session_id = ?"
        )
        .bind(session_id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        match record {
            Some(r) => {
                // Load NPC and location details
                let key_npcs = self.load_npc_references(&r.key_npcs_vec()).await?;
                let key_locations = self.load_location_references(&r.key_locations_vec()).await?;
                let bullets = r.bullet_summary_vec();
                let key_events = r.key_events_vec();
                let status = r.status_enum().unwrap_or_default();
                let prose = r.prose_text.clone();
                let cliffhanger = r.cliffhanger.clone();

                Ok(Some(SessionRecap {
                    id: r.id,
                    session_id: r.session_id,
                    campaign_id: r.campaign_id,
                    prose,
                    bullets,
                    cliffhanger,
                    key_npcs,
                    key_locations,
                    key_events,
                    status,
                    generated_at: r.generated_at,
                    edited_at: r.edited_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Update a session recap (manual edits)
    pub async fn update_session_recap(
        &self,
        session_id: &str,
        prose: Option<String>,
        bullets: Option<Vec<String>>,
        cliffhanger: Option<String>,
    ) -> RecapResult<SessionRecap> {
        // Verify the recap exists before starting the transaction
        let _existing = self.get_session_recap(session_id).await?
            .ok_or_else(|| RecapError::RecapNotFound(session_id.to_string()))?;

        let now = chrono::Utc::now().to_rfc3339();

        // Use a transaction to ensure all updates succeed atomically.
        // If any update fails, all changes are rolled back.
        let mut tx = self.pool.begin().await?;

        if let Some(p) = &prose {
            sqlx::query("UPDATE session_recaps SET prose_text = ?, edited_at = ?, generation_status = ?, updated_at = ? WHERE session_id = ?")
                .bind(p)
                .bind(&now)
                .bind(RecapStatus::Edited.as_str())
                .bind(&now)
                .bind(session_id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(b) = &bullets {
            sqlx::query("UPDATE session_recaps SET bullet_summary = ?, edited_at = ?, generation_status = ?, updated_at = ? WHERE session_id = ?")
                .bind(serde_json::to_string(b)?)
                .bind(&now)
                .bind(RecapStatus::Edited.as_str())
                .bind(&now)
                .bind(session_id)
                .execute(&mut *tx)
                .await?;
        }

        if let Some(c) = &cliffhanger {
            sqlx::query("UPDATE session_recaps SET cliffhanger = ?, edited_at = ?, generation_status = ?, updated_at = ? WHERE session_id = ?")
                .bind(c)
                .bind(&now)
                .bind(RecapStatus::Edited.as_str())
                .bind(&now)
                .bind(session_id)
                .execute(&mut *tx)
                .await?;
        }

        // Commit all changes atomically
        tx.commit().await?;

        self.get_session_recap(session_id).await?
            .ok_or_else(|| RecapError::RecapNotFound(session_id.to_string()))
    }

    // ========================================================================
    // Arc Recap Operations
    // ========================================================================

    /// Generate an arc recap
    pub async fn generate_arc_recap(
        &self,
        request: GenerateArcRecapRequest,
    ) -> RecapResult<ArcRecap> {
        info!(arc_id = %request.arc_id, "Generating arc recap");

        // Get all sessions in this arc
        let session_recaps = self.get_arc_session_recaps(&request.arc_id).await?;

        if session_recaps.is_empty() {
            return Err(RecapError::InvalidConfiguration(
                "No session recaps found for arc".to_string()
            ));
        }

        // Get arc title (from plot_arcs table if exists)
        let arc_title = self.get_arc_title(&request.arc_id).await?
            .unwrap_or_else(|| format!("Arc {}", request.arc_id));

        // Create arc recap record
        let mut recap_record = ArcRecapRecord::new(
            request.arc_id.clone(),
            request.campaign_id.clone(),
            arc_title.clone(),
        );

        let session_ids: Vec<String> = session_recaps.iter()
            .map(|r| r.session_id.clone())
            .collect();
        recap_record = recap_record.with_sessions(&session_ids);

        // Generate summary from aggregated recaps
        let summary = self.generate_arc_summary(&session_recaps).await?;
        recap_record = recap_record.with_summary(summary.clone());

        // Extract key moments
        let key_moments: Vec<String> = session_recaps.iter()
            .flat_map(|r| r.key_events.iter().cloned())
            .take(20) // Limit to top 20
            .collect();
        recap_record = recap_record.with_key_moments(&key_moments);

        // Generate character arcs if requested
        let character_arcs = if request.include_character_arcs {
            self.generate_character_arcs(&session_recaps).await?
        } else {
            Vec::new()
        };
        let character_arcs_json: Vec<serde_json::Value> = character_arcs.iter()
            .map(|c| serde_json::to_value(c).unwrap_or_default())
            .collect();
        recap_record = recap_record.with_character_arcs(&character_arcs_json);

        // Insert or update
        sqlx::query(
            r#"
            INSERT INTO arc_recaps (id, arc_id, campaign_id, title, summary, key_moments,
                character_arcs, resolved_plots, open_threads, session_ids, generation_status,
                generated_at, edited_at, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(arc_id) DO UPDATE SET
                summary = excluded.summary,
                key_moments = excluded.key_moments,
                character_arcs = excluded.character_arcs,
                session_ids = excluded.session_ids,
                generation_status = excluded.generation_status,
                generated_at = excluded.generated_at,
                updated_at = excluded.updated_at
            "#
        )
        .bind(&recap_record.id)
        .bind(&recap_record.arc_id)
        .bind(&recap_record.campaign_id)
        .bind(&recap_record.title)
        .bind(&recap_record.summary)
        .bind(&recap_record.key_moments)
        .bind(&recap_record.character_arcs)
        .bind(&recap_record.resolved_plots)
        .bind(&recap_record.open_threads)
        .bind(&recap_record.session_ids)
        .bind(RecapStatus::Complete.as_str())
        .bind(chrono::Utc::now().to_rfc3339())
        .bind::<Option<String>>(None)
        .bind(&recap_record.created_at)
        .bind(&recap_record.updated_at)
        .execute(self.pool.as_ref())
        .await?;

        Ok(ArcRecap {
            id: recap_record.id,
            arc_id: request.arc_id,
            campaign_id: request.campaign_id,
            title: arc_title,
            summary: Some(summary),
            key_moments,
            character_arcs,
            resolved_plots: Vec::new(), // TODO: Extract from plot tracking
            open_threads: Vec::new(),   // TODO: Extract from plot tracking
            session_ids,
            status: RecapStatus::Complete,
        })
    }

    /// Generate full campaign summary
    pub async fn generate_campaign_summary(&self, campaign_id: &str) -> RecapResult<String> {
        info!(campaign_id, "Generating campaign summary");

        // Get all arc recaps for the campaign
        let arc_recaps: Vec<ArcRecapRecord> = sqlx::query_as(
            "SELECT * FROM arc_recaps WHERE campaign_id = ? ORDER BY created_at"
        )
        .bind(campaign_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        if arc_recaps.is_empty() {
            // Fall back to session recaps
            let session_recaps: Vec<SessionRecapRecord> = sqlx::query_as(
                "SELECT * FROM session_recaps WHERE campaign_id = ? ORDER BY created_at"
            )
            .bind(campaign_id)
            .fetch_all(self.pool.as_ref())
            .await?;

            if session_recaps.is_empty() {
                return Ok("No sessions recorded yet for this campaign.".to_string());
            }

            // Generate from session recaps
            let summaries: Vec<String> = session_recaps.iter()
                .filter_map(|r| r.prose_text.clone())
                .collect();

            return Ok(self.combine_summaries(&summaries).await?);
        }

        // Combine arc summaries
        let summaries: Vec<String> = arc_recaps.iter()
            .filter_map(|r| r.summary.clone())
            .collect();

        self.combine_summaries(&summaries).await
    }

    // ========================================================================
    // PC Knowledge Filtering
    // ========================================================================

    /// Filter a recap by PC knowledge
    pub async fn filter_recap_by_pc(
        &self,
        recap_id: &str,
        character_id: &str,
    ) -> RecapResult<FilteredRecap> {
        // Get the original recap
        let recap_record: SessionRecapRecord = sqlx::query_as(
            "SELECT * FROM session_recaps WHERE id = ?"
        )
        .bind(recap_id)
        .fetch_optional(self.pool.as_ref())
        .await?
        .ok_or_else(|| RecapError::RecapNotFound(recap_id.to_string()))?;

        // Get or create PC knowledge filter
        let filter = self.get_or_create_knowledge_filter(recap_id, character_id).await?;

        // Filter NPCs
        let known_npcs = self.load_npc_references(&filter.knows_npcs).await?;

        // Filter locations
        let known_locations = self.load_location_references(&filter.knows_locations).await?;

        // Filter events
        let all_events = recap_record.key_events_vec();
        let known_events: Vec<String> = all_events.iter()
            .enumerate()
            .filter(|(i, _)| filter.knows_events.contains(&i.to_string()))
            .map(|(_, e)| e.clone())
            .collect();

        // Filter prose (placeholder - would use LLM to redact)
        let filtered_prose = recap_record.prose_text.clone().map(|p| {
            self.redact_unknown_entities(&p, &filter)
        });

        // Filter bullets
        let all_bullets = recap_record.bullet_summary_vec();
        let filtered_bullets: Vec<String> = all_bullets.into_iter()
            .filter(|b| self.bullet_known_to_pc(b, &filter))
            .collect();

        Ok(FilteredRecap {
            original_recap_id: recap_id.to_string(),
            character_id: character_id.to_string(),
            prose: filtered_prose,
            bullets: filtered_bullets,
            known_npcs,
            known_locations,
            known_events,
        })
    }

    /// Set PC knowledge for a recap
    pub async fn set_pc_knowledge(
        &self,
        recap_id: &str,
        character_id: &str,
        filter: PCKnowledgeFilter,
    ) -> RecapResult<()> {
        let record = PCKnowledgeFilterRecord::new(
            recap_id.to_string(),
            character_id.to_string(),
        )
        .with_known_npcs(&filter.knows_npcs)
        .with_known_locations(&filter.knows_locations)
        .with_known_events(&filter.knows_events);

        sqlx::query(
            r#"
            INSERT INTO pc_knowledge_filters (id, recap_id, character_id, knows_npc_ids,
                knows_location_ids, knows_event_ids, private_notes, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(recap_id, character_id) DO UPDATE SET
                knows_npc_ids = excluded.knows_npc_ids,
                knows_location_ids = excluded.knows_location_ids,
                knows_event_ids = excluded.knows_event_ids,
                private_notes = excluded.private_notes
            "#
        )
        .bind(&record.id)
        .bind(&record.recap_id)
        .bind(&record.character_id)
        .bind(&record.knows_npc_ids)
        .bind(&record.knows_location_ids)
        .bind(&record.knows_event_ids)
        .bind(&record.private_notes)
        .bind(&record.created_at)
        .execute(self.pool.as_ref())
        .await?;

        Ok(())
    }

    // ========================================================================
    // Internal Helper Methods
    // ========================================================================

    /// Gather session context for recap generation
    async fn gather_session_context(&self, session_id: &str) -> RecapResult<SessionContext> {
        // Get session record
        let session: SessionRecord = sqlx::query_as(
            "SELECT * FROM sessions WHERE id = ?"
        )
        .bind(session_id)
        .fetch_optional(self.pool.as_ref())
        .await?
        .ok_or_else(|| RecapError::SessionNotFound(session_id.to_string()))?;

        // Get campaign name
        let campaign_name: String = sqlx::query_scalar(
            "SELECT name FROM campaigns WHERE id = ?"
        )
        .bind(&session.campaign_id)
        .fetch_optional(self.pool.as_ref())
        .await?
        .ok_or_else(|| RecapError::CampaignNotFound(session.campaign_id.clone()))?;

        // Get session events
        let events: Vec<SessionEventRecord> = sqlx::query_as(
            "SELECT * FROM session_events WHERE session_id = ? ORDER BY timestamp"
        )
        .bind(session_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        let session_events: Vec<SessionEvent> = events.into_iter()
            .map(|e| SessionEvent {
                event_type: e.event_type,
                description: e.description,
                timestamp: e.timestamp,
                entities: e.entities
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
            })
            .collect();

        // Get session notes
        let notes: Vec<SessionNoteRecord> = sqlx::query_as(
            "SELECT * FROM session_notes WHERE session_id = ? ORDER BY created_at"
        )
        .bind(session_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        let note_contents: Vec<String> = notes.into_iter()
            .map(|n| n.content)
            .collect();

        // Get NPCs present in session
        let npc_ids: Vec<String> = session_events.iter()
            .flat_map(|e| e.entities.iter().cloned())
            .collect();

        let mut npcs_present = Vec::new();
        for npc_id in npc_ids.iter().take(20) {
            if let Ok(Some(npc)) = sqlx::query_as::<_, NpcRecord>(
                "SELECT * FROM npcs WHERE id = ?"
            )
            .bind(npc_id)
            .fetch_optional(self.pool.as_ref())
            .await
            {
                npcs_present.push(NpcContext {
                    id: npc.id,
                    name: npc.name,
                    role: npc.role,
                });
            }
        }

        // Get locations visited
        let locations_visited = Vec::new(); // TODO: Extract from events

        Ok(SessionContext {
            session_id: session_id.to_string(),
            session_number: session.session_number,
            session_title: session.title,
            campaign_name,
            events: session_events,
            notes: note_contents,
            npcs_present,
            locations_visited,
            start_time: session.started_at,
            end_time: session.ended_at,
        })
    }

    /// Generate prose recap (placeholder for LLM integration)
    async fn generate_prose(&self, context: &SessionContext, tone: Option<&str>) -> RecapResult<String> {
        // This would call the LLM in a full implementation
        let tone_desc = tone.unwrap_or("dramatic");
        let event_count = context.events.len();
        let npc_names: Vec<&str> = context.npcs_present.iter()
            .map(|n| n.name.as_str())
            .take(3)
            .collect();

        Ok(format!(
            "In Session {} of {}, the party experienced {} significant events. \
            Notable NPCs encountered included {}. \
            [This is a placeholder - full LLM generation would create {} prose here.]",
            context.session_number,
            context.campaign_name,
            event_count,
            if npc_names.is_empty() { "none".to_string() } else { npc_names.join(", ") },
            tone_desc
        ))
    }

    /// Generate bullet point summary (placeholder for LLM integration)
    async fn generate_bullets(&self, context: &SessionContext, max_bullets: usize) -> RecapResult<Vec<String>> {
        // This would call the LLM in a full implementation
        let mut bullets = Vec::new();

        for (i, event) in context.events.iter().take(max_bullets).enumerate() {
            let bullet = event.description.clone()
                .unwrap_or_else(|| format!("Event {}: {}", i + 1, event.event_type));
            bullets.push(bullet);
        }

        if bullets.is_empty() {
            bullets.push("Session events not yet recorded.".to_string());
        }

        Ok(bullets)
    }

    /// Extract cliffhanger from session end (placeholder for LLM integration)
    async fn extract_cliffhanger(&self, context: &SessionContext) -> RecapResult<Option<String>> {
        // This would call the LLM in a full implementation
        // Look for the last significant event
        if let Some(last_event) = context.events.last() {
            if let Some(desc) = &last_event.description {
                return Ok(Some(format!("What happens next? {}", desc)));
            }
        }

        Ok(None)
    }

    /// Extract key NPCs from context
    async fn extract_key_npcs(&self, context: &SessionContext) -> RecapResult<Vec<EntityReference>> {
        Ok(context.npcs_present.iter()
            .map(|n| EntityReference {
                id: n.id.clone(),
                name: n.name.clone(),
                entity_type: "npc".to_string(),
                role: Some(n.role.clone()),
            })
            .collect())
    }

    /// Extract key locations from context
    async fn extract_key_locations(&self, context: &SessionContext) -> RecapResult<Vec<EntityReference>> {
        Ok(context.locations_visited.iter()
            .map(|l| EntityReference {
                id: l.id.clone(),
                name: l.name.clone(),
                entity_type: "location".to_string(),
                role: Some(l.location_type.clone()),
            })
            .collect())
    }

    /// Load NPC references by IDs
    async fn load_npc_references(&self, npc_ids: &[String]) -> RecapResult<Vec<EntityReference>> {
        let mut refs = Vec::new();
        for id in npc_ids {
            if let Ok(Some(npc)) = sqlx::query_as::<_, NpcRecord>(
                "SELECT * FROM npcs WHERE id = ?"
            )
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await
            {
                refs.push(EntityReference {
                    id: npc.id,
                    name: npc.name,
                    entity_type: "npc".to_string(),
                    role: Some(npc.role),
                });
            }
        }
        Ok(refs)
    }

    /// Load location references by IDs
    async fn load_location_references(&self, location_ids: &[String]) -> RecapResult<Vec<EntityReference>> {
        let mut refs = Vec::new();
        for id in location_ids {
            if let Ok(Some(loc)) = sqlx::query_as::<_, LocationRecord>(
                "SELECT * FROM locations WHERE id = ?"
            )
            .bind(id)
            .fetch_optional(self.pool.as_ref())
            .await
            {
                refs.push(EntityReference {
                    id: loc.id,
                    name: loc.name,
                    entity_type: "location".to_string(),
                    role: Some(loc.location_type),
                });
            }
        }
        Ok(refs)
    }

    /// Get session recaps for an arc
    async fn get_arc_session_recaps(&self, arc_id: &str) -> RecapResult<Vec<SessionRecap>> {
        let records: Vec<SessionRecapRecord> = sqlx::query_as(
            "SELECT * FROM session_recaps WHERE arc_id = ? ORDER BY created_at"
        )
        .bind(arc_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        let mut recaps = Vec::new();
        for r in records {
            let key_npcs = self.load_npc_references(&r.key_npcs_vec()).await?;
            let key_locations = self.load_location_references(&r.key_locations_vec()).await?;
            let bullets = r.bullet_summary_vec();
            let key_events = r.key_events_vec();
            let status = r.status_enum().unwrap_or_default();
            let prose = r.prose_text.clone();
            let cliffhanger = r.cliffhanger.clone();

            recaps.push(SessionRecap {
                id: r.id,
                session_id: r.session_id,
                campaign_id: r.campaign_id,
                prose,
                bullets,
                cliffhanger,
                key_npcs,
                key_locations,
                key_events,
                status,
                generated_at: r.generated_at,
                edited_at: r.edited_at,
            });
        }

        Ok(recaps)
    }

    /// Get arc title from plot_arcs table
    async fn get_arc_title(&self, arc_id: &str) -> RecapResult<Option<String>> {
        let title: Option<String> = sqlx::query_scalar(
            "SELECT name FROM plot_arcs WHERE id = ?"
        )
        .bind(arc_id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        Ok(title)
    }

    /// Generate arc summary from session recaps
    async fn generate_arc_summary(&self, session_recaps: &[SessionRecap]) -> RecapResult<String> {
        // Placeholder for LLM integration
        let session_count = session_recaps.len();
        let total_events: usize = session_recaps.iter()
            .map(|r| r.key_events.len())
            .sum();

        Ok(format!(
            "This arc spanned {} sessions with {} significant events. \
            [Full summary would be generated by LLM from session prose.]",
            session_count,
            total_events
        ))
    }

    /// Generate character arcs from session recaps
    async fn generate_character_arcs(&self, _session_recaps: &[SessionRecap]) -> RecapResult<Vec<CharacterArcSummary>> {
        // Placeholder for LLM integration
        // Would analyze character mentions across sessions
        Ok(Vec::new())
    }

    /// Combine multiple summaries into one
    async fn combine_summaries(&self, summaries: &[String]) -> RecapResult<String> {
        // Placeholder for LLM integration
        Ok(summaries.join("\n\n"))
    }

    /// Get or create PC knowledge filter
    async fn get_or_create_knowledge_filter(
        &self,
        recap_id: &str,
        character_id: &str,
    ) -> RecapResult<PCKnowledgeFilter> {
        let record: Option<PCKnowledgeFilterRecord> = sqlx::query_as(
            "SELECT * FROM pc_knowledge_filters WHERE recap_id = ? AND character_id = ?"
        )
        .bind(recap_id)
        .bind(character_id)
        .fetch_optional(self.pool.as_ref())
        .await?;

        match record {
            Some(r) => {
                let knows_npcs = r.knows_npc_ids_vec();
                let knows_locations = r.knows_location_ids_vec();
                let knows_events = r.knows_event_ids_vec();
                Ok(PCKnowledgeFilter {
                    character_id: r.character_id,
                    knows_npcs,
                    knows_locations,
                    knows_events,
                    private_notes: r.private_notes,
                })
            }
            None => {
                // Create default filter (knows everything)
                let recap: SessionRecapRecord = sqlx::query_as(
                    "SELECT * FROM session_recaps WHERE id = ?"
                )
                .bind(recap_id)
                .fetch_one(self.pool.as_ref())
                .await?;

                let filter = PCKnowledgeFilter {
                    character_id: character_id.to_string(),
                    knows_npcs: recap.key_npcs_vec(),
                    knows_locations: recap.key_locations_vec(),
                    knows_events: (0..recap.key_events_vec().len())
                        .map(|i| i.to_string())
                        .collect(),
                    private_notes: None,
                };

                self.set_pc_knowledge(recap_id, character_id, filter.clone()).await?;
                Ok(filter)
            }
        }
    }

    /// Redact unknown entities from prose
    fn redact_unknown_entities(&self, prose: &str, _filter: &PCKnowledgeFilter) -> String {
        // Placeholder - would use NLP/LLM to identify and redact unknown entities
        prose.to_string()
    }

    /// Check if a bullet point is known to the PC
    fn bullet_known_to_pc(&self, _bullet: &str, _filter: &PCKnowledgeFilter) -> bool {
        // Placeholder - would analyze bullet for unknown entities
        true
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_recap_request_default() {
        let request = GenerateRecapRequest::default();
        assert!(request.include_prose);
        assert!(request.include_bullets);
        assert!(request.extract_cliffhanger);
        assert_eq!(request.max_bullets, Some(10));
    }

    #[test]
    fn test_entity_reference() {
        let entity = EntityReference {
            id: "npc-1".to_string(),
            name: "Gandalf".to_string(),
            entity_type: "npc".to_string(),
            role: Some("wizard".to_string()),
        };
        assert_eq!(entity.name, "Gandalf");
    }

    #[test]
    fn test_character_arc_summary() {
        let arc = CharacterArcSummary {
            character_id: "char-1".to_string(),
            character_name: "Aragorn".to_string(),
            arc_summary: "From ranger to king".to_string(),
            key_moments: vec!["Revealed lineage".to_string()],
            growth: Some("Leadership".to_string()),
        };
        assert_eq!(arc.character_name, "Aragorn");
    }

    #[test]
    fn test_pc_knowledge_filter() {
        let filter = PCKnowledgeFilter {
            character_id: "char-1".to_string(),
            knows_npcs: vec!["npc-1".to_string()],
            knows_locations: vec!["loc-1".to_string()],
            knows_events: vec!["0".to_string(), "1".to_string()],
            private_notes: None,
        };
        assert_eq!(filter.knows_npcs.len(), 1);
        assert_eq!(filter.knows_events.len(), 2);
    }

    #[test]
    fn test_session_context() {
        let context = SessionContext {
            session_id: "session-1".to_string(),
            session_number: 5,
            session_title: Some("The Dark Forest".to_string()),
            campaign_name: "Dragon's Lair".to_string(),
            events: Vec::new(),
            notes: Vec::new(),
            npcs_present: Vec::new(),
            locations_visited: Vec::new(),
            start_time: "2024-01-01T18:00:00Z".to_string(),
            end_time: Some("2024-01-01T22:00:00Z".to_string()),
        };
        assert_eq!(context.session_number, 5);
    }
}
