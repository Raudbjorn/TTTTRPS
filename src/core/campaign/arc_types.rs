//! Arc Type Definitions
//!
//! Core enum and struct types for campaign narrative arcs.
//!
//! TASK-CAMP-010: ArcType and ArcStatus enums
//! TASK-CAMP-011: CampaignArc, ArcSummary, ArcProgress structs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Arc Type Enum (TASK-CAMP-010)
// ============================================================================

/// Classification of narrative arc structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArcType {
    /// Linear progression through phases
    #[default]
    Linear,
    /// Multiple branching paths based on choices
    Branching,
    /// Open-world exploration with optional goals
    Sandbox,
    /// Investigation and clue-gathering focused
    Mystery,
    /// Planning and execution of a complex operation
    Heist,
    /// Custom arc type defined by the user
    Custom(String),
}

impl ArcType {
    /// Get display name for the arc type
    pub fn display_name(&self) -> &str {
        match self {
            Self::Linear => "Linear",
            Self::Branching => "Branching",
            Self::Sandbox => "Sandbox",
            Self::Mystery => "Mystery",
            Self::Heist => "Heist",
            Self::Custom(name) => name.as_str(),
        }
    }

    /// Get description of the arc type
    pub fn description(&self) -> &str {
        match self {
            Self::Linear => "A straightforward narrative progression through distinct phases",
            Self::Branching => "Multiple paths that diverge based on player choices",
            Self::Sandbox => "Open exploration with emergent storylines",
            Self::Mystery => "Investigation-focused arc with clues and revelations",
            Self::Heist => "Planning and executing a complex operation",
            Self::Custom(_) => "A custom arc type defined by the user",
        }
    }

    /// Check if this arc type has fixed phases
    pub fn has_fixed_phases(&self) -> bool {
        matches!(self, Self::Linear | Self::Mystery | Self::Heist)
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "linear" => Self::Linear,
            "branching" => Self::Branching,
            "sandbox" => Self::Sandbox,
            "mystery" => Self::Mystery,
            "heist" => Self::Heist,
            other => Self::Custom(other.to_string()),
        }
    }
}

impl std::fmt::Display for ArcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Arc Status Enum (TASK-CAMP-010)
// ============================================================================

/// Lifecycle status of a campaign arc
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ArcStatus {
    /// Arc is being planned but not yet started
    #[default]
    Planning,
    /// Arc is currently in progress
    Active,
    /// Arc is temporarily on hold
    Paused,
    /// Arc has been successfully completed
    Completed,
    /// Arc ended in failure (for the party)
    Failed,
    /// Arc was abandoned or canceled
    Abandoned,
}

impl ArcStatus {
    /// Check if the arc is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Abandoned)
    }

    /// Check if the arc is currently playable
    pub fn is_playable(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Check if transition to target status is valid
    pub fn can_transition_to(&self, target: &ArcStatus) -> bool {
        use ArcStatus::*;
        matches!(
            (self, target),
            // From Planning
            (Planning, Active) | (Planning, Abandoned) |
            // From Active
            (Active, Paused) | (Active, Completed) | (Active, Failed) | (Active, Abandoned) |
            // From Paused
            (Paused, Active) | (Paused, Abandoned)
            // Terminal states cannot transition
        )
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            Self::Planning => "Planning",
            Self::Active => "Active",
            Self::Paused => "Paused",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Abandoned => "Abandoned",
        }
    }
}

impl std::fmt::Display for ArcStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Campaign Arc Struct (TASK-CAMP-011)
// ============================================================================

