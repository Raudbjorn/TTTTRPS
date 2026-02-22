//! Wizard Domain Types
//!
//! Defines the core domain types for the campaign creation wizard:
//! - [`PartialCampaign`]: Accumulating draft state shared by wizard and chat
//! - [`StepData`]: Per-step input data variants
//! - [`WizardState`]: Domain wrapper around database record
//! - [`WizardError`]: Error types for wizard operations
//!
//! # Architecture
//!
//! The wizard uses a state machine pattern where each step collects specific data
//! and transitions forward or backward through the wizard flow. The [`PartialCampaign`]
//! accumulates data from all steps, allowing both the wizard UI and AI conversation
//! to contribute to the final campaign configuration.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::core::campaign::wizard::{WizardState, StepData, BasicsData};
//!
//! // Create a new wizard state
//! let state = WizardState::new("wizard-123".to_string(), true);
//!
//! // Prepare step data
//! let basics = StepData::Basics(BasicsData {
//!     name: "My Campaign".to_string(),
//!     system: "dnd5e".to_string(),
//!     description: Some("A grand adventure".to_string()),
//! });
//!
//! // Step data indicates which step it belongs to
//! assert_eq!(basics.step(), WizardStep::Basics);
//! ```
//!
//! # Serialization
//!
//! All types implement `Serialize` and `Deserialize` for IPC communication
//! with the frontend and database persistence.

use serde::{Deserialize, Serialize};

use crate::database::{CampaignIntent, WizardStateRecord, WizardStep};

// ============================================================================
// Session Scope Types
// ============================================================================

/// Campaign session scope configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionScope {
    /// Expected number of sessions (e.g., 12 for a short campaign)
    pub session_count: Option<u32>,
    /// Typical session duration in hours
    pub session_duration_hours: Option<f32>,
    /// Campaign pacing preference
    pub pacing: Option<CampaignPacing>,
    /// Expected campaign duration in months
    pub duration_months: Option<u32>,
}

/// Campaign pacing preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignPacing {
    /// Fast-paced, action-heavy
    Fast,
    /// Balanced between action and roleplay
    Balanced,
    /// Slow, roleplay and exploration focused
    Slow,
    /// Sandbox with player-driven pacing
    Sandbox,
}

impl Default for CampaignPacing {
    fn default() -> Self {
        CampaignPacing::Balanced
    }
}

// ============================================================================
// Party Composition Types
// ============================================================================

/// Party composition configuration and analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartyComposition {
    /// Player characters (if known)
    pub characters: Vec<CharacterSummary>,
    /// Party size if characters not yet defined
    pub party_size: Option<u8>,
    /// Expected party level range
    pub level_range: Option<LevelRange>,
    /// Gap analysis results
    pub gap_analysis: Option<PartyGapAnalysis>,
}

/// Generate a default UUID for backwards compatibility
fn default_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Summary of a player character for party analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterSummary {
    /// Unique ID for stable removal without index issues
    #[serde(default = "default_uuid")]
    pub id: String,
    pub name: Option<String>,
    pub class: Option<String>,
    pub subclass: Option<String>,
    pub level: Option<u8>,
    pub role: Option<PartyRole>,
}

/// Party role for gap analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartyRole {
    Tank,
    Healer,
    DamageDealer,
    Support,
    Controller,
    Utility,
    Face,
    Scout,
}

/// Level range for campaign progression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelRange {
    pub start_level: u8,
    pub end_level: u8,
}

/// Gap analysis results for party composition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartyGapAnalysis {
    /// Missing roles that might cause problems
    pub missing_roles: Vec<PartyRole>,
    /// Suggested adjustments or NPC companions
    pub suggestions: Vec<String>,
    /// Overall party balance score (0-100)
    pub balance_score: u8,
}

// ============================================================================
// Arc Structure Types
// ============================================================================

/// Campaign arc structure configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArcStructure {
    /// Arc template being used
    pub template: Option<ArcTemplate>,
    /// Custom arc phases if not using template
    pub phases: Vec<ArcPhaseConfig>,
    /// Overall narrative structure
    pub narrative_style: Option<NarrativeStyle>,
}

/// Predefined arc templates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArcTemplate {
    /// Classic hero's journey structure
    HerosJourney,
    /// Three-act structure
    ThreeAct,
    /// Five-act structure
    FiveAct,
    /// Mystery/investigation structure
    Mystery,
    /// Political intrigue structure
    PoliticalIntrigue,
    /// Dungeon delve/exploration
    DungeonDelve,
    /// Sandbox with emergent story
    Sandbox,
    /// Custom arc structure
    Custom,
}

