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
pub use claude::*;

// Re-export Gemini OAuth types and commands
pub use gemini::*;

// Re-export Copilot OAuth types and commands
pub use copilot::*;
