//! Enhanced Plot Point Type Definitions
//!
//! Extended plot point system with activation states, urgency levels,
//! dependencies, and resolution options for dynamic campaign management.
//!
//! TASK-CAMP-015: EnhancedPlotPoint with PlotPointType, ActivationState, Urgency
//! TASK-CAMP-016: PlotDependencies, ResolutionOption
//! TASK-CAMP-017: From<PlotPoint> conversion

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::plot_manager::{PlotPoint, PlotStatus};

// ============================================================================
// Plot Point Type Enum (TASK-CAMP-015)
// ============================================================================

/// Classification of plot point narrative function
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlotPointType {
    /// Initial hook to draw players in
    #[default]
    Hook,
    /// Clue or piece of information
    Clue,
    /// Escalation of tension or stakes
    Escalation,
    /// Major confrontation or challenge
    Confrontation,
    /// Key revelation or twist
    Revelation,
    /// Decision point with lasting consequences
    Decision,
    /// Resolution of a storyline
    Resolution,
    /// Background event that adds texture
    Backdrop,
    /// Character development moment
    CharacterMoment,
    /// Time-sensitive situation
    Ticking,
    /// Mystery to be solved
    Mystery,
    /// Custom plot point type
    Custom(String),
}

impl PlotPointType {
    /// Get display name
    pub fn display_name(&self) -> String {
        match self {
            Self::Hook => "Hook".to_string(),
            Self::Clue => "Clue".to_string(),
            Self::Escalation => "Escalation".to_string(),
            Self::Confrontation => "Confrontation".to_string(),
            Self::Revelation => "Revelation".to_string(),
            Self::Decision => "Decision".to_string(),
            Self::Resolution => "Resolution".to_string(),
            Self::Backdrop => "Backdrop".to_string(),
            Self::CharacterMoment => "Character Moment".to_string(),
            Self::Ticking => "Ticking Clock".to_string(),
            Self::Mystery => "Mystery".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }

    /// Get typical tension modifier for this type
    pub fn tension_modifier(&self) -> i8 {
        match self {
            Self::Hook => 1,
            Self::Clue => 0,
            Self::Escalation => 2,
            Self::Confrontation => 3,
            Self::Revelation => 2,
            Self::Decision => 1,
            Self::Resolution => -2,
            Self::Backdrop => 0,
            Self::CharacterMoment => 0,
            Self::Ticking => 2,
            Self::Mystery => 1,
            Self::Custom(_) => 0,
        }
    }

    /// Check if this type typically requires immediate attention
    pub fn is_urgent_type(&self) -> bool {
        matches!(
            self,
            Self::Ticking | Self::Confrontation | Self::Escalation
        )
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "hook" => Self::Hook,
            "clue" => Self::Clue,
            "escalation" => Self::Escalation,
            "confrontation" => Self::Confrontation,
            "revelation" => Self::Revelation,
            "decision" => Self::Decision,
            "resolution" => Self::Resolution,
            "backdrop" => Self::Backdrop,
            "character_moment" | "charactermoment" => Self::CharacterMoment,
            "ticking" | "ticking_clock" => Self::Ticking,
            "mystery" => Self::Mystery,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl std::fmt::Display for PlotPointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Activation State Enum (TASK-CAMP-015)
// ============================================================================

/// Lifecycle state of a plot point in the narrative
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ActivationState {
    /// Not yet introduced to players
    #[default]
    Dormant,
    /// Subtly hinted at but not explicit
    Foreshadowed,
    /// Directly introduced to players
    Planted,
    /// Currently driving action
    Active,
    /// Temporarily on hold
    Suspended,
    /// Successfully concluded
    Resolved,
    /// Ended without resolution (abandoned, failed, etc.)
    Expired,
}

impl ActivationState {
    /// Check if the plot point is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Resolved | Self::Expired)
    }

    /// Check if the plot point is visible to players
    pub fn is_visible(&self) -> bool {
        !matches!(self, Self::Dormant)
    }

    /// Check if the plot point is actionable
    pub fn is_actionable(&self) -> bool {
        matches!(self, Self::Planted | Self::Active)
    }

    /// Get valid transitions from this state
    pub fn valid_transitions(&self) -> Vec<ActivationState> {
        use ActivationState::*;
        match self {
            Dormant => vec![Foreshadowed, Planted, Active],
            Foreshadowed => vec![Planted, Active, Dormant],
            Planted => vec![Active, Suspended, Resolved, Expired],
            Active => vec![Suspended, Resolved, Expired],
            Suspended => vec![Active, Expired],
            Resolved => vec![], // Terminal
            Expired => vec![],  // Terminal
        }
    }

    /// Check if transition to target state is valid
    pub fn can_transition_to(&self, target: &ActivationState) -> bool {
        self.valid_transitions().contains(target)
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            Self::Dormant => "Dormant",
            Self::Foreshadowed => "Foreshadowed",
            Self::Planted => "Planted",
            Self::Active => "Active",
            Self::Suspended => "Suspended",
            Self::Resolved => "Resolved",
            Self::Expired => "Expired",
        }
    }
}

impl std::fmt::Display for ActivationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Urgency Enum (TASK-CAMP-015)
// ============================================================================

/// Time pressure on a plot point
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Urgency {
    /// No time pressure, can be addressed whenever
    #[default]
    Background,
    /// Should be addressed in the near future
    Upcoming,
    /// Needs attention this session
    Pressing,
    /// Immediate consequences if not addressed
    Critical,
    /// Deadline already passed, dealing with fallout
    Overdue,
}

impl Urgency {
    /// Get numeric urgency level (1-5)
    pub fn level(&self) -> u8 {
        match self {
            Self::Background => 1,
            Self::Upcoming => 2,
            Self::Pressing => 3,
            Self::Critical => 4,
            Self::Overdue => 5,
        }
    }

