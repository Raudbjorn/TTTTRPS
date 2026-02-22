//! Session Plan Type Definitions
//!
//! Data structures for session planning with pacing, encounters, and narrative beats.
//!
//! TASK-CAMP-014: SessionPlan, PacingBeat, Encounter, NarrativeBeat, SessionPlanStatus

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Session Plan Status
// ============================================================================

/// Lifecycle status of a session plan
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SessionPlanStatus {
    /// Draft plan, not yet finalized
    #[default]
    Draft,
    /// Ready to run
    Ready,
    /// Currently being played
    InProgress,
    /// Session completed
    Completed,
    /// Plan was canceled/abandoned
    #[serde(alias = "Cancelled")]
    Canceled,
}

impl SessionPlanStatus {
    /// Check if the plan is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Canceled)
    }

    /// Check if the plan can be edited
    pub fn can_edit(&self) -> bool {
        matches!(self, Self::Draft | Self::Ready)
    }

    /// Check if transition is valid
    pub fn can_transition_to(&self, target: &SessionPlanStatus) -> bool {
        use SessionPlanStatus::*;
        matches!(
            (self, target),
            // From Draft
            (Draft, Ready) | (Draft, Canceled) |
            // From Ready
            (Ready, InProgress) | (Ready, Draft) | (Ready, Canceled) |
            // From InProgress
            (InProgress, Completed) | (InProgress, Canceled)
        )
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            Self::Draft => "Draft",
            Self::Ready => "Ready",
            Self::InProgress => "In Progress",
            Self::Completed => "Completed",
            Self::Canceled => "Canceled",
        }
    }
}

impl std::fmt::Display for SessionPlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Pacing Type
// ============================================================================

/// Type of pacing beat within a session
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PacingType {
    /// High-action combat encounter
    CombatHeavy,
    /// Focused on dialogue and character interaction
    RoleplayFocused,
    /// Exploration and discovery
    Exploration,
    /// Puzzle or mystery solving
    Investigation,
    /// Mix of different pacing types
    #[default]
    Mixed,
    /// Calm moment, downtime
    Breather,
    /// Dramatic revelation or climax
    Climax,
    /// Opening hook to engage players
    Hook,
    /// Wrap-up and transition
    Denouement,
    /// Custom pacing type
    Custom(String),
}

impl PacingType {
    /// Get estimated duration in minutes for this pacing type
    pub fn estimated_duration_minutes(&self) -> u32 {
        match self {
            Self::CombatHeavy => 45,
            Self::RoleplayFocused => 30,
            Self::Exploration => 30,
            Self::Investigation => 40,
            Self::Mixed => 35,
            Self::Breather => 15,
            Self::Climax => 60,
            Self::Hook => 15,
            Self::Denouement => 20,
            Self::Custom(_) => 30,
        }
    }

