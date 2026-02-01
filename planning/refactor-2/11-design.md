# Design: Commands Module Refactoring

**Version:** 1.0.0
**Status:** Draft
**Last Updated:** 2026-01-25

## Overview

This document describes the comprehensive architecture for refactoring `src-tauri/src/commands_legacy.rs` (8,303 lines, 310 Tauri commands) into domain-specific modules following established extraction patterns.

### Design Goals

1. **Maintainability**: Break 8,303-line monolith into focused modules (<500 lines each)
2. **Cohesion**: Group commands by domain following Settings UI alignment
3. **Consistency**: Follow patterns established by voice/, oauth/, and archetype/ extractions
4. **Testability**: Enable unit testing through trait-based state access
5. **Backward Compatibility**: Zero breaking changes to frontend bindings

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Domain-based grouping | Mirrors Settings UI tabs, matches user mental model |
| Glob re-exports (`pub use module::*`) | Required for Tauri `__cmd__` macro exports |
| Preserve function names exactly | Frontend bindings depend on exact command names |
| Unified `CommandError` type | Eliminates 400+ `.map_err(\|e\| e.to_string())` patterns |
| Shared state access traits | Enables mock injection for unit testing |
| Submodule split at 500 lines | Keeps files navigable, IDE-friendly |

---

## Architecture

### Current State

```
src-tauri/src/
├── commands/
│   ├── mod.rs              # Re-exports + legacy bridge
│   ├── error.rs            # CommandError (exists)
│   ├── macros.rs           # Helper macros (exists)
│   ├── voice/              # [EXTRACTED] 8 submodules
│   ├── oauth/              # [EXTRACTED] 3 providers
│   └── archetype/          # [EXTRACTED] 5 submodules
└── commands_legacy.rs      # 8,303 lines, 310 commands
```

### Target State

