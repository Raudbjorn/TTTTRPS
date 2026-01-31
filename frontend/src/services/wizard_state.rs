//! Campaign Wizard State Management
//!
//! Provides reactive state management for the campaign creation wizard.
//! Uses Leptos signals and context for component communication.
//!
//! # Architecture
//! - `WizardService` - Main service with IPC bindings
//! - `WizardContext` - Reactive state container for wizard UI
//! - Context provider pattern for component tree access

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};

use crate::bindings::invoke;

// ============================================================================
// Types - Mirror backend types for frontend use
// ============================================================================

/// Wizard step enum - mirrors backend WizardStep
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WizardStep {
    #[default]
    Basics,
    Intent,
    Scope,
    Players,
    PartyComposition,
    ArcStructure,
    InitialContent,
    Review,
}

impl WizardStep {
    pub fn label(&self) -> &'static str {
        match self {
            WizardStep::Basics => "Basics",
            WizardStep::Intent => "Creative Vision",
            WizardStep::Scope => "Campaign Scope",
            WizardStep::Players => "Players",
            WizardStep::PartyComposition => "Party Composition",
            WizardStep::ArcStructure => "Story Arc",
            WizardStep::InitialContent => "Initial Content",
            WizardStep::Review => "Review",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            WizardStep::Basics => "Name and game system",
            WizardStep::Intent => "Themes, tone, and creative goals",
            WizardStep::Scope => "Session count and pacing",
            WizardStep::Players => "Player count and experience",
            WizardStep::PartyComposition => "Party roles and composition",
            WizardStep::ArcStructure => "Narrative structure",
            WizardStep::InitialContent => "Starting NPCs and locations",
            WizardStep::Review => "Review and create",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            WizardStep::Basics => 0,
            WizardStep::Intent => 1,
            WizardStep::Scope => 2,
            WizardStep::Players => 3,
            WizardStep::PartyComposition => 4,
            WizardStep::ArcStructure => 5,
            WizardStep::InitialContent => 6,
            WizardStep::Review => 7,
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            WizardStep::Basics,
            WizardStep::Intent,
            WizardStep::Scope,
            WizardStep::Players,
            WizardStep::PartyComposition,
            WizardStep::ArcStructure,
            WizardStep::InitialContent,
            WizardStep::Review,
        ]
    }

    pub fn is_skippable(&self) -> bool {
        matches!(
            self,
            WizardStep::Intent
                | WizardStep::PartyComposition
                | WizardStep::ArcStructure
                | WizardStep::InitialContent
        )
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            WizardStep::Basics => Some(WizardStep::Intent),
            WizardStep::Intent => Some(WizardStep::Scope),
            WizardStep::Scope => Some(WizardStep::Players),
            WizardStep::Players => Some(WizardStep::PartyComposition),
            WizardStep::PartyComposition => Some(WizardStep::ArcStructure),
            WizardStep::ArcStructure => Some(WizardStep::InitialContent),
            WizardStep::InitialContent => Some(WizardStep::Review),
            WizardStep::Review => None,
        }
    }

    pub fn previous(&self) -> Option<Self> {
        match self {
            WizardStep::Basics => None,
            WizardStep::Intent => Some(WizardStep::Basics),
            WizardStep::Scope => Some(WizardStep::Intent),
            WizardStep::Players => Some(WizardStep::Scope),
            WizardStep::PartyComposition => Some(WizardStep::Players),
            WizardStep::ArcStructure => Some(WizardStep::PartyComposition),
            WizardStep::InitialContent => Some(WizardStep::ArcStructure),
            WizardStep::Review => Some(WizardStep::InitialContent),
        }
    }
}

/// Campaign pacing preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CampaignPacing {
    Fast,
    #[default]
    Balanced,
    Slow,
    Sandbox,
}

impl CampaignPacing {
    pub fn label(&self) -> &'static str {
        match self {
            CampaignPacing::Fast => "Fast-Paced",
            CampaignPacing::Balanced => "Balanced",
            CampaignPacing::Slow => "Slow & Deliberate",
            CampaignPacing::Sandbox => "Sandbox",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            CampaignPacing::Fast => "Action-heavy, quick progression",
            CampaignPacing::Balanced => "Mix of action and roleplay",
            CampaignPacing::Slow => "Deep roleplay and exploration",
            CampaignPacing::Sandbox => "Player-driven, emergent pacing",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            CampaignPacing::Fast,
            CampaignPacing::Balanced,
            CampaignPacing::Slow,
            CampaignPacing::Sandbox,
        ]
    }
}

