//! Campaign Generation Pipeline Models
//!
//! Database records for the campaign creation wizard, conversation threads,
//! intent tracking, draft management, and canon status lifecycle.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Wizard Step Enum
// ============================================================================

/// Wizard step enum for campaign creation wizard state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WizardStep {
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
    pub fn as_str(&self) -> &'static str {
        match self {
            WizardStep::Basics => "basics",
            WizardStep::Intent => "intent",
            WizardStep::Scope => "scope",
            WizardStep::Players => "players",
            WizardStep::PartyComposition => "party_composition",
            WizardStep::ArcStructure => "arc_structure",
            WizardStep::InitialContent => "initial_content",
            WizardStep::Review => "review",
        }
    }

    /// Get the next step in the wizard flow (None if at the end)
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

    /// Get the previous step in the wizard flow (None if at the beginning)
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

    /// Check if this step can be skipped
    pub fn is_skippable(&self) -> bool {
        matches!(
            self,
            WizardStep::Intent
                | WizardStep::PartyComposition
                | WizardStep::ArcStructure
                | WizardStep::InitialContent
        )
    }
}

impl std::fmt::Display for WizardStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for WizardStep {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "basics" => Ok(WizardStep::Basics),
            "intent" => Ok(WizardStep::Intent),
            "scope" => Ok(WizardStep::Scope),
            "players" => Ok(WizardStep::Players),
            "party_composition" => Ok(WizardStep::PartyComposition),
            "arc_structure" => Ok(WizardStep::ArcStructure),
            "initial_content" => Ok(WizardStep::InitialContent),
            "review" => Ok(WizardStep::Review),
            _ => Err(format!("Unknown wizard step: {}", s)),
        }
    }
}

// ============================================================================
// Wizard State Record
// ============================================================================

/// Wizard state database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WizardStateRecord {
    pub id: String,
    pub current_step: String,
    pub completed_steps: String,  // JSON array of step names
    pub campaign_draft: String,   // JSON PartialCampaign
    pub conversation_thread_id: Option<String>,
    pub ai_assisted: i32,         // SQLite bool: 0 = false, 1 = true
    pub created_at: String,
    pub updated_at: String,
    pub auto_saved_at: Option<String>,
}

impl WizardStateRecord {
    pub fn new(id: String, ai_assisted: bool) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            current_step: WizardStep::Basics.to_string(),
            completed_steps: "[]".to_string(),
            campaign_draft: "{}".to_string(),
            conversation_thread_id: None,
            ai_assisted: if ai_assisted { 1 } else { 0 },
            created_at: now.clone(),
            updated_at: now,
            auto_saved_at: None,
        }
    }

    /// Get current step as typed enum
    pub fn current_step_enum(&self) -> Result<WizardStep, String> {
        WizardStep::try_from(self.current_step.as_str())
    }

    /// Check if AI assistance is enabled
    pub fn is_ai_assisted(&self) -> bool {
        self.ai_assisted != 0
    }

    /// Parse completed steps from JSON
    pub fn completed_steps_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.completed_steps).unwrap_or_default()
    }

    /// Update the auto_saved_at timestamp
    pub fn mark_auto_saved(&mut self) {
        self.auto_saved_at = Some(chrono::Utc::now().to_rfc3339());
    }
}

// ============================================================================
// Conversation Thread Records
// ============================================================================

/// Conversation purpose enum for thread categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationPurpose {
    CampaignCreation,
    SessionPlanning,
    NpcGeneration,
    CharacterBackground,
    WorldBuilding,
    General,
}

impl ConversationPurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversationPurpose::CampaignCreation => "campaign_creation",
            ConversationPurpose::SessionPlanning => "session_planning",
            ConversationPurpose::NpcGeneration => "npc_generation",
            ConversationPurpose::CharacterBackground => "character_background",
            ConversationPurpose::WorldBuilding => "world_building",
            ConversationPurpose::General => "general",
        }
    }
}

impl std::fmt::Display for ConversationPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for ConversationPurpose {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "campaign_creation" => Ok(ConversationPurpose::CampaignCreation),
            "session_planning" => Ok(ConversationPurpose::SessionPlanning),
            "npc_generation" => Ok(ConversationPurpose::NpcGeneration),
            "character_background" => Ok(ConversationPurpose::CharacterBackground),
            "world_building" => Ok(ConversationPurpose::WorldBuilding),
            "general" => Ok(ConversationPurpose::General),
            _ => Err(format!("Unknown conversation purpose: {}", s)),
        }
    }
}