    /// Get urgency from numeric level
    pub fn from_level(level: u8) -> Self {
        match level {
            0..=1 => Self::Background,
            2 => Self::Upcoming,
            3 => Self::Pressing,
            4 => Self::Critical,
            _ => Self::Overdue,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            Self::Background => "Background",
            Self::Upcoming => "Upcoming",
            Self::Pressing => "Pressing",
            Self::Critical => "Critical",
            Self::Overdue => "Overdue",
        }
    }

    /// Check if this urgency requires attention
    pub fn requires_attention(&self) -> bool {
        matches!(self, Self::Pressing | Self::Critical | Self::Overdue)
    }
}

impl std::fmt::Display for Urgency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Plot Dependencies (TASK-CAMP-016)
// ============================================================================

/// Dependencies and relationships for a plot point
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlotDependencies {
    /// Plot point IDs that must be resolved before this can activate
    pub prerequisites: Vec<String>,
    /// Plot point IDs that this plot point unlocks when resolved
    pub unlocks: Vec<String>,
    /// Plot point IDs that conflict with this one (mutually exclusive)
    pub conflicts_with: Vec<String>,
    /// Plot point IDs that are related (for context, not dependency)
    pub related_to: Vec<String>,
    /// Plot point IDs that this supersedes (replaces if both active)
    pub supersedes: Vec<String>,
    /// Arc ID this plot point belongs to
    pub arc_id: Option<String>,
    /// Phase ID within the arc
    pub phase_id: Option<String>,
}

impl PlotDependencies {
    /// Create empty dependencies
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a prerequisite
    pub fn add_prerequisite(&mut self, plot_id: &str) {
        if !self.prerequisites.contains(&plot_id.to_string()) {
            self.prerequisites.push(plot_id.to_string());
        }
    }

    /// Add an unlock target
    pub fn add_unlock(&mut self, plot_id: &str) {
        if !self.unlocks.contains(&plot_id.to_string()) {
            self.unlocks.push(plot_id.to_string());
        }
    }

    /// Add a conflict
    pub fn add_conflict(&mut self, plot_id: &str) {
        if !self.conflicts_with.contains(&plot_id.to_string()) {
            self.conflicts_with.push(plot_id.to_string());
        }
    }

    /// Add a related plot point
    pub fn add_related(&mut self, plot_id: &str) {
        if !self.related_to.contains(&plot_id.to_string()) {
            self.related_to.push(plot_id.to_string());
        }
    }

    /// Link to an arc and phase
    pub fn with_arc_phase(mut self, arc_id: &str, phase_id: Option<&str>) -> Self {
        self.arc_id = Some(arc_id.to_string());
        self.phase_id = phase_id.map(|s| s.to_string());
        self
    }

    /// Check if all prerequisites are in a given set of resolved plot IDs
    pub fn prerequisites_met(&self, resolved_ids: &[String]) -> bool {
        self.prerequisites.iter().all(|id| resolved_ids.contains(id))
    }