/// Player experience level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExperienceLevel {
    Beginner,
    Intermediate,
    Experienced,
    #[default]
    Mixed,
}

impl ExperienceLevel {
    pub fn label(&self) -> &'static str {
        match self {
            ExperienceLevel::Beginner => "New Players",
            ExperienceLevel::Intermediate => "Some Experience",
            ExperienceLevel::Experienced => "Veterans",
            ExperienceLevel::Mixed => "Mixed Group",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            ExperienceLevel::Beginner,
            ExperienceLevel::Intermediate,
            ExperienceLevel::Experienced,
            ExperienceLevel::Mixed,
        ]
    }
}

/// Arc template for story structure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArcTemplate {
    #[default]
    HerosJourney,
    ThreeAct,
    FiveAct,
    Mystery,
    PoliticalIntrigue,
    DungeonDelve,
    Sandbox,
    Custom,
}

impl ArcTemplate {
    pub fn label(&self) -> &'static str {
        match self {
            ArcTemplate::HerosJourney => "Hero's Journey",
            ArcTemplate::ThreeAct => "Three-Act Structure",
            ArcTemplate::FiveAct => "Five-Act Structure",
            ArcTemplate::Mystery => "Mystery/Investigation",
            ArcTemplate::PoliticalIntrigue => "Political Intrigue",
            ArcTemplate::DungeonDelve => "Dungeon Delve",
            ArcTemplate::Sandbox => "Sandbox",
            ArcTemplate::Custom => "Custom",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ArcTemplate::HerosJourney => "Classic journey of transformation and growth",
            ArcTemplate::ThreeAct => "Setup, confrontation, resolution",
            ArcTemplate::FiveAct => "Exposition, rising action, climax, falling action, resolution",
            ArcTemplate::Mystery => "Clues, investigation, reveals",
            ArcTemplate::PoliticalIntrigue => "Factions, alliances, betrayals",
            ArcTemplate::DungeonDelve => "Exploration, encounters, treasures",
            ArcTemplate::Sandbox => "Player-driven emergent narrative",
            ArcTemplate::Custom => "Define your own structure",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            ArcTemplate::HerosJourney,
            ArcTemplate::ThreeAct,
            ArcTemplate::FiveAct,
            ArcTemplate::Mystery,
            ArcTemplate::PoliticalIntrigue,
            ArcTemplate::DungeonDelve,
            ArcTemplate::Sandbox,
            ArcTemplate::Custom,
        ]
    }
}

/// Session scope configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionScope {
    pub session_count: Option<u32>,
    pub session_duration_hours: Option<f32>,
    pub pacing: Option<CampaignPacing>,
    pub duration_months: Option<u32>,
}

/// Party composition
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartyComposition {
    pub characters: Vec<CharacterSummary>,
    pub party_size: Option<u8>,
    pub level_range: Option<LevelRange>,
    pub gap_analysis: Option<PartyGapAnalysis>,
}

/// Generate a default UUID for backwards compatibility
fn default_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Character summary for party analysis
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

/// Party role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl PartyRole {
    pub fn label(&self) -> &'static str {
        match self {
            PartyRole::Tank => "Tank",
            PartyRole::Healer => "Healer",
            PartyRole::DamageDealer => "Damage Dealer",
            PartyRole::Support => "Support",
            PartyRole::Controller => "Controller",
            PartyRole::Utility => "Utility",
            PartyRole::Face => "Face",
            PartyRole::Scout => "Scout",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            PartyRole::Tank,
            PartyRole::Healer,
            PartyRole::DamageDealer,
            PartyRole::Support,
            PartyRole::Controller,
            PartyRole::Utility,
            PartyRole::Face,
            PartyRole::Scout,
        ]
    }
}

/// Level range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelRange {
    pub start_level: u8,
    pub end_level: u8,
}

/// Party gap analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartyGapAnalysis {
    pub missing_roles: Vec<PartyRole>,
    pub suggestions: Vec<String>,
    pub balance_score: u8,
}

/// Arc structure configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArcStructure {
    pub template: Option<ArcTemplate>,
    pub phases: Vec<ArcPhaseConfig>,
    pub narrative_style: Option<NarrativeStyle>,
}

/// Arc phase configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcPhaseConfig {
    /// Unique ID for stable removal without index issues
    #[serde(default = "default_uuid")]
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub estimated_sessions: Option<u32>,
}