/// Conversation thread database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationThreadRecord {
    pub id: String,
    pub campaign_id: Option<String>,
    pub wizard_id: Option<String>,
    pub purpose: String,
    pub title: Option<String>,
    pub active_personality: Option<String>,  // JSON PersonalityProfile
    pub message_count: i32,
    pub branched_from: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

impl ConversationThreadRecord {
    pub fn new(id: String, purpose: ConversationPurpose) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            campaign_id: None,
            wizard_id: None,
            purpose: purpose.to_string(),
            title: None,
            active_personality: None,
            message_count: 0,
            branched_from: None,
            created_at: now.clone(),
            updated_at: now,
            archived_at: None,
        }
    }

    /// Get purpose as typed enum
    pub fn purpose_enum(&self) -> Result<ConversationPurpose, String> {
        ConversationPurpose::try_from(self.purpose.as_str())
    }

    /// Check if thread is archived
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    /// Link to a wizard state
    pub fn with_wizard(mut self, wizard_id: String) -> Self {
        self.wizard_id = Some(wizard_id);
        self
    }

    /// Link to a campaign
    pub fn with_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }
}

/// Conversation message role for thread messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConversationRole {
    User,
    Assistant,
    System,
}

impl ConversationRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConversationRole::User => "user",
            ConversationRole::Assistant => "assistant",
            ConversationRole::System => "system",
        }
    }
}

impl std::fmt::Display for ConversationRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for ConversationRole {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "user" => Ok(ConversationRole::User),
            "assistant" => Ok(ConversationRole::Assistant),
            "system" => Ok(ConversationRole::System),
            _ => Err(format!("Unknown conversation role: {}", s)),
        }
    }
}

/// Conversation message database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversationMessageRecord {
    pub id: String,
    pub thread_id: String,
    pub role: String,
    pub content: String,
    pub suggestions: Option<String>,  // JSON array of Suggestion
    pub citations: Option<String>,    // JSON array of Citation
    pub created_at: String,
}

impl ConversationMessageRecord {
    pub fn new(id: String, thread_id: String, role: ConversationRole, content: String) -> Self {
        Self {
            id,
            thread_id,
            role: role.to_string(),
            content,
            suggestions: None,
            citations: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Get role as typed enum
    pub fn role_enum(&self) -> Result<ConversationRole, String> {
        ConversationRole::try_from(self.role.as_str())
    }

    /// Add suggestions to the message
    pub fn with_suggestions(mut self, suggestions: &[Suggestion]) -> Self {
        self.suggestions = Some(serde_json::to_string(suggestions).unwrap_or_default());
        self
    }

    /// Add citations to the message
    pub fn with_citations(mut self, citations: &[SourceCitationRecord]) -> Self {
        self.citations = Some(serde_json::to_string(citations).unwrap_or_default());
        self
    }

    /// Parse suggestions from JSON
    pub fn suggestions_vec(&self) -> Vec<Suggestion> {
        self.suggestions
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default()
    }
}

// ============================================================================
// Suggestion Types
// ============================================================================

/// Suggestion embedded in conversation messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub field: String,          // Field path in PartialCampaign
    pub value: serde_json::Value,
    pub rationale: String,
    pub status: SuggestionStatus,
}

/// Status of a suggestion (accepted, rejected, or pending)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionStatus {
    Pending,
    Accepted,
    Rejected,
}

impl Default for SuggestionStatus {
    fn default() -> Self {
        SuggestionStatus::Pending
    }
}

// ============================================================================
// Source Citation Types
// ============================================================================

/// Source type for citations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Rulebook,
    FlavourSource,
    Adventure,
    Homebrew,
    CampaignEntity,
    UserInput,
}

impl SourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceType::Rulebook => "rulebook",
            SourceType::FlavourSource => "flavour_source",
            SourceType::Adventure => "adventure",
            SourceType::Homebrew => "homebrew",
            SourceType::CampaignEntity => "campaign_entity",
            SourceType::UserInput => "user_input",
        }
    }
}