    /// Check if this plot point conflicts with any in a given set of active plot IDs
    pub fn has_conflict(&self, active_ids: &[String]) -> bool {
        self.conflicts_with.iter().any(|id| active_ids.contains(id))
    }
}

// ============================================================================
// Resolution Option (TASK-CAMP-016)
// ============================================================================

/// Possible outcome for resolving a plot point
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionOption {
    /// Unique identifier
    pub id: String,
    /// Short name for the outcome
    pub name: String,
    /// Description of this resolution path
    pub description: String,
    /// Is this a success outcome?
    pub is_success: bool,
    /// Consequences of this resolution (text descriptions)
    pub consequences: Vec<String>,
    /// Plot points that get unlocked by this resolution
    pub unlocks: Vec<String>,
    /// Plot points that get expired/blocked by this resolution
    pub blocks: Vec<String>,
    /// Rewards if applicable (text descriptions)
    pub rewards: Vec<String>,
    /// NPC relationship changes (NPC ID -> change description)
    pub relationship_changes: HashMap<String, String>,
    /// Was this resolution actually chosen?
    pub was_chosen: bool,
}

impl ResolutionOption {
    /// Create a new resolution option
    pub fn new(name: &str, is_success: bool) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: String::new(),
            is_success,
            consequences: Vec::new(),
            unlocks: Vec::new(),
            blocks: Vec::new(),
            rewards: Vec::new(),
            relationship_changes: HashMap::new(),
            was_chosen: false,
        }
    }

    /// Builder: set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Builder: add a consequence
    pub fn with_consequence(mut self, consequence: &str) -> Self {
        self.consequences.push(consequence.to_string());
        self
    }

    /// Builder: add an unlock
    pub fn with_unlock(mut self, plot_id: &str) -> Self {
        self.unlocks.push(plot_id.to_string());
        self
    }

    /// Builder: add a reward
    pub fn with_reward(mut self, reward: &str) -> Self {
        self.rewards.push(reward.to_string());
        self
    }

    /// Mark this option as chosen
    pub fn choose(&mut self) {
        self.was_chosen = true;
    }
}

// ============================================================================
// Enhanced Plot Point (TASK-CAMP-015)
// ============================================================================

/// Enhanced plot point with full lifecycle management
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedPlotPoint {
    /// Unique identifier
    pub id: String,
    /// Campaign this belongs to
    pub campaign_id: String,
    /// Arc ID if part of an arc
    pub arc_id: Option<String>,
    /// Plot point title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// The dramatic question this plot point poses
    pub dramatic_question: Option<String>,
    /// Type classification
    pub plot_type: PlotPointType,
    /// Current activation state
    pub activation_state: ActivationState,
    /// Legacy status (for compatibility)
    pub status: PlotStatus,
    /// Urgency level
    pub urgency: Urgency,
    /// Tension level (1-10 scale)
    pub tension_level: u8,
    /// Dependencies and relationships
    pub dependencies: PlotDependencies,
    /// Possible resolution options
    pub resolution_options: Vec<ResolutionOption>,
    /// Chosen resolution (if resolved)
    pub chosen_resolution: Option<String>,
    /// Involved NPC IDs
    pub involved_npcs: Vec<String>,
    /// Involved location IDs
    pub involved_locations: Vec<String>,
    /// Session IDs where this plot point was advanced
    pub session_history: Vec<String>,
    /// GM notes and observations
    pub notes: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// When this was created
    pub created_at: DateTime<Utc>,
    /// When this was last updated
    pub updated_at: DateTime<Utc>,
    /// When this was first activated (Planted or Active)
    pub activated_at: Option<DateTime<Utc>>,
    /// When this was resolved or expired
    pub resolved_at: Option<DateTime<Utc>>,
    /// Deadline for urgency calculation (in-world date or session number)
    pub deadline: Option<String>,
}

