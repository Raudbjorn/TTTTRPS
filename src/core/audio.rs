//! Audio Playback Module
//!
//! Handles audio playback for voice synthesis and sound effects using rodio.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to initialize audio output: {0}")]
    OutputError(String),

    #[error("Failed to decode audio file: {0}")]
    DecodeError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Playback error: {0}")]
    PlaybackError(String),

    #[error("Invalid audio ID: {0}")]
    InvalidId(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AudioError>;

// ============================================================================
// Audio Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrack {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub duration_ms: Option<u64>,
    pub track_type: TrackType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrackType {
    Voice,
    Music,
    Ambience,
    SoundEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub track_id: Option<String>,
    pub is_playing: bool,
    pub is_paused: bool,
    pub volume: f32,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            track_id: None,
            is_playing: false,
            is_paused: false,
            volume: 1.0,
            position_ms: 0,
            duration_ms: None,
        }
    }
}

// ============================================================================
// Audio Player
// ============================================================================

/// Main audio player for voice and sound effects
pub struct AudioPlayer {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    voice_sink: Arc<RwLock<Option<Sink>>>,
    music_sink: Arc<RwLock<Option<Sink>>>,
    ambience_sink: Arc<RwLock<Option<Sink>>>,
    sfx_sinks: Arc<RwLock<Vec<Sink>>>,
    current_track: Arc<RwLock<Option<String>>>,
    volumes: Arc<RwLock<AudioVolumes>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioVolumes {
    pub master: f32,
    pub voice: f32,
    pub music: f32,
    pub ambience: f32,
    pub sfx: f32,
}

impl Default for AudioVolumes {
    fn default() -> Self {
        Self {
            master: 1.0,
            voice: 1.0,
            music: 0.5,
            ambience: 0.3,
            sfx: 0.8,
        }
    }
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| AudioError::OutputError(e.to_string()))?;