/// Configuration for a single arc phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcPhaseConfig {
    /// Unique ID for stable removal without index issues
    #[serde(default = "default_uuid")]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub estimated_sessions: Option<u32>,
}

/// Narrative style preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeStyle {
    /// Linear story with clear progression
    Linear,
    /// Branching choices with consequences
    Branching,
    /// Open sandbox with player-driven narrative
    Sandbox,
    /// Episodic adventures loosely connected
    Episodic,
}

// ============================================================================
// Initial Content Types
// ============================================================================

/// Initial content configuration for campaign kickoff
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitialContent {
    /// Starting location drafts
    pub locations: Vec<LocationDraft>,
    /// Initial NPC drafts
    pub npcs: Vec<NpcDraft>,
    /// Starting plot hooks
    pub plot_hooks: Vec<PlotHookDraft>,
}

/// Draft location for initial content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationDraft {
    /// Unique ID for stable removal without index issues
    #[serde(default = "default_uuid")]
    pub id: String,
    pub name: String,
    pub location_type: Option<String>,
    pub description: Option<String>,
    pub is_starting_location: bool,
}

/// Draft NPC for initial content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDraft {
    /// Unique ID for stable removal without index issues
    #[serde(default = "default_uuid")]
    pub id: String,
    pub name: String,
    pub role: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
}

/// Draft plot hook for initial content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotHookDraft {
    /// Unique ID for stable removal without index issues
    #[serde(default = "default_uuid")]
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub hook_type: Option<PlotHookType>,
}

/// Type of plot hook
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlotHookType {
    /// Main quest hook
    MainQuest,
    /// Side quest opportunity
    SideQuest,
    /// Character background hook
    CharacterTie,
    /// World event hook
    WorldEvent,
    /// Mystery/investigation hook
    Mystery,
}

// ============================================================================
// PartialCampaign - Shared Draft State
// ============================================================================

/// Shared draft state for campaign creation.
/// Both wizard steps and conversation suggestions update this.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartialCampaign {
    // Basics step
    pub name: Option<String>,
    pub system: Option<String>,
    pub description: Option<String>,

    // Intent step
    pub intent: Option<CampaignIntent>,

    // Scope step
    pub session_scope: Option<SessionScope>,

    // Players step
    pub player_count: Option<u8>,
    pub experience_level: Option<ExperienceLevel>,

    // Party composition step
    pub party_composition: Option<PartyComposition>,

    // Arc structure step
    pub arc_structure: Option<ArcStructure>,

    // Initial content step
    pub initial_content: Option<InitialContent>,
}

/// Player experience level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceLevel {
    /// New to TTRPGs
    Beginner,
    /// Some experience
    Intermediate,
    /// Experienced players
    Experienced,
    /// Mix of experience levels
    Mixed,
}

impl PartialCampaign {
    /// Create a new empty partial campaign
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the basics step is complete (minimum required fields)
    pub fn has_basics(&self) -> bool {
        self.name.is_some() && self.system.is_some()
    }

    /// Check if the players step is complete
    pub fn has_players(&self) -> bool {
        self.player_count.is_some()
    }

    /// Validate that all required fields for campaign creation are present
    pub fn validate_for_completion(&self) -> Result<(), WizardValidationError> {
        if self.name.is_none() {
            return Err(WizardValidationError::MissingField("name".to_string()));
        }
        if self.system.is_none() {
            return Err(WizardValidationError::MissingField("system".to_string()));
        }
        if self.player_count.is_none() {
            return Err(WizardValidationError::MissingField("player_count".to_string()));
        }
        Ok(())
    }
}

// ============================================================================
// StepData - Per-Step Input
// ============================================================================

/// Per-step input data for wizard advancement.
/// Each variant contains the data collected at that step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "step", content = "data")]
pub enum StepData {
    /// Basic campaign information
    Basics(BasicsData),
    /// Campaign intent and creative vision
    Intent(IntentData),
    /// Session scope and pacing
    Scope(ScopeData),
    /// Player information
    Players(PlayersData),
    /// Party composition (optional)
    PartyComposition(PartyCompositionData),
    /// Arc structure (optional)
    ArcStructure(ArcStructureData),
    /// Initial content (optional)
    InitialContent(InitialContentData),
    /// Review confirmation (no additional data)
    Review,
}