impl EnhancedPlotPoint {
    /// Create a new enhanced plot point
    pub fn new(campaign_id: &str, title: &str, plot_type: PlotPointType) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            arc_id: None,
            title: title.to_string(),
            description: String::new(),
            dramatic_question: None,
            plot_type,
            activation_state: ActivationState::Dormant,
            status: PlotStatus::Pending,
            urgency: Urgency::Background,
            tension_level: 5,
            dependencies: PlotDependencies::default(),
            resolution_options: Vec::new(),
            chosen_resolution: None,
            involved_npcs: Vec::new(),
            involved_locations: Vec::new(),
            session_history: Vec::new(),
            notes: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            activated_at: None,
            resolved_at: None,
            deadline: None,
        }
    }

    /// Builder: set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Builder: set dramatic question
    pub fn with_dramatic_question(mut self, question: &str) -> Self {
        self.dramatic_question = Some(question.to_string());
        self
    }

    /// Builder: set arc
    pub fn with_arc(mut self, arc_id: &str) -> Self {
        self.arc_id = Some(arc_id.to_string());
        self.dependencies.arc_id = Some(arc_id.to_string());
        self
    }

    /// Builder: set tension level (clamped to 1-10)
    pub fn with_tension(mut self, level: u8) -> Self {
        self.tension_level = level.clamp(1, 10);
        self
    }

    /// Builder: set urgency
    pub fn with_urgency(mut self, urgency: Urgency) -> Self {
        self.urgency = urgency;
        self
    }

    /// Builder: add NPC
    pub fn with_npc(mut self, npc_id: &str) -> Self {
        if !self.involved_npcs.contains(&npc_id.to_string()) {
            self.involved_npcs.push(npc_id.to_string());
        }
        self
    }

    /// Builder: add location
    pub fn with_location(mut self, location_id: &str) -> Self {
        if !self.involved_locations.contains(&location_id.to_string()) {
            self.involved_locations.push(location_id.to_string());
        }
        self
    }

    /// Builder: add resolution option
    pub fn with_resolution_option(mut self, option: ResolutionOption) -> Self {
        self.resolution_options.push(option);
        self
    }

    /// Add a prerequisite dependency
    pub fn add_prerequisite(&mut self, plot_id: &str) {
        self.dependencies.add_prerequisite(plot_id);
        self.updated_at = Utc::now();
    }

    /// Add a note
    pub fn add_note(&mut self, note: &str) {
        self.notes.push(note.to_string());
        self.updated_at = Utc::now();
    }

    /// Record that this plot point was advanced in a session
    pub fn record_session(&mut self, session_id: &str) {
        if !self.session_history.contains(&session_id.to_string()) {
            self.session_history.push(session_id.to_string());
            self.updated_at = Utc::now();
        }
    }

    /// Transition to a new activation state
    pub fn transition_to(&mut self, new_state: ActivationState) -> bool {
        if !self.activation_state.can_transition_to(&new_state) {
            return false;
        }

        let now = Utc::now();

        // Track activation timestamp
        if matches!(new_state, ActivationState::Planted | ActivationState::Active)
            && self.activated_at.is_none()
        {
            self.activated_at = Some(now);
        }

        // Track resolution timestamp
        if new_state.is_terminal() {
            self.resolved_at = Some(now);
        }

        // Update legacy status for compatibility
        self.status = match new_state {
            ActivationState::Dormant | ActivationState::Foreshadowed => PlotStatus::Pending,
            ActivationState::Planted | ActivationState::Active => PlotStatus::Active,
            ActivationState::Suspended => PlotStatus::Paused,
            ActivationState::Resolved => PlotStatus::Completed,
            ActivationState::Expired => PlotStatus::Failed,
        };

        self.activation_state = new_state;
        self.updated_at = now;
        true
    }

    /// Activate this plot point
    pub fn activate(&mut self) -> bool {
        self.transition_to(ActivationState::Active)
    }

    /// Resolve this plot point with a specific resolution option
    pub fn resolve(&mut self, resolution_id: Option<&str>) -> bool {
        if let Some(res_id) = resolution_id {
            // Mark the resolution option as chosen
            if let Some(option) = self.resolution_options.iter_mut().find(|o| o.id == res_id) {
                option.was_chosen = true;
            }
            self.chosen_resolution = Some(res_id.to_string());
        }

        self.transition_to(ActivationState::Resolved)
    }

    /// Expire this plot point (failed/abandoned)
    pub fn expire(&mut self) -> bool {
        self.transition_to(ActivationState::Expired)
    }

    /// Check if prerequisites are met
    pub fn prerequisites_met(&self, resolved_ids: &[String]) -> bool {
        self.dependencies.prerequisites_met(resolved_ids)
    }

    /// Calculate effective tension (base + type modifier)
    pub fn effective_tension(&self) -> i8 {
        // Clamp tension_level to valid range before calculation to prevent overflow
        let base = self.tension_level.clamp(1, 10) as i16;
        let modifier = self.plot_type.tension_modifier() as i16;
        (base + modifier).clamp(1, 10) as i8
    }

    /// Check if this plot point needs immediate attention
    pub fn needs_attention(&self) -> bool {
        self.activation_state.is_actionable() && self.urgency.requires_attention()
    }
}

// ============================================================================
// From<PlotPoint> Conversion (TASK-CAMP-017)
// ============================================================================

