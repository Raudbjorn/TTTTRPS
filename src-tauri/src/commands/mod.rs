//! Tauri Commands Module
//!
//! All Tauri IPC commands organized by domain.
//! This module replaces the original monolithic commands.rs.

// Allow ambiguous glob re-exports for Tauri command modules.
// Multiple domain modules have submodules with common names (config, chat, events, crud, notes)
// which conflict at the namespace level but not at the function level. The actual commands
// have unique names, and Tauri's __cmd__ macro exports must be re-exported via globs.
#![allow(ambiguous_glob_reexports)]

pub mod error;
pub mod macros;
#[macro_use]
pub mod oauth;
#[macro_use]
pub mod archetype;
pub mod voice;
pub mod generation;
pub mod timeline;
pub mod system;
pub mod credentials;
pub mod world;
pub mod relationships;
pub mod usage;
pub mod audit;
pub mod combat;
pub mod campaign;
pub mod npc;
pub mod location;
pub mod session;
pub mod llm;
pub mod personality;
pub mod rag;
pub mod search;

pub mod state;
pub mod types;

// Re-export shared state (types are re-exported explicitly to avoid conflicts with legacy)
pub use state::AppState;

// Re-export error types
pub use error::{CommandError, CommandResult};

// Re-export OAuth types and commands
pub use oauth::{
    // State types
    ClaudeState, ClaudeStorageBackend,
    GeminiState, GeminiStorageBackend,
    CopilotState, CopilotStorageBackend,
    // Claude response types
    ClaudeStatusResponse, ClaudeOAuthStartResponse, ClaudeOAuthCompleteResponse,
    ClaudeLogoutResponse, ClaudeSetStorageResponse, ClaudeModelInfo,
    // Gemini response types
    GeminiStatusResponse, GeminiOAuthStartResponse, GeminiOAuthCompleteResponse,
    GeminiLogoutResponse, GeminiSetStorageResponse,
    // Copilot response types
    CopilotDeviceCodeResponse, CopilotAuthPollResult, CopilotAuthStatus,
    CopilotUsageInfo, CopilotQuotaDetail, CopilotModelInfo,
    // Claude commands
    claude_get_status, claude_start_oauth, claude_complete_oauth,
    claude_logout, claude_set_storage_backend, claude_list_models,
    // Gemini commands
    gemini_get_status, gemini_start_oauth, gemini_complete_oauth,
    gemini_logout, gemini_set_storage_backend,
    // Copilot commands
    start_copilot_auth, poll_copilot_auth, check_copilot_auth,
    logout_copilot, get_copilot_usage, get_copilot_models,
};

// Re-export voice commands (fully extracted) - using glob to include Tauri __cmd__ macros
pub use voice::*;

// Re-export generation commands (character, location) - using glob to include Tauri __cmd__ macros
pub use generation::*;

// Re-export timeline commands - using glob to include Tauri __cmd__ macros
pub use timeline::*;

// Re-export system commands - using glob to include Tauri __cmd__ macros
pub use system::*;

// Re-export credentials commands - using glob to include Tauri __cmd__ macros
pub use credentials::*;

// Re-export world state commands - using glob to include Tauri __cmd__ macros
pub use world::*;

// Re-export relationship commands - using glob to include Tauri __cmd__ macros
pub use relationships::*;

// Re-export usage tracking commands - using glob to include Tauri __cmd__ macros
pub use usage::*;

// Re-export audit log commands - using glob to include Tauri __cmd__ macros
pub use audit::*;

// Re-export combat commands - using glob to include Tauri __cmd__ macros
pub use combat::*;

// Re-export campaign commands - using glob to include Tauri __cmd__ macros
pub use campaign::*;

// Re-export NPC commands - using glob to include Tauri __cmd__ macros
pub use npc::*;

// Re-export location commands - using glob to include Tauri __cmd__ macros
pub use location::*;

// Re-export session commands - using glob to include Tauri __cmd__ macros
pub use session::*;

// Re-export LLM commands - using glob to include Tauri __cmd__ macros
pub use llm::*;

// Re-export personality commands - using glob to include Tauri __cmd__ macros
pub use personality::*;

// Re-export RAG commands - using glob to include Tauri __cmd__ macros
pub use rag::*;

// Re-export search commands - using glob to include Tauri __cmd__ macros
pub use search::*;

// Re-export extracted domain commands
pub use archetype::{
    // Types
    CreateArchetypeRequest, ArchetypeResponse, ArchetypeSummaryResponse,
    PersonalityAffinityInput, NpcRoleMappingInput, NamingCultureWeightInput,
    StatTendenciesInput, ResolutionQueryRequest, ResolvedArchetypeResponse,
    ResolutionMetadataResponse, SettingPackSummaryResponse, VocabularyBankSummaryResponse,
    CreateVocabularyBankRequest, PhraseInput, VocabularyBankResponse, PhraseOutput,
    PhraseFilterRequest, ArchetypeCacheStatsResponse,
    // CRUD commands
    create_archetype, get_archetype, list_archetypes, update_archetype,
    delete_archetype, archetype_exists, count_archetypes,
    // Vocabulary commands
    create_vocabulary_bank, get_vocabulary_bank, list_vocabulary_banks,
    update_vocabulary_bank, delete_vocabulary_bank, get_phrases,
    // Setting pack commands
    load_setting_pack, list_setting_packs, get_setting_pack,
    activate_setting_pack, deactivate_setting_pack, get_active_setting_pack,
    get_setting_pack_versions,
    // Resolution commands
    resolve_archetype, resolve_for_npc, get_archetype_cache_stats,
    clear_archetype_cache, is_archetype_registry_ready,
};

// LEGACY FILE DISABLED: All commands have been extracted to domain modules
// The commands_legacy.rs file is kept for reference but no longer used
// #[path = "../commands_legacy.rs"]
// mod commands_legacy;
// pub use commands_legacy::*;