    /// Get energy level (1-10) for pacing rhythm
    pub fn energy_level(&self) -> u8 {
        match self {
            Self::CombatHeavy => 9,
            Self::Climax => 10,
            Self::Hook => 7,
            Self::Investigation => 6,
            Self::RoleplayFocused => 5,
            Self::Exploration => 5,
            Self::Mixed => 6,
            Self::Denouement => 4,
            Self::Breather => 2,
            Self::Custom(_) => 5,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> String {
        match self {
            Self::CombatHeavy => "Combat Heavy".to_string(),
            Self::RoleplayFocused => "Roleplay Focused".to_string(),
            Self::Exploration => "Exploration".to_string(),
            Self::Investigation => "Investigation".to_string(),
            Self::Mixed => "Mixed".to_string(),
            Self::Breather => "Breather".to_string(),
            Self::Climax => "Climax".to_string(),
            Self::Hook => "Hook".to_string(),
            Self::Denouement => "Denouement".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }
}


// ============================================================================
// Pacing Beat
// ============================================================================

/// A pacing beat within a session plan
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PacingBeat {
    /// Unique identifier
    pub id: String,
    /// Order in the session (1-indexed)
    pub order: u32,
    /// Type of pacing
    pub pacing_type: PacingType,
    /// Short name/title
    pub name: String,
    /// Description of what happens
    pub description: String,
    /// Estimated duration in minutes
    pub estimated_duration: u32,
    /// Actual duration (filled in after session)
    pub actual_duration: Option<u32>,
    /// Was this beat completed?
    pub completed: bool,
    /// Notes about execution
    pub notes: Option<String>,
    /// Linked encounter ID (if combat)
    pub encounter_id: Option<String>,
    /// Linked narrative beat ID
    pub narrative_beat_id: Option<String>,
}

impl PacingBeat {
    /// Create a new pacing beat
    pub fn new(order: u32, pacing_type: PacingType, name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            order,
            pacing_type: pacing_type.clone(),
            name: name.to_string(),
            description: String::new(),
            estimated_duration: pacing_type.estimated_duration_minutes(),
            actual_duration: None,
            completed: false,
            notes: None,
            encounter_id: None,
            narrative_beat_id: None,
        }
    }

    /// Builder: set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Builder: set custom duration
    pub fn with_duration(mut self, minutes: u32) -> Self {
        self.estimated_duration = minutes;
        self
    }

    /// Builder: link encounter
    pub fn with_encounter(mut self, encounter_id: &str) -> Self {
        self.encounter_id = Some(encounter_id.to_string());
        self
    }

    /// Mark as completed with actual duration
    pub fn complete(&mut self, actual_duration: u32, notes: Option<&str>) {
        self.completed = true;
        self.actual_duration = Some(actual_duration);
        self.notes = notes.map(|s| s.to_string());
    }
}

// ============================================================================
// Encounter Difficulty
// ============================================================================

/// Difficulty rating for encounters
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EncounterDifficulty {
    /// Tutorial/easy encounter
    Trivial,
    /// Easy challenge
    Easy,
    /// Standard challenge
    #[default]
    Medium,
    /// Tough challenge
    Hard,
    /// Potentially deadly challenge
    Deadly,
    /// Boss fight
    Boss,
}

impl EncounterDifficulty {
    /// Get XP multiplier for difficulty (D&D 5e style)
    pub fn xp_multiplier(&self) -> f32 {
        match self {
            Self::Trivial => 0.5,
            Self::Easy => 1.0,
            Self::Medium => 1.5,
            Self::Hard => 2.0,
            Self::Deadly => 2.5,
            Self::Boss => 3.0,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            Self::Trivial => "Trivial",
            Self::Easy => "Easy",
            Self::Medium => "Medium",
            Self::Hard => "Hard",
            Self::Deadly => "Deadly",
            Self::Boss => "Boss",
        }
    }
}

// ============================================================================
// Planned Encounter
// ============================================================================

/// A planned encounter for the session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedEncounter {
    /// Unique identifier
    pub id: String,
    /// Encounter name
    pub name: String,
    /// Description
    pub description: String,
    /// Difficulty rating
    pub difficulty: EncounterDifficulty,
    /// Location where encounter takes place
    pub location: Option<String>,
    /// Location ID if linked to a location entity
    pub location_id: Option<String>,
    /// Monster/enemy names and counts (e.g., "3 Goblins, 1 Hobgoblin")
    pub enemies: Vec<EnemyGroup>,
    /// Total XP value (calculated)
    pub total_xp: u32,
    /// Estimated duration in minutes
    pub estimated_duration: u32,
    /// Terrain or environmental features
    pub terrain: Vec<String>,
    /// Tactical notes for the GM
    pub tactics: String,
    /// Potential rewards
    pub rewards: Vec<String>,
    /// Is this encounter optional?
    pub is_optional: bool,
    /// Was this encounter run?
    pub was_run: bool,
    /// Notes about how it went
    pub aftermath_notes: Option<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// A group of enemies in an encounter
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnemyGroup {
    /// Enemy name/type
    pub name: String,
    /// Number of this enemy
    pub count: u32,
    /// Challenge Rating (if applicable)
    pub challenge_rating: Option<f32>,
    /// XP per individual
    pub xp_per_unit: Option<u32>,
    /// Notes about this enemy group
    pub notes: Option<String>,
}

impl PlannedEncounter {
    /// Create a new planned encounter
    pub fn new(name: &str, difficulty: EncounterDifficulty) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: String::new(),
            difficulty,
            location: None,
            location_id: None,
            enemies: Vec::new(),
            total_xp: 0,
            estimated_duration: 45,
            terrain: Vec::new(),
            tactics: String::new(),
            rewards: Vec::new(),
            is_optional: false,
            was_run: false,
            aftermath_notes: None,
            metadata: HashMap::new(),
        }
    }

