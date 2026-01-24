//! Voice Provider Installation Commands
//!
//! Commands for installing, checking, and managing voice providers (Piper, Coqui, etc.).

use std::path::PathBuf;

use crate::core::voice::{
    VoiceProviderType, ProviderInstaller, InstallStatus,
    AvailablePiperVoice, get_recommended_piper_voices,
};

/// Get the voice models directory path.
/// Fallback chain: data_local_dir -> data_dir -> temp_dir (last resort, non-persistent)
/// Note: temp_dir fallback may result in models being lost on system restart.
fn get_models_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(|| {
            log::warn!("data_local_dir unavailable, falling back to data_dir");
            dirs::data_dir()
        })
        .unwrap_or_else(|| {
            log::warn!("No persistent data directory available, using temp_dir - voice models may be lost on restart");
            std::env::temp_dir()
        })
        .join("ttrpg-assistant/voice/piper")
}

// ============================================================================
// Voice Provider Installation Commands
// ============================================================================

/// Check installation status for all local voice providers
#[tauri::command]
pub async fn check_voice_provider_installations() -> Result<Vec<InstallStatus>, String> {
    let installer = ProviderInstaller::new(get_models_dir());
    Ok(installer.check_all_local().await)
}

/// Check installation status for a specific provider
#[tauri::command]
pub async fn check_voice_provider_status(provider: VoiceProviderType) -> Result<InstallStatus, String> {
    let installer = ProviderInstaller::new(get_models_dir());
    Ok(installer.check_status(&provider).await)
}

/// Install a voice provider (Piper or Coqui)
#[tauri::command]
pub async fn install_voice_provider(provider: VoiceProviderType) -> Result<InstallStatus, String> {
    let installer = ProviderInstaller::new(get_models_dir());
    installer.install(&provider).await.map_err(|e| e.to_string())
}

/// List available Piper voices for download from Hugging Face
#[tauri::command]
pub async fn list_downloadable_piper_voices() -> Result<Vec<AvailablePiperVoice>, String> {
    let installer = ProviderInstaller::new(get_models_dir());
    installer.list_available_piper_voices().await.map_err(|e| e.to_string())
}

/// Get recommended/popular Piper voices (quick, no network call)
#[tauri::command]
pub fn get_popular_piper_voices() -> Vec<(String, String, String)> {
    get_recommended_piper_voices()
        .into_iter()
        .map(|(k, n, d)| (k.to_string(), n.to_string(), d.to_string()))
        .collect()
}

/// Download a Piper voice from Hugging Face
#[tauri::command]
pub async fn download_piper_voice(voice_key: String, quality: Option<String>) -> Result<String, String> {
    let installer = ProviderInstaller::new(get_models_dir());
    let path = installer
        .download_piper_voice(&voice_key, quality.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}
