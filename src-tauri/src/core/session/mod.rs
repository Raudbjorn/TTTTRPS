//! Session Module
//!
//! Sub-modules for session management including timeline tracking,
//! advanced conditions, and session notes with AI categorization.

pub mod timeline;
pub mod conditions;
pub mod notes;

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
