//! Voice Synthesis Commands
//!
//! Commands for synthesizing speech from text.

use tauri::State;

use crate::core::voice::{
    SynthesisRequest, OutputFormat, Voice,
};
use crate::commands::AppState;

// ============================================================================
// Voice Synthesis Commands
// ============================================================================

/// Play text-to-speech audio
#[tauri::command]
pub async fn play_tts(
    text: String,
    voice_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Synthesize audio first, keeping the lock scope minimal.
    let audio_path = {
        let manager = state.voice_manager.read().await;
        let request = SynthesisRequest {
            text,
            voice_id,
            settings: None,
            output_format: OutputFormat::Wav,
        };
        let result = manager.synthesize(request).await.map_err(|e| e.to_string())?;
        result.audio_path
    }; // Read lock is released here.

    // Read audio data in a blocking task to avoid blocking async runtime
    let audio_data = tokio::task::spawn_blocking(move || std::fs::read(&audio_path))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    // Play audio in a blocking task to avoid blocking async runtime
    tokio::task::spawn_blocking(move || {
        use rodio::{Decoder, OutputStream, Sink};
        use std::io::Cursor;

        let (_stream, stream_handle) = OutputStream::try_default().map_err(|e| e.to_string())?;
        let sink = Sink::try_new(&stream_handle).map_err(|e| e.to_string())?;
        let cursor = Cursor::new(audio_data);
        let source = Decoder::new(cursor).map_err(|e| e.to_string())?;

        sink.append(source);
        sink.sleep_until_end();
        Ok::<(), String>(())
    }).await.map_err(|e| e.to_string())??;

    Ok(())
}

/// List OpenAI TTS voices (static list)
#[tauri::command]
pub fn list_openai_voices() -> Vec<Voice> {
    crate::core::voice::providers::openai::get_openai_voices()
}

/// List OpenAI TTS models
#[tauri::command]
pub fn list_openai_tts_models() -> Vec<(String, String)> {
    vec![
        ("tts-1".to_string(), "Standard quality, faster".to_string()),
        ("tts-1-hd".to_string(), "High quality, slower".to_string()),
    ]
}

/// List available ElevenLabs voices
#[tauri::command]
pub async fn list_elevenlabs_voices(api_key: String) -> Result<Vec<Voice>, String> {
    use crate::core::voice::ElevenLabsConfig;
    use crate::core::voice::providers::elevenlabs::ElevenLabsProvider;
    use crate::core::voice::providers::VoiceProvider;

    let provider = ElevenLabsProvider::new(ElevenLabsConfig {
        api_key,
        model_id: None,
    });

    provider.list_voices().await.map_err(|e| e.to_string())
}

/// List available voices from all configured providers
#[tauri::command]
pub async fn list_available_voices(state: State<'_, AppState>) -> Result<Vec<Voice>, String> {
    state.voice_manager.read().await.list_voices().await.map_err(|e| e.to_string())
}