impl Default for SourceType {
    fn default() -> Self {
        SourceType::Rulebook
    }
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for SourceType {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "rulebook" => Ok(SourceType::Rulebook),
            "flavour_source" => Ok(SourceType::FlavourSource),
            "adventure" => Ok(SourceType::Adventure),
            "homebrew" => Ok(SourceType::Homebrew),
            "campaign_entity" => Ok(SourceType::CampaignEntity),
            "user_input" => Ok(SourceType::UserInput),
            _ => Err(format!("Unknown source type: {}", s)),
        }
    }
}

/// Source citation database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SourceCitationRecord {
    pub id: String,
    pub campaign_id: Option<String>,
    pub source_type: String,
    pub source_id: Option<String>,
    pub source_name: String,
    pub location: Option<String>,     // JSON SourceLocation
    pub excerpt: Option<String>,
    pub confidence: f64,
    pub used_in: Option<String>,      // JSON array of entity IDs
    pub created_at: String,
}

impl SourceCitationRecord {
    pub fn new(
        id: String,
        source_type: SourceType,
        source_name: String,
        confidence: f64,
    ) -> Self {
        Self {
            id,
            campaign_id: None,
            source_type: source_type.to_string(),
            source_id: None,
            source_name,
            location: None,
            excerpt: None,
            confidence,
            used_in: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Get source type as typed enum
    pub fn source_type_enum(&self) -> Result<SourceType, String> {
        SourceType::try_from(self.source_type.as_str())
    }

    /// Set the source location
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(serde_json::to_string(&location).unwrap_or_default());
        self
    }

    /// Set excerpt
    pub fn with_excerpt(mut self, excerpt: String) -> Self {
        self.excerpt = Some(excerpt);
        self
    }

    /// Link to a campaign
    pub fn with_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }
}

/// Source location for precise citation references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub page: Option<u32>,
    pub section: Option<String>,
    pub chapter: Option<String>,
    pub paragraph: Option<u32>,
}

impl Default for SourceLocation {
    fn default() -> Self {
        Self {
            page: None,
            section: None,
            chapter: None,
            paragraph: None,
        }
    }
}

impl SourceLocation {
    pub fn page(page: u32) -> Self {
        Self {
            page: Some(page),
            ..Default::default()
        }
    }

    pub fn section(section: impl Into<String>) -> Self {
        Self {
            section: Some(section.into()),
            ..Default::default()
        }
    }
}

// ============================================================================
// Party Composition Record
// ============================================================================

/// Party composition database record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PartyCompositionRecord {
    pub id: String,
    pub campaign_id: String,
    pub name: String,
    pub composition: String,   // JSON party composition data
    pub analysis: Option<String>,  // JSON gap analysis
    pub created_at: String,
}

