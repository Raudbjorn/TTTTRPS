//! Generation Commands Module
//!
//! Tauri commands for the content generation orchestration layer.
//! Provides the frontend interface for generating campaign content
//! (characters, NPCs, sessions, arcs, party analysis).

// TODO: Arc and RwLock will be needed when get_orchestrator is restored
#[allow(unused_imports)]
use std::sync::Arc;
use tauri::State;
#[allow(unused_imports)]
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::commands::AppState;
#[allow(unused_imports)]
use crate::core::campaign::generation::{
    AcceptanceManager, ArcDraft, ArcGenerationRequest, ArcGenerator, ArcTemplateType,
    CharacterDraft, CharacterGenerationRequest, CharacterGenerator,
    EncounterDifficulty, GapAnalysis, GenerationOrchestrator,
    GenerationRequest, GenerationResponse, GenerationType, NpcDraft, NpcGenerationRequest,
    NpcGenerator, NpcImportance, PacingTemplate, PartyAnalysisRequest, PartyAnalyzer,
    PartySuggestion, SessionGenerationRequest, SessionGenerator, SessionPlanDraft,
    TemplateRegistry,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert generation errors to String for Tauri IPC
fn gen_err_to_string(err: impl std::fmt::Display) -> String {
    let msg = err.to_string();
    error!(error = %msg, "Generation command error");
    msg
}

/// Create a GenerationOrchestrator from AppState
///
/// TODO: GenerationOrchestrator::new expects Arc<SearchClient> which was the
/// HTTP-based meilisearch_sdk client. With embedded MeilisearchLib, the orchestrator
/// needs to be updated to accept EmbeddedSearch or remove the search_client dependency
/// (it's marked #[allow(dead_code)] in the struct, so it may not be actively used).
async fn get_orchestrator(
    _state: &State<'_, AppState>,
) -> Result<GenerationOrchestrator, String> {
    // TODO: GenerationOrchestrator::new expects Arc<SearchClient> which was the
    // HTTP-based meilisearch_sdk client. With embedded MeilisearchLib, the orchestrator
    // needs to be updated to accept EmbeddedSearch or remove the search_client dependency
    // (it's marked #[allow(dead_code)] in the struct, so it may not be actively used).
    //
    // Original code that needs migration:
    // let llm_router = state.llm_router.read().await;
    // let search_client = state.search_client.clone();
    // let database = state.database.clone();
    // let registry = TemplateRegistry::with_defaults().await;
    // Ok(GenerationOrchestrator::new(
    //     Arc::new(RwLock::new(llm_router.clone())),
    //     search_client,
    //     registry,
    //     database,
    // ))

    Err("GenerationOrchestrator not yet migrated to MeilisearchLib. The search_client dependency needs to be removed or replaced with EmbeddedSearch.".to_string())
}

/// Create an AcceptanceManager from AppState
#[allow(dead_code)]
fn get_acceptance_manager(state: &State<'_, AppState>) -> AcceptanceManager {
    AcceptanceManager::new(state.database.clone())
}

// ============================================================================
// Character Generation Commands
// ============================================================================

/// Generate a character background.
///
/// Creates a rich backstory with relationships, locations, plot hooks, and secrets.
///
/// # Arguments
/// * `request` - Character generation request with name, class, race, and player requests
///
/// # Returns
/// Generated character draft with extracted entities
#[tauri::command]
pub async fn generate_character_background(
    request: CharacterGenerationRequest,
    state: State<'_, AppState>,
) -> Result<GenerationResponse, String> {
    info!(
        character_name = %request.character_name,
        class = %request.character_class,
        "Generating character background"
    );

    let orchestrator = get_orchestrator(&state).await?;
    let gen_request = request.to_generation_request();

    orchestrator
        .generate(gen_request)
        .await
        .map_err(gen_err_to_string)
}

/// Parse a character generation response.
///
/// Extracts structured data from the raw LLM response.
///
/// # Arguments
/// * `response` - Raw JSON response from generation
///
/// # Returns
/// Parsed character draft
#[tauri::command]
pub fn parse_character_response(response: serde_json::Value) -> Result<CharacterDraft, String> {
    debug!("Parsing character response");
    CharacterGenerator::parse_response(&response).map_err(gen_err_to_string)
}

// ============================================================================
// NPC Generation Commands
// ============================================================================

/// Generate an NPC with AI assistance.
///
/// Creates an NPC with personality, motivations, relationships, and optional stat block.
/// Uses the generation orchestrator for LLM-powered content creation.
///
/// # Arguments
/// * `request` - NPC generation request with role, description, and importance level
///
/// # Returns
/// Generated NPC draft
#[tauri::command]
pub async fn generate_npc_with_ai(
    request: NpcGenerationRequest,
    state: State<'_, AppState>,
) -> Result<GenerationResponse, String> {
    info!(
        npc_role = %request.npc_role,
        importance = ?request.importance,
        "Generating NPC with AI"
    );

    let orchestrator = get_orchestrator(&state).await?;
    let gen_request = request.to_generation_request();

    orchestrator
        .generate(gen_request)
        .await
        .map_err(gen_err_to_string)
}

/// Parse an NPC generation response.
///
/// # Arguments
/// * `response` - Raw JSON response from generation
///
/// # Returns
/// Parsed NPC draft
#[tauri::command]
pub fn parse_npc_response(response: serde_json::Value) -> Result<NpcDraft, String> {
    debug!("Parsing NPC response");
    NpcGenerator::parse_response(&response).map_err(gen_err_to_string)
}

// ============================================================================
// Session Plan Generation Commands
// ============================================================================

/// Generate a session plan.
///
/// Creates a structured session plan with narrative beats, encounters, and pacing.
///
/// # Arguments
/// * `request` - Session generation request with objective, duration, and pacing style
///
/// # Returns
/// Generated session plan draft
#[tauri::command]
pub async fn generate_session_plan(
    request: SessionGenerationRequest,
    state: State<'_, AppState>,
) -> Result<GenerationResponse, String> {
    info!(
        objective = %request.objective,
        duration_hours = request.session_duration_hours,
        pacing = ?request.pacing_style,
        "Generating session plan"
    );

    let orchestrator = get_orchestrator(&state).await?;
    let gen_request = request.to_generation_request();

    orchestrator
        .generate(gen_request)
        .await
        .map_err(gen_err_to_string)
}

/// Parse a session plan generation response.
///
/// # Arguments
/// * `response` - Raw JSON response from generation
///
/// # Returns
/// Parsed session plan draft
#[tauri::command]
pub fn parse_session_response(response: serde_json::Value) -> Result<SessionPlanDraft, String> {
    debug!("Parsing session plan response");
    SessionGenerator::parse_response(&response).map_err(gen_err_to_string)
}

/// Calculate encounter difficulty.
///
/// Uses simplified D&D 5e encounter difficulty calculation.
///
/// # Arguments
/// * `party_level` - Average party level
/// * `party_size` - Number of party members
/// * `enemy_cr` - Challenge rating of enemies
/// * `enemy_count` - Number of enemies
///
/// # Returns
/// Calculated encounter difficulty
#[tauri::command]
pub fn calculate_encounter_difficulty(
    party_level: u8,
    party_size: u8,
    enemy_cr: f32,
    enemy_count: u8,
) -> EncounterDifficulty {
    SessionGenerator::calculate_encounter_difficulty(party_level, party_size, enemy_cr, enemy_count)
}

// ============================================================================
// Arc Generation Commands
// ============================================================================

/// Generate a narrative arc outline.
///
/// Creates a multi-session arc with phases, milestones, and tension curve.
///
/// # Arguments
/// * `request` - Arc generation request with theme, antagonist concept, and template
///
/// # Returns
/// Generated arc draft
#[tauri::command]
pub async fn generate_arc(
    request: ArcGenerationRequest,
    state: State<'_, AppState>,
) -> Result<GenerationResponse, String> {
    info!(
        arc_concept = %request.arc_concept,
        arc_type = ?request.arc_type,
        "Generating narrative arc"
    );

    let orchestrator = get_orchestrator(&state).await?;
    let gen_request = request.to_generation_request();

    orchestrator
        .generate(gen_request)
        .await
        .map_err(gen_err_to_string)
}

/// Parse an arc generation response.
///
/// # Arguments
/// * `response` - Raw JSON response from generation
///
/// # Returns
/// Parsed arc draft
#[tauri::command]
pub fn parse_arc_response(response: serde_json::Value) -> Result<ArcDraft, String> {
    debug!("Parsing arc response");
    ArcGenerator::parse_response(&response).map_err(gen_err_to_string)
}

// ============================================================================
// Party Analysis Commands
// ============================================================================

/// Analyze party composition.
///
/// Identifies gaps, strengths, and provides recommendations for party balance.
///
/// # Arguments
/// * `request` - Party analysis request with character details
///
/// # Returns
/// Generated party analysis with suggestions
#[tauri::command]
pub async fn analyze_party_composition(
    request: PartyAnalysisRequest,
    state: State<'_, AppState>,
) -> Result<GenerationResponse, String> {
    info!(
        campaign_type = ?request.campaign_type,
        party_size = request.party_details.len(),
        "Analyzing party composition"
    );

    let orchestrator = get_orchestrator(&state).await?;
    let gen_request = request.to_generation_request();

    orchestrator
        .generate(gen_request)
        .await
        .map_err(gen_err_to_string)
}

/// Perform static party gap analysis.
///
/// A quick analysis without LLM call, based on role coverage.
///
/// # Arguments
/// * `request` - Party analysis request with character details
///
/// # Returns
/// Gap analysis with strengths, weaknesses, and recommendations
#[tauri::command]
pub fn analyze_party_gaps(request: PartyAnalysisRequest) -> GapAnalysis {
    debug!(party_size = request.party_details.len(), "Analyzing party gaps");
    PartyAnalyzer::static_analysis(&request.party_details)
}

/// Parse a party analysis response.
///
/// # Arguments
/// * `response` - Raw JSON response from generation
///
/// # Returns
/// Parsed party suggestions
#[tauri::command]
pub fn parse_party_response(response: serde_json::Value) -> Result<PartySuggestion, String> {
    debug!("Parsing party analysis response");
    PartyAnalyzer::parse_response(&response).map_err(gen_err_to_string)
}

// ============================================================================
// Generic Generation Commands
// ============================================================================

/// Generate content using the orchestrator.
///
/// Generic generation endpoint for any supported content type.
///
/// # Arguments
/// * `request` - Generation request with type and variables
///
/// # Returns
/// Generated content response
#[tauri::command]
pub async fn generate_content(
    request: GenerationRequest,
    state: State<'_, AppState>,
) -> Result<GenerationResponse, String> {
    info!(
        generation_type = ?request.generation_type,
        campaign_id = ?request.campaign_id,
        "Generating content"
    );

    let orchestrator = get_orchestrator(&state).await?;

    orchestrator
        .generate(request)
        .await
        .map_err(gen_err_to_string)
}

/// List available generation types.
///
/// Returns all supported content generation types.
#[tauri::command]
pub fn list_generation_types() -> Vec<GenerationType> {
    vec![
        GenerationType::CharacterBackground,
        GenerationType::Npc,
        GenerationType::SessionPlan,
        GenerationType::ArcOutline,
        GenerationType::PartyAnalysis,
        GenerationType::Location,
        GenerationType::QuestHook,
        GenerationType::Encounter,
    ]
}

/// List available pacing templates.
///
/// Returns all session pacing templates with their encounter distributions.
#[tauri::command]
pub fn list_pacing_templates() -> Vec<PacingTemplateInfo> {
    vec![
        PacingTemplateInfo {
            template: PacingTemplate::CombatHeavy,
            name: "Combat Heavy".to_string(),
            description: "Heavy on combat encounters (60% combat)".to_string(),
        },
        PacingTemplateInfo {
            template: PacingTemplate::RoleplayFocused,
            name: "Roleplay Focused".to_string(),
            description: "Focused on roleplay and social encounters (55% social)".to_string(),
        },
        PacingTemplateInfo {
            template: PacingTemplate::Exploration,
            name: "Exploration".to_string(),
            description: "Emphasis on exploration and discovery (45% exploration)".to_string(),
        },
        PacingTemplateInfo {
            template: PacingTemplate::Mixed,
            name: "Mixed".to_string(),
            description: "Balanced mix of all elements".to_string(),
        },
        PacingTemplateInfo {
            template: PacingTemplate::Dramatic,
            name: "Dramatic".to_string(),
            description: "High tension, dramatic confrontations".to_string(),
        },
        PacingTemplateInfo {
            template: PacingTemplate::Mystery,
            name: "Mystery".to_string(),
            description: "Puzzle and mystery solving focus (35% puzzle)".to_string(),
        },
    ]
}

/// List available arc templates.
///
/// Returns all narrative arc templates.
#[tauri::command]
pub fn list_arc_templates() -> Vec<ArcTemplateInfo> {
    vec![
        ArcTemplateInfo {
            template: ArcTemplateType::HerosJourney,
            name: "Hero's Journey".to_string(),
            description: "Classic monomyth structure with departure, initiation, return".to_string(),
            phases: vec![
                "Call to Adventure".to_string(),
                "Crossing the Threshold".to_string(),
                "Road of Trials".to_string(),
                "The Ordeal".to_string(),
                "Return with the Elixir".to_string(),
            ],
        },
        ArcTemplateInfo {
            template: ArcTemplateType::ThreeAct,
            name: "Three Act".to_string(),
            description: "Traditional three-act dramatic structure".to_string(),
            phases: vec![
                "Setup".to_string(),
                "Confrontation".to_string(),
                "Resolution".to_string(),
            ],
        },
        ArcTemplateInfo {
            template: ArcTemplateType::FiveAct,
            name: "Five Act".to_string(),
            description: "Shakespearean five-act structure".to_string(),
            phases: vec![
                "Exposition".to_string(),
                "Rising Action".to_string(),
                "Climax".to_string(),
                "Falling Action".to_string(),
                "Denouement".to_string(),
            ],
        },
        ArcTemplateInfo {
            template: ArcTemplateType::Mystery,
            name: "Mystery".to_string(),
            description: "Investigation-focused arc with clues and revelations".to_string(),
            phases: vec![
                "The Hook".to_string(),
                "Investigation".to_string(),
                "Complications".to_string(),
                "Revelation".to_string(),
                "Confrontation".to_string(),
            ],
        },
        ArcTemplateInfo {
            template: ArcTemplateType::DungeonDelve,
            name: "Dungeon Delve".to_string(),
            description: "Classic dungeon exploration with progressive challenges".to_string(),
            phases: vec![
                "Discovery".to_string(),
                "Exploration".to_string(),
                "Deep Descent".to_string(),
                "The Heart".to_string(),
                "Escape".to_string(),
            ],
        },
        ArcTemplateInfo {
            template: ArcTemplateType::PoliticalIntrigue,
            name: "Political Intrigue".to_string(),
            description: "Court politics, alliances, and betrayals".to_string(),
            phases: vec![
                "Entry to Court".to_string(),
                "Building Alliances".to_string(),
                "The Plot Thickens".to_string(),
                "Betrayal".to_string(),
                "Reckoning".to_string(),
            ],
        },
        ArcTemplateInfo {
            template: ArcTemplateType::Custom,
            name: "Custom".to_string(),
            description: "Define your own arc structure".to_string(),
            phases: vec![],
        },
    ]
}

/// List NPC importance levels.
///
/// Returns all NPC importance levels with descriptions.
#[tauri::command]
pub fn list_npc_importance_levels() -> Vec<NpcImportanceInfo> {
    vec![
        NpcImportanceInfo {
            level: NpcImportance::Minor,
            name: "Minor".to_string(),
            description: "Background NPCs with minimal detail".to_string(),
            include_stats: false,
        },
        NpcImportanceInfo {
            level: NpcImportance::Supporting,
            name: "Supporting".to_string(),
            description: "Supporting NPCs with moderate detail".to_string(),
            include_stats: false,
        },
        NpcImportanceInfo {
            level: NpcImportance::Major,
            name: "Major".to_string(),
            description: "Major NPCs with full detail and stats".to_string(),
            include_stats: true,
        },
        NpcImportanceInfo {
            level: NpcImportance::Key,
            name: "Key".to_string(),
            description: "Key antagonists or allies with extensive detail".to_string(),
            include_stats: true,
        },
    ]
}

// ============================================================================
// Info Types for Frontend
// ============================================================================

/// Information about a pacing template
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PacingTemplateInfo {
    pub template: PacingTemplate,
    pub name: String,
    pub description: String,
}

/// Information about an arc template
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArcTemplateInfo {
    pub template: ArcTemplateType,
    pub name: String,
    pub description: String,
    pub phases: Vec<String>,
}

/// Information about NPC importance level
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NpcImportanceInfo {
    pub level: NpcImportance,
    pub name: String,
    pub description: String,
    pub include_stats: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_generation_types() {
        let types = list_generation_types();
        assert!(!types.is_empty());
        assert!(types.contains(&GenerationType::Npc));
        assert!(types.contains(&GenerationType::SessionPlan));
    }

    #[test]
    fn test_list_pacing_templates() {
        let templates = list_pacing_templates();
        assert_eq!(templates.len(), 6);
    }

    #[test]
    fn test_list_arc_templates() {
        let templates = list_arc_templates();
        assert_eq!(templates.len(), 7);
    }

    #[test]
    fn test_list_npc_importance_levels() {
        let levels = list_npc_importance_levels();
        assert_eq!(levels.len(), 4);
        assert!(!levels[0].include_stats);
        assert!(levels[2].include_stats);
    }

    #[test]
    fn test_calculate_encounter_difficulty() {
        let diff = calculate_encounter_difficulty(5, 4, 0.5, 2);
        assert_eq!(diff, EncounterDifficulty::Easy);
    }
}
