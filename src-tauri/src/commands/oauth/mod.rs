//! OAuth Command Modules
//!
//! Handles OAuth flows for Claude, Gemini, and Copilot providers.
//! Each provider has its own submodule with state types and Tauri commands.

mod common;
pub mod claude;
pub mod gemini;
pub mod copilot;

// Re-export common types
pub use common::*;

// Re-export Claude OAuth types and commands
pub use claude::{
    // State types
    ClaudeState,
    ClaudeStorageBackend,
    // Response types
    ClaudeStatusResponse,
    ClaudeOAuthStartResponse,
    ClaudeOAuthCompleteResponse,
    ClaudeLogoutResponse,
    ClaudeSetStorageResponse,
    ClaudeModelInfo,
    // Commands
    claude_get_status,
    claude_start_oauth,
    claude_complete_oauth,
    claude_logout,
    claude_set_storage_backend,
    claude_list_models,
};

// Re-export Gemini OAuth types and commands
pub use gemini::{
    // State types
    GeminiState,
    GeminiStorageBackend,
    // Response types
    GeminiStatusResponse,
    GeminiOAuthStartResponse,
    GeminiOAuthCompleteResponse,
    GeminiLogoutResponse,
    GeminiSetStorageResponse,
    // Commands
    gemini_get_status,
    gemini_start_oauth,
    gemini_complete_oauth,
    gemini_logout,
    gemini_set_storage_backend,
};

// Re-export Copilot OAuth types and commands
pub use copilot::{
    // State types
    CopilotState,
    CopilotStorageBackend,
    // Response types
    CopilotDeviceCodeResponse,
    CopilotAuthPollResult,
    CopilotAuthStatus,
    CopilotUsageInfo,
    CopilotQuotaDetail,
    CopilotModelInfo,
    CopilotSetStorageResponse,
    // Commands
    start_copilot_auth,
    poll_copilot_auth,
    check_copilot_auth,
    logout_copilot,
    get_copilot_usage,
    get_copilot_models,
    copilot_set_storage_backend,
};
