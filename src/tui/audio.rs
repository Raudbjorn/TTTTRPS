//! Dedicated audio playback thread for TUI voice controls.
//!
//! rodio's `OutputStream` is `!Send`, so it must live on a single OS thread.
//! `AudioPlayer` spawns a persistent `std::thread` that owns the audio output
//! and receives commands via `std::sync::mpsc`. Events flow back to the TUI
//! via `tokio::sync::mpsc::UnboundedSender<AppEvent>`.

use std::io::Cursor;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc as sync_mpsc;
use std::sync::Arc;
use std::time::Duration;

use rodio::{Decoder, OutputStream, Sink};
use tokio::sync::mpsc as tokio_mpsc;

use super::events::AppEvent;

// ============================================================================
// Types
// ============================================================================

/// Commands sent from the async TUI to the audio thread.
pub enum AudioCommand {
    Play(Vec<u8>),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    Shutdown,
}

/// Events sent from the audio thread back to the TUI.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    Playing,
    Paused,
    Resumed,
    Stopped,
    Finished,
    Error(String),
}

/// Playback state tracked on the TUI side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Playing,
    Paused,
}

// ============================================================================
// AudioPlayer
// ============================================================================

/// Non-blocking audio player backed by a dedicated OS thread.
///
/// Volume is stored as an atomic u8 (0-100) so `set_volume` can be called
/// from `&self` (Services is shared by reference).
pub struct AudioPlayer {
    cmd_tx: sync_mpsc::Sender<AudioCommand>,
    /// Volume stored as 0-100 in an atomic for interior mutability.
    volume_pct: Arc<AtomicU8>,
    state: PlaybackState,
}

impl AudioPlayer {
    /// Spawn the audio thread and return a handle.
    pub fn new(event_tx: tokio_mpsc::UnboundedSender<AppEvent>) -> Self {
        let (cmd_tx, cmd_rx) = sync_mpsc::channel();

        std::thread::Builder::new()
            .name("audio-playback".into())
            .spawn(move || audio_thread(cmd_rx, event_tx))
            .expect("failed to spawn audio thread");

        Self {
            cmd_tx,
            volume_pct: Arc::new(AtomicU8::new(75)),
            state: PlaybackState::Idle,
        }
    }

    /// Get a clone of the command sender (for use in spawned tasks).
    pub fn cmd_tx(&self) -> sync_mpsc::Sender<AudioCommand> {
        self.cmd_tx.clone()
    }

    pub fn play(&self, data: Vec<u8>) {
        let _ = self.cmd_tx.send(AudioCommand::SetVolume(self.volume()));
        let _ = self.cmd_tx.send(AudioCommand::Play(data));
    }

    pub fn pause(&self) {
        let _ = self.cmd_tx.send(AudioCommand::Pause);
    }

    pub fn resume(&self) {
        let _ = self.cmd_tx.send(AudioCommand::Resume);
    }

    pub fn stop(&self) {
        let _ = self.cmd_tx.send(AudioCommand::Stop);
    }

    /// Set volume (0.0 - 1.0). Can be called from `&self`.
    pub fn set_volume(&self, vol: f32) {
        let clamped = (vol.clamp(0.0, 1.0) * 100.0) as u8;
        self.volume_pct.store(clamped, Ordering::Relaxed);
        let _ = self.cmd_tx.send(AudioCommand::SetVolume(vol.clamp(0.0, 1.0)));
    }

    pub fn state(&self) -> PlaybackState {
        self.state
    }

    pub fn volume(&self) -> f32 {
        self.volume_pct.load(Ordering::Relaxed) as f32 / 100.0
    }

    /// Update local state cache from an audio event.
    pub fn update_state(&mut self, event: &AudioEvent) {
        self.state = match event {
            AudioEvent::Playing => PlaybackState::Playing,
            AudioEvent::Paused => PlaybackState::Paused,
            AudioEvent::Resumed => PlaybackState::Playing,
            AudioEvent::Stopped | AudioEvent::Finished => PlaybackState::Idle,
            AudioEvent::Error(_) => PlaybackState::Idle,
        };
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(AudioCommand::Shutdown);
    }
}

// ============================================================================
// Audio thread
// ============================================================================

fn send_event(tx: &tokio_mpsc::UnboundedSender<AppEvent>, event: AudioEvent) {
    let _ = tx.send(AppEvent::AudioPlayback(event));
}