```
src-tauri/src/commands/
├── mod.rs                  # Re-exports all modules (~300 lines)
├── error.rs                # Unified CommandError (~80 lines)
├── macros.rs               # State access macros (~50 lines)
├── state.rs                # AppState + init (extracted from legacy)
├── types.rs                # Shared request/response types (~200 lines)
│
├── voice/                  # [DONE] Voice synthesis (8 files)
├── oauth/                  # [DONE] OAuth flows (4 files)
├── archetype/              # [DONE] Archetype system (5 files)
│
├── llm/                    # LLM configuration, chat, streaming
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── types.rs            # Request/response types (~100 lines)
│   ├── config.rs           # configure_llm, get_llm_config (~150 lines)
│   ├── chat.rs             # chat, stream_chat (~200 lines)
│   ├── streaming.rs        # Stream management (~150 lines)
│   ├── models.rs           # Model listing commands (~200 lines)
│   ├── router.rs           # Router stats/health (~150 lines)
│   └── model_selector.rs   # Model selection commands (~80 lines)
│
├── campaign/               # Campaign management
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── types.rs            # Campaign types (~100 lines)
│   ├── crud.rs             # CRUD operations (~150 lines)
│   ├── theme.rs            # Theme commands (~100 lines)
│   ├── snapshots.rs        # Snapshot commands (~150 lines)
│   ├── notes.rs            # Campaign notes (~200 lines)
│   ├── stats.rs            # Campaign statistics (~80 lines)
│   └── versioning.rs       # Version management (~200 lines)
│
├── session/                # Session and combat
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── types.rs            # Session types (~150 lines)
│   ├── lifecycle.rs        # start/end/get session (~150 lines)
│   ├── chat.rs             # Chat session management (~250 lines)
│   ├── combat.rs           # Combat state (~200 lines)
│   ├── combatants.rs       # Combatant CRUD (~200 lines)
│   ├── conditions.rs       # Condition management (~250 lines)
│   ├── timeline.rs         # Timeline events (~150 lines)
│   └── notes.rs            # Session notes (~300 lines)
│
├── npc/                    # NPC management
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── types.rs            # NPC types (~100 lines)
│   ├── generation.rs       # NPC generation (~150 lines)
│   ├── crud.rs             # CRUD operations (~150 lines)
│   ├── conversations.rs    # NPC conversations (~200 lines)
│   ├── vocabulary.rs       # Vocabulary banks (~150 lines)
│   ├── naming.rs           # Naming rules (~100 lines)
│   ├── dialects.rs         # Dialect transformation (~100 lines)
│   └── indexes.rs          # NPC indexing (~100 lines)
│
├── personality/            # Personality system
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── types.rs            # Request/response types (~150 lines)
│   ├── active.rs           # Active personality (~200 lines)
│   ├── context.rs          # Personality context (~150 lines)
│   ├── styling.rs          # Dialogue/narration styling (~200 lines)
│   ├── preview.rs          # Personality preview (~150 lines)
│   ├── templates.rs        # Setting templates (~250 lines)
│   ├── blending.rs         # Personality blending (~200 lines)
│   └── contextual.rs       # Contextual personality (~250 lines)
│
├── search/                 # Search and library
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── types.rs            # Search types (~150 lines)
│   ├── query.rs            # search, hybrid_search (~200 lines)
│   ├── suggestions.rs      # Search hints/suggestions (~100 lines)
│   ├── library.rs          # Library document CRUD (~200 lines)
│   ├── ingestion.rs        # Document ingestion (~300 lines)
│   ├── extraction.rs       # Extraction settings (~200 lines)
│   ├── ttrpg_docs.rs       # TTRPG document queries (~250 lines)
│   ├── embeddings.rs       # Embedding configuration (~300 lines)
│   ├── analytics.rs        # Search analytics (~300 lines)
│   └── meilisearch.rs      # Meilisearch health/chat (~200 lines)
│
├── world/                  # World state
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── types.rs            # World state types (~100 lines)
│   ├── state.rs            # World state CRUD (~150 lines)
│   ├── calendar.rs         # Calendar/date management (~150 lines)
│   └── events.rs           # World events (~150 lines)
│
├── relationships/          # Entity relationships
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── types.rs            # Relationship types (~80 lines)
│   ├── crud.rs             # CRUD operations (~150 lines)
│   └── graph.rs            # Graph queries (~200 lines)
│
├── generation/             # Character and location generation
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── character.rs        # Character generation (~150 lines)
│   └── location.rs         # Location generation/CRUD (~400 lines)
│
├── credentials/            # API key management
│   ├── mod.rs              # Re-exports (~30 lines)
│   └── api_keys.rs         # save/get/delete API keys (~100 lines)
│
├── usage/                  # Usage tracking
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── types.rs            # Usage types (~80 lines)
│   └── tracking.rs         # Usage stats and budgets (~150 lines)
│
├── audit/                  # Audit logging
│   ├── mod.rs              # Re-exports (~30 lines)
│   ├── types.rs            # Audit types (~80 lines)
│   └── logs.rs             # Audit log queries (~200 lines)
│
└── system/                 # System utilities
    ├── mod.rs              # Re-exports (~30 lines)
    ├── info.rs             # App version, system info (~80 lines)
    ├── audio.rs            # Audio volumes (~50 lines)
    └── browser.rs          # URL opening (~50 lines)
```

### Component Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Frontend (Leptos WASM)                              │
│                         frontend/src/bindings.rs                             │
│                    (auto-generated IPC wrappers - NO CHANGES)                │
└─────────────────────────────┬───────────────────────────────────────────────┘
                              │ Tauri IPC (invoke)
┌─────────────────────────────▼───────────────────────────────────────────────┐
│                          commands/mod.rs                                     │
│                                                                              │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐          │
│  │   llm/   │ │ campaign/│ │ session/ │ │   npc/   │ │  search/ │  ...     │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘          │
│       │            │            │            │            │                  │
│  ┌────▼────────────▼────────────▼────────────▼────────────▼────┐            │
│  │                        state.rs                              │            │
│  │                   (AppState struct)                          │            │
│  └──────────────────────────┬───────────────────────────────────┘            │
│                             │                                                 │
│  ┌──────────────────────────▼───────────────────────────────────┐            │
│  │                       error.rs                                │            │
│  │              (CommandError, CommandResult)                    │            │
│  └───────────────────────────────────────────────────────────────┘            │
└──────────────────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────────────────────┐
│                            core/                                             │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐               │
│  │ llm/router │ │session_mgr │ │personality │ │ archetype/ │  ...          │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘               │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Components and Interfaces

### Component 1: Unified Error Type (`error.rs`)

**Purpose**: Eliminate repetitive error conversion patterns across 310 commands.