        Ok(Self {
            _stream: stream,
            stream_handle,
            voice_sink: Arc::new(RwLock::new(None)),
            music_sink: Arc::new(RwLock::new(None)),
            ambience_sink: Arc::new(RwLock::new(None)),
            sfx_sinks: Arc::new(RwLock::new(Vec::new())),
            current_track: Arc::new(RwLock::new(None)),
            volumes: Arc::new(RwLock::new(AudioVolumes::default())),
        })
    }

    // ========================================================================
    // Voice Playback
    // ========================================================================

    /// Play voice audio (NPC speech)
    pub fn play_voice(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::open(path.as_ref())
            .map_err(|_| AudioError::FileNotFound(path.as_ref().display().to_string()))?;

        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        // Stop any existing voice
        self.stop_voice();

        let volumes = self.volumes.read().unwrap();
        let volume = volumes.master * volumes.voice;
        drop(volumes);

        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::PlaybackError(e.to_string()))?;

        sink.set_volume(volume);
        sink.append(source);

        *self.voice_sink.write().unwrap() = Some(sink);
        *self.current_track.write().unwrap() = Some(path.as_ref().display().to_string());

        Ok(())
    }

    /// Stop voice playback
    pub fn stop_voice(&self) {
        if let Some(sink) = self.voice_sink.write().unwrap().take() {
            sink.stop();
        }
        *self.current_track.write().unwrap() = None;
    }

    /// Check if voice is currently playing
    pub fn is_voice_playing(&self) -> bool {
        self.voice_sink.read().unwrap()
            .as_ref()
            .map(|s| !s.empty())
            .unwrap_or(false)
    }

    /// Set voice volume (0.0 - 1.0)
    pub fn set_voice_volume(&self, volume: f32) {
        self.volumes.write().unwrap().voice = volume.clamp(0.0, 1.0);
        self.update_voice_volume();
    }

    fn update_voice_volume(&self) {
        let volumes = self.volumes.read().unwrap();
        let volume = volumes.master * volumes.voice;
        drop(volumes);

        if let Some(sink) = self.voice_sink.read().unwrap().as_ref() {
            sink.set_volume(volume);
        }
    }

    // ========================================================================
    // Music Playback
    // ========================================================================

    /// Play background music (loops)
    pub fn play_music(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::open(path.as_ref())
            .map_err(|_| AudioError::FileNotFound(path.as_ref().display().to_string()))?;

        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| AudioError::DecodeError(e.to_string()))?
            .repeat_infinite();

        // Stop existing music
        self.stop_music();

        let volumes = self.volumes.read().unwrap();
        let volume = volumes.master * volumes.music;
        drop(volumes);

        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::PlaybackError(e.to_string()))?;

        sink.set_volume(volume);
        sink.append(source);

        *self.music_sink.write().unwrap() = Some(sink);

        Ok(())
    }

    /// Stop music playback
    pub fn stop_music(&self) {
        if let Some(sink) = self.music_sink.write().unwrap().take() {
            sink.stop();
        }
    }

    /// Pause music
    pub fn pause_music(&self) {
        if let Some(sink) = self.music_sink.read().unwrap().as_ref() {
            sink.pause();
        }
    }

    /// Resume music
    pub fn resume_music(&self) {
        if let Some(sink) = self.music_sink.read().unwrap().as_ref() {
            sink.play();
        }
    }

    /// Set music volume (0.0 - 1.0)
    pub fn set_music_volume(&self, volume: f32) {
        self.volumes.write().unwrap().music = volume.clamp(0.0, 1.0);
        self.update_music_volume();
    }

    fn update_music_volume(&self) {
        let volumes = self.volumes.read().unwrap();
        let volume = volumes.master * volumes.music;
        drop(volumes);

        if let Some(sink) = self.music_sink.read().unwrap().as_ref() {
            sink.set_volume(volume);
        }
    }

    // ========================================================================
    // Ambience Playback
    // ========================================================================

    /// Play ambient sounds (loops)
    pub fn play_ambience(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::open(path.as_ref())
            .map_err(|_| AudioError::FileNotFound(path.as_ref().display().to_string()))?;

        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| AudioError::DecodeError(e.to_string()))?
            .repeat_infinite();

        // Stop existing ambience
        self.stop_ambience();

        let volumes = self.volumes.read().unwrap();
        let volume = volumes.master * volumes.ambience;
        drop(volumes);

        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::PlaybackError(e.to_string()))?;

        sink.set_volume(volume);
        sink.append(source);

        *self.ambience_sink.write().unwrap() = Some(sink);

        Ok(())
    }

    /// Stop ambience playback
    pub fn stop_ambience(&self) {
        if let Some(sink) = self.ambience_sink.write().unwrap().take() {
            sink.stop();
        }
    }

    /// Set ambience volume (0.0 - 1.0)
    pub fn set_ambience_volume(&self, volume: f32) {
        self.volumes.write().unwrap().ambience = volume.clamp(0.0, 1.0);
        self.update_ambience_volume();
    }

    fn update_ambience_volume(&self) {
        let volumes = self.volumes.read().unwrap();
        let volume = volumes.master * volumes.ambience;
        drop(volumes);

        if let Some(sink) = self.ambience_sink.read().unwrap().as_ref() {
            sink.set_volume(volume);
        }
    }

    // ========================================================================
    // Sound Effects
    // ========================================================================

    /// Play a sound effect (fire and forget)
    pub fn play_sfx(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::open(path.as_ref())
            .map_err(|_| AudioError::FileNotFound(path.as_ref().display().to_string()))?;

        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        let volumes = self.volumes.read().unwrap();
        let volume = volumes.master * volumes.sfx;
        drop(volumes);

        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| AudioError::PlaybackError(e.to_string()))?;

        sink.set_volume(volume);
        sink.append(source);

        // Clean up finished sinks
        {
            let mut sinks = self.sfx_sinks.write().unwrap();
            sinks.retain(|s| !s.empty());
            sinks.push(sink);
        }

        Ok(())
    }

    /// Set SFX volume (0.0 - 1.0)
    pub fn set_sfx_volume(&self, volume: f32) {
        self.volumes.write().unwrap().sfx = volume.clamp(0.0, 1.0);
    }

    // ========================================================================
    // Master Controls
    // ========================================================================

    /// Set master volume (0.0 - 1.0)
    pub fn set_master_volume(&self, volume: f32) {
        self.volumes.write().unwrap().master = volume.clamp(0.0, 1.0);
        self.update_all_volumes();
    }

    fn update_all_volumes(&self) {
        self.update_voice_volume();
        self.update_music_volume();
        self.update_ambience_volume();
    }

    /// Get current volume settings
    pub fn get_volumes(&self) -> AudioVolumes {
        self.volumes.read().unwrap().clone()
    }

    /// Stop all audio
    pub fn stop_all(&self) {
        self.stop_voice();
        self.stop_music();
        self.stop_ambience();

        let mut sinks = self.sfx_sinks.write().unwrap();
        for sink in sinks.drain(..) {
            sink.stop();
        }
    }

    /// Mute all audio
    pub fn mute_all(&self) {
        if let Some(sink) = self.voice_sink.read().unwrap().as_ref() {
            sink.set_volume(0.0);
        }
        if let Some(sink) = self.music_sink.read().unwrap().as_ref() {
            sink.set_volume(0.0);
        }
        if let Some(sink) = self.ambience_sink.read().unwrap().as_ref() {
            sink.set_volume(0.0);
        }
    }

    /// Unmute all audio
    pub fn unmute_all(&self) {
        self.update_all_volumes();
    }

    /// Get current playback state
    pub fn get_state(&self) -> PlaybackState {
        let track_id = self.current_track.read().unwrap().clone();
        let is_playing = self.is_voice_playing();
        let volumes = self.volumes.read().unwrap();

        PlaybackState {
            track_id,
            is_playing,
            is_paused: false,
            volume: volumes.master,
            position_ms: 0, // Would need additional tracking
            duration_ms: None,
        }
    }
}