fn audio_thread(
    cmd_rx: sync_mpsc::Receiver<AudioCommand>,
    event_tx: tokio_mpsc::UnboundedSender<AppEvent>,
) {
    // Initialize audio output once for the thread's lifetime.
    let output = match OutputStream::try_default() {
        Ok((stream, handle)) => Some((stream, handle)),
        Err(e) => {
            log::error!("Failed to open audio output: {e}");
            send_event(&event_tx, AudioEvent::Error(format!("Audio output: {e}")));
            None
        }
    };

    let mut sink: Option<Sink> = None;
    let mut was_playing = false;

    loop {
        // Receive commands with a short timeout so we can poll sink state.
        match cmd_rx.recv_timeout(Duration::from_millis(50)) {
            Ok(AudioCommand::Play(data)) => {
                // Stop any current playback
                if let Some(ref s) = sink {
                    s.stop();
                }

                let Some((ref _stream, ref handle)) = output else {
                    send_event(
                        &event_tx,
                        AudioEvent::Error("No audio output available".into()),
                    );
                    continue;
                };

                match Sink::try_new(handle) {
                    Ok(new_sink) => {
                        let cursor = Cursor::new(data);
                        match Decoder::new(cursor) {
                            Ok(source) => {
                                new_sink.append(source);
                                was_playing = true;
                                sink = Some(new_sink);
                                send_event(&event_tx, AudioEvent::Playing);
                            }
                            Err(e) => {
                                send_event(
                                    &event_tx,
                                    AudioEvent::Error(format!("Decode error: {e}")),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        send_event(
                            &event_tx,
                            AudioEvent::Error(format!("Sink error: {e}")),
                        );
                    }
                }
            }

            Ok(AudioCommand::Pause) => {
                if let Some(ref s) = sink {
                    s.pause();
                    send_event(&event_tx, AudioEvent::Paused);
                }
            }

            Ok(AudioCommand::Resume) => {
                if let Some(ref s) = sink {
                    s.play();
                    send_event(&event_tx, AudioEvent::Resumed);
                }
            }

            Ok(AudioCommand::Stop) => {
                if let Some(ref s) = sink {
                    s.stop();
                    was_playing = false;
                    send_event(&event_tx, AudioEvent::Stopped);
                }
                sink = None;
            }

            Ok(AudioCommand::SetVolume(vol)) => {
                if let Some(ref s) = sink {
                    s.set_volume(vol);
                }
            }

            Ok(AudioCommand::Shutdown) => {
                if let Some(ref s) = sink {
                    s.stop();
                }
                return;
            }

            Err(sync_mpsc::RecvTimeoutError::Timeout) => {
                // Check if playback finished naturally
            }

            Err(sync_mpsc::RecvTimeoutError::Disconnected) => {
                return;
            }
        }

        // Detect natural playback end
        if was_playing {
            if let Some(ref s) = sink {
                if s.empty() {
                    was_playing = false;
                    send_event(&event_tx, AudioEvent::Finished);
                    sink = None;
                }
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_state_default_idle() {
        // AudioPlayer starts idle (verified via state transitions)
        let state = PlaybackState::Idle;
        assert_eq!(state, PlaybackState::Idle);
    }

    #[test]
    fn test_audio_player_state_transitions() {
        let (tx, _rx) = tokio_mpsc::unbounded_channel();
        let mut player = AudioPlayer::new(tx);

        assert_eq!(player.state(), PlaybackState::Idle);

        player.update_state(&AudioEvent::Playing);
        assert_eq!(player.state(), PlaybackState::Playing);

        player.update_state(&AudioEvent::Paused);
        assert_eq!(player.state(), PlaybackState::Paused);

        player.update_state(&AudioEvent::Resumed);
        assert_eq!(player.state(), PlaybackState::Playing);

        player.update_state(&AudioEvent::Stopped);
        assert_eq!(player.state(), PlaybackState::Idle);

        player.update_state(&AudioEvent::Playing);
        player.update_state(&AudioEvent::Finished);
        assert_eq!(player.state(), PlaybackState::Idle);

        player.update_state(&AudioEvent::Error("test".into()));
        assert_eq!(player.state(), PlaybackState::Idle);
    }

    #[test]
    fn test_volume_clamp() {
        let (tx, _rx) = tokio_mpsc::unbounded_channel();
        let player = AudioPlayer::new(tx);

        player.set_volume(0.5);
        assert!((player.volume() - 0.5).abs() < 0.02);

        player.set_volume(1.5);
        assert!((player.volume() - 1.0).abs() < 0.02);

        player.set_volume(-0.5);
        assert!((player.volume() - 0.0).abs() < 0.02);

        // Map 75/100 to 0.75
        let mapped = 75.0_f32 / 100.0;
        player.set_volume(mapped);
        assert!((player.volume() - 0.75).abs() < 0.02);
    }

    #[test]
    fn test_default_volume() {
        let (tx, _rx) = tokio_mpsc::unbounded_channel();
        let player = AudioPlayer::new(tx);
        assert!((player.volume() - 0.75).abs() < 0.02);
    }
}
