//! Phase Type Definitions
//!
//! Data structures for arc phases within campaign narratives.
//!
//! TASK-CAMP-012: ArcPhase, PhaseStatus, SessionRange, PhaseProgress

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Phase Status Enum
// ============================================================================

/// Lifecycle status of an arc phase
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    /// Phase is planned but not yet started
    #[default]
    Pending,
    /// Phase is currently active
    Active,
    /// Phase has been completed
    Completed,
    /// Phase was skipped (not required for arc completion)
    Skipped,
}

impl PhaseStatus {
    /// Check if phase is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Skipped)
    }

    /// Check if phase can be activated
    pub fn can_activate(&self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Check if transition is valid
    pub fn can_transition_to(&self, target: &PhaseStatus) -> bool {
        use PhaseStatus::*;
        matches!(
            (self, target),
            // From Pending
            (Pending, Active) | (Pending, Skipped) |
            // From Active
            (Active, Completed) | (Active, Skipped)
            // Terminal states cannot transition
        )
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        match self {
            Self::Pending => "Pending",
            Self::Active => "Active",
            Self::Completed => "Completed",
            Self::Skipped => "Skipped",
        }
    }
}

impl std::fmt::Display for PhaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

// ============================================================================
// Session Range
// ============================================================================

/// Expected session range for a phase
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRange {
    /// Minimum expected sessions
    pub min: u32,
    /// Maximum expected sessions
    pub max: u32,
}

impl SessionRange {
    /// Create a new session range
    pub fn new(min: u32, max: u32) -> Self {
        Self { min, max }
    }

    /// Create a fixed-length range (min == max)
    pub fn fixed(sessions: u32) -> Self {
        Self {
            min: sessions,
            max: sessions,
        }
    }

    /// Validate that min <= max
    pub fn validate(&self) -> bool {
        self.min <= self.max
    }

    /// Get the midpoint estimate
    pub fn estimate(&self) -> u32 {
        (self.min + self.max) / 2
    }

    /// Get the range span
    pub fn span(&self) -> u32 {
        self.max.saturating_sub(self.min)
    }

    /// Check if a session count is within range
    pub fn contains(&self, sessions: u32) -> bool {
        sessions >= self.min && sessions <= self.max
    }
}

impl Default for SessionRange {
    fn default() -> Self {
        Self { min: 1, max: 3 }
    }
}

// ============================================================================
// Arc Phase
// ============================================================================

/// A phase within a campaign arc
///
/// Phases represent distinct narrative segments within an arc,
/// such as "Setup", "Rising Action", "Climax", etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcPhase {
    /// Unique identifier
    pub id: String,
    /// Arc this phase belongs to
    pub arc_id: String,
    /// Campaign ID (denormalized for filtering)
    pub campaign_id: String,
    /// Phase name
    pub name: String,
    /// Description of what happens in this phase
    pub description: String,
    /// Current status
    pub status: PhaseStatus,
    /// Order within the arc (1-indexed)
    pub phase_order: u32,
    /// Expected session range
    pub expected_sessions: SessionRange,
    /// Actual sessions spent in this phase
    pub actual_sessions: u32,
    /// Session IDs associated with this phase
    pub associated_sessions: Vec<String>,
    /// Milestone IDs for this phase
    pub milestones: Vec<String>,
    /// Plot point IDs associated with this phase
    pub plot_points: Vec<String>,
    /// Key events or beats expected in this phase
    pub key_events: Vec<String>,
    /// Notes specific to this phase
    pub notes: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// When this phase was created
    pub created_at: DateTime<Utc>,
    /// When this phase was last updated
    pub updated_at: DateTime<Utc>,
    /// When this phase was activated
    pub activated_at: Option<DateTime<Utc>>,
    /// When this phase was completed
    pub completed_at: Option<DateTime<Utc>>,
}