/// A narrative arc within a campaign
///
/// Arcs represent major story segments with distinct phases, milestones,
/// and narrative goals.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignArc {
    /// Unique identifier
    pub id: String,
    /// Campaign this arc belongs to
    pub campaign_id: String,
    /// Arc name
    pub name: String,
    /// Detailed description
    pub description: String,
    /// Story premise or hook
    pub premise: String,
    /// Arc type classification
    pub arc_type: ArcType,
    /// Current status
    pub status: ArcStatus,
    /// Is this the main story arc?
    pub is_main_arc: bool,
    /// Display order within campaign
    pub display_order: i32,
    /// IDs of phases in this arc (ordered)
    pub phases: Vec<String>,
    /// IDs of plot points associated with this arc
    pub plot_points: Vec<String>,
    /// Primary antagonist or opposing force
    pub antagonist: Option<String>,
    /// Key themes explored in this arc
    pub themes: Vec<String>,
    /// Expected outcome or resolution
    pub expected_resolution: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// When this arc was created
    pub created_at: DateTime<Utc>,
    /// When this arc was last updated
    pub updated_at: DateTime<Utc>,
    /// When this arc was started (status -> Active)
    pub started_at: Option<DateTime<Utc>>,
    /// When this arc was completed/ended
    pub ended_at: Option<DateTime<Utc>>,
}

impl CampaignArc {
    /// Create a new campaign arc
    pub fn new(campaign_id: &str, name: &str, arc_type: ArcType) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            name: name.to_string(),
            description: String::new(),
            premise: String::new(),
            arc_type,
            status: ArcStatus::Planning,
            is_main_arc: false,
            display_order: 0,
            phases: Vec::new(),
            plot_points: Vec::new(),
            antagonist: None,
            themes: Vec::new(),
            expected_resolution: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            started_at: None,
            ended_at: None,
        }
    }

    /// Builder: set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Builder: set premise
    pub fn with_premise(mut self, premise: &str) -> Self {
        self.premise = premise.to_string();
        self
    }

    /// Builder: mark as main arc
    pub fn as_main_arc(mut self) -> Self {
        self.is_main_arc = true;
        self
    }

    /// Builder: set display order
    pub fn with_order(mut self, order: i32) -> Self {
        self.display_order = order;
        self
    }

    /// Builder: set antagonist
    pub fn with_antagonist(mut self, antagonist: &str) -> Self {
        self.antagonist = Some(antagonist.to_string());
        self
    }

    /// Builder: add themes
    pub fn with_themes(mut self, themes: Vec<String>) -> Self {
        self.themes = themes;
        self
    }

    /// Add a phase ID
    pub fn add_phase(&mut self, phase_id: &str) {
        if !self.phases.contains(&phase_id.to_string()) {
            self.phases.push(phase_id.to_string());
            self.updated_at = Utc::now();
        }
    }

    /// Add a plot point ID
    pub fn add_plot_point(&mut self, plot_point_id: &str) {
        if !self.plot_points.contains(&plot_point_id.to_string()) {
            self.plot_points.push(plot_point_id.to_string());
            self.updated_at = Utc::now();
        }
    }

    /// Get summary for listing
    pub fn to_summary(&self) -> ArcSummary {
        ArcSummary {
            id: self.id.clone(),
            campaign_id: self.campaign_id.clone(),
            name: self.name.clone(),
            arc_type: self.arc_type.clone(),
            status: self.status.clone(),
            is_main_arc: self.is_main_arc,
            display_order: self.display_order,
            phase_count: self.phases.len(),
            plot_point_count: self.plot_points.len(),
            created_at: self.created_at,
        }
    }
}

// ============================================================================
// Arc Summary (TASK-CAMP-011)
// ============================================================================

/// Summary of an arc for listing operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcSummary {
    /// Unique identifier
    pub id: String,
    /// Campaign this arc belongs to
    pub campaign_id: String,
    /// Arc name
    pub name: String,
    /// Arc type classification
    pub arc_type: ArcType,
    /// Current status
    pub status: ArcStatus,
    /// Is this the main story arc?
    pub is_main_arc: bool,
    /// Display order within campaign
    pub display_order: i32,
    /// Number of phases
    pub phase_count: usize,
    /// Number of associated plot points
    pub plot_point_count: usize,
    /// When this arc was created
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Arc Progress (TASK-CAMP-011) - CRITICAL-CAMP-003 compliant
// ============================================================================

