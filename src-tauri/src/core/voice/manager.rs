//! Voice Manager with Audio Cache Integration (TASK-005)
//!
//! Manages voice synthesis providers and integrates with the AudioCache
//! for efficient caching of synthesized audio.

use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::core::voice::types::{
    Result, SynthesisRequest, SynthesisResult, VoiceConfig, VoiceProviderType,
    VoiceError, Voice,
};
use crate::core::voice::providers::{
    VoiceProvider, elevenlabs::ElevenLabsProvider, fish_audio::FishAudioProvider,
    ollama::OllamaProvider, openai::OpenAIVoiceProvider, piper::PiperProvider,
    ChatterboxProvider, GptSoVitsProvider, XttsV2Provider, FishSpeechProvider, DiaProvider,
};
use crate::core::voice::cache::{AudioCache, CacheKeyParams, CacheConfig, CacheStats, CacheError};

use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

/// Voice Manager with integrated audio caching
pub struct VoiceManager {
    config: VoiceConfig,
    providers: HashMap<String, Box<dyn VoiceProvider>>,
    cache_dir: PathBuf,
    /// The audio cache instance (lazily initialized)
    cache: RwLock<Option<Arc<AudioCache>>>,
    /// Cache configuration
    cache_config: CacheConfig,
    pub queue: Vec<crate::core::voice::types::QueuedVoice>,
    pub is_playing: bool,
}

impl VoiceManager {
    pub fn new(config: VoiceConfig) -> Self {
        let mut providers: HashMap<String, Box<dyn VoiceProvider>> = HashMap::new();

        // Initialize cloud providers
        if let Some(cfg) = &config.elevenlabs {
            providers.insert("elevenlabs".to_string(), Box::new(ElevenLabsProvider::new(cfg.clone())));
        }

        if let Some(cfg) = &config.fish_audio {
            providers.insert("fish_audio".to_string(), Box::new(FishAudioProvider::new(cfg.clone())));
        }

        // Initialize local/self-hosted providers
        if let Some(cfg) = &config.ollama {
            providers.insert("ollama".to_string(), Box::new(OllamaProvider::new(cfg.clone())));
        }

        if let Some(cfg) = &config.chatterbox {
            providers.insert("chatterbox".to_string(), Box::new(ChatterboxProvider::new(cfg.clone())));
        }

        if let Some(cfg) = &config.gpt_sovits {
            providers.insert("gpt_sovits".to_string(), Box::new(GptSoVitsProvider::new(cfg.clone())));
        }

        if let Some(cfg) = &config.xtts_v2 {
            providers.insert("xtts_v2".to_string(), Box::new(XttsV2Provider::new(cfg.clone())));
        }

        if let Some(cfg) = &config.fish_speech {
            providers.insert("fish_speech".to_string(), Box::new(FishSpeechProvider::new(cfg.clone())));
        }

        if let Some(cfg) = &config.dia {
            providers.insert("dia".to_string(), Box::new(DiaProvider::new(cfg.clone())));
        }

        if let Some(cfg) = &config.openai {
            providers.insert("openai".to_string(), Box::new(OpenAIVoiceProvider::new(cfg.clone())));
        }

        // Initialize Piper
        let piper_config = config.piper.clone().unwrap_or(crate::core::voice::types::PiperConfig { models_dir: None });
        providers.insert("piper".to_string(), Box::new(PiperProvider::new(piper_config)));

        let cache_dir = config.cache_dir.clone().unwrap_or_else(|| PathBuf::from("./voice_cache"));

        Self {
            config,
            providers,
            cache_dir,
            cache: RwLock::new(None),
            cache_config: CacheConfig::default(),
            queue: Vec::new(),
            is_playing: false,
        }
    }

    /// Create a VoiceManager with custom cache configuration
    pub fn with_cache_config(config: VoiceConfig, cache_config: CacheConfig) -> Self {
        let mut manager = Self::new(config);
        manager.cache_config = cache_config;
        manager
    }

    /// Get the current voice configuration
    pub fn get_config(&self) -> &VoiceConfig {
        &self.config
    }