**Current Interface** (exists, extend as needed):
```rust
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("LLM error: {0}")]
    Llm(String),

    #[error("Voice error: {0}")]
    Voice(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

// Extended variants for new modules
impl CommandError {
    // Campaign-specific errors
    pub fn campaign_not_found(id: &str) -> Self {
        Self::NotFound(format!("Campaign not found: {}", id))
    }

    // Session-specific errors
    pub fn session_not_found(id: &str) -> Self {
        Self::NotFound(format!("Session not found: {}", id))
    }

    // NPC-specific errors
    pub fn npc_not_found(id: &str) -> Self {
        Self::NotFound(format!("NPC not found: {}", id))
    }
}

pub type CommandResult<T> = Result<T, CommandError>;
```

**Usage Pattern**:
```rust
// Before (current pattern in commands_legacy.rs)
db.get_campaign(&id).await.map_err(|e| e.to_string())?

// After (with unified error type)
db.get_campaign(&id).await?  // CommandError::Database auto-converts
```

### Component 2: State Access Module (`state.rs`)

**Purpose**: Extract `AppState` struct and initialization from `commands_legacy.rs`.

**Interface**:
```rust
//! Application state shared across all Tauri commands.

use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

/// Central application state, managed by Tauri.
pub struct AppState {
    // Database
    pub database: Database,

    // LLM
    pub llm_client: RwLock<Option<LLMClient>>,
    pub llm_config: RwLock<Option<LLMConfig>>,
    pub llm_router: AsyncRwLock<LLMRouter>,
    pub llm_manager: Arc<AsyncRwLock<LLMManager>>,

    // Campaign/Session
    pub campaign_manager: CampaignManager,
    pub session_manager: SessionManager,
    pub npc_store: NPCStore,

    // Credentials
    pub credentials: CredentialManager,

    // Voice (already extracted)
    pub voice_manager: Arc<AsyncRwLock<VoiceManager>>,

    // Search/Meilisearch
    pub sidecar_manager: Arc<SidecarManager>,
    pub search_client: Arc<SearchClient>,
    pub ingestion_pipeline: Arc<MeilisearchPipeline>,

    // Personality
    pub personality_store: Arc<PersonalityStore>,
    pub personality_manager: Arc<PersonalityApplicationManager>,

    // Campaign Extensions
    pub version_manager: VersionManager,
    pub world_state_manager: WorldStateManager,
    pub relationship_manager: RelationshipManager,
    pub location_manager: LocationManager,

    // Extraction
    pub extraction_settings: AsyncRwLock<ExtractionSettings>,

    // OAuth (already extracted)
    pub claude: Arc<ClaudeState>,
    pub gemini: Arc<GeminiState>,
    pub copilot: Arc<CopilotState>,

    // Archetype (already extracted)
    pub archetype_registry: AsyncRwLock<Option<Arc<ArchetypeRegistry>>>,
    pub vocabulary_manager: AsyncRwLock<Option<Arc<VocabularyBankManager>>>,
    pub setting_pack_loader: Arc<SettingPackLoader>,

    // Phase 4: Personality Extensions
    pub template_store: Arc<SettingTemplateStore>,
    pub blend_rule_store: Arc<BlendRuleStore>,
    pub personality_blender: Arc<PersonalityBlender>,
    pub contextual_personality_manager: Arc<ContextualPersonalityManager>,
}

impl AppState {
    /// Initialize all default state components.
    pub fn init_defaults() -> Self { /* ... */ }
}
```

**State Access Trait** (for testing):
```rust
/// Trait for accessing database from state.
#[async_trait]
pub trait DatabaseAccess {
    async fn database(&self) -> &Database;
}

/// Trait for accessing LLM router from state.
#[async_trait]
pub trait RouterAccess {
    async fn router(&self) -> tokio::sync::RwLockReadGuard<'_, LLMRouter>;
    async fn router_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, LLMRouter>;
}
```

### Component 3: Shared Types Module (`types.rs`)

**Purpose**: Define request/response types shared across multiple modules.

