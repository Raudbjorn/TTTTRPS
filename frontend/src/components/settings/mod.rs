#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::bindings::{
    configure_llm, get_llm_config, save_api_key, check_llm_health, LLMSettings, HealthStatus,
    configure_voice, get_voice_config, detect_voice_providers, VoiceConfig, ElevenLabsConfig, OllamaConfig,
    OpenAIVoiceConfig, ChatterboxConfig, GptSoVitsConfig, XttsV2Config, FishSpeechConfig, DiaConfig,
    ProviderStatus, VoiceProviderDetection,
    check_meilisearch_health, reindex_library, MeilisearchStatus,
    list_ollama_models, OllamaModel,
    list_claude_models, list_openai_models, list_gemini_models,
    list_openrouter_models, list_provider_models, ModelInfo,
    list_openai_voices, list_openai_tts_models, list_elevenlabs_voices, Voice
};
use crate::components::design_system::{Button, ButtonVariant, Input, Select, Card, CardHeader, CardBody, Badge, BadgeVariant};

pub mod theme_settings;
use theme_settings::ThemeSettings;

#[derive(Clone, PartialEq)]
pub enum LLMProvider {
    Ollama,
    Claude,
    Gemini,
    OpenAI,
    OpenRouter,
    Mistral,
    Groq,
    Together,
    Cohere,
    DeepSeek,
}

impl std::fmt::Display for LLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProvider::Ollama => write!(f, "Ollama (Local)"),
            LLMProvider::Claude => write!(f, "Claude (Anthropic)"),
            LLMProvider::Gemini => write!(f, "Gemini (Google)"),
            LLMProvider::OpenAI => write!(f, "OpenAI"),
            LLMProvider::OpenRouter => write!(f, "OpenRouter (400+ models)"),
            LLMProvider::Mistral => write!(f, "Mistral AI"),
            LLMProvider::Groq => write!(f, "Groq (Fast)"),
            LLMProvider::Together => write!(f, "Together AI"),
            LLMProvider::Cohere => write!(f, "Cohere"),
            LLMProvider::DeepSeek => write!(f, "DeepSeek"),
        }
    }
}

impl LLMProvider {
    fn to_string_key(&self) -> String {
        match self {
            LLMProvider::Ollama => "ollama".to_string(),
            LLMProvider::Claude => "claude".to_string(),
            LLMProvider::Gemini => "gemini".to_string(),
            LLMProvider::OpenAI => "openai".to_string(),
            LLMProvider::OpenRouter => "openrouter".to_string(),
            LLMProvider::Mistral => "mistral".to_string(),
            LLMProvider::Groq => "groq".to_string(),
            LLMProvider::Together => "together".to_string(),
            LLMProvider::Cohere => "cohere".to_string(),
            LLMProvider::DeepSeek => "deepseek".to_string(),
        }
    }

    fn requires_api_key(&self) -> bool {
        !matches!(self, LLMProvider::Ollama)
    }
}

// ============================================================================
// Voice Provider Metadata (centralized)
// ============================================================================

/// Metadata for a voice provider
struct VoiceProviderInfo {
    id: &'static str,
    display_name: &'static str,
    is_local: bool,
    default_url: Option<&'static str>,
}

/// All self-hosted voice providers with their metadata.
/// Note: `id` values must match VoiceProviderType enum variant names exactly
/// (e.g., "Ollama", "XttsV2") since detection uses serde-serialized enum strings.
const LOCAL_VOICE_PROVIDERS: &[VoiceProviderInfo] = &[
    VoiceProviderInfo { id: "Ollama", display_name: "Ollama", is_local: true, default_url: Some("http://localhost:11434") },
    VoiceProviderInfo { id: "Chatterbox", display_name: "Chatterbox", is_local: true, default_url: Some("http://localhost:8000") },
    VoiceProviderInfo { id: "GptSoVits", display_name: "GPT-SoVITS", is_local: true, default_url: Some("http://localhost:9880") },
    VoiceProviderInfo { id: "XttsV2", display_name: "XTTS-v2 (Coqui)", is_local: true, default_url: Some("http://localhost:5002") },
    VoiceProviderInfo { id: "FishSpeech", display_name: "Fish Speech", is_local: true, default_url: Some("http://localhost:7860") },
    VoiceProviderInfo { id: "Dia", display_name: "Dia", is_local: true, default_url: Some("http://localhost:8003") },
];

/// All cloud voice providers
const CLOUD_VOICE_PROVIDERS: &[VoiceProviderInfo] = &[
    VoiceProviderInfo { id: "ElevenLabs", display_name: "ElevenLabs", is_local: false, default_url: None },
    VoiceProviderInfo { id: "OpenAI", display_name: "OpenAI TTS", is_local: false, default_url: None },
    VoiceProviderInfo { id: "FishAudio", display_name: "Fish Audio (Cloud)", is_local: false, default_url: None },
];

