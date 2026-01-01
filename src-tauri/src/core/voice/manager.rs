use std::path::PathBuf;
use tokio::fs;
use std::collections::HashMap;
use crate::core::voice::types::{Result, SynthesisRequest, SynthesisResult, VoiceConfig, VoiceProviderType, VoiceError, Voice};
use crate::core::voice::providers::{VoiceProvider};
use crate::core::voice::providers::elevenlabs::ElevenLabsProvider;
use crate::core::voice::providers::fish_audio::FishAudioProvider;
use crate::core::voice::providers::ollama::OllamaProvider;
use crate::core::voice::providers::openai::OpenAIVoiceProvider;

use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

pub struct VoiceManager {
    config: VoiceConfig,
    providers: HashMap<String, Box<dyn VoiceProvider>>,
    cache_dir: PathBuf,
    pub queue: Vec<crate::core::voice::types::QueuedVoice>,
    pub is_playing: bool,
}

impl VoiceManager {
    pub fn new(config: VoiceConfig) -> Self {
        let mut providers: HashMap<String, Box<dyn VoiceProvider>> = HashMap::new();

        // Initialize providers based on config
        if let Some(cfg) = &config.elevenlabs {
             providers.insert("elevenlabs".to_string(), Box::new(ElevenLabsProvider::new(cfg.clone())));
        }

        if let Some(cfg) = &config.fish_audio {
             providers.insert("fish_audio".to_string(), Box::new(FishAudioProvider::new(cfg.clone())));
        }

        if let Some(cfg) = &config.ollama {
             providers.insert("ollama".to_string(), Box::new(OllamaProvider::new(cfg.clone())));
        }

        if let Some(cfg) = &config.openai {
             providers.insert("openai".to_string(), Box::new(OpenAIVoiceProvider::new(cfg.clone())));
        }

        let cache_dir = config.cache_dir.clone().unwrap_or_else(|| PathBuf::from("./voice_cache"));

        Self {
            config,
            providers,
            cache_dir,
            queue: Vec::new(),
            is_playing: false,
        }
    }

    pub fn get_config(&self) -> &VoiceConfig {
        &self.config
    }

    pub fn add_to_queue(&mut self, text: String, voice_id: String) -> crate::core::voice::types::QueuedVoice {
        let item = crate::core::voice::types::QueuedVoice {
            id: uuid::Uuid::new_v4().to_string(),
            text,
            voice_id,
            status: crate::core::voice::types::VoiceStatus::Pending,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.queue.push(item.clone());
        item
    }

    pub fn get_queue(&self) -> Vec<crate::core::voice::types::QueuedVoice> {
        self.queue.clone()
    }

    pub fn remove_from_queue(&mut self, id: &str) {
        self.queue.retain(|item| item.id != id);
    }

    pub fn update_status(&mut self, id: &str, status: crate::core::voice::types::VoiceStatus) {
        if let Some(item) = self.queue.iter_mut().find(|i| i.id == id) {
            item.status = status;
        }
    }

    pub fn get_next_pending(&self) -> Option<crate::core::voice::types::QueuedVoice> {
        self.queue.iter()
            .find(|item| matches!(item.status, crate::core::voice::types::VoiceStatus::Pending))
            .cloned()
    }

    pub async fn synthesize(&self, request: SynthesisRequest) -> Result<SynthesisResult> {
        // 1. Check Cache
        let cache_key = self.cache_key(&request);
        let cache_path = self.cache_dir.join(&cache_key);

        if cache_path.exists() {
             return Ok(SynthesisResult {
                audio_path: cache_path,
                duration_ms: None,
                format: request.output_format,
                cached: true,
            });
        }

        // 2. Select Provider
        let provider_id = match self.config.provider {
            VoiceProviderType::ElevenLabs => "elevenlabs",
            VoiceProviderType::FishAudio => "fish_audio",
            VoiceProviderType::Ollama => "ollama",
            VoiceProviderType::OpenAI => "openai",
            VoiceProviderType::System => return Err(VoiceError::NotConfigured("System TTS not supported yet".to_string())),
            VoiceProviderType::Disabled => return Err(VoiceError::NotConfigured("Voice synthesis disabled".to_string())),
        };

        let provider = self.providers.get(provider_id)
            .ok_or_else(|| VoiceError::NotConfigured(format!("Provider {} not configured", provider_id)))?;

        // 3. Synthesize
        // Ensure cache directory exists
        if !self.cache_dir.exists() {
             fs::create_dir_all(&self.cache_dir).await?;
        }

        let audio_data = provider.synthesize(&request).await?;

        // 4. Cache
        fs::write(&cache_path, &audio_data).await?;

        Ok(SynthesisResult {
            audio_path: cache_path,
            duration_ms: None,
            format: request.output_format,
            cached: false,
        })
    }

    pub fn play_audio(&self, audio_data: Vec<u8>) -> Result<()> {
        let (_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| VoiceError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| VoiceError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        let cursor = Cursor::new(audio_data);
        let source = Decoder::new(cursor)
            .map_err(|e| VoiceError::IoError(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        sink.append(source);
        sink.sleep_until_end();
        Ok(())
    }

    pub async fn list_voices(&self) -> Result<Vec<Voice>> {
        let mut all_voices = Vec::new();
        // Maybe we just want voices from the ACTIVE provider?
        // Or all configured ones? Let's do active for simplicity or loop all.
        // For now, let's just loop all providers we have.
        for provider in self.providers.values() {
             if let Ok(mut voices) = provider.list_voices().await {
                 all_voices.append(&mut voices);
             }
        }
        Ok(all_voices)
    }

    fn cache_key(&self, request: &SynthesisRequest) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.text.hash(&mut hasher);
        request.voice_id.hash(&mut hasher);
        self.config.provider.hash(&mut hasher); // Also hash provider so switching providers doesn't use wrong cache

        format!(
            "{:x}.{}",
            hasher.finish(),
            request.output_format.extension()
        )
    }
}