impl PartyCompositionRecord {
    pub fn new(id: String, campaign_id: String, name: String, composition: String) -> Self {
        Self {
            id,
            campaign_id,
            name,
            composition,
            analysis: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Add analysis to the composition
    pub fn with_analysis(mut self, analysis: serde_json::Value) -> Self {
        self.analysis = Some(serde_json::to_string(&analysis).unwrap_or_default());
        self
    }
}

// ============================================================================
// Campaign Intent Records
// ============================================================================

/// Campaign intent database record - stable anchor for tone and creative vision
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CampaignIntentRecord {
    pub id: String,
    pub campaign_id: Option<String>,
    pub fantasy: String,
    pub player_experiences: String,  // JSON array
    pub constraints: String,         // JSON array
    pub themes: String,              // JSON array
    pub tone_keywords: String,       // JSON array
    pub avoid: String,               // JSON array
    pub created_at: String,
    pub updated_at: String,
    pub migrated_from: Option<String>,
}

impl CampaignIntentRecord {
    pub fn new(id: String, fantasy: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            campaign_id: None,
            fantasy,
            player_experiences: "[]".to_string(),
            constraints: "[]".to_string(),
            themes: "[]".to_string(),
            tone_keywords: "[]".to_string(),
            avoid: "[]".to_string(),
            created_at: now.clone(),
            updated_at: now,
            migrated_from: None,
        }
    }

    /// Link to a campaign
    pub fn with_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    /// Set player experiences
    pub fn with_experiences(mut self, experiences: &[String]) -> Self {
        self.player_experiences = serde_json::to_string(experiences).unwrap_or_default();
        self
    }

    /// Set constraints
    pub fn with_constraints(mut self, constraints: &[String]) -> Self {
        self.constraints = serde_json::to_string(constraints).unwrap_or_default();
        self
    }

    /// Set themes
    pub fn with_themes(mut self, themes: &[String]) -> Self {
        self.themes = serde_json::to_string(themes).unwrap_or_default();
        self
    }

    /// Set tone keywords
    pub fn with_tone_keywords(mut self, keywords: &[String]) -> Self {
        self.tone_keywords = serde_json::to_string(keywords).unwrap_or_default();
        self
    }

    /// Set avoid list
    pub fn with_avoid(mut self, avoid: &[String]) -> Self {
        self.avoid = serde_json::to_string(avoid).unwrap_or_default();
        self
    }

    /// Parse player experiences from JSON
    pub fn player_experiences_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.player_experiences).unwrap_or_default()
    }

    /// Parse constraints from JSON
    pub fn constraints_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.constraints).unwrap_or_default()
    }

    /// Parse themes from JSON
    pub fn themes_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.themes).unwrap_or_default()
    }

    /// Parse tone keywords from JSON
    pub fn tone_keywords_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.tone_keywords).unwrap_or_default()
    }

    /// Parse avoid list from JSON
    pub fn avoid_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.avoid).unwrap_or_default()
    }
}

/// Campaign intent - stable anchor for tone and creative vision (domain type)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CampaignIntent {
    /// Core fantasy: "grim political thriller", "heroic dungeon crawl", "cosmic horror"
    pub fantasy: String,
    /// Desired player experiences: "mystery", "power fantasy", "tragedy", "discovery"
    pub player_experiences: Vec<String>,
    /// Hard constraints: "low magic", "urban only", "PG-13", "no character death"
    pub constraints: Vec<String>,
    /// Themes to weave through: "corruption of power", "found family", "redemption"
    pub themes: Vec<String>,
    /// Tone keywords: "dark", "humorous", "epic", "intimate", "gritty"
    pub tone_keywords: Vec<String>,
    /// What to avoid: "graphic violence", "romantic subplots", "real-world politics"
    pub avoid: Vec<String>,
}

impl CampaignIntent {
    pub fn new(fantasy: impl Into<String>) -> Self {
        Self {
            fantasy: fantasy.into(),
            ..Default::default()
        }
    }

    /// Convert to database record
    pub fn to_record(&self, id: String) -> CampaignIntentRecord {
        CampaignIntentRecord::new(id, self.fantasy.clone())
            .with_experiences(&self.player_experiences)
            .with_constraints(&self.constraints)
            .with_themes(&self.themes)
            .with_tone_keywords(&self.tone_keywords)
            .with_avoid(&self.avoid)
    }

    /// Convert from database record
    pub fn from_record(record: &CampaignIntentRecord) -> Self {
        Self {
            fantasy: record.fantasy.clone(),
            player_experiences: record.player_experiences_vec(),
            constraints: record.constraints_vec(),
            themes: record.themes_vec(),
            tone_keywords: record.tone_keywords_vec(),
            avoid: record.avoid_vec(),
        }
    }
}

// ============================================================================
// Trust Level Enum
// ============================================================================

/// Trust level for generated content - indicates reliability
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    /// Directly from indexed rulebooks/sourcebooks (e.g., spell stats, monster CR)
    Canonical,
    /// Logically derived from rules/lore (e.g., "a cleric would likely...")
    Derived,
    /// Pure AI invention with no source backing
    Creative,
    /// Generation attempted to cite source but couldn't verify
    Unverified,
}