/// Helper to check if a provider ID is local
fn is_local_provider(id: &str) -> bool {
    LOCAL_VOICE_PROVIDERS.iter().any(|p| p.id == id)
}

/// Helper to check if a provider ID is cloud
fn is_cloud_provider(id: &str) -> bool {
    CLOUD_VOICE_PROVIDERS.iter().any(|p| p.id == id)
}

/// Get default URL for a provider
fn get_provider_default_url(id: &str) -> Option<&'static str> {
    LOCAL_VOICE_PROVIDERS.iter()
        .find(|p| p.id == id)
        .and_then(|p| p.default_url)
}

#[component]
pub fn Settings() -> Element {
    let mut selected_provider = use_signal(|| LLMProvider::Ollama);
    let mut api_key_or_host = use_signal(|| "http://localhost:11434".to_string());
    let mut model_name = use_signal(|| "llama3.2".to_string());
    let mut embedding_model = use_signal(|| "nomic-embed-text".to_string());
    let mut save_status = use_signal(|| String::new());
    let mut is_saving = use_signal(|| false);
    let mut health_status = use_signal(|| Option::<HealthStatus>::None);

    // Voice Signals
    let mut selected_voice_provider = use_signal(|| "Disabled".to_string());
    let mut voice_api_key_or_host = use_signal(|| String::new());
    let mut voice_model_id = use_signal(|| String::new());
    let mut voice_provider_detection = use_signal(|| VoiceProviderDetection::default());
    let mut is_detecting_providers = use_signal(|| false);
    let mut selected_voice_id = use_signal(|| String::new());
    let mut available_voices = use_signal(|| Vec::<Voice>::new());
    let mut openai_tts_models = use_signal(|| Vec::<(String, String)>::new());
    let mut is_loading_voices = use_signal(|| false);

    // Meilisearch Signals
    let mut meili_status = use_signal(|| Option::<MeilisearchStatus>::None);
    let mut is_reindexing = use_signal(|| false);
    let mut reindex_status = use_signal(|| String::new());

    // Ollama models list
    let mut ollama_models = use_signal(|| Vec::<OllamaModel>::new());
    // Cloud provider models list
    let mut cloud_models = use_signal(|| Vec::<ModelInfo>::new());
    let mut is_loading_models = use_signal(|| false);

    // Reusable closure to detect voice providers (used in init and refresh button)
    let refresh_voice_detection = move || {
        spawn(async move {
            is_detecting_providers.set(true);
            if let Ok(detection) = detect_voice_providers().await {
                voice_provider_detection.set(detection);
            }
            is_detecting_providers.set(false);
        });
    };

    // Function to fetch Ollama models
    let fetch_ollama_models = move |host: String| {
        spawn(async move {
            is_loading_models.set(true);
            match list_ollama_models(host).await {
                Ok(models) => {
                    ollama_models.set(models);
                }
                Err(_) => {
                    ollama_models.set(Vec::new());
                }
            }
            is_loading_models.set(false);
        });
    };

    // Function to fetch cloud provider models (with API key)
    let fetch_cloud_models = move |provider: LLMProvider, api_key: Option<String>| {
        spawn(async move {
            is_loading_models.set(true);
            let models = match provider {
                // Providers with dedicated API endpoints
                LLMProvider::Claude => list_claude_models(api_key).await.unwrap_or_default(),
                LLMProvider::OpenAI => list_openai_models(api_key).await.unwrap_or_default(),
                LLMProvider::Gemini => list_gemini_models(api_key).await.unwrap_or_default(),
                LLMProvider::OpenRouter => list_openrouter_models().await.unwrap_or_default(),
                // Providers using LiteLLM catalog
                p @ (LLMProvider::Mistral
                | LLMProvider::Groq
                | LLMProvider::Together
                | LLMProvider::Cohere
                | LLMProvider::DeepSeek) => {
                    list_provider_models(p.to_string_key()).await.unwrap_or_default()
                }
                _ => Vec::new(),
            };
            cloud_models.set(models);
            is_loading_models.set(false);
        });
    };

    // Function to fetch voices based on provider
    let fetch_voices = move |provider: String, api_key: Option<String>| {
        spawn(async move {
            is_loading_voices.set(true);
            match provider.as_str() {
                "OpenAI" => {
                    // OpenAI voices are static, no API call needed
                    if let Ok(voices) = list_openai_voices().await {
                        available_voices.set(voices);
                    }
                    // Also fetch TTS models
                    if let Ok(models) = list_openai_tts_models().await {
                        openai_tts_models.set(models);
                    }
                }
                "ElevenLabs" => {
                    if let Some(key) = api_key {
                        if !key.is_empty() && !key.starts_with("*") {
                            if let Ok(voices) = list_elevenlabs_voices(key).await {
                                available_voices.set(voices);
                            }
                        }
                    }
                }
                _ => {
                    available_voices.set(Vec::new());
                }
            }
            is_loading_voices.set(false);
        });
    };

    // Load existing config on mount
    use_effect(move || {
        spawn(async move {
            if let Ok(Some(config)) = get_llm_config().await {
                match config.provider.as_str() {
                    "ollama" => {
                        selected_provider.set(LLMProvider::Ollama);
                        let host = config.host.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
                        api_key_or_host.set(host.clone());
                        // Fetch available models
                        if let Ok(models) = list_ollama_models(host).await {
                            ollama_models.set(models);
                        }
                    }
                    "claude" => {
                        selected_provider.set(LLMProvider::Claude);
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_claude_models(None).await {
                            cloud_models.set(models);
                        }
                    }
                    "gemini" => {
                        selected_provider.set(LLMProvider::Gemini);
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_gemini_models(None).await {
                            cloud_models.set(models);
                        }
                    }
                    "openai" => {
                        selected_provider.set(LLMProvider::OpenAI);
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_openai_models(None).await {
                            cloud_models.set(models);
                        }
                    }
                    "openrouter" => {
                        selected_provider.set(LLMProvider::OpenRouter);
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_openrouter_models().await {
                            cloud_models.set(models);
                        }
                    }
                    "mistral" => {
                        selected_provider.set(LLMProvider::Mistral);
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_provider_models("mistral".to_string()).await {
                            cloud_models.set(models);
                        }
                    }
                    "groq" => {
                        selected_provider.set(LLMProvider::Groq);
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_provider_models("groq".to_string()).await {
                            cloud_models.set(models);
                        }
                    }
                    "together" => {
                        selected_provider.set(LLMProvider::Together);
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_provider_models("together".to_string()).await {
                            cloud_models.set(models);
                        }
                    }
                    "cohere" => {
                        selected_provider.set(LLMProvider::Cohere);
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_provider_models("cohere".to_string()).await {
                            cloud_models.set(models);
                        }
                    }
                    "deepseek" => {
                        selected_provider.set(LLMProvider::DeepSeek);
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_provider_models("deepseek".to_string()).await {
                            cloud_models.set(models);
                        }
                    }
                    _ => {}
                }
                model_name.set(config.model);
                if let Some(emb) = config.embedding_model {
                    embedding_model.set(emb);
                }
            } else {
                // No config, but if Ollama is default, fetch models
                if let Ok(models) = list_ollama_models("http://localhost:11434".to_string()).await {
                    ollama_models.set(models);
                }
            }

            // Load Voice Config
            if let Ok(config) = get_voice_config().await {
                let provider_str = match config.provider.as_str() {
                    "ElevenLabs" => "ElevenLabs",
                    "Ollama" => "Ollama",
                    "FishAudio" => "FishAudio",
                    "OpenAI" => "OpenAI",
                    _ => "Disabled",
                };
                selected_voice_provider.set(provider_str.to_string());

                match provider_str {
                    "ElevenLabs" => {
                        if let Some(c) = config.elevenlabs {
                            voice_api_key_or_host.set(c.api_key.clone());
                            voice_model_id.set(c.model_id.unwrap_or_default());
                            // Fetch ElevenLabs voices if API key is valid
                            if !c.api_key.is_empty() && !c.api_key.starts_with("*") {
                                if let Ok(voices) = list_elevenlabs_voices(c.api_key).await {
                                    available_voices.set(voices);
                                }
                            }
                        }
                    }
                    "Ollama" => {
                        if let Some(c) = config.ollama {
                            voice_api_key_or_host.set(c.base_url);
                            voice_model_id.set(c.model);
                        }
                    }
                    "OpenAI" => {
                        if let Some(c) = config.openai {
                            voice_api_key_or_host.set(c.api_key);
                            voice_model_id.set(c.model);
                            selected_voice_id.set(c.voice);
                        }
                        // Fetch OpenAI voices (static list)
                        if let Ok(voices) = list_openai_voices().await {
                            available_voices.set(voices);
                        }
                        if let Ok(models) = list_openai_tts_models().await {
                            openai_tts_models.set(models);
                        }
                    }
                    _ => {}
                }
            }

            // Check health
            if let Ok(status) = check_llm_health().await {
                health_status.set(Some(status));
            }

            // Check Meilisearch health
            if let Ok(status) = check_meilisearch_health().await {
                meili_status.set(Some(status));
            }

            // Detect available voice providers (using shared closure)
            refresh_voice_detection();
        });
    });

    let test_connection = move |_: MouseEvent| {
        spawn(async move {
            save_status.set("Testing connection...".to_string());
            match check_llm_health().await {
                Ok(status) => {
                    health_status.set(Some(status.clone()));
                    if status.healthy {
                        save_status.set(format!("Connected to {}", status.provider));
                    } else {
                        save_status.set(format!("Connection failed: {}", status.message));
                    }
                }
                Err(e) => {
                    save_status.set(format!("Error: {}", e));
                }
            }
        });
    };

    let save_settings = move |_: MouseEvent| {
        is_saving.set(true);
        save_status.set("Saving...".to_string());

        let provider = selected_provider.read().to_string_key();
        let api_key_or_host_val = api_key_or_host.read().clone();
        let model = model_name.read().clone();
        let emb_model = embedding_model.read().clone();

        spawn(async move {
            let settings = match provider.as_str() {
                "ollama" => LLMSettings {
                    provider: "ollama".to_string(),
                    api_key: None,
                    host: Some(api_key_or_host_val),
                    model,
                    embedding_model: Some(emb_model),
                },
                "claude" => {
                    // Save API key securely first
                    if !api_key_or_host_val.is_empty() {
                        if let Err(e) = save_api_key("claude".to_string(), api_key_or_host_val.clone()).await {
                            save_status.set(format!("Failed to save API key: {}", e));
                            is_saving.set(false);
                            return;
                        }
                    }
                    LLMSettings {
                        provider: "claude".to_string(),
                        api_key: if api_key_or_host_val.is_empty() { None } else { Some(api_key_or_host_val) },
                        host: None,
                        model,
                        embedding_model: None,
                    }
                }
                "gemini" => {
                    // Save API key securely first
                    if !api_key_or_host_val.is_empty() {
                        if let Err(e) = save_api_key("gemini".to_string(), api_key_or_host_val.clone()).await {
                            save_status.set(format!("Failed to save API key: {}", e));
                            is_saving.set(false);
                            return;
                        }
                    }
                    LLMSettings {
                        provider: "gemini".to_string(),
                        api_key: if api_key_or_host_val.is_empty() { None } else { Some(api_key_or_host_val) },
                        host: None,
                        model,
                        embedding_model: None,
                    }
                    }

                "openai" => {
                    // Save API key securely first
                    if !api_key_or_host_val.is_empty() {
                        if let Err(e) = save_api_key("openai".to_string(), api_key_or_host_val.clone()).await {
                            save_status.set(format!("Failed to save API key: {}", e));
                            is_saving.set(false);
                            return;
                        }
                    }
                    LLMSettings {
                        provider: "openai".to_string(),
                        api_key: if api_key_or_host_val.is_empty() { None } else { Some(api_key_or_host_val) },
                        host: None,
                        model,
                        embedding_model: None,
                    }
                }
                _ => {
                    save_status.set("Unknown provider".to_string());
                    is_saving.set(false);
                    return;
                }
            };

            match configure_llm(settings).await {
                Ok(msg) => {
                    save_status.set(msg);
                    // Refresh health status
                    if let Ok(status) = check_llm_health().await {
                        health_status.set(Some(status));
                    }
                }
                Err(e) => {
                    save_status.set(format!("Error: {}", e));
                }
            }
            is_saving.set(false);
            // Save Voice Settings
            let voice_prov = selected_voice_provider.read().clone();
            let voice_val = voice_api_key_or_host.read().clone();
            let voice_mod = voice_model_id.read().clone();
            let voice_id = selected_voice_id.read().clone();

            let voice_config = if voice_prov == "Disabled" {
                VoiceConfig {
                    provider: "Disabled".to_string(),
                    cache_dir: None,
                    default_voice_id: None,
                    elevenlabs: None,
                    fish_audio: None,
                    openai: None,
                    ollama: None,
                    chatterbox: None,
                    gpt_sovits: None,
                    xtts_v2: None,
                    fish_speech: None,
                    dia: None,
                }
            } else {
                let mut base = VoiceConfig {
                    provider: voice_prov.clone(),
                    cache_dir: None,
                    default_voice_id: None,
                    elevenlabs: None,
                    fish_audio: None,
                    openai: None,
                    ollama: None,
                    chatterbox: None,
                    gpt_sovits: None,
                    xtts_v2: None,
                    fish_speech: None,
                    dia: None,
                };

                match voice_prov.as_str() {
                    "ElevenLabs" => {
                        base.elevenlabs = Some(ElevenLabsConfig {
                            api_key: voice_val,
                            model_id: Some(voice_mod),
                        });
                    }
                    "Ollama" => {
                        base.ollama = Some(OllamaConfig {
                            base_url: voice_val,
                            model: voice_mod,
                        });
                    }
                    "OpenAI" => {
                        base.openai = Some(OpenAIVoiceConfig {
                            api_key: voice_val,
                            model: voice_mod,
                            voice: voice_id,
                        });
                    }
                    "Chatterbox" => {
                        base.chatterbox = Some(ChatterboxConfig {
                            base_url: voice_val,
                            reference_audio: None,
                            exaggeration: Some(0.5),
                            cfg_weight: Some(0.5),
                        });
                    }
                    "GptSoVits" => {
                        base.gpt_sovits = Some(GptSoVitsConfig {
                            base_url: voice_val,
                            reference_audio: None,
                            reference_text: None,
                            language: Some("en".to_string()),
                            speaker_id: None,
                        });
                    }
                    "XttsV2" => {
                        base.xtts_v2 = Some(XttsV2Config {
                            base_url: voice_val,
                            speaker_wav: None,
                            language: Some("en".to_string()),
                        });
                    }
                    "FishSpeech" => {
                        base.fish_speech = Some(FishSpeechConfig {
                            base_url: voice_val,
                            reference_audio: None,
                            reference_text: None,
                        });
                    }
                    "Dia" => {
                        base.dia = Some(DiaConfig {
                            base_url: voice_val,
                            voice_id: None,
                            dialogue_mode: Some(false),
                        });
                    }
                    _ => {}
                }
                base
            };

            if let Err(e) = configure_voice(voice_config).await {
                 save_status.set(format!("Voice Config Error: {}", e));
                 is_saving.set(false);
                 return;
            }

            is_saving.set(false);
        });
    };

    let placeholder_text = match *selected_provider.read() {
        LLMProvider::Ollama => "http://localhost:11434",
        LLMProvider::Claude => "sk-ant-...",
        LLMProvider::Gemini => "AIza...",
        LLMProvider::OpenAI => "sk-...",
        LLMProvider::OpenRouter => "sk-or-...",
        LLMProvider::Mistral => "API Key",
        LLMProvider::Groq => "gsk_...",
        LLMProvider::Together => "API Key",
        LLMProvider::Cohere => "API Key",
        LLMProvider::DeepSeek => "sk-...",
    };

    let label_text = match *selected_provider.read() {
        LLMProvider::Ollama => "Ollama Host",
        LLMProvider::Claude => "Claude API Key",
        LLMProvider::Gemini => "Gemini API Key",
        LLMProvider::OpenAI => "OpenAI API Key",
        LLMProvider::OpenRouter => "OpenRouter API Key",
        LLMProvider::Mistral => "Mistral API Key",
        LLMProvider::Groq => "Groq API Key",
        LLMProvider::Together => "Together API Key",
        LLMProvider::Cohere => "Cohere API Key",
        LLMProvider::DeepSeek => "DeepSeek API Key",
    };

    let show_embedding_model = matches!(*selected_provider.read(), LLMProvider::Ollama);

    rsx! {
        div {
            class: "p-8 bg-theme-primary text-theme-primary min-h-screen font-sans transition-colors duration-300",
            div {
                class: "max-w-2xl mx-auto space-y-6",
                div {
                    class: "flex items-center justify-between",
                    div {
                        class: "flex items-center gap-4",
                        Link { to: crate::Route::Chat {}, class: "text-gray-400 hover:text-white transition-colors",
                             "← Back"
                        }
                        h1 { class: "text-2xl font-bold", "Settings" }
                    }
                    if let Some(status) = health_status.read().as_ref() {
                         Badge {
                            variant: if status.healthy { BadgeVariant::Success } else { BadgeVariant::Error },
                            "{status.message}"
                        }
                    }
                }

                Card {
                    CardHeader {
                         h2 { class: "text-lg font-semibold", "LLM Configuration" }
                    }
                    CardBody {
                        class: "space-y-4",
                        // Provider Selection
                        div {
                            label { class: "block text-sm font-medium text-theme-secondary mb-1", "Provider" }
                            Select {
                                value: selected_provider.read().to_string_key(),
                                onchange: move |val: String| {
                                    let provider = match val.as_str() {
                                        "Claude" => LLMProvider::Claude,
                                        "Gemini" => LLMProvider::Gemini,
                                        "OpenAI" => LLMProvider::OpenAI,
                                        "OpenRouter" => LLMProvider::OpenRouter,
                                        "Mistral" => LLMProvider::Mistral,
                                        "Groq" => LLMProvider::Groq,
                                        "Together" => LLMProvider::Together,
                                        "Cohere" => LLMProvider::Cohere,
                                        "DeepSeek" => LLMProvider::DeepSeek,
                                        _ => LLMProvider::Ollama,
                                    };
                                    selected_provider.set(provider.clone());
                                    // Reset defaults
                                    match provider {
                                        LLMProvider::Ollama => {
                                             api_key_or_host.set("http://localhost:11434".to_string());
                                             model_name.set("llama3.2".to_string());
                                             fetch_ollama_models("http://localhost:11434".to_string());
                                        }
                                        LLMProvider::Claude => {
                                             api_key_or_host.set(String::new());
                                             model_name.set("claude-3-5-sonnet-20241022".to_string());
                                             fetch_cloud_models(LLMProvider::Claude, None);
                                        }
                                        LLMProvider::Gemini => {
                                             api_key_or_host.set(String::new());
                                             model_name.set("gemini-1.5-pro".to_string());
                                             fetch_cloud_models(LLMProvider::Gemini, None);
                                        }
                                        LLMProvider::OpenAI => {
                                             api_key_or_host.set(String::new());
                                             model_name.set("gpt-4o".to_string());
                                             fetch_cloud_models(LLMProvider::OpenAI, None);
                                        }
                                        LLMProvider::OpenRouter => {
                                             api_key_or_host.set(String::new());
                                             model_name.set("openai/gpt-4o".to_string());
                                             fetch_cloud_models(LLMProvider::OpenRouter, None);
                                        }
                                        LLMProvider::Mistral => {
                                             api_key_or_host.set(String::new());
                                             model_name.set("mistral-large-latest".to_string());
                                             fetch_cloud_models(LLMProvider::Mistral, None);
                                        }
                                        LLMProvider::Groq => {
                                             api_key_or_host.set(String::new());
                                             model_name.set("llama-3.3-70b-versatile".to_string());
                                             fetch_cloud_models(LLMProvider::Groq, None);
                                        }
                                        LLMProvider::Together => {
                                             api_key_or_host.set(String::new());
                                             model_name.set("meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo".to_string());
                                             fetch_cloud_models(LLMProvider::Together, None);
                                        }
                                        LLMProvider::Cohere => {
                                             api_key_or_host.set(String::new());
                                             model_name.set("command-r-plus".to_string());
                                             fetch_cloud_models(LLMProvider::Cohere, None);
                                        }
                                        LLMProvider::DeepSeek => {
                                             api_key_or_host.set(String::new());
                                             model_name.set("deepseek-chat".to_string());
                                             fetch_cloud_models(LLMProvider::DeepSeek, None);
                                        }
                                    }
                                },
                                option { value: "Ollama", selected: matches!(*selected_provider.read(), LLMProvider::Ollama), "Ollama (Local)" }
                                option { value: "OpenRouter", selected: matches!(*selected_provider.read(), LLMProvider::OpenRouter), "OpenRouter (400+ models)" }
                                option { value: "Claude", selected: matches!(*selected_provider.read(), LLMProvider::Claude), "Claude (Anthropic)" }
                                option { value: "OpenAI", selected: matches!(*selected_provider.read(), LLMProvider::OpenAI), "OpenAI" }
                                option { value: "Gemini", selected: matches!(*selected_provider.read(), LLMProvider::Gemini), "Gemini (Google)" }
                                option { value: "Mistral", selected: matches!(*selected_provider.read(), LLMProvider::Mistral), "Mistral AI" }
                                option { value: "Groq", selected: matches!(*selected_provider.read(), LLMProvider::Groq), "Groq (Fast)" }
                                option { value: "Together", selected: matches!(*selected_provider.read(), LLMProvider::Together), "Together AI" }
                                option { value: "Cohere", selected: matches!(*selected_provider.read(), LLMProvider::Cohere), "Cohere" }
                                option { value: "DeepSeek", selected: matches!(*selected_provider.read(), LLMProvider::DeepSeek), "DeepSeek" }
                            }
                        }

                        // API Key / Host
                        div {
                            label { class: "block text-sm font-medium text-theme-secondary mb-1", "{label_text}" }
                            Input {
                                r#type: if matches!(*selected_provider.read(), LLMProvider::Ollama) { "text" } else { "password" },
                                placeholder: "{placeholder_text}",
                                value: "{api_key_or_host}",
                                oninput: move |val| api_key_or_host.set(val)
                            }
                        }

                        // Model Name - dropdown for all providers
                        div {
                            label { class: "block text-sm font-medium text-theme-secondary mb-1", "Model" }
                            if matches!(*selected_provider.read(), LLMProvider::Ollama) {
                                // Ollama dropdown
                                select {
                                    class: "w-full p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none",
                                    value: "{model_name}",
                                    onchange: move |e| model_name.set(e.value()),
                                    if ollama_models.read().is_empty() {
                                        option { value: "{model_name}", "{model_name}" }
                                    }
                                    for model in ollama_models.read().iter() {
                                        option {
                                            value: "{model.name}",
                                            selected: *model_name.read() == model.name,
                                            if let Some(ref size) = model.size {
                                                "{model.name} ({size})"
                                            } else {
                                                "{model.name}"
                                            }
                                        }
                                    }
                                }
                            } else {
                                // Cloud provider dropdown
                                select {
                                    class: "w-full p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none",
                                    value: "{model_name}",
                                    onchange: move |e| model_name.set(e.value()),
                                    if cloud_models.read().is_empty() {
                                        option { value: "{model_name}", "{model_name}" }
                                    }
                                    for model in cloud_models.read().iter() {
                                        option {
                                            value: "{model.id}",
                                            selected: *model_name.read() == model.id,
                                            if let Some(ref desc) = model.description {
                                                "{model.name} - {desc}"
                                            } else {
                                                "{model.name}"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Embedding Model
                        if show_embedding_model {
                            div {
                                label { class: "block text-sm font-medium text-theme-secondary mb-1", "Embedding Model" }
                                Input {
                                    placeholder: "nomic-embed-text",
                                    value: "{embedding_model}",
                                    oninput: move |val| embedding_model.set(val)
                                }
                            }
                        }
                    }
                }

                Card {
                    CardHeader {
                        div {
                            class: "flex items-center justify-between",
                            h2 { class: "text-lg font-semibold", "Voice Configuration" }
                            if *is_detecting_providers.read() {
                                span { class: "text-sm text-gray-400", "Detecting..." }
                            }
                        }
                    }
                    CardBody {
                        class: "space-y-4",
                        div {
                            label { class: "block text-sm font-medium text-theme-secondary mb-1", "Voice Provider" }
                            select {
                                class: "w-full p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none",
                                value: "{selected_voice_provider}",
                                onchange: move |e| {
                                    let val = e.value();
                                    selected_voice_provider.set(val.clone());
                                    available_voices.set(Vec::new());
                                    // Set defaults based on provider using centralized metadata
                                    if let Some(url) = get_provider_default_url(&val) {
                                        voice_api_key_or_host.set(url.to_string());
                                        let model = match val.as_str() {
                                            "Ollama" => "bark",
                                            _ => "",
                                        };
                                        voice_model_id.set(model.to_string());
                                    } else if is_cloud_provider(&val) {
                                        voice_api_key_or_host.set(String::new());
                                        match val.as_str() {
                                            "ElevenLabs" => {
                                                voice_model_id.set("eleven_multilingual_v2".to_string());
                                            }
                                            "OpenAI" => {
                                                voice_model_id.set("tts-1".to_string());
                                                selected_voice_id.set("alloy".to_string());
                                                fetch_voices("OpenAI".to_string(), None);
                                            }
                                            _ => {}
                                        }
                                    }
                                },
                                option { value: "Disabled", "Disabled" }
                                // Cloud providers
                                optgroup { label: "Cloud Providers",
                                    option { value: "OpenAI", "OpenAI TTS" }
                                    option { value: "ElevenLabs", "ElevenLabs" }
                                    option { value: "FishAudio", "Fish Audio (Cloud)" }
                                }
                                // Self-hosted providers with availability status (using centralized metadata)
                                optgroup { label: "Self-Hosted (Local)",
                                    {
                                        let detection = voice_provider_detection.read();
                                        rsx! {
                                            for provider_info in LOCAL_VOICE_PROVIDERS {
                                                {
                                                    // VoiceProviderType enum serializes to PascalCase strings
                                                    // (e.g., "Ollama", "XttsV2") matching provider_info.id
                                                    let is_available = detection.providers.iter()
                                                        .find(|p| p.provider == provider_info.id)
                                                        .map(|p| p.available)
                                                        .unwrap_or(false);
                                                    let display = if is_available {
                                                        format!("{} [running]", provider_info.display_name)
                                                    } else {
                                                        format!("{} [not detected]", provider_info.display_name)
                                                    };
                                                    let style = if is_available { "" } else { "color: #888;" };
                                                    rsx! {
                                                        option {
                                                            value: provider_info.id,
                                                            style: style,
                                                            "{display}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Provider-specific configuration
                        if *selected_voice_provider.read() != "Disabled" {
                            // API Key / Base URL (using centralized helpers)
                            {
                                let provider = selected_voice_provider.read().clone();
                                let provider_is_local = is_local_provider(&provider);

                                rsx! {
                                    div {
                                        label { class: "block text-sm font-medium text-theme-secondary mb-1",
                                            if provider_is_local { "Base URL" } else { "API Key" }
                                        }
                                        Input {
                                            r#type: if provider_is_local { "text" } else { "password" },
                                            placeholder: if provider_is_local {
                                                get_provider_default_url(&provider).unwrap_or("")
                                            } else { "sk-..." },
                                            value: "{voice_api_key_or_host}",
                                            oninput: move |val| voice_api_key_or_host.set(val)
                                        }
                                    }

                                    // OpenAI-specific configuration
                                    if provider == "OpenAI" {
                                        div {
                                            label { class: "block text-sm font-medium text-theme-secondary mb-1", "TTS Model" }
                                            select {
                                                class: "w-full p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none",
                                                value: "{voice_model_id}",
                                                onchange: move |e| voice_model_id.set(e.value()),
                                                if openai_tts_models.read().is_empty() {
                                                    option { value: "tts-1", "TTS-1 (Fast)" }
                                                    option { value: "tts-1-hd", "TTS-1 HD (High Quality)" }
                                                }
                                                for (id, name) in openai_tts_models.read().iter() {
                                                    option {
                                                        value: "{id}",
                                                        selected: *voice_model_id.read() == *id,
                                                        "{name}"
                                                    }
                                                }
                                            }
                                        }

                                        div {
                                            label { class: "block text-sm font-medium text-theme-secondary mb-1", "Voice" }
                                            select {
                                                class: "w-full p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none",
                                                value: "{selected_voice_id}",
                                                onchange: move |e| selected_voice_id.set(e.value()),
                                                if available_voices.read().is_empty() {
                                                    option { value: "alloy", "Alloy - Neutral and balanced" }
                                                    option { value: "echo", "Echo - Warm and clear" }
                                                    option { value: "fable", "Fable - British accent" }
                                                    option { value: "onyx", "Onyx - Deep and authoritative" }
                                                    option { value: "nova", "Nova - Friendly and upbeat" }
                                                    option { value: "shimmer", "Shimmer - Warm and pleasant" }
                                                }
                                                for voice in available_voices.read().iter() {
                                                    option {
                                                        value: "{voice.id}",
                                                        selected: *selected_voice_id.read() == voice.id,
                                                        if let Some(ref desc) = voice.description {
                                                            "{voice.name} - {desc}"
                                                        } else {
                                                            "{voice.name}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // ElevenLabs-specific configuration
                                    if provider == "ElevenLabs" {
                                        div {
                                            label { class: "block text-sm font-medium text-theme-secondary mb-1", "Model ID" }
                                            Input {
                                                placeholder: "eleven_multilingual_v2",
                                                value: "{voice_model_id}",
                                                oninput: move |val| voice_model_id.set(val)
                                            }
                                        }

                                        if !available_voices.read().is_empty() {
                                            div {
                                                label { class: "block text-sm font-medium text-theme-secondary mb-1", "Voice" }
                                                select {
                                                    class: "w-full p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none",
                                                    value: "{selected_voice_id}",
                                                    onchange: move |e| selected_voice_id.set(e.value()),
                                                    for voice in available_voices.read().iter() {
                                                        option {
                                                            value: "{voice.id}",
                                                            selected: *selected_voice_id.read() == voice.id,
                                                            "{voice.name}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }

                                    // Ollama/local provider model input
                                    if provider == "Ollama" {
                                        div {
                                            label { class: "block text-sm font-medium text-theme-secondary mb-1", "Voice Model" }
                                            Input {
                                                placeholder: "bark",
                                                value: "{voice_model_id}",
                                                oninput: move |val| voice_model_id.set(val)
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Refresh detection button (using shared closure)
                        div {
                            class: "pt-2",
                            Button {
                                variant: ButtonVariant::Secondary,
                                loading: *is_detecting_providers.read(),
                                onclick: move |_| refresh_voice_detection(),
                                "Refresh Detection"
                            }
                        }
                    }
                }

                ThemeSettings {}

                Card {
                    CardHeader {
                        div {
                            class: "flex items-center justify-between",
                            h2 { class: "text-lg font-semibold", "Search Engine" }
                            if let Some(status) = meili_status.read().as_ref() {
                                Badge {
                                    variant: if status.healthy { BadgeVariant::Success } else { BadgeVariant::Error },
                                    if status.healthy { "Connected" } else { "Offline" }
                                }
                            }
                        }
                    }
                    CardBody {
                        class: "space-y-4",
                        // Host info
                        if let Some(status) = meili_status.read().as_ref() {
                            div {
                                class: "text-sm text-theme-secondary",
                                span { class: "font-medium", "Host: " }
                                span { "{status.host}" }
                            }
                        }

                        // Document counts
                        if let Some(status) = meili_status.read().as_ref() {
                            if let Some(counts) = &status.document_counts {
                                div {
                                    class: "space-y-2",
                                    h3 { class: "text-sm font-medium text-theme-secondary", "Index Document Counts" }
                                    div {
                                        class: "grid grid-cols-2 gap-2 text-sm",
                                        for (index, count) in counts.iter() {
                                            div {
                                                class: "flex justify-between px-2 py-1 bg-theme-secondary rounded",
                                                span { class: "font-mono", "{index}" }
                                                span { class: "text-theme-accent", "{count}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Reindex status message
                        if !reindex_status.read().is_empty() {
                            div {
                                class: if reindex_status.read().contains("Error") { "text-red-400 text-sm" } else { "text-green-400 text-sm" },
                                "{reindex_status}"
                            }
                        }

                        // Reindex button
                        div {
                            class: "pt-2",
                            Button {
                                variant: ButtonVariant::Secondary,
                                loading: *is_reindexing.read(),
                                onclick: move |_| {
                                    is_reindexing.set(true);
                                    reindex_status.set("Re-indexing...".to_string());
                                    spawn(async move {
                                        match reindex_library(None).await {
                                            Ok(msg) => {
                                                reindex_status.set(msg);
                                                // Refresh status
                                                if let Ok(status) = check_meilisearch_health().await {
                                                    meili_status.set(Some(status));
                                                }
                                            }
                                            Err(e) => {
                                                reindex_status.set(format!("Error: {}", e));
                                            }
                                        }
                                        is_reindexing.set(false);
                                    });
                                },
                                "Clear All Indexes"
                            }
                        }
                    }
                }

                // Actions
                div {
                    class: "flex justify-end gap-4 pt-4 border-t border-gray-700",
                    if !save_status.read().is_empty() {
                         span {
                            class: if save_status.read().contains("Error") || save_status.read().contains("Failed") { "text-red-400 self-center mr-auto" } else { "text-green-400 self-center mr-auto" },
                            "{save_status}"
                         }
                    }
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: test_connection,
                        "Test Connection"
                    }
                    Button {
                        variant: ButtonVariant::Primary,
                        loading: *is_saving.read(),
                        onclick: save_settings,
                        "Save Changes"
                    }
                }
            }
        }
    }
}