    /// Get or initialize the audio cache
    async fn get_cache(&self) -> Result<Arc<AudioCache>> {
        // Check if cache already exists
        {
            let guard = self.cache.read().await;
            if let Some(ref cache) = *guard {
                return Ok(cache.clone());
            }
        }

        // Initialize cache
        let mut guard = self.cache.write().await;

        // Double-check (another thread might have initialized)
        if let Some(ref cache) = *guard {
            return Ok(cache.clone());
        }

        // Create the cache
        let cache = AudioCache::new(self.cache_dir.clone(), self.cache_config.clone())
            .await
            .map_err(|e| VoiceError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to initialize audio cache: {}", e)
            )))?;

        let cache_arc = Arc::new(cache);
        *guard = Some(cache_arc.clone());

        Ok(cache_arc)
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> Result<CacheStats> {
        let cache = self.get_cache().await?;
        Ok(cache.stats().await)
    }

    /// Clear all cache entries
    pub async fn clear_cache(&self) -> Result<()> {
        let cache = self.get_cache().await?;
        cache.clear().await.map_err(|e| VoiceError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to clear cache: {}", e)
        )))
    }

    /// Clear cache entries by tag (e.g., session_id, npc_id)
    pub async fn clear_cache_by_tag(&self, tag: &str) -> Result<usize> {
        let cache = self.get_cache().await?;
        cache.clear_by_tag(tag).await.map_err(|e| VoiceError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to clear cache by tag: {}", e)
        )))
    }

    /// Prune cache entries older than the specified age
    pub async fn prune_cache(&self, max_age_seconds: i64) -> Result<usize> {
        let cache = self.get_cache().await?;
        cache.prune_older_than(max_age_seconds).await.map_err(|e| VoiceError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to prune cache: {}", e)
        )))
    }

    /// Add an item to the voice queue
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

    /// Get all items in the voice queue
    pub fn get_queue(&self) -> Vec<crate::core::voice::types::QueuedVoice> {
        self.queue.clone()
    }

    /// Remove an item from the queue by ID
    pub fn remove_from_queue(&mut self, id: &str) {
        self.queue.retain(|item| item.id != id);
    }

    /// Update the status of a queue item
    pub fn update_status(&mut self, id: &str, status: crate::core::voice::types::VoiceStatus) {
        if let Some(item) = self.queue.iter_mut().find(|i| i.id == id) {
            item.status = status;
        }
    }

    /// Get the next pending item in the queue
    pub fn get_next_pending(&self) -> Option<crate::core::voice::types::QueuedVoice> {
        self.queue.iter()
            .find(|item| matches!(item.status, crate::core::voice::types::VoiceStatus::Pending))
            .cloned()
    }

    /// Synthesize audio with caching support
    ///
    /// Uses the AudioCache to check for cached audio before synthesizing.
    /// If not cached, synthesizes and stores the result.
    pub async fn synthesize(&self, request: SynthesisRequest) -> Result<SynthesisResult> {
        self.synthesize_with_tags(request, &[]).await
    }

    /// Synthesize audio with caching support and custom tags
    ///
    /// Tags can be used to group cache entries (e.g., by session_id, npc_id, campaign_id)
    /// for bulk operations like clearing all audio for a specific session.
    pub async fn synthesize_with_tags(&self, request: SynthesisRequest, tags: &[String]) -> Result<SynthesisResult> {
        // Get the provider
        let provider_id = self.get_provider_id()?;
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| VoiceError::NotConfigured(format!("Provider {} not configured", provider_id)))?;

        // Generate cache key using SHA256
        let settings = request.settings.clone().unwrap_or_default();
        let cache_key_params = CacheKeyParams::new(
            &request.text,
            self.config.provider.clone(),
            &request.voice_id,
            &settings,
            request.output_format.clone(),
        );
        let cache_key = cache_key_params.to_key();

        // Get or initialize cache
        let cache = self.get_cache().await?;

        // Use get_or_synthesize for atomic check-and-store
        let tags_vec: Vec<String> = tags.to_vec();
        let request_clone = request.clone();

        let result_path = cache.get_or_synthesize(
            &cache_key,
            request.output_format.clone(),
            &tags_vec,
            || async {
                // This closure is only called if the key is not in cache
                let audio_data = provider.synthesize(&request_clone).await
                    .map_err(|e| CacheError::IoError(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Synthesis failed: {}", e)
                    )))?;
                Ok(audio_data)
            }
        ).await;

        match result_path {
            Ok(path) => {
                // Determine if this was a cache hit by checking if the path already existed
                // The cache updates access time on get, so we can check stats
                let stats = cache.stats().await;
                let cached = stats.hits > 0;

                Ok(SynthesisResult {
                    audio_path: path,
                    duration_ms: None,
                    format: request.output_format,
                    cached,
                })
            }
            Err(e) => Err(VoiceError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Cache operation failed: {}", e)
            )))
        }
    }

    /// Get the provider ID string for the current provider
    fn get_provider_id(&self) -> Result<&'static str> {
        match self.config.provider {
            VoiceProviderType::ElevenLabs => Ok("elevenlabs"),
            VoiceProviderType::FishAudio => Ok("fish_audio"),
            VoiceProviderType::OpenAI => Ok("openai"),
            VoiceProviderType::Piper => Ok("piper"),
            VoiceProviderType::Ollama => Ok("ollama"),
            VoiceProviderType::Chatterbox => Ok("chatterbox"),
            VoiceProviderType::GptSoVits => Ok("gpt_sovits"),
            VoiceProviderType::XttsV2 => Ok("xtts_v2"),
            VoiceProviderType::FishSpeech => Ok("fish_speech"),
            VoiceProviderType::Dia => Ok("dia"),
            VoiceProviderType::System => Err(VoiceError::NotConfigured("System TTS not supported yet".to_string())),
            VoiceProviderType::Disabled => Err(VoiceError::NotConfigured("Voice synthesis disabled".to_string())),
        }
    }

    /// Play audio data through the system audio output
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

    /// List all available voices from configured providers
    pub async fn list_voices(&self) -> Result<Vec<Voice>> {
        let mut all_voices = Vec::new();
        for provider in self.providers.values() {
            if let Ok(mut voices) = provider.list_voices().await {
                all_voices.append(&mut voices);
            }
        }
        Ok(all_voices)
    }

    /// Get the cache directory path
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }
}