    /// Add an enemy group to the encounter
    pub fn add_enemy_group(&mut self, group: EnemyGroup) {
        if let Some(xp) = group.xp_per_unit {
            self.total_xp += xp * group.count;
        }
        self.enemies.push(group);
    }

    /// Recalculate total XP from enemy groups
    pub fn recalculate_xp(&mut self) {
        self.total_xp = self
            .enemies
            .iter()
            .filter_map(|g| g.xp_per_unit.map(|xp| xp * g.count))
            .sum();
    }

    /// Get adjusted XP based on difficulty multiplier
    pub fn adjusted_xp(&self) -> u32 {
        (self.total_xp as f32 * self.difficulty.xp_multiplier()) as u32
    }
}

// ============================================================================
// Narrative Beat
// ============================================================================

/// A narrative beat or story moment in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrativeBeat {
    /// Unique identifier
    pub id: String,
    /// Beat name
    pub name: String,
    /// Description of what should happen
    pub description: String,
    /// Dramatic question this beat addresses
    pub dramatic_question: Option<String>,
    /// Information to reveal to players
    pub reveals: Vec<String>,
    /// NPCs involved
    pub involved_npcs: Vec<String>,
    /// NPC IDs if linked
    pub involved_npc_ids: Vec<String>,
    /// Plot points this beat advances
    pub advances_plot_points: Vec<String>,
    /// Is this beat required for session success?
    pub is_required: bool,
    /// Was this beat delivered?
    pub was_delivered: bool,
    /// Notes about delivery
    pub delivery_notes: Option<String>,
}

impl NarrativeBeat {
    /// Create a new narrative beat
    pub fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: String::new(),
            dramatic_question: None,
            reveals: Vec::new(),
            involved_npcs: Vec::new(),
            involved_npc_ids: Vec::new(),
            advances_plot_points: Vec::new(),
            is_required: false,
            was_delivered: false,
            delivery_notes: None,
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

    /// Builder: add a reveal
    pub fn with_reveal(mut self, reveal: &str) -> Self {
        self.reveals.push(reveal.to_string());
        self
    }

    /// Builder: mark as required
    pub fn as_required(mut self) -> Self {
        self.is_required = true;
        self
    }
}

// ============================================================================
// Session Plan
// ============================================================================