impl StepData {
    /// Get the wizard step this data corresponds to
    pub fn step(&self) -> WizardStep {
        match self {
            StepData::Basics(_) => WizardStep::Basics,
            StepData::Intent(_) => WizardStep::Intent,
            StepData::Scope(_) => WizardStep::Scope,
            StepData::Players(_) => WizardStep::Players,
            StepData::PartyComposition(_) => WizardStep::PartyComposition,
            StepData::ArcStructure(_) => WizardStep::ArcStructure,
            StepData::InitialContent(_) => WizardStep::InitialContent,
            StepData::Review => WizardStep::Review,
        }
    }
}

/// Data for the Basics step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicsData {
    pub name: String,
    pub system: String,
    pub description: Option<String>,
}

/// Data for the Intent step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentData {
    pub fantasy: String,
    pub player_experiences: Vec<String>,
    pub constraints: Vec<String>,
    pub themes: Vec<String>,
    pub tone_keywords: Vec<String>,
    pub avoid: Vec<String>,
}

impl IntentData {
    /// Convert to CampaignIntent
    pub fn to_intent(&self) -> CampaignIntent {
        CampaignIntent {
            fantasy: self.fantasy.clone(),
            player_experiences: self.player_experiences.clone(),
            constraints: self.constraints.clone(),
            themes: self.themes.clone(),
            tone_keywords: self.tone_keywords.clone(),
            avoid: self.avoid.clone(),
        }
    }
}

/// Data for the Scope step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeData {
    pub session_count: Option<u32>,
    pub session_duration_hours: Option<f32>,
    pub pacing: Option<CampaignPacing>,
    pub duration_months: Option<u32>,
}

impl ScopeData {
    /// Convert to SessionScope
    pub fn to_scope(&self) -> SessionScope {
        SessionScope {
            session_count: self.session_count,
            session_duration_hours: self.session_duration_hours,
            pacing: self.pacing,
            duration_months: self.duration_months,
        }
    }
}

/// Data for the Players step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayersData {
    pub player_count: u8,
    pub experience_level: Option<ExperienceLevel>,
}

/// Data for the PartyComposition step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyCompositionData {
    pub characters: Vec<CharacterSummary>,
    pub party_size: Option<u8>,
    pub level_range: Option<LevelRange>,
}

impl PartyCompositionData {
    /// Convert to PartyComposition
    pub fn to_composition(&self) -> PartyComposition {
        PartyComposition {
            characters: self.characters.clone(),
            party_size: self.party_size,
            level_range: self.level_range.clone(),
            gap_analysis: None,
        }
    }
}

/// Data for the ArcStructure step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcStructureData {
    pub template: Option<ArcTemplate>,
    pub phases: Vec<ArcPhaseConfig>,
    pub narrative_style: Option<NarrativeStyle>,
}

impl ArcStructureData {
    /// Convert to ArcStructure
    pub fn to_arc_structure(&self) -> ArcStructure {
        ArcStructure {
            template: self.template,
            phases: self.phases.clone(),
            narrative_style: self.narrative_style,
        }
    }
}

/// Data for the InitialContent step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialContentData {
    pub locations: Vec<LocationDraft>,
    pub npcs: Vec<NpcDraft>,
    pub plot_hooks: Vec<PlotHookDraft>,
}

impl InitialContentData {
    /// Convert to InitialContent
    pub fn to_initial_content(&self) -> InitialContent {
        InitialContent {
            locations: self.locations.clone(),
            npcs: self.npcs.clone(),
            plot_hooks: self.plot_hooks.clone(),
        }
    }
}

// ============================================================================
// WizardState - Domain Wrapper
// ============================================================================

/// Domain wrapper around WizardStateRecord with parsed fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardState {
    /// Unique identifier
    pub id: String,
    /// Current step in the wizard
    pub current_step: WizardStep,
    /// Steps that have been completed
    pub completed_steps: Vec<WizardStep>,
    /// Accumulated campaign draft
    pub campaign_draft: PartialCampaign,
    /// Associated conversation thread ID (for AI-assisted mode)
    pub conversation_thread_id: Option<String>,
    /// Whether AI assistance is enabled
    pub ai_assisted: bool,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
    /// Last auto-save timestamp
    pub auto_saved_at: Option<String>,
}

