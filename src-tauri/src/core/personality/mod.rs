//! Personality Module
//!
//! Provides personality profiles and application layer for:
//! - NPC dialogue styling
//! - Narration tone matching
//! - Chat response personality injection
//!
//! Re-exports from the personality_base module and adds the application layer.

// Re-export everything from the base personality module
pub use crate::core::personality_base::*;

pub mod application;
pub use application::*;