**Interface**:
```rust
//! Shared request/response types for Tauri commands.

use serde::{Deserialize, Serialize};

// ============================================================================
// LLM Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequestPayload {
    pub message: String,
    pub system_prompt: Option<String>,
    pub personality_id: Option<String>,
    pub context: Option<Vec<String>>,
    #[serde(default)]
    pub use_rag: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponsePayload {
    pub content: String,
    pub model: String,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LLMSettings {
    pub provider: String,
    pub api_key: Option<String>,
    pub host: Option<String>,
    pub model: String,
    pub embedding_model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub provider: String,
    pub healthy: bool,
    pub message: String,
}

// ============================================================================
// Search Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchOptions {
    pub query: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub filters: Option<SearchFilters>,
}

// ... additional shared types
```

### Component 4: Module Re-export Pattern

**Purpose**: Ensure all commands are properly exported for Tauri registration.

**Pattern** (following voice/ module):
```rust
// commands/llm/mod.rs
//! LLM Commands Module
//!
//! Commands for LLM configuration, chat, streaming, and model management.

pub mod types;
pub mod config;
pub mod chat;
pub mod streaming;
pub mod models;
pub mod router;
pub mod model_selector;

// Re-export all commands using glob to include Tauri __cmd__ macros
pub use types::*;
pub use config::*;
pub use chat::*;
pub use streaming::*;
pub use models::*;
pub use router::*;
pub use model_selector::*;
```

**Main mod.rs**:
```rust
// commands/mod.rs
//! Tauri Commands Module
//!
//! All Tauri IPC commands organized by domain.

pub mod error;
pub mod macros;
pub mod state;
pub mod types;

// Domain modules
pub mod llm;
pub mod campaign;
pub mod session;
pub mod npc;
pub mod personality;
pub mod search;
pub mod world;
pub mod relationships;
pub mod generation;
pub mod credentials;
pub mod usage;
pub mod audit;
pub mod system;

// Already extracted modules
#[macro_use]
pub mod oauth;
#[macro_use]
pub mod archetype;
pub mod voice;

// Re-export error types
pub use error::{CommandError, CommandResult};

// Re-export state
pub use state::AppState;

// Re-export all domain commands using glob exports
pub use llm::*;
pub use campaign::*;
pub use session::*;
pub use npc::*;
pub use personality::*;
pub use search::*;
pub use world::*;
pub use relationships::*;
pub use generation::*;
pub use credentials::*;
pub use usage::*;
pub use audit::*;
pub use system::*;

// Already extracted - use glob for Tauri __cmd__ macros
pub use oauth::*;
pub use archetype::*;
pub use voice::*;
```

---

## Module Specifications

### LLM Module (`commands/llm/`)

**Command Count**: 24 commands
**Estimated Lines**: ~1,060 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `config.rs` | `configure_llm`, `get_llm_config` | LLM provider configuration |
| `chat.rs` | `chat` | Non-streaming chat |
| `streaming.rs` | `stream_chat`, `cancel_stream`, `get_active_streams` | SSE streaming |
| `models.rs` | `list_ollama_models`, `list_claude_models`, `list_openai_models`, `list_gemini_models`, `list_openrouter_models`, `list_provider_models` | Model enumeration |
| `router.rs` | `get_router_stats`, `get_router_health`, `get_router_costs`, `estimate_request_cost`, `get_healthy_providers`, `set_routing_strategy`, `run_provider_health_checks`, `check_llm_health` | Router management |
| `model_selector.rs` | `get_model_selection`, `get_model_selection_for_prompt`, `set_model_override` | Model selection |

### Campaign Module (`commands/campaign/`)

**Command Count**: 19 commands
**Estimated Lines**: ~1,010 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `crud.rs` | `list_campaigns`, `create_campaign`, `get_campaign`, `update_campaign`, `delete_campaign` | Basic CRUD |
| `theme.rs` | `get_campaign_theme`, `set_campaign_theme`, `get_theme_preset` | Theme management |
| `snapshots.rs` | `create_snapshot`, `list_snapshots`, `restore_snapshot`, `export_campaign`, `import_campaign` | Snapshots and export |
| `notes.rs` | `add_campaign_note`, `get_campaign_notes`, `search_campaign_notes`, `delete_campaign_note`, `generate_campaign_cover` | Campaign notes |
| `stats.rs` | `get_campaign_stats` | Statistics |
| `versioning.rs` | `create_campaign_version`, `list_campaign_versions`, `get_campaign_version`, `compare_campaign_versions`, `rollback_campaign`, `delete_campaign_version`, `add_version_tag`, `mark_version_milestone` | Version control |