/// Narrative style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NarrativeStyle {
    #[default]
    Linear,
    Branching,
    Sandbox,
    Episodic,
}

impl NarrativeStyle {
    pub fn label(&self) -> &'static str {
        match self {
            NarrativeStyle::Linear => "Linear",
            NarrativeStyle::Branching => "Branching",
            NarrativeStyle::Sandbox => "Sandbox",
            NarrativeStyle::Episodic => "Episodic",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            NarrativeStyle::Linear,
            NarrativeStyle::Branching,
            NarrativeStyle::Sandbox,
            NarrativeStyle::Episodic,
        ]
    }
}

/// Initial content configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitialContent {
    pub locations: Vec<LocationDraft>,
    pub npcs: Vec<NpcDraft>,
    pub plot_hooks: Vec<PlotHookDraft>,
}

/// Draft location
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

/// Draft NPC
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

/// Draft plot hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotHookDraft {
    /// Unique ID for stable removal without index issues
    #[serde(default = "default_uuid")]
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub hook_type: Option<PlotHookType>,
}

/// Plot hook type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlotHookType {
    MainQuest,
    SideQuest,
    CharacterTie,
    WorldEvent,
    Mystery,
}

impl PlotHookType {
    pub fn label(&self) -> &'static str {
        match self {
            PlotHookType::MainQuest => "Main Quest",
            PlotHookType::SideQuest => "Side Quest",
            PlotHookType::CharacterTie => "Character Background",
            PlotHookType::WorldEvent => "World Event",
            PlotHookType::Mystery => "Mystery",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            PlotHookType::MainQuest,
            PlotHookType::SideQuest,
            PlotHookType::CharacterTie,
            PlotHookType::WorldEvent,
            PlotHookType::Mystery,
        ]
    }
}

/// Campaign intent
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignIntent {
    pub fantasy: String,
    pub player_experiences: Vec<String>,
    pub constraints: Vec<String>,
    pub themes: Vec<String>,
    pub tone_keywords: Vec<String>,
    pub avoid: Vec<String>,
}

/// Partial campaign draft - accumulated wizard data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PartialCampaign {
    pub name: Option<String>,
    pub system: Option<String>,
    pub description: Option<String>,
    pub intent: Option<CampaignIntent>,
    pub session_scope: Option<SessionScope>,
    pub player_count: Option<u8>,
    pub experience_level: Option<ExperienceLevel>,
    pub party_composition: Option<PartyComposition>,
    pub arc_structure: Option<ArcStructure>,
    pub initial_content: Option<InitialContent>,
}

impl PartialCampaign {
    pub fn has_basics(&self) -> bool {
        self.name.is_some() && self.system.is_some()
    }

    pub fn has_players(&self) -> bool {
        self.player_count.is_some()
    }
}

/// Wizard state from backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardState {
    pub id: String,
    pub current_step: WizardStep,
    pub completed_steps: Vec<WizardStep>,
    pub campaign_draft: PartialCampaign,
    pub conversation_thread_id: Option<String>,
    pub ai_assisted: bool,
    pub created_at: String,
    pub updated_at: String,
    pub auto_saved_at: Option<String>,
}

/// Wizard summary for listing
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

// ============================================================================
// Step Data Types - Per-step input
// ============================================================================

