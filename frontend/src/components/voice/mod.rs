//! Voice Components Module (TASK-004)
//!
//! Provides UI components for voice profile management and synthesis.

mod profile_manager;

pub use profile_manager::{
    VoiceProfileCard, VoiceProfileFilters, VoiceProfileGrid, VoiceProfileManager,
    VoiceProfileSelector,
};