impl TrustLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustLevel::Canonical => "canonical",
            TrustLevel::Derived => "derived",
            TrustLevel::Creative => "creative",
            TrustLevel::Unverified => "unverified",
        }
    }

    /// Returns true if this content can be used without GM review
    pub fn is_reliable(&self) -> bool {
        matches!(self, TrustLevel::Canonical | TrustLevel::Derived)
    }

    /// Returns the minimum confidence threshold for this trust level
    pub fn min_confidence(&self) -> f64 {
        match self {
            TrustLevel::Canonical => 0.95,
            TrustLevel::Derived => 0.75,
            TrustLevel::Creative => 0.0,
            TrustLevel::Unverified => 0.0,
        }
    }
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for TrustLevel {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "canonical" => Ok(TrustLevel::Canonical),
            "derived" => Ok(TrustLevel::Derived),
            "creative" => Ok(TrustLevel::Creative),
            "unverified" => Ok(TrustLevel::Unverified),
            _ => Err(format!("Unknown trust level: {}", s)),
        }
    }
}

impl Default for TrustLevel {
    fn default() -> Self {
        TrustLevel::Creative
    }
}

// ============================================================================
// Canon Status Enum
// ============================================================================

/// Canon status for progressive commitment lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonStatus {
    /// Initial generation, not yet reviewed
    Draft,
    /// GM has reviewed and approved, but not yet used in play
    Approved,
    /// Used in a session - now part of campaign history
    Canonical,
    /// Retconned or replaced - kept for history but not active
    Deprecated,
}

impl CanonStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CanonStatus::Draft => "draft",
            CanonStatus::Approved => "approved",
            CanonStatus::Canonical => "canonical",
            CanonStatus::Deprecated => "deprecated",
        }
    }

    /// Can this content be freely edited?
    pub fn is_editable(&self) -> bool {
        matches!(self, CanonStatus::Draft | CanonStatus::Approved)
    }

    /// Is this content "locked" by play history?
    pub fn is_locked(&self) -> bool {
        matches!(self, CanonStatus::Canonical | CanonStatus::Deprecated)
    }

    /// Can this status transition to the target status?
    pub fn can_transition_to(&self, target: CanonStatus) -> bool {
        match (self, target) {
            // Draft can go to Approved or stay Draft
            (CanonStatus::Draft, CanonStatus::Approved) => true,
            (CanonStatus::Draft, CanonStatus::Draft) => true,
            // Approved can go to Canonical, back to Draft, or stay Approved
            (CanonStatus::Approved, CanonStatus::Canonical) => true,
            (CanonStatus::Approved, CanonStatus::Draft) => true,
            (CanonStatus::Approved, CanonStatus::Approved) => true,
            // Canonical can only be deprecated
            (CanonStatus::Canonical, CanonStatus::Deprecated) => true,
            (CanonStatus::Canonical, CanonStatus::Canonical) => true,
            // Deprecated is terminal
            (CanonStatus::Deprecated, CanonStatus::Deprecated) => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for CanonStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for CanonStatus {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "draft" => Ok(CanonStatus::Draft),
            "approved" => Ok(CanonStatus::Approved),
            "canonical" => Ok(CanonStatus::Canonical),
            "deprecated" => Ok(CanonStatus::Deprecated),
            _ => Err(format!("Unknown canon status: {}", s)),
        }
    }
}

impl Default for CanonStatus {
    fn default() -> Self {
        CanonStatus::Draft
    }
}

// ============================================================================
// Generation Draft Record
// ============================================================================

/// Generation draft database record - content awaiting GM review
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GenerationDraftRecord {
    pub id: String,
    pub campaign_id: Option<String>,
    pub wizard_id: Option<String>,
    pub entity_type: String,
    pub data: String,              // JSON entity data
    pub status: String,            // CanonStatus as string
    pub trust_level: String,       // TrustLevel as string
    pub trust_confidence: f64,
    pub citations: String,         // JSON array of citation IDs
    pub created_at: String,
    pub updated_at: String,
    pub applied_entity_id: Option<String>,
}

