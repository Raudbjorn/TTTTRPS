//! Campaign Management Module
//!
//! Provides campaign versioning, world state tracking, entity relationship management,
//! and campaign generation features (arcs, phases, milestones, session planning).

pub mod versioning;
pub mod world_state;
pub mod relationships;

// Campaign Generation modules (TASK-CAMP-001 through TASK-CAMP-017)
pub mod meilisearch_indexes;
pub mod meilisearch_client;
pub mod migration;
pub mod arc_types;
pub mod phase_types;
pub mod milestone_types;

// Campaign Intelligence Pipeline (Phase 1 - Campaign Generation Overhaul)
pub mod pipeline;
pub mod grounding;

// Campaign Creation Wizard (Phase 2 - Campaign Generation Overhaul)
pub mod wizard;

// Generation Orchestration Layer (Phase 4 - Campaign Generation Overhaul)
pub mod generation;

// Conversation Management (Phase 5 - Campaign Generation Overhaul)
pub mod conversation;

// Quick Reference Cards & Cheat Sheets (Phase 9 - Campaign Generation Overhaul)
pub mod quick_reference;
pub mod cheat_sheet;

// Random Tables & Session Recaps (Phase 8 - Campaign Generation Overhaul)
pub mod dice;
pub mod random_table;
pub mod recap;

// Re-exports for convenience
pub use versioning::{
    CampaignVersion, VersionType, CampaignDiff, DiffEntry, DiffOperation, VersionManager,
};
pub use world_state::{
    WorldState, WorldEvent, WorldEventType, LocationState, NpcRelationshipState,
    InGameDate, WorldStateManager,
};
pub use relationships::{
    EntityRelationship, RelationshipType, EntityType, RelationshipStrength,
    RelationshipManager, EntityGraph, GraphNode, GraphEdge,
};

// Campaign Generation re-exports
pub use meilisearch_indexes::{
    IndexConfig, CampaignArcsIndexConfig, SessionPlansIndexConfig, PlotPointsIndexConfig,
    INDEX_CAMPAIGN_ARCS, INDEX_SESSION_PLANS, INDEX_PLOT_POINTS,
    all_campaign_indexes, get_index_configs,
};
pub use meilisearch_client::{
    MeilisearchCampaignClient, MeilisearchCampaignError, MEILISEARCH_BATCH_SIZE,
};
pub use migration::{
    MigrationReport, MigrationStatus, MigrationOptions, MigrationState,
    MigrationError, defaults as migration_defaults,
};
pub use arc_types::{
    ArcType, ArcStatus, CampaignArc, ArcSummary, ArcProgress,
};
pub use phase_types::{
    PhaseStatus, SessionRange, ArcPhase, PhaseProgress,
};
pub use milestone_types::{
    MilestoneType, MilestoneStatus, Milestone, MilestoneRef,
};

// Campaign Intelligence Pipeline re-exports
pub use pipeline::{
    TrustLevel, CanonStatus, CampaignIntent, EntityDraft, Citation,
    TrustThresholds, PipelineError, PipelineResult,
    WizardStep, ConversationPurpose, ConversationRole, SourceType, SourceLocation,
    AcceptanceDecision, SuggestionStatus,
};

// Campaign Creation Wizard re-exports
pub use wizard::{
    WizardManager, WizardState, WizardSummary, WizardError, WizardValidationError,
    PartialCampaign, StepData, BasicsData, IntentData, ScopeData, PlayersData,
    PartyCompositionData, ArcStructureData, InitialContentData,
    SessionScope, CampaignPacing, PartyComposition, CharacterSummary, PartyRole,
    LevelRange, PartyGapAnalysis, ArcStructure, ArcTemplate, ArcPhaseConfig,
    NarrativeStyle, InitialContent, LocationDraft, NpcDraft, PlotHookDraft, PlotHookType,
    ExperienceLevel, validate_step_transition,
};

