//! Session Recap Commands
//!
//! Phase 8 of the Campaign Generation Overhaul.
//!
//! Tauri IPC commands for session and arc recap generation.
//!
//! # Commands
//!
//! ## Session Recaps
//! - [`generate_session_recap`]: Generate a recap for a session
//! - [`get_session_recap`]: Get an existing recap
//! - [`update_session_recap`]: Edit a generated recap
//!
//! ## Arc Recaps
//! - [`generate_arc_recap`]: Generate a recap for an arc
//! - [`get_arc_recap`]: Get an existing arc recap
//!
//! ## Campaign Summary
//! - [`generate_campaign_summary`]: Generate full campaign summary
//!
//! ## PC Knowledge Filtering
//! - [`filter_recap_by_pc`]: Get PC-filtered version of a recap
//! - [`set_pc_knowledge`]: Set what a PC knows

use std::sync::Arc;
use tauri::State;
use tracing::{info, debug, error};

use crate::commands::AppState;
use crate::core::campaign::{
    RecapGenerator, SessionRecap, ArcRecap, FilteredRecap,
    GenerateRecapRequest, GenerateArcRecapRequest,
    PCKnowledgeFilter, RecapError,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a RecapGenerator from AppState
fn get_recap_generator(state: &State<'_, AppState>) -> RecapGenerator {
    let pool = Arc::new(state.database.pool().clone());
    RecapGenerator::new(pool)
}

/// Convert RecapError to String for Tauri IPC
fn recap_err_to_string(err: RecapError) -> String {
    error!(error = %err, "Recap command error");
    err.to_string()
}

// ============================================================================
// Session Recap Commands
// ============================================================================

/// Generate a session recap.
///
/// Creates prose, bullet points, and extracts cliffhangers from session data.
///
/// # Arguments
/// * `session_id` - The session to generate a recap for
/// * `campaign_id` - The campaign ID
/// * `include_prose` - Generate read-aloud prose
/// * `include_bullets` - Generate bullet point summary
/// * `extract_cliffhanger` - Extract cliffhanger from session end
/// * `max_bullets` - Maximum number of bullet points
/// * `tone` - Tone for prose (dramatic, casual, epic, etc.)
///
/// # Returns
/// The generated session recap
#[tauri::command]
pub async fn generate_session_recap(
    session_id: String,
    campaign_id: String,
    include_prose: Option<bool>,
    include_bullets: Option<bool>,
    extract_cliffhanger: Option<bool>,
    max_bullets: Option<usize>,
    tone: Option<String>,
    state: State<'_, AppState>,
) -> Result<SessionRecap, String> {
    info!(session_id = %session_id, "Generating session recap");

    let generator = get_recap_generator(&state);

    let request = GenerateRecapRequest {
        session_id,
        campaign_id,
        include_prose: include_prose.unwrap_or(true),
        include_bullets: include_bullets.unwrap_or(true),
        extract_cliffhanger: extract_cliffhanger.unwrap_or(true),
        max_bullets,
        tone,
    };

    generator
        .generate_session_recap(request)
        .await
        .map_err(recap_err_to_string)
}

/// Get an existing session recap.
///
/// # Arguments
/// * `session_id` - The session ID
///
/// # Returns
/// The session recap if it exists
#[tauri::command]
pub async fn get_session_recap(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Option<SessionRecap>, String> {
    debug!(session_id = %session_id, "Getting session recap");

    let generator = get_recap_generator(&state);
    generator
        .get_session_recap(&session_id)
        .await
        .map_err(recap_err_to_string)
}

/// Update a session recap.
///
/// Allows manual editing of generated content.
///
/// # Arguments
/// * `session_id` - The session ID
/// * `prose` - Optional new prose text
/// * `bullets` - Optional new bullet points
/// * `cliffhanger` - Optional new cliffhanger
///
/// # Returns
/// The updated session recap
#[tauri::command]
pub async fn update_session_recap(
    session_id: String,
    prose: Option<String>,
    bullets: Option<Vec<String>>,
    cliffhanger: Option<String>,
    state: State<'_, AppState>,
) -> Result<SessionRecap, String> {
    info!(session_id = %session_id, "Updating session recap");

    let generator = get_recap_generator(&state);
    generator
        .update_session_recap(&session_id, prose, bullets, cliffhanger)
        .await
        .map_err(recap_err_to_string)
}

// ============================================================================
// Arc Recap Commands
// ============================================================================

/// Generate an arc recap.
///
/// Aggregates session recaps into an arc-level summary.
///
/// # Arguments
/// * `arc_id` - The arc to generate a recap for
/// * `campaign_id` - The campaign ID
/// * `include_character_arcs` - Include character arc summaries
/// * `include_resolved_plots` - Include resolved plot threads
/// * `include_open_threads` - Include open threads
///
/// # Returns
/// The generated arc recap
#[tauri::command]
pub async fn generate_arc_recap(
    arc_id: String,
    campaign_id: String,
    include_character_arcs: Option<bool>,
    include_resolved_plots: Option<bool>,
    include_open_threads: Option<bool>,
    state: State<'_, AppState>,
) -> Result<ArcRecap, String> {
    info!(arc_id = %arc_id, "Generating arc recap");

    let generator = get_recap_generator(&state);

    let request = GenerateArcRecapRequest {
        arc_id,
        campaign_id,
        include_character_arcs: include_character_arcs.unwrap_or(true),
        include_resolved_plots: include_resolved_plots.unwrap_or(true),
        include_open_threads: include_open_threads.unwrap_or(true),
    };

    generator
        .generate_arc_recap(request)
        .await
        .map_err(recap_err_to_string)
}

/// Get an existing arc recap.
///
/// # Arguments
/// * `arc_id` - The arc ID
///
/// # Returns
/// The arc recap if it exists
#[tauri::command]
pub async fn get_arc_recap(
    arc_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ArcRecap>, String> {
    debug!(arc_id = %arc_id, "Getting arc recap");

    let pool = Arc::new(state.database.pool().clone());

    let record: Option<crate::database::ArcRecapRecord> = sqlx::query_as(
        "SELECT * FROM arc_recaps WHERE arc_id = ?"
    )
    .bind(&arc_id)
    .fetch_optional(pool.as_ref())
    .await
    .map_err(|e| e.to_string())?;

    match record {
        Some(r) => {
            let character_arcs: Vec<crate::core::campaign::CharacterArcSummary> =
                serde_json::from_str(&r.character_arcs).unwrap_or_else(|e| {
                    log::warn!("Failed to parse character_arcs for arc_recap id={}, arc_id={}: {}", r.id, r.arc_id, e);
                    Vec::new()
                });
            let key_moments = serde_json::from_str(&r.key_moments).unwrap_or_else(|e| {
                log::warn!("Failed to parse key_moments for arc_recap id={}, arc_id={}: {}", r.id, r.arc_id, e);
                Vec::new()
            });
            let resolved_plots = serde_json::from_str(&r.resolved_plots).unwrap_or_else(|e| {
                log::warn!("Failed to parse resolved_plots for arc_recap id={}, arc_id={}: {}", r.id, r.arc_id, e);
                Vec::new()
            });
            let open_threads = serde_json::from_str(&r.open_threads).unwrap_or_else(|e| {
                log::warn!("Failed to parse open_threads for arc_recap id={}, arc_id={}: {}", r.id, r.arc_id, e);
                Vec::new()
            });
            let session_ids = r.session_ids_vec();
            let status = r.status_enum().unwrap_or_default();

            Ok(Some(ArcRecap {
                id: r.id,
                arc_id: r.arc_id,
                campaign_id: r.campaign_id,
                title: r.title,
                summary: r.summary.clone(),
                key_moments,
                character_arcs,
                resolved_plots,
                open_threads,
                session_ids,
                status,
            }))
        }
        None => Ok(None),
    }
}

// ============================================================================
// Campaign Summary Commands
// ============================================================================

/// Generate a full campaign summary.
///
/// Aggregates all arc and session recaps into a comprehensive summary.
///
/// # Arguments
/// * `campaign_id` - The campaign ID
///
/// # Returns
/// The campaign summary text
#[tauri::command]
pub async fn generate_campaign_summary(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    info!(campaign_id = %campaign_id, "Generating campaign summary");

    let generator = get_recap_generator(&state);
    generator
        .generate_campaign_summary(&campaign_id)
        .await
        .map_err(recap_err_to_string)
}

// ============================================================================
// PC Knowledge Filtering Commands
// ============================================================================

/// Filter a recap by PC knowledge.
///
/// Returns a version of the recap showing only what a specific PC knows.
///
/// # Arguments
/// * `recap_id` - The recap ID
/// * `character_id` - The character/PC ID
///
/// # Returns
/// The filtered recap
#[tauri::command]
pub async fn filter_recap_by_pc(
    recap_id: String,
    character_id: String,
    state: State<'_, AppState>,
) -> Result<FilteredRecap, String> {
    debug!(recap_id = %recap_id, character_id = %character_id, "Filtering recap by PC");

    let generator = get_recap_generator(&state);
    generator
        .filter_recap_by_pc(&recap_id, &character_id)
        .await
        .map_err(recap_err_to_string)
}

/// Set PC knowledge for a recap.
///
/// Defines what NPCs, locations, and events a character knows about.
///
/// # Arguments
/// * `recap_id` - The recap ID
/// * `character_id` - The character/PC ID
/// * `knows_npcs` - List of NPC IDs the character knows
/// * `knows_locations` - List of location IDs the character knows
/// * `knows_events` - List of event indices the character witnessed
/// * `private_notes` - Optional private notes for this character
#[tauri::command]
pub async fn set_pc_knowledge(
    recap_id: String,
    character_id: String,
    knows_npcs: Vec<String>,
    knows_locations: Vec<String>,
    knows_events: Vec<String>,
    private_notes: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!(recap_id = %recap_id, character_id = %character_id, "Setting PC knowledge");

    let generator = get_recap_generator(&state);

    let filter = PCKnowledgeFilter {
        character_id: character_id.clone(),
        knows_npcs,
        knows_locations,
        knows_events,
        private_notes,
    };

    generator
        .set_pc_knowledge(&recap_id, &character_id, filter)
        .await
        .map_err(recap_err_to_string)
}

/// Get PC knowledge for a recap.
///
/// # Arguments
/// * `recap_id` - The recap ID
/// * `character_id` - The character/PC ID
///
/// # Returns
/// The PC knowledge filter
#[tauri::command]
pub async fn get_pc_knowledge(
    recap_id: String,
    character_id: String,
    state: State<'_, AppState>,
) -> Result<Option<PCKnowledgeFilter>, String> {
    debug!(recap_id = %recap_id, character_id = %character_id, "Getting PC knowledge");

    let pool = Arc::new(state.database.pool().clone());

    let record: Option<crate::database::PCKnowledgeFilterRecord> = sqlx::query_as(
        "SELECT * FROM pc_knowledge_filters WHERE recap_id = ? AND character_id = ?"
    )
    .bind(&recap_id)
    .bind(&character_id)
    .fetch_optional(pool.as_ref())
    .await
    .map_err(|e| e.to_string())?;

    Ok(record.map(|r| {
        let knows_npcs = r.knows_npc_ids_vec();
        let knows_locations = r.knows_location_ids_vec();
        let knows_events = r.knows_event_ids_vec();
        PCKnowledgeFilter {
            character_id: r.character_id,
            knows_npcs,
            knows_locations,
            knows_events,
            private_notes: r.private_notes,
        }
    }))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_recap_request_defaults() {
        // Use Default impl to test actual default values
        let request = GenerateRecapRequest {
            session_id: "session-1".to_string(),
            campaign_id: "campaign-1".to_string(),
            ..Default::default()
        };

        // Verify defaults from Default impl
        assert!(request.include_prose);
        assert!(request.include_bullets);
        assert!(request.extract_cliffhanger);
        assert_eq!(request.max_bullets, Some(10));
        assert_eq!(request.tone, Some("dramatic".to_string()));
    }
}