impl GenerationDraftRecord {
    pub fn new(id: String, entity_type: String, data: serde_json::Value) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            campaign_id: None,
            wizard_id: None,
            entity_type,
            data: serde_json::to_string(&data).unwrap_or_default(),
            status: CanonStatus::Draft.as_str().to_string(),
            trust_level: TrustLevel::Creative.as_str().to_string(),
            trust_confidence: 0.0,
            citations: "[]".to_string(),
            created_at: now.clone(),
            updated_at: now,
            applied_entity_id: None,
        }
    }

    /// Get status as typed enum
    pub fn status_enum(&self) -> Result<CanonStatus, String> {
        CanonStatus::try_from(self.status.as_str())
    }

    /// Get trust level as typed enum
    pub fn trust_level_enum(&self) -> Result<TrustLevel, String> {
        TrustLevel::try_from(self.trust_level.as_str())
    }

    /// Link to a campaign
    pub fn with_campaign(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    /// Link to a wizard
    pub fn with_wizard(mut self, wizard_id: String) -> Self {
        self.wizard_id = Some(wizard_id);
        self
    }

    /// Set trust level and confidence
    pub fn with_trust(mut self, trust: TrustLevel, confidence: f64) -> Self {
        self.trust_level = trust.as_str().to_string();
        self.trust_confidence = confidence;
        self
    }

    /// Set citations
    pub fn with_citations(mut self, citation_ids: &[String]) -> Self {
        self.citations = serde_json::to_string(citation_ids).unwrap_or_default();
        self
    }

    /// Parse citations from JSON
    pub fn citations_vec(&self) -> Vec<String> {
        serde_json::from_str(&self.citations).unwrap_or_default()
    }

    /// Check if draft can be edited
    pub fn is_editable(&self) -> bool {
        self.status_enum()
            .map(|s| s.is_editable())
            .unwrap_or(false)
    }
}

// ============================================================================
// Canon Status Log Record
// ============================================================================

/// Canon status log database record - audit trail for status transitions
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CanonStatusLogRecord {
    pub id: String,
    pub draft_id: String,
    pub previous_status: String,
    pub new_status: String,
    pub reason: Option<String>,
    pub triggered_by: Option<String>,
    pub timestamp: String,
}

impl CanonStatusLogRecord {
    pub fn new(
        id: String,
        draft_id: String,
        previous_status: CanonStatus,
        new_status: CanonStatus,
    ) -> Self {
        Self {
            id,
            draft_id,
            previous_status: previous_status.as_str().to_string(),
            new_status: new_status.as_str().to_string(),
            reason: None,
            triggered_by: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Add reason for the status change
    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }

    /// Add trigger source (user, system, auto-promotion)
    pub fn with_trigger(mut self, triggered_by: String) -> Self {
        self.triggered_by = Some(triggered_by);
        self
    }

    /// Get previous status as typed enum
    pub fn previous_status_enum(&self) -> Result<CanonStatus, String> {
        CanonStatus::try_from(self.previous_status.as_str())
    }

    /// Get new status as typed enum
    pub fn new_status_enum(&self) -> Result<CanonStatus, String> {
        CanonStatus::try_from(self.new_status.as_str())
    }
}

// ============================================================================
// Acceptance Event Types
// ============================================================================

/// Acceptance decision enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceDecision {
    Approved,
    Rejected,
    Modified,
    AppliedToCampaign,
}

impl AcceptanceDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            AcceptanceDecision::Approved => "approved",
            AcceptanceDecision::Rejected => "rejected",
            AcceptanceDecision::Modified => "modified",
            AcceptanceDecision::AppliedToCampaign => "applied_to_campaign",
        }
    }
}

impl std::fmt::Display for AcceptanceDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for AcceptanceDecision {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "approved" => Ok(AcceptanceDecision::Approved),
            "rejected" => Ok(AcceptanceDecision::Rejected),
            "modified" => Ok(AcceptanceDecision::Modified),
            "applied_to_campaign" => Ok(AcceptanceDecision::AppliedToCampaign),
            _ => Err(format!("Unknown acceptance decision: {}", s)),
        }
    }
}

/// Acceptance event database record - tracks GM decisions on drafts
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AcceptanceEventRecord {
    pub id: String,
    pub draft_id: String,
    pub entity_type: String,
    pub decision: String,
    pub modifications: Option<String>,  // JSON diff/patch
    pub reason: Option<String>,
    pub timestamp: String,
}