/// Step data for advancing wizard
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "step", content = "data")]
pub enum StepData {
    Basics(BasicsData),
    Intent(IntentData),
    Scope(ScopeData),
    Players(PlayersData),
    PartyComposition(PartyCompositionData),
    ArcStructure(ArcStructureData),
    InitialContent(InitialContentData),
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicsData {
    pub name: String,
    pub system: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentData {
    pub fantasy: String,
    pub player_experiences: Vec<String>,
    pub constraints: Vec<String>,
    pub themes: Vec<String>,
    pub tone_keywords: Vec<String>,
    pub avoid: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeData {
    pub session_count: Option<u32>,
    pub session_duration_hours: Option<f32>,
    pub pacing: Option<CampaignPacing>,
    pub duration_months: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayersData {
    pub player_count: u8,
    pub experience_level: Option<ExperienceLevel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyCompositionData {
    pub characters: Vec<CharacterSummary>,
    pub party_size: Option<u8>,
    pub level_range: Option<LevelRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArcStructureData {
    pub template: Option<ArcTemplate>,
    pub phases: Vec<ArcPhaseConfig>,
    pub narrative_style: Option<NarrativeStyle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitialContentData {
    pub locations: Vec<LocationDraft>,
    pub npcs: Vec<NpcDraft>,
    pub plot_hooks: Vec<PlotHookDraft>,
}

// ============================================================================
// IPC Bindings
// ============================================================================

/// Start a new campaign wizard
pub async fn start_campaign_wizard(ai_assisted: bool) -> Result<WizardState, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        ai_assisted: bool,
    }
    invoke("start_campaign_wizard", &Args { ai_assisted }).await
}

/// Get wizard state by ID
pub async fn get_wizard_state(wizard_id: String) -> Result<Option<WizardState>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        wizard_id: String,
    }
    invoke("get_wizard_state", &Args { wizard_id }).await
}

/// List incomplete wizards
pub async fn list_incomplete_wizards() -> Result<Vec<WizardSummary>, String> {
    #[derive(Serialize)]
    struct Args {}
    invoke("list_incomplete_wizards", &Args {}).await
}

/// Delete a wizard
pub async fn delete_wizard(wizard_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        wizard_id: String,
    }
    crate::bindings::invoke_void("delete_wizard", &Args { wizard_id }).await
}

/// Advance wizard step
pub async fn advance_wizard_step(
    wizard_id: String,
    step_data: StepData,
) -> Result<WizardState, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        wizard_id: String,
        step_data: StepData,
    }
    invoke("advance_wizard_step", &Args { wizard_id, step_data }).await
}

/// Go back to previous step
pub async fn wizard_go_back(wizard_id: String) -> Result<WizardState, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        wizard_id: String,
    }
    invoke("wizard_go_back", &Args { wizard_id }).await
}

/// Skip current step
pub async fn wizard_skip_step(wizard_id: String) -> Result<WizardState, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        wizard_id: String,
    }
    invoke("wizard_skip_step", &Args { wizard_id }).await
}

/// Update wizard draft without advancing
pub async fn update_wizard_draft(
    wizard_id: String,
    draft: PartialCampaign,
) -> Result<WizardState, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        wizard_id: String,
        draft: PartialCampaign,
    }
    invoke("update_wizard_draft", &Args { wizard_id, draft }).await
}

/// Complete the wizard and create campaign
pub async fn complete_wizard(wizard_id: String) -> Result<crate::bindings::Campaign, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        wizard_id: String,
    }
    invoke("complete_wizard", &Args { wizard_id }).await
}

/// Cancel wizard with optional draft save
pub async fn cancel_wizard(wizard_id: String, save_draft: bool) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        wizard_id: String,
        save_draft: bool,
    }
    crate::bindings::invoke_void("cancel_wizard", &Args { wizard_id, save_draft }).await
}

/// Auto-save wizard state
pub async fn auto_save_wizard(
    wizard_id: String,
    partial_data: Option<PartialCampaign>,
) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Args {
        wizard_id: String,
        partial_data: Option<PartialCampaign>,
    }
    crate::bindings::invoke_void("auto_save_wizard", &Args { wizard_id, partial_data }).await
}

// ============================================================================
// Wizard Context - Reactive State Management
// ============================================================================

/// Reactive context for the campaign wizard
#[derive(Clone, Copy)]
pub struct WizardContext {
    /// Current wizard state (None if wizard not started)
    pub wizard_state: RwSignal<Option<WizardState>>,
    /// Current step (derived from wizard_state but with local optimistic updates)
    pub current_step: RwSignal<WizardStep>,
    /// Whether the wizard is loading
    pub is_loading: RwSignal<bool>,
    /// Whether an operation is in progress
    pub is_saving: RwSignal<bool>,
    /// Error message if any
    pub error: RwSignal<Option<String>>,
    /// Whether auto-save is pending
    pub auto_save_pending: RwSignal<bool>,
    /// Last auto-save timestamp
    pub last_auto_save: RwSignal<Option<String>>,
    /// Whether AI assistance is enabled
    pub ai_assisted: RwSignal<bool>,
    /// Trigger for refreshing wizard state
    pub refresh_trigger: Trigger,
}

impl WizardContext {
    /// Create a new wizard context
    pub fn new() -> Self {
        Self {
            wizard_state: RwSignal::new(None),
            current_step: RwSignal::new(WizardStep::Basics),
            is_loading: RwSignal::new(false),
            is_saving: RwSignal::new(false),
            error: RwSignal::new(None),
            auto_save_pending: RwSignal::new(false),
            last_auto_save: RwSignal::new(None),
            ai_assisted: RwSignal::new(true),
            refresh_trigger: Trigger::new(),
        }
    }