// Conversation Management re-exports (Phase 5 - Campaign Generation Overhaul)
pub use conversation::{
    ConversationManager, ConversationThread, ConversationMessage,
    ConversationError, MessagePagination, PaginatedMessages, ThreadListOptions,
    Citation as ConversationCitation, GeneratedResponse, ClarifyingQuestion,
    SuggestionAcceptResult, SuggestionRejectResult,
    get_system_prompt, parse_response, parse_clarifying_questions,
};

// Content Grounding Layer re-exports (Phase 3 - Campaign Generation Overhaul)
pub use grounding::{
    // Citation Builder
    CitationBuilder,
    // Rulebook Linker
    RulebookLinker, RulebookReference, ReferenceType, LinkedContent,
    ValidationReport, ValidatedReference, InvalidReference,
    // Usage Tracker
    UsageTracker, UsageTrackerError, UsageResult, UsageSummary, UsageOptions,
    // Flavour Searcher
    FlavourSearcher, FlavourSearchError, FlavourResult, FlavourFilters,
    LoreResult, LoreCategory, NameResult, NameType, LocationResult, LocationType,
    // Grounder Trait
    Grounder, GroundedContent, CombinedGrounder,
};

// Generation Orchestration Layer re-exports (Phase 4 - Campaign Generation Overhaul)
pub use generation::{
    // Template System
    TemplateRegistry, GenerationTemplate, TemplateType, TemplateError,
    // Orchestrator
    GenerationOrchestrator, GenerationRequest, GenerationResponse,
    GenerationType, GenerationError, GenerationConfig,
    // Context Assembly
    ContextAssembler, AssembledContext, TokenBudget, ContextError,
    // Trust Assignment
    TrustAssigner, TrustAssignment, ClaimAnalysis, TrustError,
    // Acceptance Management
    AcceptanceManager, AcceptanceError, DraftAction, AppliedEntity,
    // Character Generation
    CharacterGenerator, CharacterGenerationRequest, CharacterDraft,
    // NPC Generation (renamed to avoid conflict with wizard::NpcDraft)
    NpcGenerator, NpcGenerationRequest, NpcDraft as GeneratedNpcDraft, NpcImportance,
    // Session Generation
    SessionGenerator, SessionGenerationRequest, SessionPlanDraft,
    PacingTemplate, EncounterDifficulty,
    // Party Analysis
    PartyAnalyzer, PartyAnalysisRequest, PartySuggestion, GapAnalysis,
    // Arc Generation
    ArcGenerator, ArcGenerationRequest, ArcDraft, ArcTemplateType, TensionCurve,
};

// Quick Reference Cards & Cheat Sheets re-exports (Phase 9 - Campaign Generation Overhaul)
pub use quick_reference::{
    QuickReferenceCardManager, QuickReferenceError,
    RenderedCard, HoverPreview, QuickStat,
    CardTray, PinnedCard,
    NpcCardRenderer, LocationCardRenderer, CharacterCardRenderer,
    MAX_PINNED_CARDS, DEFAULT_CACHE_TTL_HOURS,
};
pub use cheat_sheet::{
    CheatSheetBuilder, CheatSheet, CheatSheetSection, CheatSheetItem,
    CheatSheetError, CheatSheetOptions, SectionType,
    TruncationWarning, HtmlExporter,
};

// Random Tables & Session Recaps re-exports (Phase 8 - Campaign Generation Overhaul)
pub use dice::{
    DiceNotation, DiceType, DiceRoller, DiceError, DiceResult,
    RollResult, SingleRoll,
};
pub use random_table::{
    RandomTableEngine, RandomTableError, RandomTableResult,
    RandomTable, TableEntry, TableRollResult,
    CreateTableRequest, TableEntryInput, RollRequest,
};
pub use recap::{
    RecapGenerator, RecapError, RecapResult,
    SessionRecap, ArcRecap, FilteredRecap,
    GenerateRecapRequest, GenerateArcRecapRequest,
    EntityReference, CharacterArcSummary, PCKnowledgeFilter,
};
