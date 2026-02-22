//! Milestone Type Definitions
//!
//! Data structures for tracking milestones within arc phases.
//!
//! TASK-CAMP-013: Milestone, MilestoneType, MilestoneStatus, MilestoneRef

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Milestone Type Enum
// ============================================================================

/// Classification of milestone requirement level
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneType {
    /// Must be achieved to complete the phase
    #[default]
    Required,
    /// Can be skipped without affecting phase completion
    Optional,
    /// Only required if certain conditions are met
    Conditional(String),
}

impl MilestoneType {
    /// Check if this milestone is always required
    pub fn is_required(&self) -> bool {
        matches!(self, Self::Required)
    }

    /// Check if this milestone is optional
    pub fn is_optional(&self) -> bool {
        matches!(self, Self::Optional)
    }

    /// Check if this milestone is conditional
    pub fn is_conditional(&self) -> bool {
        matches!(self, Self::Conditional(_))
    }

    /// Get the condition expression if conditional
    pub fn condition(&self) -> Option<&str> {
        match self {
            Self::Conditional(condition) => Some(condition.as_str()),
            _ => None,
        }
    }

    /// Get display name
    pub fn display_name(&self) -> String {
        match self {
            Self::Required => "Required".to_string(),
            Self::Optional => "Optional".to_string(),
            Self::Conditional(condition) => format!("Conditional: {}", condition),
        }
    }
}

impl std::fmt::Display for MilestoneType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Milestone Status Enum
// ============================================================================

/// Current status of a milestone
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MilestoneStatus {
    /// Not yet achieved
    #[default]
    Pending,
    /// Successfully achieved
    Achieved,
    /// Explicitly skipped (only valid for Optional type)
    Skipped,
    /// Failed to achieve (typically for Required milestones)
    Failed,
}

impl MilestoneStatus {
    /// Check if the milestone is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Achieved | Self::Skipped | Self::Failed)
    }

    /// Check if the milestone was completed successfully
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Achieved)
    }

    /// Check if transition is valid
    pub fn can_transition_to(&self, target: &MilestoneStatus) -> bool {
        use MilestoneStatus::*;
        matches!(
            (self, target),
            // From Pending
            (Pending, Achieved) | (Pending, Skipped) | (Pending, Failed)
            // Terminal states cannot transition
        )
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            Self::Pending => "Pending",
            Self::Achieved => "Achieved",
            Self::Skipped => "Skipped",
            Self::Failed => "Failed",
        }
    }
}

impl std::fmt::Display for MilestoneStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Milestone Struct
// ============================================================================

/// A milestone within a phase
///
/// Milestones represent specific achievements or events that should happen
/// during a phase, such as "Party discovers the hidden passage" or
/// "Big Bad Evil Guy is defeated".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    /// Unique identifier
    pub id: String,
    /// Phase this milestone belongs to
    pub phase_id: String,
    /// Arc ID (denormalized for filtering)
    pub arc_id: String,
    /// Campaign ID (denormalized for filtering)
    pub campaign_id: String,
    /// Milestone name
    pub name: String,
    /// Description of what achieving this milestone means
    pub description: String,
    /// Type of milestone
    pub milestone_type: MilestoneType,
    /// Current status
    pub status: MilestoneStatus,
    /// IDs of prerequisite milestones (must be achieved first)
    pub prerequisites: Vec<String>,
    /// Session ID where this milestone was achieved
    pub achieved_in_session: Option<String>,
    /// Notes about how the milestone was achieved
    pub achievement_notes: Option<String>,
    /// Related plot point IDs
    pub related_plot_points: Vec<String>,
    /// Related NPC IDs
    pub related_npcs: Vec<String>,
    /// Related location IDs
    pub related_locations: Vec<String>,
    /// Order within the phase (for display)
    pub display_order: i32,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// When this milestone was created
    pub created_at: DateTime<Utc>,
    /// When this milestone was last updated
    pub updated_at: DateTime<Utc>,
    /// When this milestone was achieved
    pub achieved_at: Option<DateTime<Utc>>,
}

