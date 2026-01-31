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
    ClaudeGateState,
    ClaudeGateStorageBackend,
    // Response types
    ClaudeGateStatusResponse,
    ClaudeGateOAuthStartResponse,
    ClaudeGateOAuthCompleteResponse,
    ClaudeGateLogoutResponse,
    ClaudeGateSetStorageResponse,
    ClaudeGateModelInfo,
    // Commands
    claude_gate_get_status,
    claude_gate_start_oauth,
    claude_gate_complete_oauth,
    claude_gate_logout,
    claude_gate_set_storage_backend,
    claude_gate_list_models,
};

// Re-export Gemini OAuth types and commands
pub use gemini::{
    // State types
    GeminiGateState,
    GeminiGateStorageBackend,
    // Response types
    GeminiGateStatusResponse,
    GeminiGateOAuthStartResponse,
    GeminiGateOAuthCompleteResponse,
    GeminiGateLogoutResponse,
    GeminiGateSetStorageResponse,
    // Commands
    gemini_gate_get_status,
    gemini_gate_start_oauth,
    gemini_gate_complete_oauth,
    gemini_gate_logout,
    gemini_gate_set_storage_backend,
};

// Re-export Copilot OAuth types and commands
pub use copilot::{
    // State types
    CopilotGateState,
    CopilotGateStorageBackend,
    // Response types
    CopilotDeviceCodeResponse,
    CopilotAuthPollResult,
    CopilotAuthStatus,
    CopilotUsageInfo,
    CopilotQuotaDetail,
    CopilotGateModelInfo,
    CopilotGateSetStorageResponse,
    // Commands
    start_copilot_auth,
    poll_copilot_auth,
    check_copilot_auth,
    logout_copilot,
    get_copilot_usage,
    get_copilot_models,
    copilot_gate_set_storage_backend,
};