### Session Module (`commands/session/`)

**Command Count**: 36 commands
**Estimated Lines**: ~1,680 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `lifecycle.rs` | `start_session`, `get_session`, `get_active_session`, `list_sessions`, `end_session`, `create_planned_session`, `start_planned_session`, `reorder_session` | Session lifecycle |
| `chat.rs` | `get_or_create_chat_session`, `get_active_chat_session`, `get_chat_messages`, `add_chat_message`, `update_chat_message`, `link_chat_to_game_session`, `end_chat_session_and_spawn_new`, `clear_chat_messages`, `list_chat_sessions`, `get_chat_sessions_for_game` | Chat management |
| `combat.rs` | `start_combat`, `end_combat`, `get_combat` | Combat state |
| `combatants.rs` | `add_combatant`, `remove_combatant`, `next_turn`, `get_current_combatant`, `damage_combatant`, `heal_combatant` | Combatant management |
| `conditions.rs` | `add_condition`, `remove_condition`, `add_condition_advanced`, `remove_condition_by_id`, `apply_advanced_condition`, `remove_advanced_condition`, `get_combatant_conditions`, `tick_conditions_end_of_turn`, `tick_conditions_start_of_turn`, `list_condition_templates` | Condition system |
| `timeline.rs` | `add_timeline_event`, `get_session_timeline`, `get_timeline_summary`, `get_timeline_events_by_type` | Timeline events |
| `notes.rs` | `create_session_note`, `get_session_note`, `update_session_note`, `delete_session_note`, `list_session_notes`, `search_session_notes`, `get_notes_by_category`, `get_notes_by_tag`, `categorize_note_ai`, `link_entity_to_note`, `unlink_entity_from_note` | Session notes |

### NPC Module (`commands/npc/`)

**Command Count**: 18 commands
**Estimated Lines**: ~1,060 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `generation.rs` | `generate_npc` | NPC generation |
| `crud.rs` | `get_npc`, `list_npcs`, `update_npc`, `delete_npc`, `search_npcs` | CRUD operations |
| `conversations.rs` | `list_npc_conversations`, `get_npc_conversation`, `add_npc_message`, `mark_npc_read`, `list_npc_summaries`, `reply_as_npc` | NPC conversations |
| `vocabulary.rs` | `load_vocabulary_bank`, `get_vocabulary_directory`, `get_vocabulary_phrase` | Vocabulary banks |
| `naming.rs` | `load_naming_rules`, `get_names_directory`, `get_random_name_structure`, `validate_naming_rules` | Naming rules |
| `dialects.rs` | `load_dialect`, `get_dialects_directory`, `apply_dialect` | Dialect transformation |
| `indexes.rs` | `initialize_npc_indexes`, `get_npc_indexes_stats`, `clear_npc_indexes` | NPC indexing |

### Personality Module (`commands/personality/`)

**Command Count**: 34 commands
**Estimated Lines**: ~1,580 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `active.rs` | `set_active_personality`, `get_active_personality`, `get_personality_prompt`, `set_personality_active`, `list_personalities` | Active personality |
| `context.rs` | `get_personality_context`, `get_session_personality_context`, `set_personality_context`, `clear_session_personality_context` | Context management |
| `styling.rs` | `apply_personality_to_text`, `set_narrator_personality`, `assign_npc_personality`, `unassign_npc_personality`, `set_scene_mood`, `set_personality_settings`, `style_npc_dialogue`, `build_npc_system_prompt`, `build_npc_system_prompt_stub`, `build_narration_prompt`, `get_session_system_prompt` | Styling commands |
| `preview.rs` | `preview_personality`, `preview_personality_extended`, `generate_personality_preview`, `test_personality` | Preview generation |
| `templates.rs` | `list_personality_templates`, `filter_templates_by_game_system`, `filter_templates_by_setting`, `search_personality_templates`, `get_template_preview`, `apply_template_to_campaign`, `create_template_from_personality`, `export_personality_template`, `import_personality_template` | Setting templates |
| `blending.rs` | `set_blend_rule`, `get_blend_rule`, `list_blend_rules`, `delete_blend_rule`, `get_blender_cache_stats`, `get_blend_rule_cache_stats` | Personality blending |
| `contextual.rs` | `detect_gameplay_context`, `get_contextual_personality`, `get_current_context`, `clear_context_history`, `get_contextual_personality_config`, `set_contextual_personality_config`, `list_gameplay_contexts` | Contextual personality |