impl AcceptanceEventRecord {
    pub fn new(
        id: String,
        draft_id: String,
        entity_type: String,
        decision: AcceptanceDecision,
    ) -> Self {
        Self {
            id,
            draft_id,
            entity_type,
            decision: decision.to_string(),
            modifications: None,
            reason: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Get decision as typed enum
    pub fn decision_enum(&self) -> Result<AcceptanceDecision, String> {
        AcceptanceDecision::try_from(self.decision.as_str())
    }

    /// Add modifications (for modified decisions)
    pub fn with_modifications(mut self, modifications: serde_json::Value) -> Self {
        self.modifications = Some(serde_json::to_string(&modifications).unwrap_or_default());
        self
    }

    /// Add reason for the decision
    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }
}

// ============================================================================
// Entity Draft Generic Type
// ============================================================================

/// Generic entity draft wrapper with status and trust tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDraft<T> {
    pub id: String,
    pub data: T,
    pub status: CanonStatus,
    pub trust: TrustLevel,
    pub trust_confidence: f64,
    pub citations: Vec<String>,  // Citation IDs
    pub created_at: String,
    pub updated_at: String,
}

impl<T: Clone + Default + Serialize> EntityDraft<T> {
    pub fn new(id: String, data: T) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id,
            data,
            status: CanonStatus::Draft,
            trust: TrustLevel::Creative,
            trust_confidence: 0.0,
            citations: Vec::new(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Set trust level and confidence
    pub fn with_trust(mut self, trust: TrustLevel, confidence: f64) -> Self {
        self.trust = trust;
        self.trust_confidence = confidence;
        self
    }

    /// Add citations
    pub fn with_citations(mut self, citations: Vec<String>) -> Self {
        self.citations = citations;
        self
    }

    /// Update the data and mark as updated
    pub fn update_data(&mut self, data: T) {
        self.data = data;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Check if draft can be edited
    pub fn is_editable(&self) -> bool {
        self.status.is_editable()
    }

    /// Check if trust is reliable
    pub fn is_reliable(&self) -> bool {
        self.trust.is_reliable()
    }
}

impl<T: Clone + Default + Serialize + for<'de> serde::Deserialize<'de>> EntityDraft<T> {
    /// Convert to database record
    pub fn to_record(&self, entity_type: String) -> GenerationDraftRecord {
        let data_json = serde_json::to_value(&self.data).unwrap_or_default();
        GenerationDraftRecord::new(self.id.clone(), entity_type, data_json)
            .with_trust(self.trust, self.trust_confidence)
            .with_citations(&self.citations)
    }

    /// Convert from database record (requires entity_type check by caller)
    pub fn from_record(record: &GenerationDraftRecord) -> Result<Self, String> {
        let data: T = serde_json::from_str(&record.data)
            .map_err(|e| format!("Failed to parse entity data: {}", e))?;

        Ok(Self {
            id: record.id.clone(),
            data,
            status: record.status_enum()?,
            trust: record.trust_level_enum()?,
            trust_confidence: record.trust_confidence,
            citations: record.citations_vec(),
            created_at: record.created_at.clone(),
            updated_at: record.updated_at.clone(),
        })
    }
}

// ============================================================================
// Citation Domain Type
// ============================================================================

/// Citation domain type (used in memory, not database)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub id: String,
    pub source_type: SourceType,
    pub source_id: Option<String>,
    pub source_name: String,
    pub location: Option<SourceLocation>,
    pub excerpt: Option<String>,
    pub confidence: f64,
}

impl Citation {
    pub fn new(source_type: SourceType, source_name: impl Into<String>, confidence: f64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_type,
            source_id: None,
            source_name: source_name.into(),
            location: None,
            excerpt: None,
            confidence,
        }
    }

    /// Convert to database record
    pub fn to_record(&self) -> SourceCitationRecord {
        let mut record = SourceCitationRecord::new(
            self.id.clone(),
            self.source_type,
            self.source_name.clone(),
            self.confidence,
        );
        record.source_id = self.source_id.clone();
        if let Some(loc) = &self.location {
            record.location = Some(serde_json::to_string(loc).unwrap_or_default());
        }
        record.excerpt = self.excerpt.clone();
        record
    }

    /// Convert from database record
    pub fn from_record(record: &SourceCitationRecord) -> Result<Self, String> {
        let source_type = record.source_type_enum()?;
        let location: Option<SourceLocation> = record
            .location
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok());

        Ok(Self {
            id: record.id.clone(),
            source_type,
            source_id: record.source_id.clone(),
            source_name: record.source_name.clone(),
            location,
            excerpt: record.excerpt.clone(),
            confidence: record.confidence,
        })
    }
}