impl From<PlotPoint> for EnhancedPlotPoint {
    fn from(legacy: PlotPoint) -> Self {
        use super::campaign::migration::defaults;

        let plot_type = PlotPointType::from_str(defaults::DEFAULT_PLOT_TYPE);
        // Use stable as_str() methods instead of Debug formatting
        let activation_state = ActivationState::from_str(defaults::map_status_to_activation_state(
            legacy.status.as_str(),
        ));
        let urgency = Urgency::from_str(defaults::map_priority_to_urgency(
            legacy.priority.as_str(),
        ));

        let mut dependencies = PlotDependencies::default();
        dependencies.prerequisites = legacy.prerequisites.clone();
        dependencies.unlocks = legacy.unlocks.clone();

        Self {
            id: legacy.id,
            campaign_id: legacy.campaign_id,
            arc_id: None,
            title: legacy.title.clone(),
            description: legacy.description,
            dramatic_question: Some(defaults::generate_dramatic_question(&legacy.title)),
            plot_type,
            activation_state,
            status: legacy.status,
            urgency,
            tension_level: defaults::DEFAULT_TENSION_LEVEL,
            dependencies,
            resolution_options: Vec::new(),
            chosen_resolution: None,
            involved_npcs: legacy.involved_npcs,
            involved_locations: legacy.involved_locations,
            session_history: Vec::new(),
            notes: legacy.notes,
            tags: legacy.tags,
            metadata: HashMap::new(),
            created_at: legacy.created_at,
            updated_at: legacy.updated_at,
            activated_at: legacy.started_at,
            resolved_at: legacy.resolved_at,
            deadline: None,
        }
    }
}

impl ActivationState {
    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "dormant" => Self::Dormant,
            "foreshadowed" => Self::Foreshadowed,
            "planted" => Self::Planted,
            "active" => Self::Active,
            "suspended" => Self::Suspended,
            "resolved" => Self::Resolved,
            "expired" => Self::Expired,
            _ => Self::Dormant,
        }
    }
}

impl Urgency {
    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "background" => Self::Background,
            "upcoming" => Self::Upcoming,
            "pressing" => Self::Pressing,
            "critical" => Self::Critical,
            "overdue" => Self::Overdue,
            _ => Self::Background,
        }
    }
}

// ============================================================================
// Plot Point Summary
// ============================================================================

/// Summary for listing plot points
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlotPointSummary {
    /// Unique identifier
    pub id: String,
    /// Campaign ID
    pub campaign_id: String,
    /// Arc ID if part of an arc
    pub arc_id: Option<String>,
    /// Title
    pub title: String,
    /// Type classification
    pub plot_type: PlotPointType,
    /// Current activation state
    pub activation_state: ActivationState,
    /// Urgency level
    pub urgency: Urgency,
    /// Tension level
    pub tension_level: u8,
    /// Needs attention flag
    pub needs_attention: bool,
    /// Number of involved NPCs
    pub npc_count: usize,
    /// Number of involved locations
    pub location_count: usize,
    /// Tags
    pub tags: Vec<String>,
}