/// A comprehensive session plan
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPlan {
    /// Unique identifier
    pub id: String,
    /// Campaign ID
    pub campaign_id: String,
    /// Session ID (if linked to a specific session)
    pub session_id: Option<String>,
    /// Arc ID (if part of an arc)
    pub arc_id: Option<String>,
    /// Phase ID (if part of a phase)
    pub phase_id: Option<String>,
    /// Session number (for ordering)
    pub session_number: Option<u32>,
    /// Plan title
    pub title: String,
    /// One-sentence summary
    pub summary: String,
    /// Central dramatic questions for this session
    pub dramatic_questions: Vec<String>,
    /// Current status
    pub status: SessionPlanStatus,
    /// Is this a template?
    pub is_template: bool,
    /// Pacing beats (ordered)
    pub pacing_beats: Vec<PacingBeat>,
    /// Planned encounters
    pub encounters: Vec<PlannedEncounter>,
    /// Narrative beats
    pub narrative_beats: Vec<NarrativeBeat>,
    /// Plot point IDs to potentially activate
    pub plot_points_to_activate: Vec<String>,
    /// Plot point IDs to potentially resolve
    pub plot_points_to_resolve: Vec<String>,
    /// Milestone IDs to potentially achieve
    pub milestones_to_achieve: Vec<String>,
    /// NPC IDs likely to appear
    pub expected_npcs: Vec<String>,
    /// Location IDs likely to be visited
    pub expected_locations: Vec<String>,
    /// Estimated total duration (minutes)
    pub estimated_duration: u32,
    /// Actual duration (filled after session)
    pub actual_duration: Option<u32>,
    /// Pre-session preparation notes
    pub prep_notes: String,
    /// Post-session notes
    pub session_notes: Option<String>,
    /// Contingency plans (what if players go off-script)
    pub contingencies: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// When this plan was created
    pub created_at: DateTime<Utc>,
    /// When this plan was last updated
    pub updated_at: DateTime<Utc>,
}

impl SessionPlan {
    /// Create a new session plan
    pub fn new(campaign_id: &str, title: &str) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            session_id: None,
            arc_id: None,
            phase_id: None,
            session_number: None,
            title: title.to_string(),
            summary: String::new(),
            dramatic_questions: Vec::new(),
            status: SessionPlanStatus::Draft,
            is_template: false,
            pacing_beats: Vec::new(),
            encounters: Vec::new(),
            narrative_beats: Vec::new(),
            plot_points_to_activate: Vec::new(),
            plot_points_to_resolve: Vec::new(),
            milestones_to_achieve: Vec::new(),
            expected_npcs: Vec::new(),
            expected_locations: Vec::new(),
            estimated_duration: 180, // 3 hours default
            actual_duration: None,
            prep_notes: String::new(),
            session_notes: None,
            contingencies: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a template session plan
    pub fn new_template(campaign_id: &str, title: &str) -> Self {
        let mut plan = Self::new(campaign_id, title);
        plan.is_template = true;
        plan
    }

    /// Builder: set session number
    pub fn with_session_number(mut self, number: u32) -> Self {
        self.session_number = Some(number);
        self
    }

    /// Builder: link to arc and phase
    pub fn with_arc_phase(mut self, arc_id: &str, phase_id: &str) -> Self {
        self.arc_id = Some(arc_id.to_string());
        self.phase_id = Some(phase_id.to_string());
        self
    }

    /// Builder: add a dramatic question
    pub fn with_dramatic_question(mut self, question: &str) -> Self {
        self.dramatic_questions.push(question.to_string());
        self
    }

    /// Builder: set estimated duration
    pub fn with_duration(mut self, minutes: u32) -> Self {
        self.estimated_duration = minutes;
        self
    }

    /// Add a pacing beat
    pub fn add_pacing_beat(&mut self, beat: PacingBeat) {
        self.pacing_beats.push(beat);
        self.recalculate_duration();
        self.updated_at = Utc::now();
    }

    /// Add an encounter
    pub fn add_encounter(&mut self, encounter: PlannedEncounter) {
        self.encounters.push(encounter);
        self.updated_at = Utc::now();
    }

    /// Add a narrative beat
    pub fn add_narrative_beat(&mut self, beat: NarrativeBeat) {
        self.narrative_beats.push(beat);
        self.updated_at = Utc::now();
    }

    /// Recalculate estimated duration from pacing beats
    pub fn recalculate_duration(&mut self) {
        self.estimated_duration = self
            .pacing_beats
            .iter()
            .map(|b| b.estimated_duration)
            .sum();
    }

    /// Get completion percentage based on completed pacing beats
    pub fn completion_percentage(&self) -> f64 {
        if self.pacing_beats.is_empty() {
            return 0.0;
        }

        let completed = self.pacing_beats.iter().filter(|b| b.completed).count();
        ((completed as f64 / self.pacing_beats.len() as f64) * 100.0).clamp(0.0, 100.0)
    }

