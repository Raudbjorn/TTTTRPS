//! Voice Preset Commands
//!
//! Commands for voice profile presets (built-in DM personas).

use crate::core::voice::{VoiceProfile, get_dm_presets, get_presets_by_tag, get_preset_by_id};

// ============================================================================
// Voice Preset Commands
// ============================================================================

/// List all voice profile presets (built-in DM personas)
#[tauri::command]
pub fn list_voice_presets() -> Vec<VoiceProfile> {
    get_dm_presets()
}

/// List voice presets filtered by tag
#[tauri::command]
pub fn list_voice_presets_by_tag(tag: String) -> Vec<VoiceProfile> {
    get_presets_by_tag(&tag)
}

/// Get a specific voice preset by ID
#[tauri::command]
pub fn get_voice_preset(preset_id: String) -> Option<VoiceProfile> {
    get_preset_by_id(&preset_id)
}