impl ArcPhase {
    /// Create a new phase
    pub fn new(arc_id: &str, campaign_id: &str, name: &str, phase_order: u32) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            arc_id: arc_id.to_string(),
            campaign_id: campaign_id.to_string(),
            name: name.to_string(),
            description: String::new(),
            status: PhaseStatus::Pending,
            phase_order,
            expected_sessions: SessionRange::default(),
            actual_sessions: 0,
            associated_sessions: Vec::new(),
            milestones: Vec::new(),
            plot_points: Vec::new(),
            key_events: Vec::new(),
            notes: Vec::new(),
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            activated_at: None,
            completed_at: None,
        }
    }

    /// Builder: set description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Builder: set expected sessions
    pub fn with_expected_sessions(mut self, min: u32, max: u32) -> Self {
        self.expected_sessions = SessionRange::new(min, max);
        self
    }

    /// Builder: add key events
    pub fn with_key_events(mut self, events: Vec<String>) -> Self {
        self.key_events = events;
        self
    }

    /// Associate a session with this phase
    pub fn associate_session(&mut self, session_id: &str) {
        if !self.associated_sessions.contains(&session_id.to_string()) {
            self.associated_sessions.push(session_id.to_string());
            self.actual_sessions = self.associated_sessions.len() as u32;
            self.updated_at = Utc::now();
        }
    }

    /// Add a milestone to this phase
    pub fn add_milestone(&mut self, milestone_id: &str) {
        if !self.milestones.contains(&milestone_id.to_string()) {
            self.milestones.push(milestone_id.to_string());
            self.updated_at = Utc::now();
        }
    }

    /// Add a plot point to this phase
    pub fn add_plot_point(&mut self, plot_point_id: &str) {
        if !self.plot_points.contains(&plot_point_id.to_string()) {
            self.plot_points.push(plot_point_id.to_string());
            self.updated_at = Utc::now();
        }
    }

    /// Activate this phase
    pub fn activate(&mut self) -> bool {
        if self.status.can_activate() {
            self.status = PhaseStatus::Active;
            self.activated_at = Some(Utc::now());
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Complete this phase
    pub fn complete(&mut self) -> bool {
        if self.status == PhaseStatus::Active {
            self.status = PhaseStatus::Completed;
            self.completed_at = Some(Utc::now());
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Skip this phase
    pub fn skip(&mut self) -> bool {
        if self.status.can_transition_to(&PhaseStatus::Skipped) {
            self.status = PhaseStatus::Skipped;
            self.completed_at = Some(Utc::now());
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Check if this phase is over session budget
    pub fn is_over_budget(&self) -> bool {
        self.actual_sessions > self.expected_sessions.max
    }

    /// Check if this phase is under session budget
    pub fn is_under_budget(&self) -> bool {
        self.status.is_terminal() && self.actual_sessions < self.expected_sessions.min
    }
}

// ============================================================================
// Phase Progress
// ============================================================================

/// Progress tracking for a phase
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseProgress {
    /// Phase ID
    pub phase_id: String,
    /// Phase name
    pub phase_name: String,
    /// Current status
    pub status: PhaseStatus,
    /// Total milestones in phase
    pub total_milestones: usize,
    /// Achieved milestones
    pub achieved_milestones: usize,
    /// Sessions spent
    pub sessions_spent: u32,
    /// Expected session range
    pub expected_sessions: SessionRange,
    /// Percent complete (milestones-based)
    pub percent_complete: f64,
    /// Is the phase over budget?
    pub is_over_budget: bool,
}

impl PhaseProgress {
    /// Create new phase progress
    pub fn new(
        phase_id: &str,
        phase_name: &str,
        status: PhaseStatus,
        total_milestones: usize,
        achieved_milestones: usize,
        sessions_spent: u32,
        expected_sessions: SessionRange,
    ) -> Self {
        let percent_complete = if total_milestones == 0 {
            // If no milestones, base on status
            if status.is_terminal() {
                100.0
            } else {
                0.0
            }
        } else {
            ((achieved_milestones as f64 / total_milestones as f64) * 100.0).clamp(0.0, 100.0)
        };

        let is_over_budget = sessions_spent > expected_sessions.max;

        Self {
            phase_id: phase_id.to_string(),
            phase_name: phase_name.to_string(),
            status,
            total_milestones,
            achieved_milestones,
            sessions_spent,
            expected_sessions,
            percent_complete,
            is_over_budget,
        }
    }
}

// ============================================================================
// Phase Templates
// ============================================================================

/// Template for creating default phases
#[derive(Debug, Clone)]
pub struct PhaseTemplate {
    pub name: &'static str,
    pub description: &'static str,
    pub expected_sessions: SessionRange,
}

/// Get default phases for Linear arc type
pub fn linear_arc_phases() -> Vec<PhaseTemplate> {
    vec![
        PhaseTemplate {
            name: "Setup",
            description: "Establish the status quo, introduce characters and setting",
            expected_sessions: SessionRange::new(1, 2),
        },
        PhaseTemplate {
            name: "Inciting Incident",
            description: "The event that disrupts the status quo and starts the adventure",
            expected_sessions: SessionRange::new(1, 1),
        },
        PhaseTemplate {
            name: "Rising Action",
            description: "Challenges and complications that build tension",
            expected_sessions: SessionRange::new(2, 4),
        },
        PhaseTemplate {
            name: "Midpoint",
            description: "A major revelation or turning point",
            expected_sessions: SessionRange::new(1, 2),
        },
        PhaseTemplate {
            name: "Climax",
            description: "The decisive confrontation or challenge",
            expected_sessions: SessionRange::new(1, 2),
        },
        PhaseTemplate {
            name: "Denouement",
            description: "Resolution and aftermath",
            expected_sessions: SessionRange::new(1, 1),
        },
    ]
}

/// Get default phases for Mystery arc type
pub fn mystery_arc_phases() -> Vec<PhaseTemplate> {
    vec![
        PhaseTemplate {
            name: "Crime/Hook",
            description: "The mystery is presented and investigators are engaged",
            expected_sessions: SessionRange::new(1, 2),
        },
        PhaseTemplate {
            name: "Investigation",
            description: "Gathering clues, interviewing witnesses, following leads",
            expected_sessions: SessionRange::new(3, 5),
        },
        PhaseTemplate {
            name: "Revelation",
            description: "Major clues come together, suspect identified",
            expected_sessions: SessionRange::new(1, 2),
        },
        PhaseTemplate {
            name: "Complications",
            description: "New obstacles, false leads, or escalation",
            expected_sessions: SessionRange::new(1, 3),
        },
        PhaseTemplate {
            name: "Confrontation",
            description: "Face the culprit, resolve the mystery",
            expected_sessions: SessionRange::new(1, 2),
        },
    ]
}

/// Get default phases for Heist arc type
pub fn heist_arc_phases() -> Vec<PhaseTemplate> {
    vec![
        PhaseTemplate {
            name: "Briefing",
            description: "The job is presented, stakes established",
            expected_sessions: SessionRange::new(1, 1),
        },
        PhaseTemplate {
            name: "Reconnaissance",
            description: "Scouting the target, gathering intelligence",
            expected_sessions: SessionRange::new(1, 2),
        },
        PhaseTemplate {
            name: "Planning",
            description: "Assembling the team, planning the approach",
            expected_sessions: SessionRange::new(1, 2),
        },
        PhaseTemplate {
            name: "Execution",
            description: "Carrying out the heist",
            expected_sessions: SessionRange::new(2, 3),
        },
        PhaseTemplate {
            name: "Escape",
            description: "Getting away, dealing with complications",
            expected_sessions: SessionRange::new(1, 2),
        },
    ]
}

/// Get default phases for Sandbox arc type
pub fn sandbox_arc_phases() -> Vec<PhaseTemplate> {
    vec![PhaseTemplate {
        name: "World Introduction",
        description: "Open exploration and world discovery",
        expected_sessions: SessionRange::new(1, 100), // Essentially unlimited
    }]
}

/// Get default phases for Branching arc type
pub fn branching_arc_phases() -> Vec<PhaseTemplate> {
    vec![
        PhaseTemplate {
            name: "Introduction",
            description: "Setup before the branching point",
            expected_sessions: SessionRange::new(1, 2),
        },
        PhaseTemplate {
            name: "Decision Point",
            description: "Critical choice that determines path",
            expected_sessions: SessionRange::new(1, 1),
        },
        PhaseTemplate {
            name: "Path A",
            description: "First possible branch",
            expected_sessions: SessionRange::new(2, 4),
        },
        PhaseTemplate {
            name: "Path B",
            description: "Second possible branch",
            expected_sessions: SessionRange::new(2, 4),
        },
        PhaseTemplate {
            name: "Convergence",
            description: "Paths may merge or reach similar conclusions",
            expected_sessions: SessionRange::new(1, 2),
        },
        PhaseTemplate {
            name: "Resolution",
            description: "Final outcome based on chosen path",
            expected_sessions: SessionRange::new(1, 2),
        },
    ]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_status_transitions() {
        use PhaseStatus::*;

        assert!(Pending.can_transition_to(&Active));
        assert!(Pending.can_transition_to(&Skipped));
        assert!(!Pending.can_transition_to(&Completed));

        assert!(Active.can_transition_to(&Completed));
        assert!(!Completed.can_transition_to(&Active));
    }

    #[test]
    fn test_session_range() {
        let range = SessionRange::new(2, 4);
        assert!(range.validate());
        assert_eq!(range.estimate(), 3);
        assert!(range.contains(3));
        assert!(!range.contains(5));

        let invalid_range = SessionRange::new(5, 3);
        assert!(!invalid_range.validate());
    }

    #[test]
    fn test_session_range_fixed() {
        let fixed = SessionRange::fixed(3);
        assert_eq!(fixed.min, 3);
        assert_eq!(fixed.max, 3);
        assert_eq!(fixed.estimate(), 3);
        assert_eq!(fixed.span(), 0);
    }

    #[test]
    fn test_arc_phase_new() {
        let phase = ArcPhase::new("arc-1", "camp-1", "Rising Action", 3);

        assert!(!phase.id.is_empty());
        assert_eq!(phase.arc_id, "arc-1");
        assert_eq!(phase.campaign_id, "camp-1");
        assert_eq!(phase.name, "Rising Action");
        assert_eq!(phase.phase_order, 3);
        assert_eq!(phase.status, PhaseStatus::Pending);
    }

    #[test]
    fn test_arc_phase_lifecycle() {
        let mut phase = ArcPhase::new("arc-1", "camp-1", "Test Phase", 1);

        // Activate
        assert!(phase.activate());
        assert_eq!(phase.status, PhaseStatus::Active);
        assert!(phase.activated_at.is_some());

        // Can't activate again
        assert!(!phase.activate());

        // Complete
        assert!(phase.complete());
        assert_eq!(phase.status, PhaseStatus::Completed);
        assert!(phase.completed_at.is_some());
    }

    #[test]
    fn test_arc_phase_skip() {
        let mut phase = ArcPhase::new("arc-1", "camp-1", "Optional Phase", 1);

        // Can skip from pending
        assert!(phase.skip());
        assert_eq!(phase.status, PhaseStatus::Skipped);
    }

    #[test]
    fn test_arc_phase_sessions() {
        let mut phase = ArcPhase::new("arc-1", "camp-1", "Test", 1)
            .with_expected_sessions(2, 4);

        phase.associate_session("session-1");
        phase.associate_session("session-2");
        phase.associate_session("session-3");

        assert_eq!(phase.actual_sessions, 3);
        assert!(!phase.is_over_budget());

        phase.associate_session("session-4");
        phase.associate_session("session-5");

        assert!(phase.is_over_budget());
    }

    #[test]
    fn test_phase_progress() {
        let progress = PhaseProgress::new(
            "phase-1",
            "Test Phase",
            PhaseStatus::Active,
            10,
            5,
            2,
            SessionRange::new(1, 3),
        );

        assert_eq!(progress.percent_complete, 50.0);
        assert!(!progress.is_over_budget);
    }

    #[test]
    fn test_phase_progress_over_budget() {
        let progress = PhaseProgress::new(
            "phase-1",
            "Test Phase",
            PhaseStatus::Active,
            10,
            5,
            5,
            SessionRange::new(1, 3),
        );

        assert!(progress.is_over_budget);
    }

    #[test]
    fn test_linear_arc_phases() {
        let phases = linear_arc_phases();
        assert_eq!(phases.len(), 6);
        assert_eq!(phases[0].name, "Setup");
        assert_eq!(phases[5].name, "Denouement");
    }

    #[test]
    fn test_mystery_arc_phases() {
        let phases = mystery_arc_phases();
        assert_eq!(phases.len(), 5);
        assert_eq!(phases[0].name, "Crime/Hook");
    }

    #[test]
    fn test_heist_arc_phases() {
        let phases = heist_arc_phases();
        assert_eq!(phases.len(), 5);
        assert_eq!(phases[0].name, "Briefing");
        assert_eq!(phases[4].name, "Escape");
    }

    #[test]
    fn test_serialization() {
        let phase = ArcPhase::new("arc-1", "camp-1", "Test", 1);
        let json = serde_json::to_string(&phase).unwrap();

        // Check camelCase
        assert!(json.contains("arcId"));
        assert!(json.contains("campaignId"));
        assert!(json.contains("phaseOrder"));

        // Round-trip
        let parsed: ArcPhase = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "Test");
    }
}