    /// Check if all required narrative beats were delivered
    pub fn all_required_delivered(&self) -> bool {
        self.narrative_beats
            .iter()
            .filter(|b| b.is_required)
            .all(|b| b.was_delivered)
    }

    /// Mark session as in progress
    pub fn start(&mut self) -> bool {
        if self.status.can_transition_to(&SessionPlanStatus::InProgress) {
            self.status = SessionPlanStatus::InProgress;
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Mark session as completed
    pub fn complete(&mut self, actual_duration: u32, notes: Option<&str>) -> bool {
        if self.status.can_transition_to(&SessionPlanStatus::Completed) {
            self.status = SessionPlanStatus::Completed;
            self.actual_duration = Some(actual_duration);
            self.session_notes = notes.map(|s| s.to_string());
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Clone this plan as a template
    pub fn to_template(&self, new_title: &str) -> Self {
        let mut template = self.clone();
        template.id = uuid::Uuid::new_v4().to_string();
        template.title = new_title.to_string();
        template.is_template = true;
        template.session_id = None;
        template.session_number = None;
        template.status = SessionPlanStatus::Draft;
        template.actual_duration = None;
        template.session_notes = None;
        template.created_at = Utc::now();
        template.updated_at = Utc::now();

        // Reset beat completion status
        for beat in &mut template.pacing_beats {
            beat.completed = false;
            beat.actual_duration = None;
            beat.notes = None;
        }

        for encounter in &mut template.encounters {
            encounter.was_run = false;
            encounter.aftermath_notes = None;
        }

        for beat in &mut template.narrative_beats {
            beat.was_delivered = false;
            beat.delivery_notes = None;
        }

        template
    }

    /// Instantiate a new plan from this template
    pub fn instantiate(&self, session_number: u32) -> Self {
        let mut plan = self.clone();
        plan.id = uuid::Uuid::new_v4().to_string();
        plan.is_template = false;
        plan.session_number = Some(session_number);
        plan.status = SessionPlanStatus::Draft;
        plan.created_at = Utc::now();
        plan.updated_at = Utc::now();

        // Build ID mappings for encounters and narrative beats
        let mut encounter_id_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut narrative_beat_id_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        // Reset IDs for encounters and build mapping
        for encounter in &mut plan.encounters {
            let old_id = encounter.id.clone();
            let new_id = uuid::Uuid::new_v4().to_string();
            encounter_id_map.insert(old_id, new_id.clone());
            encounter.id = new_id;
        }

        // Reset IDs for narrative beats and build mapping
        for beat in &mut plan.narrative_beats {
            let old_id = beat.id.clone();
            let new_id = uuid::Uuid::new_v4().to_string();
            narrative_beat_id_map.insert(old_id, new_id.clone());
            beat.id = new_id;
        }

        // Reset pacing beat IDs and update references
        for beat in &mut plan.pacing_beats {
            beat.id = uuid::Uuid::new_v4().to_string();
            // Update encounter_id reference if present
            if let Some(ref old_eid) = beat.encounter_id {
                beat.encounter_id = encounter_id_map.get(old_eid).cloned();
            }
            // Update narrative_beat_id reference if present
            if let Some(ref old_nid) = beat.narrative_beat_id {
                beat.narrative_beat_id = narrative_beat_id_map.get(old_nid).cloned();
            }
        }

        plan
    }
}

// ============================================================================
// Pacing Templates
// ============================================================================

/// Template for common session pacing patterns
pub mod pacing_templates {
    use super::*;

    /// Generate pacing beats for a combat-heavy session
    pub fn combat_heavy() -> Vec<PacingBeat> {
        vec![
            PacingBeat::new(1, PacingType::Hook, "Opening Hook")
                .with_description("Engage players and set up the session")
                .with_duration(15),
            PacingBeat::new(2, PacingType::CombatHeavy, "First Encounter")
                .with_description("Initial combat encounter to warm up")
                .with_duration(45),
            PacingBeat::new(3, PacingType::Breather, "Short Rest/Roleplay")
                .with_description("Recovery and character interaction")
                .with_duration(20),
            PacingBeat::new(4, PacingType::CombatHeavy, "Main Encounter")
                .with_description("Primary combat challenge of the session")
                .with_duration(60),
            PacingBeat::new(5, PacingType::Denouement, "Aftermath")
                .with_description("Wrap up and tease next session")
                .with_duration(20),
        ]
    }

    /// Generate pacing beats for a roleplay-focused session
    pub fn roleplay_focused() -> Vec<PacingBeat> {
        vec![
            PacingBeat::new(1, PacingType::Hook, "Opening Scene")
                .with_description("Set the scene and engage players")
                .with_duration(20),
            PacingBeat::new(2, PacingType::RoleplayFocused, "Key NPC Interaction")
                .with_description("Important conversation or negotiation")
                .with_duration(40),
            PacingBeat::new(3, PacingType::Investigation, "Information Gathering")
                .with_description("Players gather clues or intel")
                .with_duration(30),
            PacingBeat::new(4, PacingType::RoleplayFocused, "Party Discussion")
                .with_description("Players plan and debate")
                .with_duration(25),
            PacingBeat::new(5, PacingType::Climax, "Key Decision")
                .with_description("Major choice or revelation")
                .with_duration(30),
            PacingBeat::new(6, PacingType::Denouement, "Consequences Begin")
                .with_description("Initial fallout from decision")
                .with_duration(15),
        ]
    }

    /// Generate pacing beats for an exploration session
    pub fn exploration() -> Vec<PacingBeat> {
        vec![
            PacingBeat::new(1, PacingType::Hook, "Arrival")
                .with_description("Arrive at new location, establish setting")
                .with_duration(20),
            PacingBeat::new(2, PacingType::Exploration, "Initial Exploration")
                .with_description("First area discovery")
                .with_duration(35),
            PacingBeat::new(3, PacingType::Mixed, "Obstacle or Encounter")
                .with_description("Challenge that requires problem-solving")
                .with_duration(40),
            PacingBeat::new(4, PacingType::Exploration, "Deeper Discovery")
                .with_description("Find something significant")
                .with_duration(30),
            PacingBeat::new(5, PacingType::Breather, "Camp/Rest")
                .with_description("Downtime for character moments")
                .with_duration(20),
            PacingBeat::new(6, PacingType::Climax, "Major Discovery")
                .with_description("Find the thing they came for")
                .with_duration(25),
        ]
    }

    /// Generate pacing beats for a mixed session
    pub fn mixed() -> Vec<PacingBeat> {
        vec![
            PacingBeat::new(1, PacingType::Hook, "Session Start")
                .with_description("Recap and hook into action")
                .with_duration(15),
            PacingBeat::new(2, PacingType::RoleplayFocused, "Roleplay Segment")
                .with_description("Character interaction or NPC scene")
                .with_duration(30),
            PacingBeat::new(3, PacingType::CombatHeavy, "Combat Encounter")
                .with_description("Action sequence")
                .with_duration(45),
            PacingBeat::new(4, PacingType::Exploration, "Exploration/Investigation")
                .with_description("Discovery segment")
                .with_duration(30),
            PacingBeat::new(5, PacingType::Climax, "Climactic Moment")
                .with_description("Session high point")
                .with_duration(30),
            PacingBeat::new(6, PacingType::Denouement, "Wind Down")
                .with_description("Wrap up loose ends")
                .with_duration(20),
        ]
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_plan_status_transitions() {
        use SessionPlanStatus::*;

        assert!(Draft.can_transition_to(&Ready));
        assert!(Ready.can_transition_to(&InProgress));
        assert!(InProgress.can_transition_to(&Completed));
        assert!(!Completed.can_transition_to(&Draft));
    }

    #[test]
    fn test_pacing_beat_new() {
        let beat = PacingBeat::new(1, PacingType::CombatHeavy, "Boss Fight");
        assert_eq!(beat.order, 1);
        assert_eq!(beat.name, "Boss Fight");
        assert_eq!(beat.estimated_duration, 45); // Default for combat
    }

    #[test]
    fn test_pacing_beat_complete() {
        let mut beat = PacingBeat::new(1, PacingType::RoleplayFocused, "Test");
        beat.complete(35, Some("Went well"));

        assert!(beat.completed);
        assert_eq!(beat.actual_duration, Some(35));
        assert_eq!(beat.notes, Some("Went well".to_string()));
    }

    #[test]
    fn test_planned_encounter() {
        let mut encounter = PlannedEncounter::new("Goblin Ambush", EncounterDifficulty::Medium);

        encounter.add_enemy_group(EnemyGroup {
            name: "Goblin".to_string(),
            count: 4,
            challenge_rating: Some(0.25),
            xp_per_unit: Some(50),
            notes: None,
        });

        assert_eq!(encounter.total_xp, 200);
        assert_eq!(encounter.adjusted_xp(), 300); // 200 * 1.5 for Medium
    }

    #[test]
    fn test_session_plan_new() {
        let plan = SessionPlan::new("camp-1", "Session 5: The Dark Tower");

        assert!(!plan.id.is_empty());
        assert_eq!(plan.campaign_id, "camp-1");
        assert_eq!(plan.title, "Session 5: The Dark Tower");
        assert_eq!(plan.status, SessionPlanStatus::Draft);
        assert!(!plan.is_template);
    }

    #[test]
    fn test_session_plan_pacing() {
        let mut plan = SessionPlan::new("camp-1", "Test Session");

        plan.add_pacing_beat(PacingBeat::new(1, PacingType::Hook, "Hook").with_duration(15));
        plan.add_pacing_beat(PacingBeat::new(2, PacingType::CombatHeavy, "Combat").with_duration(45));

        assert_eq!(plan.estimated_duration, 60);
    }

    #[test]
    fn test_session_plan_completion() {
        let mut plan = SessionPlan::new("camp-1", "Test");
        plan.add_pacing_beat(PacingBeat::new(1, PacingType::Hook, "Beat 1"));
        plan.add_pacing_beat(PacingBeat::new(2, PacingType::Climax, "Beat 2"));

        assert_eq!(plan.completion_percentage(), 0.0);

        plan.pacing_beats[0].completed = true;
        assert!((plan.completion_percentage() - 50.0).abs() < 0.01);

        plan.pacing_beats[1].completed = true;
        assert!((plan.completion_percentage() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_session_plan_template() {
        let plan = SessionPlan::new("camp-1", "Original")
            .with_dramatic_question("Will they survive?");

        let template = plan.to_template("Combat Template");

        assert!(template.is_template);
        assert_ne!(template.id, plan.id);
        assert_eq!(template.dramatic_questions, plan.dramatic_questions);
    }

    #[test]
    fn test_session_plan_from_template() {
        let template = SessionPlan::new_template("camp-1", "Combat Template");

        let plan = template.instantiate(5);

        assert!(!plan.is_template);
        assert_eq!(plan.session_number, Some(5));
        assert_ne!(plan.id, template.id);
    }

    #[test]
    fn test_pacing_templates() {
        let combat = pacing_templates::combat_heavy();
        assert_eq!(combat.len(), 5);

        let roleplay = pacing_templates::roleplay_focused();
        assert_eq!(roleplay.len(), 6);

        let exploration = pacing_templates::exploration();
        assert_eq!(exploration.len(), 6);
    }

    #[test]
    fn test_serialization() {
        let plan = SessionPlan::new("camp-1", "Test")
            .with_dramatic_question("Will they win?");

        let json = serde_json::to_string(&plan).unwrap();

        // Check camelCase
        assert!(json.contains("campaignId"));
        assert!(json.contains("dramaticQuestions"));
        assert!(json.contains("isTemplate"));

        // Round-trip
        let parsed: SessionPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.title, "Test");
    }
}