impl Milestone {
    /// Create a new milestone
    pub fn new(
        phase_id: &str,
        arc_id: &str,
        campaign_id: &str,
        name: &str,
        milestone_type: MilestoneType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            phase_id: phase_id.to_string(),
            arc_id: arc_id.to_string(),
            campaign_id: campaign_id.to_string(),
            name: name.to_string(),
            description: String::new(),
            milestone_type,
            status: MilestoneStatus::Pending,
            prerequisites: Vec::new(),
            achieved_in_session: None,
            achievement_notes: None,
            related_plot_points: Vec::new(),
            related_npcs: Vec::new(),
            related_locations: Vec::new(),
            display_order: 0,
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            achieved_at: None,
        }
    }

    /// Builder: set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Builder: set prerequisites
    pub fn with_prerequisites(mut self, prerequisites: Vec<String>) -> Self {
        self.prerequisites = prerequisites;
        self
    }

    /// Builder: add a prerequisite
    pub fn requires(mut self, prerequisite_id: &str) -> Self {
        if !self.prerequisites.contains(&prerequisite_id.to_string()) {
            self.prerequisites.push(prerequisite_id.to_string());
        }
        self
    }

    /// Builder: set display order
    pub fn with_order(mut self, order: i32) -> Self {
        self.display_order = order;
        self
    }

    /// Builder: link to plot point
    pub fn with_plot_point(mut self, plot_point_id: &str) -> Self {
        if !self.related_plot_points.contains(&plot_point_id.to_string()) {
            self.related_plot_points.push(plot_point_id.to_string());
        }
        self
    }

    /// Builder: link to NPC
    pub fn with_npc(mut self, npc_id: &str) -> Self {
        if !self.related_npcs.contains(&npc_id.to_string()) {
            self.related_npcs.push(npc_id.to_string());
        }
        self
    }

    /// Builder: link to location
    pub fn with_location(mut self, location_id: &str) -> Self {
        if !self.related_locations.contains(&location_id.to_string()) {
            self.related_locations.push(location_id.to_string());
        }
        self
    }

    /// Check if all prerequisites are achieved
    /// Takes a closure to check if a milestone ID is achieved
    pub fn prerequisites_met<F>(&self, is_achieved: F) -> bool
    where
        F: Fn(&str) -> bool,
    {
        self.prerequisites.iter().all(|prereq_id| is_achieved(prereq_id))
    }

    /// Check if this milestone can be achieved
    pub fn can_achieve(&self) -> bool {
        self.status == MilestoneStatus::Pending
    }

    /// Check if this milestone can be skipped
    pub fn can_skip(&self) -> bool {
        self.status == MilestoneStatus::Pending && self.milestone_type.is_optional()
    }

    /// Achieve this milestone
    pub fn achieve(&mut self, session_id: Option<&str>, notes: Option<&str>) -> bool {
        if !self.can_achieve() {
            return false;
        }

        self.status = MilestoneStatus::Achieved;
        self.achieved_at = Some(Utc::now());
        self.achieved_in_session = session_id.map(|s| s.to_string());
        self.achievement_notes = notes.map(|s| s.to_string());
        self.updated_at = Utc::now();
        true
    }

    /// Skip this milestone (only valid for Optional milestones)
    pub fn skip(&mut self) -> bool {
        if !self.can_skip() {
            return false;
        }

        self.status = MilestoneStatus::Skipped;
        self.updated_at = Utc::now();
        true
    }

    /// Mark this milestone as failed
    pub fn fail(&mut self, notes: Option<&str>) -> bool {
        if self.status != MilestoneStatus::Pending {
            return false;
        }

        self.status = MilestoneStatus::Failed;
        self.achievement_notes = notes.map(|s| s.to_string());
        self.updated_at = Utc::now();
        true
    }

    /// Get a reference for display purposes
    pub fn to_ref(&self) -> MilestoneRef {
        MilestoneRef {
            id: self.id.clone(),
            name: self.name.clone(),
            milestone_type: self.milestone_type.clone(),
            status: self.status.clone(),
            phase_id: self.phase_id.clone(),
        }
    }
}

// ============================================================================
// Milestone Reference
// ============================================================================