### Search Module (`commands/search/`)

**Command Count**: 45 commands
**Estimated Lines**: ~2,210 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `query.rs` | `search`, `hybrid_search` | Search execution |
| `suggestions.rs` | `get_search_suggestions`, `get_search_hints`, `expand_query`, `correct_query` | Query assistance |
| `library.rs` | `list_library_documents`, `delete_library_document`, `update_library_document`, `rebuild_library_metadata`, `clear_and_reingest_document` | Library CRUD |
| `ingestion.rs` | `ingest_document`, `ingest_document_two_phase`, `import_layout_json`, `ingest_pdf` | Document ingestion |
| `extraction.rs` | `get_extraction_settings`, `save_extraction_settings`, `get_supported_formats`, `get_extraction_presets`, `check_ocr_availability` | Extraction config |
| `ttrpg_docs.rs` | `list_ttrpg_documents_by_source`, `list_ttrpg_documents_by_type`, `list_ttrpg_documents_by_system`, `search_ttrpg_documents_by_name`, `list_ttrpg_documents_by_cr`, `get_ttrpg_document`, `get_ttrpg_document_attributes`, `find_ttrpg_documents_by_attribute`, `delete_ttrpg_document`, `get_ttrpg_document_stats`, `count_ttrpg_documents_by_type`, `get_ttrpg_ingestion_job`, `get_ttrpg_ingestion_job_by_document`, `list_pending_ttrpg_ingestion_jobs`, `list_active_ttrpg_ingestion_jobs` | TTRPG documents |
| `embeddings.rs` | `get_vector_store_status`, `configure_meilisearch_embedder`, `setup_ollama_embeddings`, `get_embedder_status`, `list_ollama_embedding_models`, `list_local_embedding_models`, `setup_local_embeddings` | Embeddings config |
| `analytics.rs` | `get_search_analytics`, `get_popular_queries`, `get_cache_stats`, `get_trending_queries`, `get_zero_result_queries`, `get_click_distribution`, `record_search_selection`, `get_search_analytics_db`, `get_popular_queries_db`, `get_cache_stats_db`, `get_trending_queries_db`, `get_zero_result_queries_db`, `get_click_distribution_db`, `record_search_event`, `record_search_selection_db`, `cleanup_search_analytics` | Analytics |
| `meilisearch.rs` | `check_meilisearch_health`, `reindex_library`, `list_chat_providers`, `configure_chat_workspace`, `get_chat_workspace_settings`, `configure_meilisearch_chat` | Meilisearch management |

### World Module (`commands/world/`)

**Command Count**: 10 commands
**Estimated Lines**: ~600 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `state.rs` | `get_world_state`, `update_world_state` | World state CRUD |
| `calendar.rs` | `set_in_game_date`, `advance_in_game_date`, `get_in_game_date`, `set_calendar_config`, `get_calendar_config` | Calendar management |
| `events.rs` | `add_world_event`, `get_world_events`, `get_recent_world_events` | World events |

### Relationships Module (`commands/relationships/`)

**Command Count**: 9 commands
**Estimated Lines**: ~580 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `crud.rs` | `create_entity_relationship`, `get_entity_relationship`, `update_entity_relationship`, `delete_entity_relationship`, `list_entity_relationships` | CRUD operations |
| `graph.rs` | `get_relationships_for_entity`, `get_relationships_between_entities`, `get_entity_graph`, `get_ego_graph` | Graph queries |

### Generation Module (`commands/generation/`)

**Command Count**: 21 commands
**Estimated Lines**: ~700 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `character.rs` | `generate_character`, `get_supported_systems`, `list_system_info`, `get_system_info`, `generate_character_advanced` | Character generation |
| `location.rs` | `generate_location`, `generate_location_quick`, `get_location_types`, `save_location`, `get_location`, `list_campaign_locations`, `delete_location`, `update_location`, `list_location_types`, `add_location_connection`, `remove_location_connection`, `search_locations`, `get_connected_locations`, `add_location_inhabitant`, `remove_location_inhabitant`, `add_location_secret`, `add_location_encounter`, `set_location_map_reference` | Location generation |