/// Progress tracking for an arc
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcProgress {
    /// Arc ID
    pub arc_id: String,
    /// Total number of phases
    pub total_phases: usize,
    /// Number of completed phases
    pub completed_phases: usize,
    /// Total number of milestones
    pub total_milestones: usize,
    /// Number of achieved milestones
    pub achieved_milestones: usize,
    /// Percentage complete (0.0 - 100.0) - CLAMPED per CRITICAL-CAMP-003
    pub percent_complete: f64,
    /// Current phase ID (if any)
    pub current_phase_id: Option<String>,
    /// Current phase name (if any)
    pub current_phase_name: Option<String>,
    /// Estimated sessions remaining
    pub estimated_sessions_remaining: Option<u32>,
}

impl ArcProgress {
    /// Create new arc progress with calculated percentage
    ///
    /// Implements CRITICAL-CAMP-003: percent_complete is clamped to 0.0-100.0
    pub fn new(
        arc_id: &str,
        total_phases: usize,
        completed_phases: usize,
        total_milestones: usize,
        achieved_milestones: usize,
    ) -> Self {
        let percent_complete = Self::calculate_percent(
            completed_phases,
            total_phases,
            achieved_milestones,
            total_milestones,
        );

        Self {
            arc_id: arc_id.to_string(),
            total_phases,
            completed_phases,
            total_milestones,
            achieved_milestones,
            percent_complete,
            current_phase_id: None,
            current_phase_name: None,
            estimated_sessions_remaining: None,
        }
    }

    /// Calculate percentage complete with bounds checking (CRITICAL-CAMP-003)
    ///
    /// Uses weighted average: 60% phases, 40% milestones
    fn calculate_percent(
        completed_phases: usize,
        total_phases: usize,
        achieved_milestones: usize,
        total_milestones: usize,
    ) -> f64 {
        // Handle division by zero
        if total_phases == 0 && total_milestones == 0 {
            return 0.0;
        }

        let phase_weight = 0.6;
        let milestone_weight = 0.4;

        let phase_percent = if total_phases > 0 {
            (completed_phases as f64 / total_phases as f64) * 100.0
        } else {
            0.0
        };

        let milestone_percent = if total_milestones > 0 {
            (achieved_milestones as f64 / total_milestones as f64) * 100.0
        } else {
            0.0
        };

        // Adjust weights if one category is empty
        let (effective_phase_weight, effective_milestone_weight) = if total_phases == 0 {
            (0.0, 1.0)
        } else if total_milestones == 0 {
            (1.0, 0.0)
        } else {
            (phase_weight, milestone_weight)
        };

        let raw_percent = (phase_percent * effective_phase_weight)
            + (milestone_percent * effective_milestone_weight);

        // CRITICAL-CAMP-003: Clamp to valid range
        raw_percent.clamp(0.0, 100.0)
    }

    /// Update with current phase info
    pub fn with_current_phase(mut self, phase_id: &str, phase_name: &str) -> Self {
        self.current_phase_id = Some(phase_id.to_string());
        self.current_phase_name = Some(phase_name.to_string());
        self
    }