impl From<&EnhancedPlotPoint> for PlotPointSummary {
    fn from(plot: &EnhancedPlotPoint) -> Self {
        Self {
            id: plot.id.clone(),
            campaign_id: plot.campaign_id.clone(),
            arc_id: plot.arc_id.clone(),
            title: plot.title.clone(),
            plot_type: plot.plot_type.clone(),
            activation_state: plot.activation_state.clone(),
            urgency: plot.urgency.clone(),
            tension_level: plot.tension_level,
            needs_attention: plot.needs_attention(),
            npc_count: plot.involved_npcs.len(),
            location_count: plot.involved_locations.len(),
            tags: plot.tags.clone(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::plot_manager::PlotPriority;

    #[test]
    fn test_plot_point_type() {
        assert_eq!(PlotPointType::Hook.tension_modifier(), 1);
        assert_eq!(PlotPointType::Confrontation.tension_modifier(), 3);
        assert_eq!(PlotPointType::Resolution.tension_modifier(), -2);

        assert!(PlotPointType::Ticking.is_urgent_type());
        assert!(!PlotPointType::Backdrop.is_urgent_type());
    }

    #[test]
    fn test_activation_state_transitions() {
        use ActivationState::*;

        assert!(Dormant.can_transition_to(&Active));
        assert!(Active.can_transition_to(&Resolved));
        assert!(!Resolved.can_transition_to(&Active));
        assert!(!Expired.can_transition_to(&Dormant));
    }

    #[test]
    fn test_urgency_levels() {
        assert_eq!(Urgency::Background.level(), 1);
        assert_eq!(Urgency::Critical.level(), 4);

        assert_eq!(Urgency::from_level(3), Urgency::Pressing);
        assert_eq!(Urgency::from_level(10), Urgency::Overdue);

        assert!(Urgency::Critical.requires_attention());
        assert!(!Urgency::Background.requires_attention());
    }

    #[test]
    fn test_enhanced_plot_point_new() {
        let plot = EnhancedPlotPoint::new("camp-1", "Find the Artifact", PlotPointType::Hook);

        assert!(!plot.id.is_empty());
        assert_eq!(plot.title, "Find the Artifact");
        assert_eq!(plot.plot_type, PlotPointType::Hook);
        assert_eq!(plot.activation_state, ActivationState::Dormant);
        assert_eq!(plot.tension_level, 5);
    }

    #[test]
    fn test_enhanced_plot_point_lifecycle() {
        let mut plot = EnhancedPlotPoint::new("camp-1", "Test", PlotPointType::Confrontation);

        // Activate
        assert!(plot.activate());
        assert_eq!(plot.activation_state, ActivationState::Active);
        assert!(plot.activated_at.is_some());

        // Resolve
        assert!(plot.resolve(None));
        assert_eq!(plot.activation_state, ActivationState::Resolved);
        assert!(plot.resolved_at.is_some());
        assert_eq!(plot.status, PlotStatus::Completed);
    }

    #[test]
    fn test_plot_dependencies() {
        let mut deps = PlotDependencies::new();
        deps.add_prerequisite("plot-1");
        deps.add_prerequisite("plot-2");

        assert!(deps.prerequisites_met(&["plot-1".to_string(), "plot-2".to_string()]));
        assert!(!deps.prerequisites_met(&["plot-1".to_string()]));
    }

    #[test]
    fn test_resolution_option() {
        let mut option = ResolutionOption::new("Victory", true)
            .with_description("The party succeeds")
            .with_consequence("The villain is defeated")
            .with_reward("100 gold pieces");

        assert_eq!(option.name, "Victory");
        assert!(option.is_success);
        assert!(!option.was_chosen);

        option.choose();
        assert!(option.was_chosen);
    }

    #[test]
    fn test_effective_tension() {
        let plot = EnhancedPlotPoint::new("camp-1", "Test", PlotPointType::Confrontation)
            .with_tension(7);

        // 7 base + 3 confrontation modifier = 10
        assert_eq!(plot.effective_tension(), 10);

        let plot2 = EnhancedPlotPoint::new("camp-1", "Test", PlotPointType::Resolution)
            .with_tension(3);

        // 3 base - 2 resolution modifier = 1
        assert_eq!(plot2.effective_tension(), 1);
    }

    #[test]
    fn test_from_legacy_plot_point() {
        let legacy = PlotPoint::new("camp-1", "Save the Princess", PlotPriority::Main);

        let enhanced: EnhancedPlotPoint = legacy.into();

        assert_eq!(enhanced.title, "Save the Princess");
        assert_eq!(enhanced.campaign_id, "camp-1");
        assert!(enhanced.dramatic_question.is_some());
        assert_eq!(enhanced.activation_state, ActivationState::Dormant);
    }

    #[test]
    fn test_needs_attention() {
        let mut plot = EnhancedPlotPoint::new("camp-1", "Urgent", PlotPointType::Ticking)
            .with_urgency(Urgency::Critical);

        // Dormant + Critical = no attention needed (not actionable)
        assert!(!plot.needs_attention());

        // Active + Critical = needs attention
        plot.activate();
        assert!(plot.needs_attention());

        // Active + Background = no attention needed
        plot.urgency = Urgency::Background;
        assert!(!plot.needs_attention());
    }

    #[test]
    fn test_serialization() {
        let plot = EnhancedPlotPoint::new("camp-1", "Test", PlotPointType::Mystery)
            .with_description("A mysterious occurrence")
            .with_dramatic_question("What happened?");

        let json = serde_json::to_string(&plot).unwrap();

        // Check camelCase
        assert!(json.contains("campaignId"));
        assert!(json.contains("plotType"));
        assert!(json.contains("activationState"));
        assert!(json.contains("dramaticQuestion"));

        // Round-trip
        let parsed: EnhancedPlotPoint = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.title, "Test");
        assert_eq!(parsed.plot_type, PlotPointType::Mystery);
    }
}