impl WizardState {
    /// Create a new wizard state
    pub fn new(id: String, ai_assisted: bool) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            current_step: WizardStep::Basics,
            completed_steps: Vec::new(),
            campaign_draft: PartialCampaign::new(),
            conversation_thread_id: None,
            ai_assisted,
            created_at: now.clone(),
            updated_at: now,
            auto_saved_at: None,
        }
    }

    /// Create from database record
    pub fn from_record(record: WizardStateRecord) -> Result<Self, WizardValidationError> {
        let current_step = record.current_step_enum()
            .map_err(|e| WizardValidationError::InvalidStep(e))?;

        let completed_steps: Vec<WizardStep> = record
            .completed_steps_vec()
            .into_iter()
            .filter_map(|s| WizardStep::try_from(s.as_str()).ok())
            .collect();

        let campaign_draft: PartialCampaign = match serde_json::from_str(&record.campaign_draft) {
            Ok(draft) => draft,
            Err(e) => {
                tracing::warn!(
                    wizard_id = %record.id,
                    error = %e,
                    "Failed to parse campaign_draft, using defaults"
                );
                PartialCampaign::default()
            }
        };

        // Read ai_assisted before moving fields
        let ai_assisted = record.is_ai_assisted();

        Ok(Self {
            id: record.id,
            current_step,
            completed_steps,
            campaign_draft,
            conversation_thread_id: record.conversation_thread_id,
            ai_assisted,
            created_at: record.created_at,
            updated_at: record.updated_at,
            auto_saved_at: record.auto_saved_at,
        })
    }

    /// Convert to database record
    pub fn to_record(&self) -> WizardStateRecord {
        let completed_steps_json = serde_json::to_string(
            &self.completed_steps.iter().map(|s| s.as_str()).collect::<Vec<_>>()
        ).unwrap_or_else(|_| "[]".to_string());

        let campaign_draft_json = serde_json::to_string(&self.campaign_draft)
            .unwrap_or_else(|_| "{}".to_string());

        WizardStateRecord {
            id: self.id.clone(),
            current_step: self.current_step.to_string(),
            completed_steps: completed_steps_json,
            campaign_draft: campaign_draft_json,
            conversation_thread_id: self.conversation_thread_id.clone(),
            ai_assisted: if self.ai_assisted { 1 } else { 0 },
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
            auto_saved_at: self.auto_saved_at.clone(),
        }
    }

    /// Check if a step has been completed
    pub fn is_step_completed(&self, step: WizardStep) -> bool {
        self.completed_steps.contains(&step)
    }

    /// Check if the wizard can advance to the next step
    pub fn can_advance(&self) -> bool {
        self.current_step.next().is_some()
    }

    /// Check if the wizard can go back to the previous step
    pub fn can_go_back(&self) -> bool {
        self.current_step.previous().is_some()
    }

    /// Check if the current step can be skipped
    pub fn can_skip_current(&self) -> bool {
        self.current_step.is_skippable()
    }

    /// Check if the wizard is ready for completion
    pub fn is_ready_for_completion(&self) -> bool {
        self.current_step == WizardStep::Review
    }

    /// Validate the current step's data
    pub fn validate_current_step(&self) -> Result<(), WizardValidationError> {
        match self.current_step {
            WizardStep::Basics => {
                if !self.campaign_draft.has_basics() {
                    return Err(WizardValidationError::IncompleteStep(
                        "Basics requires name and system".to_string()
                    ));
                }
            }
            WizardStep::Players => {
                if !self.campaign_draft.has_players() {
                    return Err(WizardValidationError::IncompleteStep(
                        "Players requires player count".to_string()
                    ));
                }
            }
            WizardStep::Review => {
                self.campaign_draft.validate_for_completion()?;
            }
            // Optional steps don't require validation
            _ => {}
        }
        Ok(())
    }

    /// Get progress percentage (0-100)
    pub fn progress_percent(&self) -> u8 {
        let total_steps = 8; // Total number of wizard steps
        let completed = self.completed_steps.len();
        ((completed as f32 / total_steps as f32) * 100.0) as u8
    }
}

