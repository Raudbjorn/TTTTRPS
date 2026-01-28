//! Database Models
//!
//! This module contains all database model types organized by domain.
//!
//! ## Modules
//!
//! - [`core`] - Core entity records (campaigns, sessions, characters, locations)
//! - [`chat`] - Chat session and message records
//! - [`generation`] - Campaign generation pipeline (wizard, drafts, canon status)
//! - [`ttrpg`] - TTRPG game mechanics (NPCs, combat, random tables)
//! - [`analytics`] - Search and usage analytics
//! - [`recap`] - Session and arc recaps
//! - [`cards`] - Quick reference cards and cheat sheets
//!
//! All types are re-exported at the module root for backwards compatibility.

pub mod analytics;
pub mod cards;
pub mod chat;
pub mod core;
pub mod generation;
pub mod recap;
pub mod ttrpg;

#[cfg(test)]
mod tests;

// ============================================================================
// Re-exports for backwards compatibility
// ============================================================================

// Core module
pub use core::{
    CampaignRecord,
    CampaignVersionRecord,
    CharacterRecord,
    ConversationMessage,
    DocumentRecord,
    EntityRelationshipRecord,
    EntityType,
    LocationRecord,
    NpcConversation,
    PersonalityRecord,
    SessionEventRecord,
    SessionNoteRecord,
    SessionRecord,
    SnapshotRecord,
};

// Chat module
pub use chat::{
    ChatMessageRecord,
    ChatSessionStatus,
    GlobalChatSessionRecord,
    MessageRole,
    ProviderUsageStats,
    UsageRecord,
    UsageStats,
    VoiceProfileRecord,
};

// Generation module
pub use generation::{
    AcceptanceDecision,
    AcceptanceEventRecord,
    CampaignIntent,
    CampaignIntentRecord,
    CanonStatus,
    CanonStatusLogRecord,
    Citation,
    ConversationMessageRecord,
    ConversationPurpose,
    ConversationRole,
    ConversationThreadRecord,
    EntityDraft,
    GenerationDraftRecord,
    PartyCompositionRecord,
    SourceCitationRecord,
    SourceLocation,
    SourceType,
    Suggestion,
    SuggestionStatus,
    TrustLevel,
    WizardStateRecord,
    WizardStep,
};

// TTRPG module
pub use ttrpg::{
    AbilityScores,
    CombatRecord,
    CombatStateRecord,
    NpcRecord,
    RandomTableEntryRecord,
    RandomTableRecord,
    RandomTableType,
    RollHistoryRecord,
    StatBlock,
    StatBlockAction,
    TTRPGDocumentAttribute,
    TTRPGDocumentRecord,
    TTRPGIngestionJob,
    TableResultType,
};

// Analytics module
pub use analytics::{
    SearchAnalyticsRecord,
    SearchQueryStatsRecord,
    SearchSelectionRecord,
};

// Recap module
pub use recap::{
    ArcRecapRecord,
    PCKnowledgeFilterRecord,
    RecapStatus,
    RecapType,
    SessionRecapRecord,
};

// Cards module
pub use cards::{
    CardCacheRecord,
    CardEntityType,
    CheatSheetPreferenceRecord,
    DisclosureLevel,
    IncludeStatus,
    PinnedCardRecord,
    PreferenceType,
};