    /// Get the wizard ID if one exists
    pub fn wizard_id(&self) -> Option<String> {
        self.wizard_state.get().map(|s| s.id)
    }

    /// Get the campaign draft
    pub fn draft(&self) -> PartialCampaign {
        self.wizard_state
            .get()
            .map(|s| s.campaign_draft)
            .unwrap_or_default()
    }

    /// Check if a step is completed
    pub fn is_step_completed(&self, step: WizardStep) -> bool {
        self.wizard_state
            .get()
            .map(|s| s.completed_steps.contains(&step))
            .unwrap_or(false)
    }

    /// Get progress percentage
    pub fn progress_percent(&self) -> u8 {
        self.wizard_state
            .get()
            .map(|s| {
                let completed = s.completed_steps.len();
                ((completed as f32 / 8.0) * 100.0) as u8
            })
            .unwrap_or(0)
    }

    /// Check if can advance to next step
    pub fn can_advance(&self) -> bool {
        self.current_step.get().next().is_some()
    }

    /// Check if can go back
    pub fn can_go_back(&self) -> bool {
        self.current_step.get().previous().is_some()
    }

    /// Check if current step can be skipped
    pub fn can_skip(&self) -> bool {
        self.current_step.get().is_skippable()
    }

    /// Set error and clear after delay
    pub fn set_error(&self, msg: String) {
        self.error.set(Some(msg));
    }

    /// Clear error
    pub fn clear_error(&self) {
        self.error.set(None);
    }
}

impl Default for WizardContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Provide wizard context to the component tree
pub fn provide_wizard_context() {
    provide_context(WizardContext::new());
}

/// Use the wizard context from anywhere in the tree
pub fn use_wizard_context() -> WizardContext {
    expect_context::<WizardContext>()
}

/// Try to get wizard context (returns None if not provided)
pub fn try_use_wizard_context() -> Option<WizardContext> {
    use_context::<WizardContext>()
}

// ============================================================================
// Wizard Actions - Async operations
// ============================================================================

/// Start a new wizard session
pub fn start_wizard_action(ctx: WizardContext) -> impl Fn(bool) + Clone {
    move |ai_assisted: bool| {
        let ctx = ctx;
        spawn_local(async move {
            ctx.is_loading.set(true);
            ctx.error.set(None);

            match start_campaign_wizard(ai_assisted).await {
                Ok(state) => {
                    ctx.current_step.set(state.current_step);
                    ctx.ai_assisted.set(state.ai_assisted);
                    ctx.wizard_state.set(Some(state));
                }
                Err(e) => {
                    ctx.set_error(e);
                }
            }

            ctx.is_loading.set(false);
        });
    }
}

/// Resume an existing wizard
pub fn resume_wizard_action(ctx: WizardContext) -> impl Fn(String) + Clone {
    move |wizard_id: String| {
        let ctx = ctx;
        spawn_local(async move {
            ctx.is_loading.set(true);
            ctx.error.set(None);

            match get_wizard_state(wizard_id).await {
                Ok(Some(state)) => {
                    ctx.current_step.set(state.current_step);
                    ctx.ai_assisted.set(state.ai_assisted);
                    ctx.wizard_state.set(Some(state));
                }
                Ok(None) => {
                    ctx.set_error("Wizard not found".to_string());
                }
                Err(e) => {
                    ctx.set_error(e);
                }
            }

            ctx.is_loading.set(false);
        });
    }
}

/// Advance to the next step
pub fn advance_step_action(ctx: WizardContext) -> impl Fn(StepData) + Clone {
    move |step_data: StepData| {
        let ctx = ctx;
        let wizard_id = ctx.wizard_id();

        if let Some(id) = wizard_id {
            spawn_local(async move {
                ctx.is_saving.set(true);
                ctx.error.set(None);

                match advance_wizard_step(id, step_data).await {
                    Ok(state) => {
                        ctx.current_step.set(state.current_step);
                        ctx.wizard_state.set(Some(state));
                    }
                    Err(e) => {
                        ctx.set_error(e);
                    }
                }

                ctx.is_saving.set(false);
            });
        }
    }
}