/// Summary of a wizard state for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardSummary {
    pub id: String,
    pub campaign_name: Option<String>,
    pub current_step: WizardStep,
    pub progress_percent: u8,
    pub ai_assisted: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&WizardState> for WizardSummary {
    fn from(state: &WizardState) -> Self {
        Self {
            id: state.id.clone(),
            campaign_name: state.campaign_draft.name.clone(),
            current_step: state.current_step,
            progress_percent: state.progress_percent(),
            ai_assisted: state.ai_assisted,
            created_at: state.created_at.clone(),
            updated_at: state.updated_at.clone(),
        }
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during wizard operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum WizardError {
    #[error("Wizard not found: {0}")]
    NotFound(String),

    #[error("Invalid step transition: cannot move from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Validation error: {0}")]
    Validation(#[from] WizardValidationError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Wizard already completed")]
    AlreadyCompleted,

    #[error("Cannot skip non-skippable step: {0}")]
    CannotSkip(String),
}

/// Validation errors for wizard data
#[derive(Debug, Clone, thiserror::Error)]
pub enum WizardValidationError {
    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid step: {0}")]
    InvalidStep(String),

    #[error("Incomplete step: {0}")]
    IncompleteStep(String),

    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },
}

// ============================================================================
// Step Transition Logic
// ============================================================================

/// Validates that a step transition is allowed
pub fn validate_step_transition(from: WizardStep, to: WizardStep) -> Result<(), WizardError> {
    // Can always stay on current step
    if from == to {
        return Ok(());
    }

    // Check if moving forward
    if let Some(next) = from.next() {
        if next == to {
            return Ok(());
        }
    }

    // Check if moving backward
    if let Some(prev) = from.previous() {
        if prev == to {
            return Ok(());
        }
    }

    // Allow skipping to next non-skipped step if current is skippable
    if from.is_skippable() {
        if let Some(next) = from.next() {
            if to == next || (next.is_skippable() && next.next() == Some(to)) {
                return Ok(());
            }
        }
    }

    Err(WizardError::InvalidTransition {
        from: from.to_string(),
        to: to.to_string(),
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_campaign_validation() {
        let mut draft = PartialCampaign::new();

        // Should fail without required fields
        assert!(draft.validate_for_completion().is_err());

        // Add required fields
        draft.name = Some("Test Campaign".to_string());
        draft.system = Some("dnd5e".to_string());
        draft.player_count = Some(4);

        // Should now pass
        assert!(draft.validate_for_completion().is_ok());
    }

    #[test]
    fn test_wizard_state_creation() {
        let state = WizardState::new("test-id".to_string(), true);

        assert_eq!(state.current_step, WizardStep::Basics);
        assert!(state.completed_steps.is_empty());
        assert!(state.ai_assisted);
        assert_eq!(state.progress_percent(), 0);
    }

    #[test]
    fn test_wizard_state_navigation() {
        let mut state = WizardState::new("test-id".to_string(), false);

        // Should be able to advance from first step
        assert!(state.can_advance());

        // Should not be able to go back from first step
        assert!(!state.can_go_back());

        // First step (Basics) is not skippable
        assert!(!state.can_skip_current());

        // Move to Intent (which is skippable)
        state.current_step = WizardStep::Intent;
        assert!(state.can_skip_current());
    }

    #[test]
    fn test_step_transition_validation() {
        // Valid forward transition
        assert!(validate_step_transition(WizardStep::Basics, WizardStep::Intent).is_ok());

        // Valid backward transition
        assert!(validate_step_transition(WizardStep::Intent, WizardStep::Basics).is_ok());

        // Stay on same step
        assert!(validate_step_transition(WizardStep::Basics, WizardStep::Basics).is_ok());

        // Invalid skip from non-skippable
        assert!(validate_step_transition(WizardStep::Basics, WizardStep::Scope).is_err());

        // Invalid jump across multiple steps
        assert!(validate_step_transition(WizardStep::Basics, WizardStep::Review).is_err());
    }

    #[test]
    fn test_step_data_step_extraction() {
        let basics = StepData::Basics(BasicsData {
            name: "Test".to_string(),
            system: "dnd5e".to_string(),
            description: None,
        });
        assert_eq!(basics.step(), WizardStep::Basics);

        let intent = StepData::Intent(IntentData {
            fantasy: "Epic adventure".to_string(),
            player_experiences: vec![],
            constraints: vec![],
            themes: vec![],
            tone_keywords: vec![],
            avoid: vec![],
        });
        assert_eq!(intent.step(), WizardStep::Intent);
    }

    #[test]
    fn test_wizard_summary_from_state() {
        let mut state = WizardState::new("test-id".to_string(), true);
        state.campaign_draft.name = Some("My Campaign".to_string());
        state.completed_steps.push(WizardStep::Basics);

        let summary = WizardSummary::from(&state);
        assert_eq!(summary.id, "test-id");
        assert_eq!(summary.campaign_name, Some("My Campaign".to_string()));
        assert!(summary.ai_assisted);
        assert_eq!(summary.progress_percent, 12); // 1/8 steps = 12.5%
    }

    #[test]
    fn test_intent_data_conversion() {
        let intent_data = IntentData {
            fantasy: "Dark fantasy".to_string(),
            player_experiences: vec!["mystery".to_string()],
            constraints: vec!["no gore".to_string()],
            themes: vec!["redemption".to_string()],
            tone_keywords: vec!["dark".to_string()],
            avoid: vec!["humor".to_string()],
        };

        let intent = intent_data.to_intent();
        assert_eq!(intent.fantasy, "Dark fantasy");
        assert_eq!(intent.player_experiences, vec!["mystery"]);
    }
}