### Credentials Module (`commands/credentials/`)

**Command Count**: 4 commands
**Estimated Lines**: ~130 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `api_keys.rs` | `save_api_key`, `get_api_key`, `delete_api_key`, `list_stored_providers` | API key management |

### Usage Module (`commands/usage/`)

**Command Count**: 7 commands
**Estimated Lines**: ~230 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `tracking.rs` | `get_usage_stats`, `get_usage_by_period`, `get_cost_breakdown`, `get_budget_status`, `set_budget_limit`, `get_provider_usage`, `reset_usage_session` | Usage tracking |

### Audit Module (`commands/audit/`)

**Command Count**: 6 commands
**Estimated Lines**: ~280 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `logs.rs` | `get_audit_logs`, `query_audit_logs`, `export_audit_logs`, `clear_old_logs`, `get_audit_summary`, `get_security_events` | Audit log queries |

### System Module (`commands/system/`)

**Command Count**: 5 commands
**Estimated Lines**: ~210 total

| Submodule | Commands | Description |
|-----------|----------|-------------|
| `info.rs` | `get_app_version`, `get_app_system_info` | Application info |
| `audio.rs` | `get_audio_volumes`, `get_sfx_categories` | Audio utilities |
| `browser.rs` | `open_url_in_browser` | External links |

---

## Cross-Module Dependencies

### Dependency Graph

```
                    ┌─────────────┐
                    │   state.rs  │
                    │  (AppState) │
                    └──────┬──────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
    ┌────▼────┐      ┌────▼────┐      ┌────▼────┐
    │  llm/   │      │campaign/│      │session/ │
    └────┬────┘      └────┬────┘      └────┬────┘
         │                │                 │
         │           ┌────▼────┐           │
         │           │  npc/   │◄──────────┘
         │           └────┬────┘
         │                │
         └───────┐   ┌────▼────┐
                 │   │personal-│
                 └──►│  ity/   │
                     └─────────┘
```

### Import Patterns by Module

| Module | Imports From |
|--------|--------------|
| `llm/` | `state.rs`, `error.rs`, `types.rs` |
| `campaign/` | `state.rs`, `error.rs`, `types.rs` |
| `session/` | `state.rs`, `error.rs`, `types.rs`, `llm/types` (for chat) |
| `npc/` | `state.rs`, `error.rs`, `types.rs`, `personality/` |
| `personality/` | `state.rs`, `error.rs`, `types.rs`, `llm/` |
| `search/` | `state.rs`, `error.rs`, `types.rs` |
| `world/` | `state.rs`, `error.rs`, `campaign/` |
| `relationships/` | `state.rs`, `error.rs` |
| `generation/` | `state.rs`, `error.rs`, `types.rs`, `llm/` |
| `credentials/` | `state.rs`, `error.rs` |
| `usage/` | `state.rs`, `error.rs` |
| `audit/` | `state.rs`, `error.rs` |
| `system/` | `state.rs`, `error.rs` |

---

## Settings UI Alignment

Commands are grouped to align with the frontend Settings tabs.

| Settings Tab | Primary Module(s) | Secondary Module(s) |
|--------------|-------------------|---------------------|
| **General** | `system/` | - |
| **Intelligence** | `llm/`, `oauth/` | `personality/` |
| **Voice** | `voice/` | - |
| **Data & Library** | `search/`, `credentials/` | `generation/` |
| **Extraction** | `search/extraction.rs` | - |

---

## Testing Strategy

### Unit Test Pattern

Each command should have a corresponding test in `#[cfg(test)]` module.

```rust
// commands/campaign/crud.rs

#[tauri::command]
pub fn get_campaign(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<Campaign>, String> {
    state.campaign_manager.get(&id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::mock_app_state;

    #[test]
    fn test_get_campaign_found() {
        let state = mock_app_state();
        state.campaign_manager.create(Campaign::new("test-id", "Test"));

        let result = get_campaign("test-id".to_string(), State::new(&state));
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_get_campaign_not_found() {
        let state = mock_app_state();

        let result = get_campaign("nonexistent".to_string(), State::new(&state));
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
```

### Mock State Pattern

