//! Session Module
//!
//! Submodules for session management including timeline tracking,
//! advanced conditions, combat state, session notes with AI categorization,
//! and session planning with pacing templates.

pub mod timeline;
pub mod conditions;
pub mod combat;
pub mod notes;
pub mod plan_types;

// Re-exports for convenience
pub use timeline::{
    TimelineEvent, TimelineEventType, EventSeverity, EntityRef,
    SessionTimeline, TimelineSummary, CombatSummary, KeyMoment,
};

pub use conditions::{
    ConditionDuration, SaveTiming, ConditionEffect, StackingRule,
    AdvancedCondition, ConditionTracker, ConditionTemplates,
};

pub use notes::{
    NoteCategory, EntityType, EntityLink, SessionNote,
    NotesManager, CategorizationRequest, CategorizationResponse,
    DetectedEntity, NoteExport,
    build_categorization_prompt, parse_categorization_response, response_to_categories,
};

pub use plan_types::{
    SessionPlanStatus, PacingType, PacingBeat,
    EncounterDifficulty, PlannedEncounter, EnemyGroup,
    NarrativeBeat, SessionPlan,
    pacing_templates,
};

pub use combat::{
    CombatState, CombatStatus, Combatant, CombatantType,
    CombatEvent, CombatEventType, TurnResult,
};