/// Lightweight reference to a milestone for display
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneRef {
    /// Milestone ID
    pub id: String,
    /// Milestone name
    pub name: String,
    /// Type of milestone
    pub milestone_type: MilestoneType,
    /// Current status
    pub status: MilestoneStatus,
    /// Parent phase ID
    pub phase_id: String,
}

impl MilestoneRef {
    /// Check if this is a blocking milestone (required and not achieved)
    pub fn is_blocking(&self) -> bool {
        self.milestone_type.is_required() && self.status == MilestoneStatus::Pending
    }
}

// ============================================================================
// Milestone Summary for Phase Completion
// ============================================================================

/// Summary of milestone completion for a phase
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MilestoneSummary {
    /// Phase ID
    pub phase_id: String,
    /// Total milestones
    pub total: usize,
    /// Required milestones
    pub required: usize,
    /// Achieved milestones
    pub achieved: usize,
    /// Skipped milestones
    pub skipped: usize,
    /// Failed milestones
    pub failed: usize,
    /// Required milestones that are still pending
    pub blocking: usize,
    /// Can the phase be completed?
    pub can_complete_phase: bool,
}

impl MilestoneSummary {
    /// Calculate summary from a list of milestones
    pub fn from_milestones(phase_id: &str, milestones: &[Milestone]) -> Self {
        let total = milestones.len();
        let required = milestones
            .iter()
            .filter(|m| m.milestone_type.is_required())
            .count();
        let achieved = milestones
            .iter()
            .filter(|m| m.status == MilestoneStatus::Achieved)
            .count();
        let skipped = milestones
            .iter()
            .filter(|m| m.status == MilestoneStatus::Skipped)
            .count();
        let failed = milestones
            .iter()
            .filter(|m| m.status == MilestoneStatus::Failed)
            .count();

        // Blocking = required milestones that are still pending
        let blocking = milestones
            .iter()
            .filter(|m| m.milestone_type.is_required() && m.status == MilestoneStatus::Pending)
            .count();

        // Phase can be completed if all required milestones are achieved
        let required_achieved = milestones
            .iter()
            .filter(|m| m.milestone_type.is_required() && m.status == MilestoneStatus::Achieved)
            .count();
        let can_complete_phase = required_achieved == required;

        Self {
            phase_id: phase_id.to_string(),
            total,
            required,
            achieved,
            skipped,
            failed,
            blocking,
            can_complete_phase,
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
    fn test_milestone_type() {
        assert!(MilestoneType::Required.is_required());
        assert!(MilestoneType::Optional.is_optional());
        assert!(MilestoneType::Conditional("has_key".to_string()).is_conditional());

        let cond = MilestoneType::Conditional("test_condition".to_string());
        assert_eq!(cond.condition(), Some("test_condition"));
    }

    #[test]
    fn test_milestone_status_transitions() {
        use MilestoneStatus::*;

        assert!(Pending.can_transition_to(&Achieved));
        assert!(Pending.can_transition_to(&Skipped));
        assert!(Pending.can_transition_to(&Failed));

        assert!(!Achieved.can_transition_to(&Pending));
        assert!(!Skipped.can_transition_to(&Achieved));
    }

    #[test]
    fn test_milestone_new() {
        let milestone = Milestone::new(
            "phase-1",
            "arc-1",
            "camp-1",
            "Defeat the Dragon",
            MilestoneType::Required,
        );

        assert!(!milestone.id.is_empty());
        assert_eq!(milestone.name, "Defeat the Dragon");
        assert_eq!(milestone.milestone_type, MilestoneType::Required);
        assert_eq!(milestone.status, MilestoneStatus::Pending);
    }

    #[test]
    fn test_milestone_achieve() {
        let mut milestone = Milestone::new(
            "phase-1",
            "arc-1",
            "camp-1",
            "Find the Key",
            MilestoneType::Required,
        );

        assert!(milestone.can_achieve());
        assert!(milestone.achieve(Some("session-5"), Some("Found in the dungeon")));
        assert_eq!(milestone.status, MilestoneStatus::Achieved);
        assert!(milestone.achieved_at.is_some());
        assert_eq!(milestone.achieved_in_session, Some("session-5".to_string()));
        assert_eq!(
            milestone.achievement_notes,
            Some("Found in the dungeon".to_string())
        );

        // Can't achieve again
        assert!(!milestone.achieve(None, None));
    }

    #[test]
    fn test_milestone_skip_optional() {
        let mut optional = Milestone::new(
            "phase-1",
            "arc-1",
            "camp-1",
            "Optional Sidequest",
            MilestoneType::Optional,
        );

        assert!(optional.can_skip());
        assert!(optional.skip());
        assert_eq!(optional.status, MilestoneStatus::Skipped);
    }

    #[test]
    fn test_milestone_skip_required_fails() {
        let mut required = Milestone::new(
            "phase-1",
            "arc-1",
            "camp-1",
            "Required Task",
            MilestoneType::Required,
        );

        assert!(!required.can_skip());
        assert!(!required.skip());
        assert_eq!(required.status, MilestoneStatus::Pending);
    }

    #[test]
    fn test_milestone_prerequisites() {
        let milestone = Milestone::new(
            "phase-1",
            "arc-1",
            "camp-1",
            "Open the Door",
            MilestoneType::Required,
        )
        .requires("prereq-1")
        .requires("prereq-2");

        assert_eq!(milestone.prerequisites.len(), 2);

        // All achieved
        assert!(milestone.prerequisites_met(|id| id == "prereq-1" || id == "prereq-2"));

        // One not achieved
        assert!(!milestone.prerequisites_met(|id| id == "prereq-1"));
    }

    #[test]
    fn test_milestone_ref() {
        let milestone = Milestone::new(
            "phase-1",
            "arc-1",
            "camp-1",
            "Test Milestone",
            MilestoneType::Required,
        );

        let ref_obj = milestone.to_ref();
        assert_eq!(ref_obj.name, "Test Milestone");
        assert!(ref_obj.is_blocking()); // Required + Pending = blocking
    }

    #[test]
    fn test_milestone_summary() {
        let milestones = vec![
            Milestone::new("p1", "a1", "c1", "M1", MilestoneType::Required),
            {
                let mut m = Milestone::new("p1", "a1", "c1", "M2", MilestoneType::Required);
                m.status = MilestoneStatus::Achieved;
                m
            },
            {
                let mut m = Milestone::new("p1", "a1", "c1", "M3", MilestoneType::Optional);
                m.status = MilestoneStatus::Skipped;
                m
            },
        ];

        let summary = MilestoneSummary::from_milestones("p1", &milestones);

        assert_eq!(summary.total, 3);
        assert_eq!(summary.required, 2);
        assert_eq!(summary.achieved, 1);
        assert_eq!(summary.skipped, 1);
        assert_eq!(summary.blocking, 1); // One required still pending
        assert!(!summary.can_complete_phase); // Not all required milestones achieved
    }

    #[test]
    fn test_milestone_summary_can_complete() {
        let milestones = vec![
            {
                let mut m = Milestone::new("p1", "a1", "c1", "M1", MilestoneType::Required);
                m.status = MilestoneStatus::Achieved;
                m
            },
            {
                let mut m = Milestone::new("p1", "a1", "c1", "M2", MilestoneType::Required);
                m.status = MilestoneStatus::Achieved;
                m
            },
            Milestone::new("p1", "a1", "c1", "M3", MilestoneType::Optional),
        ];

        let summary = MilestoneSummary::from_milestones("p1", &milestones);

        assert!(summary.can_complete_phase); // All required milestones achieved
        assert_eq!(summary.blocking, 0);
    }

    #[test]
    fn test_serialization() {
        let milestone = Milestone::new(
            "phase-1",
            "arc-1",
            "camp-1",
            "Test",
            MilestoneType::Conditional("has_item:key".to_string()),
        );

        let json = serde_json::to_string(&milestone).unwrap();

        // Check camelCase
        assert!(json.contains("phaseId"));
        assert!(json.contains("milestoneType"));
        assert!(json.contains("achievementNotes"));

        // Round-trip
        let parsed: Milestone = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test");
        assert!(matches!(parsed.milestone_type, MilestoneType::Conditional(_)));
    }
}
