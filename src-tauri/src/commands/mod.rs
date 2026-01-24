//! Tauri Commands Module
//!
//! All Tauri IPC commands organized by domain.
//! This module replaces the original monolithic commands.rs.

pub mod error;
pub mod macros;
#[macro_use]
pub mod oauth;
#[macro_use]
pub mod archetype;
pub mod voice;

// Note: types.rs duplicates types from commands_legacy - commented out during migration
// pub mod types;

// Re-export error types
pub use error::{CommandError, CommandResult};

// Re-export OAuth types and commands (these will shadow the ones from commands_legacy)
pub use oauth::{
    // State types
    ClaudeGateState, ClaudeGateStorageBackend,
    GeminiGateState, GeminiGateStorageBackend,
    CopilotGateState, CopilotGateStorageBackend,
    // Claude response types
    ClaudeGateStatusResponse, ClaudeGateOAuthStartResponse, ClaudeGateOAuthCompleteResponse,
    ClaudeGateLogoutResponse, ClaudeGateSetStorageResponse, ClaudeGateModelInfo,
    // Gemini response types
    GeminiGateStatusResponse, GeminiGateOAuthStartResponse, GeminiGateOAuthCompleteResponse,
    GeminiGateLogoutResponse, GeminiGateSetStorageResponse,
    // Copilot response types
    CopilotDeviceCodeResponse, CopilotAuthPollResult, CopilotAuthStatus,
    CopilotUsageInfo, CopilotQuotaDetail, CopilotGateModelInfo,
    // Claude commands
    claude_gate_get_status, claude_gate_start_oauth, claude_gate_complete_oauth,
    claude_gate_logout, claude_gate_set_storage_backend, claude_gate_list_models,
    // Gemini commands
    gemini_gate_get_status, gemini_gate_start_oauth, gemini_gate_complete_oauth,
    gemini_gate_logout, gemini_gate_set_storage_backend,
    // Copilot commands
    start_copilot_auth, poll_copilot_auth, check_copilot_auth,
    logout_copilot, get_copilot_usage, get_copilot_models,
};

// Re-export voice commands (fully extracted) - using glob to include Tauri __cmd__ macros
pub use voice::*;

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

// Temporary: Re-export everything from the original commands.rs until extraction is complete
// This will be removed as commands are extracted to domain modules
#[path = "../commands_legacy.rs"]
mod commands_legacy;
pub use commands_legacy::*;
