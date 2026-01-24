//! Voice Cache Commands
//!
//! Commands for managing the audio synthesis cache.

use tauri::State;

use crate::core::voice::{CacheStats, CacheEntry};
use crate::commands::AppState;

// ============================================================================
// Audio Cache Commands
// ============================================================================

/// Audio cache size information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioCacheSizeInfo {
    pub current_size_bytes: u64,
    pub max_size_bytes: u64,
    pub entry_count: usize,
    pub usage_percent: f64,
}

/// Get audio cache statistics
///
/// Returns comprehensive cache statistics including:
/// - Hit/miss counts and rate
/// - Current and max cache size
/// - Entry counts by format
/// - Average entry size
/// - Oldest entry age
#[tauri::command]
pub async fn get_audio_cache_stats(state: State<'_, AppState>) -> Result<CacheStats, String> {
    let voice_manager = state.voice_manager.read().await;
    voice_manager.get_cache_stats().await.map_err(|e| e.to_string())
}

/// Clear audio cache entries by tag
///
/// Removes all cached audio entries that have the specified tag.
/// Tags can be used to group entries by session_id, npc_id, campaign_id, etc.
///
/// # Arguments
/// * `tag` - The tag to filter by (e.g., "session:abc123", "npc:wizard_01")
///
/// # Returns
/// The number of entries removed
#[tauri::command]
pub async fn clear_audio_cache_by_tag(
    tag: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let voice_manager = state.voice_manager.read().await;
    voice_manager.clear_cache_by_tag(&tag).await.map_err(|e| e.to_string())
}

/// Clear all audio cache entries
///
/// Removes all cached audio files and resets cache statistics.
/// Use with caution as this will force re-synthesis of all audio.
#[tauri::command]
pub async fn clear_audio_cache(state: State<'_, AppState>) -> Result<(), String> {
    let voice_manager = state.voice_manager.read().await;
    voice_manager.clear_cache().await.map_err(|e| e.to_string())
}

/// Prune old audio cache entries
///
/// Removes cache entries older than the specified age.
/// Useful for automatic cleanup of stale audio files.
///
/// # Arguments
/// * `max_age_seconds` - Maximum age in seconds; entries older than this will be removed
///
/// # Returns
/// The number of entries removed
#[tauri::command]
pub async fn prune_audio_cache(
    max_age_seconds: i64,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let voice_manager = state.voice_manager.read().await;
    voice_manager.prune_cache(max_age_seconds).await.map_err(|e| e.to_string())
}

/// List cached audio entries
///
/// Returns all cache entries with metadata including:
/// - File path and size
/// - Creation and last access times
/// - Access count
/// - Associated tags
/// - Audio format and duration
#[tauri::command]
pub async fn list_audio_cache_entries(state: State<'_, AppState>) -> Result<Vec<CacheEntry>, String> {
    let voice_manager = state.voice_manager.read().await;
    voice_manager.list_cache_entries().await.map_err(|e| e.to_string())
}

/// Get cache size information
///
/// Returns the current cache size and maximum allowed size in bytes.
#[tauri::command]
pub async fn get_audio_cache_size(state: State<'_, AppState>) -> Result<AudioCacheSizeInfo, String> {
    let voice_manager = state.voice_manager.read().await;
    let stats = voice_manager.get_cache_stats().await.map_err(|e| e.to_string())?;

    Ok(AudioCacheSizeInfo {
        current_size_bytes: stats.current_size_bytes,
        max_size_bytes: stats.max_size_bytes,
        entry_count: stats.entry_count,
        usage_percent: if stats.max_size_bytes > 0 {
            (stats.current_size_bytes as f64 / stats.max_size_bytes as f64) * 100.0
        } else {
            0.0
        },
    })
}
