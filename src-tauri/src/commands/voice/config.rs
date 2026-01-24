//! Voice Configuration Commands
//!
//! Commands for configuring voice providers and managing voice settings.

use tauri::State;

use crate::core::voice::{
    VoiceManager, VoiceConfig, VoiceProviderDetection,
    detect_providers, Voice,
};
use crate::commands::AppState;

/// Helper to save voice config to disk
fn save_voice_config_disk(app_handle: &tauri::AppHandle, config: &VoiceConfig) {
    use tauri::Manager;
    if let Some(app_data) = app_handle.path().app_data_dir().ok() {
        let config_path = app_data.join("voice_config.json");
        if let Ok(json) = serde_json::to_string_pretty(config) {
            let _ = std::fs::write(config_path, json);
        }
    }
}

// ============================================================================
// Voice Configuration Commands
// ============================================================================

#[tauri::command]
pub async fn configure_voice(
    config: VoiceConfig,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    // 1. If API keys are provided in config, save them securely and mask them in config
    if let Some(elevenlabs) = config.elevenlabs.clone() {
        if !elevenlabs.api_key.is_empty() && elevenlabs.api_key != "********" {
            state.credentials.store_secret("elevenlabs_api_key", &elevenlabs.api_key)
                .map_err(|e| e.to_string())?;
        }
    }

    let mut effective_config = config.clone();

    // Restore secrets from credential manager if masked
    if let Some(ref mut elevenlabs) = effective_config.elevenlabs {
        if elevenlabs.api_key.is_empty() || elevenlabs.api_key == "********" {
             if let Ok(secret) = state.credentials.get_secret("elevenlabs_api_key") {
                 elevenlabs.api_key = secret;
             }
        }
    }

    // Save config to disk with MASKED secrets (never write plaintext secrets)
    let mut config_for_disk = effective_config.clone();
    if let Some(ref mut elevenlabs) = config_for_disk.elevenlabs {
        if !elevenlabs.api_key.is_empty() {
            elevenlabs.api_key = String::new(); // Mask for disk storage
        }
    }
    save_voice_config_disk(&app_handle, &config_for_disk);

    let new_manager = VoiceManager::new(effective_config);

    // Update state
    let mut manager = state.voice_manager.write().await;
    *manager = new_manager;
    Ok("Voice configuration updated successfully".to_string())
}

#[tauri::command]
pub async fn get_voice_config(state: State<'_, AppState>) -> Result<VoiceConfig, String> {
    let manager = state.voice_manager.read().await;
    let mut config = manager.get_config().clone();
    // Mask secrets
    if let Some(ref mut elevenlabs) = config.elevenlabs {
        if !elevenlabs.api_key.is_empty() {
            elevenlabs.api_key = "********".to_string();
        }
    }
    Ok(config)
}

/// Detect available voice providers on the system
/// Returns status for each local TTS service (running/not running)
#[tauri::command]
pub async fn detect_voice_providers() -> Result<VoiceProviderDetection, String> {
    Ok(detect_providers().await)
}

/// List all available voices from configured providers
#[tauri::command]
pub async fn list_all_voices(state: State<'_, AppState>) -> Result<Vec<Voice>, String> {
    state.voice_manager.read().await.list_voices().await.map_err(|e| e.to_string())
}