    /// Update with session estimate
    pub fn with_session_estimate(mut self, remaining: u32) -> Self {
        self.estimated_sessions_remaining = Some(remaining);
        self
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_type_default() {
        let arc_type: ArcType = Default::default();
        assert_eq!(arc_type, ArcType::Linear);
    }

    #[test]
    fn test_arc_type_display() {
        assert_eq!(ArcType::Mystery.display_name(), "Mystery");
        assert_eq!(ArcType::Custom("Survival".to_string()).display_name(), "Survival");
    }

    #[test]
    fn test_arc_type_from_str() {
        assert_eq!(ArcType::from_str("linear"), ArcType::Linear);
        assert_eq!(ArcType::from_str("MYSTERY"), ArcType::Mystery);
        assert_eq!(ArcType::from_str("custom_type"), ArcType::Custom("custom_type".to_string()));
    }

    #[test]
    fn test_arc_status_transitions() {
        use ArcStatus::*;

        assert!(Planning.can_transition_to(&Active));
        assert!(Planning.can_transition_to(&Abandoned));
        assert!(!Planning.can_transition_to(&Completed));

        assert!(Active.can_transition_to(&Paused));
        assert!(Active.can_transition_to(&Completed));
        assert!(!Active.can_transition_to(&Planning));

        // Terminal states cannot transition
        assert!(!Completed.can_transition_to(&Active));
        assert!(!Failed.can_transition_to(&Planning));
    }

    #[test]
    fn test_arc_status_is_terminal() {
        assert!(!ArcStatus::Planning.is_terminal());
        assert!(!ArcStatus::Active.is_terminal());
        assert!(ArcStatus::Completed.is_terminal());
        assert!(ArcStatus::Failed.is_terminal());
        assert!(ArcStatus::Abandoned.is_terminal());
    }

    #[test]
    fn test_campaign_arc_new() {
        let arc = CampaignArc::new("camp-1", "The Dark Tower", ArcType::Linear);

        assert!(!arc.id.is_empty());
        assert_eq!(arc.campaign_id, "camp-1");
        assert_eq!(arc.name, "The Dark Tower");
        assert_eq!(arc.arc_type, ArcType::Linear);
        assert_eq!(arc.status, ArcStatus::Planning);
        assert!(!arc.is_main_arc);
    }

    #[test]
    fn test_campaign_arc_builder() {
        let arc = CampaignArc::new("camp-1", "Test Arc", ArcType::Mystery)
            .with_description("A mysterious arc")
            .with_premise("Something strange is happening")
            .as_main_arc()
            .with_antagonist("The Shadow")
            .with_order(1);

        assert_eq!(arc.description, "A mysterious arc");
        assert_eq!(arc.premise, "Something strange is happening");
        assert!(arc.is_main_arc);
        assert_eq!(arc.antagonist, Some("The Shadow".to_string()));
        assert_eq!(arc.display_order, 1);
    }

    #[test]
    fn test_arc_progress_calculation() {
        // Standard case
        let progress = ArcProgress::new("arc-1", 4, 2, 10, 5);
        // 60% * (2/4 * 100) + 40% * (5/10 * 100) = 60% * 50 + 40% * 50 = 30 + 20 = 50
        assert!((progress.percent_complete - 50.0).abs() < 0.01);

        // All complete
        let complete = ArcProgress::new("arc-2", 4, 4, 10, 10);
        assert!((complete.percent_complete - 100.0).abs() < 0.01);

        // Nothing complete
        let nothing = ArcProgress::new("arc-3", 4, 0, 10, 0);
        assert!((nothing.percent_complete - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_arc_progress_zero_division() {
        // No phases, no milestones - should not panic, return 0
        let progress = ArcProgress::new("arc-1", 0, 0, 0, 0);
        assert!((progress.percent_complete - 0.0).abs() < 0.01);

        // No phases, some milestones
        let progress = ArcProgress::new("arc-2", 0, 0, 10, 5);
        // 100% weight on milestones: 50%
        assert!((progress.percent_complete - 50.0).abs() < 0.01);

        // Some phases, no milestones
        let progress = ArcProgress::new("arc-3", 4, 2, 0, 0);
        // 100% weight on phases: 50%
        assert!((progress.percent_complete - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_arc_progress_clamping() {
        // Test that progress is always clamped to 0-100
        let progress = ArcProgress::new("arc-1", 4, 4, 10, 10);
        assert!(progress.percent_complete <= 100.0);
        assert!(progress.percent_complete >= 0.0);
    }

    #[test]
    fn test_arc_summary() {
        let mut arc = CampaignArc::new("camp-1", "Test Arc", ArcType::Heist);
        arc.phases = vec!["p1".to_string(), "p2".to_string()];
        arc.plot_points = vec!["pp1".to_string()];
        arc.is_main_arc = true;

        let summary = arc.to_summary();

        assert_eq!(summary.name, "Test Arc");
        assert_eq!(summary.arc_type, ArcType::Heist);
        assert_eq!(summary.phase_count, 2);
        assert_eq!(summary.plot_point_count, 1);
        assert!(summary.is_main_arc);
    }

    #[test]
    fn test_serialization() {
        let arc = CampaignArc::new("camp-1", "Test", ArcType::Linear);
        let json = serde_json::to_string(&arc).unwrap();

        // Check camelCase serialization
        assert!(json.contains("campaignId"));
        assert!(json.contains("arcType"));
        assert!(json.contains("isMainArc"));

        // Round-trip
        let parsed: CampaignArc = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test");
    }
}