// ============================================================================
// Audio Queue
// ============================================================================

/// Queue for sequential voice playback
pub struct VoiceQueue {
    queue: Arc<RwLock<Vec<PathBuf>>>,
    player: Arc<AudioPlayer>,
}

impl VoiceQueue {
    pub fn new(player: Arc<AudioPlayer>) -> Self {
        Self {
            queue: Arc::new(RwLock::new(Vec::new())),
            player,
        }
    }

    /// Add audio file to queue
    pub fn enqueue(&self, path: impl AsRef<Path>) {
        self.queue.write().unwrap().push(path.as_ref().to_path_buf());
    }

    /// Clear the queue
    pub fn clear(&self) {
        self.queue.write().unwrap().clear();
    }

    /// Get queue length
    pub fn len(&self) -> usize {
        self.queue.read().unwrap().len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.read().unwrap().is_empty()
    }

    /// Play next item in queue (returns true if something was played)
    pub fn play_next(&self) -> Result<bool> {
        // Don't play next if current is still playing
        if self.player.is_voice_playing() {
            return Ok(false);
        }

        let next = self.queue.write().unwrap().pop();

        if let Some(path) = next {
            self.player.play_voice(&path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Skip current and play next
    pub fn skip(&self) -> Result<bool> {
        self.player.stop_voice();
        self.play_next()
    }
}

// ============================================================================
// Preset Sound Effects
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundPack {
    pub name: String,
    pub sounds: HashMap<String, PathBuf>,
}

/// Common TTRPG sound effect categories
pub fn get_sfx_categories() -> Vec<String> {
    vec![
        "combat_hit".to_string(),
        "combat_miss".to_string(),
        "combat_critical".to_string(),
        "spell_cast".to_string(),
        "spell_fire".to_string(),
        "spell_ice".to_string(),
        "spell_lightning".to_string(),
        "spell_heal".to_string(),
        "door_open".to_string(),
        "door_close".to_string(),
        "chest_open".to_string(),
        "coins_drop".to_string(),
        "dice_roll".to_string(),
        "fanfare_victory".to_string(),
        "fanfare_defeat".to_string(),
        "monster_roar".to_string(),
        "monster_death".to_string(),
        "footsteps_stone".to_string(),
        "footsteps_grass".to_string(),
        "ambient_tavern".to_string(),
        "ambient_forest".to_string(),
        "ambient_dungeon".to_string(),
        "ambient_rain".to_string(),
        "ambient_fire".to_string(),
    ]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_volumes_default() {
        let volumes = AudioVolumes::default();
        assert_eq!(volumes.master, 1.0);
        assert_eq!(volumes.voice, 1.0);
        assert_eq!(volumes.music, 0.5);
    }

    #[test]
    fn test_playback_state_default() {
        let state = PlaybackState::default();
        assert!(!state.is_playing);
        assert!(!state.is_paused);
        assert_eq!(state.volume, 1.0);
    }

    #[test]
    fn test_sfx_categories() {
        let categories = get_sfx_categories();
        assert!(categories.contains(&"dice_roll".to_string()));
        assert!(categories.contains(&"spell_cast".to_string()));
    }

    // Note: Actual audio playback tests require audio hardware
    // and are best done as integration tests
}