/// Go back to previous step
pub fn go_back_action(ctx: WizardContext) -> impl Fn() + Clone {
    move || {
        let ctx = ctx;
        let wizard_id = ctx.wizard_id();

        if let Some(id) = wizard_id {
            spawn_local(async move {
                ctx.is_saving.set(true);
                ctx.error.set(None);

                match wizard_go_back(id).await {
                    Ok(state) => {
                        ctx.current_step.set(state.current_step);
                        ctx.wizard_state.set(Some(state));
                    }
                    Err(e) => {
                        ctx.set_error(e);
                    }
                }

                ctx.is_saving.set(false);
            });
        }
    }
}

/// Skip current step
pub fn skip_step_action(ctx: WizardContext) -> impl Fn() + Clone {
    move || {
        let ctx = ctx;
        let wizard_id = ctx.wizard_id();

        if let Some(id) = wizard_id {
            spawn_local(async move {
                ctx.is_saving.set(true);
                ctx.error.set(None);

                match wizard_skip_step(id).await {
                    Ok(state) => {
                        ctx.current_step.set(state.current_step);
                        ctx.wizard_state.set(Some(state));
                    }
                    Err(e) => {
                        ctx.set_error(e);
                    }
                }

                ctx.is_saving.set(false);
            });
        }
    }
}

/// Complete the wizard
pub fn complete_wizard_action(
    ctx: WizardContext,
) -> impl Fn(Callback<crate::bindings::Campaign>) + Clone {
    move |on_complete: Callback<crate::bindings::Campaign>| {
        let ctx = ctx;
        let wizard_id = ctx.wizard_id();

        if let Some(id) = wizard_id {
            spawn_local(async move {
                ctx.is_saving.set(true);
                ctx.error.set(None);

                match complete_wizard(id).await {
                    Ok(campaign) => {
                        ctx.wizard_state.set(None);
                        ctx.current_step.set(WizardStep::Basics);
                        on_complete.run(campaign);
                    }
                    Err(e) => {
                        ctx.set_error(e);
                    }
                }

                ctx.is_saving.set(false);
            });
        }
    }
}

/// Cancel the wizard
pub fn cancel_wizard_action(ctx: WizardContext) -> impl Fn(bool, Option<Callback<()>>) + Clone {
    move |save_draft: bool, on_cancel: Option<Callback<()>>| {
        let ctx = ctx;
        let wizard_id = ctx.wizard_id();

        if let Some(id) = wizard_id {
            spawn_local(async move {
                ctx.is_saving.set(true);
                ctx.error.set(None);

                match cancel_wizard(id, save_draft).await {
                    Ok(()) => {
                        ctx.wizard_state.set(None);
                        ctx.current_step.set(WizardStep::Basics);
                        if let Some(cb) = on_cancel {
                            cb.run(());
                        }
                    }
                    Err(e) => {
                        ctx.set_error(e);
                    }
                }

                ctx.is_saving.set(false);
            });
        } else if let Some(cb) = on_cancel {
            cb.run(());
        }
    }
}

/// Trigger auto-save
pub fn auto_save_action(ctx: WizardContext) -> impl Fn(Option<PartialCampaign>) + Clone {
    move |partial_data: Option<PartialCampaign>| {
        let ctx = ctx;
        let wizard_id = ctx.wizard_id();

        if let Some(id) = wizard_id {
            ctx.auto_save_pending.set(true);
            spawn_local(async move {
                match auto_save_wizard(id, partial_data).await {
                    Ok(()) => {
                        ctx.last_auto_save
                            .set(Some(chrono::Utc::now().to_rfc3339()));
                    }
                    Err(_) => {
                        // Silent failure for auto-save, don't disrupt user
                    }
                }
                ctx.auto_save_pending.set(false);
            });
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_step_navigation() {
        assert_eq!(WizardStep::Basics.next(), Some(WizardStep::Intent));
        assert_eq!(WizardStep::Review.next(), None);
        assert_eq!(WizardStep::Basics.previous(), None);
        assert_eq!(WizardStep::Intent.previous(), Some(WizardStep::Basics));
    }

    #[test]
    fn test_wizard_step_skippable() {
        assert!(!WizardStep::Basics.is_skippable());
        assert!(WizardStep::Intent.is_skippable());
        assert!(!WizardStep::Scope.is_skippable());
        assert!(!WizardStep::Players.is_skippable());
        assert!(WizardStep::PartyComposition.is_skippable());
        assert!(!WizardStep::Review.is_skippable());
    }

    #[test]
    fn test_partial_campaign() {
        let mut draft = PartialCampaign::default();
        assert!(!draft.has_basics());

        draft.name = Some("Test Campaign".to_string());
        draft.system = Some("dnd5e".to_string());
        assert!(draft.has_basics());
    }
}