```rust
// commands/test_utils.rs (or state.rs)

#[cfg(test)]
pub fn mock_app_state() -> AppState {
    AppState {
        database: Database::in_memory().unwrap(),
        llm_client: RwLock::new(None),
        llm_config: RwLock::new(None),
        campaign_manager: CampaignManager::new(),
        session_manager: SessionManager::new(),
        // ... minimal initialization for testing
    }
}
```

### Integration Test Pattern

```rust
// tests/commands/campaign_integration.rs

#[tokio::test]
async fn test_campaign_workflow() {
    let state = setup_test_app_state().await;

    // Create campaign
    let id = create_campaign(
        CreateCampaignRequest { name: "Test".into(), /* ... */ },
        State::new(&state),
    ).unwrap();

    // Add notes
    add_campaign_note(
        NoteRequest { campaign_id: id.clone(), /* ... */ },
        State::new(&state),
    ).unwrap();

    // Verify
    let notes = get_campaign_notes(id, State::new(&state)).unwrap();
    assert_eq!(notes.len(), 1);
}
```

---

## Migration Strategy

### Phase 1: Foundation (state.rs, types.rs)
1. Extract `AppState` to `commands/state.rs`
2. Extract shared types to `commands/types.rs`
3. Keep `commands_legacy.rs` bridge functional

### Phase 2: Low-Dependency Modules
1. `credentials/` - No cross-module dependencies
2. `usage/` - Only uses UsageTrackerState
3. `audit/` - Only uses AuditLoggerState
4. `system/` - No state dependencies

### Phase 3: Core Domain Modules
1. `llm/` - Foundation for chat
2. `campaign/` - Foundation for session
3. `session/` - Depends on campaign
4. `npc/` - Depends on session, personality

### Phase 4: Complex Modules
1. `personality/` - Depends on llm, npc
2. `search/` - Largest module, many submodules
3. `world/` - Depends on campaign
4. `relationships/` - Depends on campaign
5. `generation/` - Depends on llm

### Phase 5: Cleanup
1. Remove `commands_legacy.rs` bridge
2. Update `commands/mod.rs` final re-exports
3. Run full test suite
4. Update documentation

---

## Command Count Summary

| Module | Commands | Est. Lines |
|--------|----------|------------|
| voice/ | (extracted) | ~1,270 |
| oauth/ | (extracted) | ~850 |
| archetype/ | (extracted) | ~1,200 |
| llm/ | 24 | ~1,060 |
| campaign/ | 19 | ~1,010 |
| session/ | 36 | ~1,680 |
| npc/ | 18 | ~1,060 |
| personality/ | 34 | ~1,580 |
| search/ | 45 | ~2,210 |
| world/ | 10 | ~600 |
| relationships/ | 9 | ~580 |
| generation/ | 21 | ~700 |
| credentials/ | 4 | ~130 |
| usage/ | 7 | ~230 |
| audit/ | 6 | ~280 |
| system/ | 5 | ~210 |
| **Total** | **~310** | **~14,650** |

**Note**: Line estimates include types, helper functions, and test modules. Actual command logic is more compact. The total exceeds `commands_legacy.rs` (8,303 lines) because:
- Types are explicitly defined in each module
- Helper functions are extracted and documented
- Test modules are included
- Module organization adds small overhead

The benefit is ~500 lines/file max vs one 8,303 line monolith.

---

## Non-Goals

1. **No frontend changes** - All command signatures remain identical
2. **No new features** - Pure refactoring
3. **No command renames** - Frontend bindings depend on exact names
4. **No state restructuring** - AppState fields remain compatible
5. **No async->sync or sync->async conversions** - Preserve existing signatures

---

## Success Criteria

1. All 310 commands accessible via Tauri IPC
2. No frontend binding changes required
3. All existing tests pass
4. No new compiler warnings
5. Each file under 500 lines
6. Settings UI tabs map to command modules
7. CI pipeline passes

---

## References

- [Existing voice/ module](../../../src-tauri/src/commands/voice/mod.rs)
- [Existing oauth/ module](../../../src-tauri/src/commands/oauth/mod.rs)
- [Existing archetype/ module](../../../src-tauri/src/commands/archetype/mod.rs)
- [Existing error.rs](../../../src-tauri/src/commands/error.rs)
- [Existing macros.rs](../../../src-tauri/src/commands/macros.rs)
