//! Settings page component for Leptos
//! Settings components

use leptos::prelude::*;
use leptos::ev;
use leptos_router::hooks::use_navigate;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    check_llm_health, check_meilisearch_health, configure_llm, configure_voice, get_llm_config,
    get_voice_config, list_claude_models, list_elevenlabs_voices, list_gemini_models,
    list_ollama_models, list_openai_models, list_openai_tts_models, list_openai_voices,
    list_openrouter_models, list_provider_models, reindex_library, save_api_key,
    ElevenLabsConfig, HealthStatus, LLMSettings, MeilisearchStatus, ModelInfo, OllamaConfig,
    OllamaModel, OpenAIVoiceConfig, Voice, VoiceConfig,
    // Audio cache imports (TASK-005)
    get_audio_cache_stats, get_audio_cache_size, clear_audio_cache, prune_audio_cache,
    VoiceCacheStats, AudioCacheSizeInfo, format_bytes,
    // Claude Code CLI imports
    ClaudeCodeStatus, get_claude_code_status, claude_code_login,
};
use crate::components::design_system::{Badge, BadgeVariant, Button, ButtonVariant, Card, CardBody, CardHeader, Input, Select, Slider};
use crate::services::theme_service::{ThemeState, ThemeWeights};

// ============================================================================
// LLM Provider Enum
// ============================================================================

#[derive(Clone, PartialEq, Debug)]
pub enum LLMProvider {
    Ollama,
    Claude,
    ClaudeCode,
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
            LLMProvider::ClaudeCode => write!(f, "Claude Code (CLI)"),
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
            LLMProvider::ClaudeCode => "claude-code".to_string(),
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

    fn from_string(s: &str) -> Self {
        match s {
            "Claude" | "claude" => LLMProvider::Claude,
            "ClaudeCode" | "claude-code" => LLMProvider::ClaudeCode,
            "Gemini" | "gemini" => LLMProvider::Gemini,
            "OpenAI" | "openai" => LLMProvider::OpenAI,
            "OpenRouter" | "openrouter" => LLMProvider::OpenRouter,
            "Mistral" | "mistral" => LLMProvider::Mistral,
            "Groq" | "groq" => LLMProvider::Groq,
            "Together" | "together" => LLMProvider::Together,
            "Cohere" | "cohere" => LLMProvider::Cohere,
            "DeepSeek" | "deepseek" => LLMProvider::DeepSeek,
            _ => LLMProvider::Ollama,
        }
    }

    #[allow(dead_code)]
    fn requires_api_key(&self) -> bool {
        !matches!(self, LLMProvider::Ollama | LLMProvider::ClaudeCode)
    }

    fn placeholder_text(&self) -> &'static str {
        match self {
            LLMProvider::Ollama => "http://localhost:11434",
            LLMProvider::Claude => "sk-ant-...",
            LLMProvider::ClaudeCode => "(No API key needed)",
            LLMProvider::Gemini => "AIza...",
            LLMProvider::OpenAI => "sk-...",
            LLMProvider::OpenRouter => "sk-or-...",
            LLMProvider::Mistral => "API Key",
            LLMProvider::Groq => "gsk_...",
            LLMProvider::Together => "API Key",
            LLMProvider::Cohere => "API Key",
            LLMProvider::DeepSeek => "sk-...",
        }
    }

    fn label_text(&self) -> &'static str {
        match self {
            LLMProvider::Ollama => "Ollama Host",
            LLMProvider::Claude => "Claude API Key",
            LLMProvider::ClaudeCode => "Claude Code Status",
            LLMProvider::Gemini => "Gemini API Key",
            LLMProvider::OpenAI => "OpenAI API Key",
            LLMProvider::OpenRouter => "OpenRouter API Key",
            LLMProvider::Mistral => "Mistral API Key",
            LLMProvider::Groq => "Groq API Key",
            LLMProvider::Together => "Together API Key",
            LLMProvider::Cohere => "Cohere API Key",
            LLMProvider::DeepSeek => "DeepSeek API Key",
        }
    }

    fn default_model(&self) -> &'static str {
        match self {
            LLMProvider::Ollama => "llama3.2",
            LLMProvider::Claude => "claude-3-5-sonnet-20241022",
            LLMProvider::ClaudeCode => "claude-sonnet-4-20250514",
            LLMProvider::Gemini => "gemini-1.5-pro",
            LLMProvider::OpenAI => "gpt-4o",
            LLMProvider::OpenRouter => "openai/gpt-4o",
            LLMProvider::Mistral => "mistral-large-latest",
            LLMProvider::Groq => "llama-3.3-70b-versatile",
            LLMProvider::Together => "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo",
            LLMProvider::Cohere => "command-r-plus",
            LLMProvider::DeepSeek => "deepseek-chat",
        }
    }
}

// ============================================================================
// Settings Component
// ============================================================================

#[component]
pub fn Settings() -> impl IntoView {
    let navigate = use_navigate();

    // LLM Configuration Signals
    let selected_provider = RwSignal::new(LLMProvider::Ollama);
    let api_key_or_host = RwSignal::new("http://localhost:11434".to_string());
    let model_name = RwSignal::new("llama3.2".to_string());
    let embedding_model = RwSignal::new("nomic-embed-text".to_string());
    let save_status = RwSignal::new(String::new());
    let is_saving = RwSignal::new(false);
    let health_status = RwSignal::new(Option::<HealthStatus>::None);

    // Voice Configuration Signals
    let selected_voice_provider = RwSignal::new("Disabled".to_string());
    let voice_api_key_or_host = RwSignal::new(String::new());
    let voice_model_id = RwSignal::new(String::new());
    let selected_voice_id = RwSignal::new(String::new());
    let available_voices = RwSignal::new(Vec::<Voice>::new());
    let openai_tts_models = RwSignal::new(Vec::<(String, String)>::new());
    let is_loading_voices = RwSignal::new(false);

    // Meilisearch Signals
    let meili_status = RwSignal::new(Option::<MeilisearchStatus>::None);
    let is_reindexing = RwSignal::new(false);
    let reindex_status = RwSignal::new(String::new());

    // Model list signals
    let ollama_models = RwSignal::new(Vec::<OllamaModel>::new());
    let cloud_models = RwSignal::new(Vec::<ModelInfo>::new());
    let is_loading_models = RwSignal::new(false);

    // Claude Code CLI status
    let claude_code_status = RwSignal::new(ClaudeCodeStatus::default());
    let is_claude_code_logging_in = RwSignal::new(false);

    // Theme state
    let theme_state = expect_context::<ThemeState>();
    let theme_value = RwSignal::new(
        theme_state.current_preset.get().unwrap_or_else(|| "fantasy".to_string())
    );
    let is_custom_theme = RwSignal::new(theme_state.current_preset.get().is_none());

    // Individual weight signals (synced with global state initial value)
    let weights = theme_state.weights.get();
    let weight_fantasy = RwSignal::new(weights.fantasy);
    let weight_cosmic = RwSignal::new(weights.cosmic);
    let weight_terminal = RwSignal::new(weights.terminal);
    let weight_noir = RwSignal::new(weights.noir);
    let weight_neon = RwSignal::new(weights.neon);

    // Sync weight signals when global weights change (e.g. from preset selection)
    Effect::new(move |_| {
        let w = theme_state.weights.get();
        weight_fantasy.set(w.fantasy);
        weight_cosmic.set(w.cosmic);
        weight_terminal.set(w.terminal);
        weight_noir.set(w.noir);
        weight_neon.set(w.neon);
    });

    // Helper to update custom weights
    let update_weights = move || {
        let w = ThemeWeights {
            fantasy: weight_fantasy.get(),
            cosmic: weight_cosmic.get(),
            terminal: weight_terminal.get(),
            noir: weight_noir.get(),
            neon: weight_neon.get(),
        };
        theme_state.set_weights(w);
        is_custom_theme.set(true);
        theme_value.set("custom".to_string());
    };

    // Provider select signal for the Select component
    let provider_select_value = RwSignal::new("Ollama".to_string());

    // Load existing config on mount
    Effect::new(move |_| {
        spawn_local(async move {
            // Load LLM config
            if let Ok(Some(config)) = get_llm_config().await {
                let provider = LLMProvider::from_string(&config.provider);
                selected_provider.set(provider.clone());
                provider_select_value.set(match provider {
                    LLMProvider::Ollama => "Ollama".to_string(),
                    LLMProvider::Claude => "Claude".to_string(),
                    LLMProvider::ClaudeCode => "ClaudeCode".to_string(),
                    LLMProvider::Gemini => "Gemini".to_string(),
                    LLMProvider::OpenAI => "OpenAI".to_string(),
                    LLMProvider::OpenRouter => "OpenRouter".to_string(),
                    LLMProvider::Mistral => "Mistral".to_string(),
                    LLMProvider::Groq => "Groq".to_string(),
                    LLMProvider::Together => "Together".to_string(),
                    LLMProvider::Cohere => "Cohere".to_string(),
                    LLMProvider::DeepSeek => "DeepSeek".to_string(),
                });

                match provider {
                    LLMProvider::Ollama => {
                        let host = config
                            .host
                            .clone()
                            .unwrap_or_else(|| "http://localhost:11434".to_string());
                        api_key_or_host.set(host.clone());
                        if let Ok(models) = list_ollama_models(host).await {
                            ollama_models.set(models);
                        }
                    }
                    LLMProvider::Claude => {
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_claude_models(None).await {
                            cloud_models.set(models);
                        }
                    }
                    LLMProvider::Gemini => {
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_gemini_models(None).await {
                            cloud_models.set(models);
                        }
                    }
                    LLMProvider::OpenAI => {
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_openai_models(None).await {
                            cloud_models.set(models);
                        }
                    }
                    LLMProvider::OpenRouter => {
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_openrouter_models().await {
                            cloud_models.set(models);
                        }
                    }
                    LLMProvider::Mistral
                    | LLMProvider::Groq
                    | LLMProvider::Together
                    | LLMProvider::Cohere
                    | LLMProvider::DeepSeek => {
                        api_key_or_host.set(String::new());
                        if let Ok(models) = list_provider_models(provider.to_string_key()).await {
                            cloud_models.set(models);
                        }
                    }
                    LLMProvider::ClaudeCode => {
                        // No API key needed for Claude Code
                        api_key_or_host.set(String::new());
                    }
                }
                model_name.set(config.model);
                if let Some(emb) = config.embedding_model {
                    embedding_model.set(emb);
                }
            } else {
                // Default: fetch Ollama models
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
                            if !c.api_key.is_empty() && !c.api_key.starts_with('*') {
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

            // Check LLM health
            if let Ok(status) = check_llm_health().await {
                health_status.set(Some(status));
            }

            // Check Claude Code CLI status
            if let Ok(status) = get_claude_code_status().await {
                claude_code_status.set(status);
            }

            // Check Meilisearch health
            if let Ok(status) = check_meilisearch_health().await {
                meili_status.set(Some(status));
            }
        });
    });

    // Helper function to fetch Ollama models
    let fetch_ollama_models = move |host: String| {
        spawn_local(async move {
            is_loading_models.set(true);
            match list_ollama_models(host).await {
                Ok(models) => ollama_models.set(models),
                Err(_) => ollama_models.set(Vec::new()),
            }
            is_loading_models.set(false);
        });
    };

    // Helper function to fetch cloud provider models
    let fetch_cloud_models = move |provider: LLMProvider, api_key: Option<String>| {
        spawn_local(async move {
            is_loading_models.set(true);
            let models = match provider {
                LLMProvider::Claude => list_claude_models(api_key).await.unwrap_or_default(),
                LLMProvider::OpenAI => list_openai_models(api_key).await.unwrap_or_default(),
                LLMProvider::Gemini => list_gemini_models(api_key).await.unwrap_or_default(),
                LLMProvider::OpenRouter => list_openrouter_models().await.unwrap_or_default(),
                LLMProvider::Mistral
                | LLMProvider::Groq
                | LLMProvider::Together
                | LLMProvider::Cohere
                | LLMProvider::DeepSeek => {
                    list_provider_models(provider.to_string_key())
                        .await
                        .unwrap_or_default()
                }
                _ => Vec::new(),
            };
            cloud_models.set(models);
            is_loading_models.set(false);
        });
    };

    // Helper function to fetch voices
    let fetch_voices = move |provider: String, api_key: Option<String>| {
        spawn_local(async move {
            is_loading_voices.set(true);
            match provider.as_str() {
                "OpenAI" => {
                    if let Ok(voices) = list_openai_voices().await {
                        available_voices.set(voices);
                    }
                    if let Ok(models) = list_openai_tts_models().await {
                        openai_tts_models.set(models);
                    }
                }
                "ElevenLabs" => {
                    if let Some(key) = api_key {
                        if !key.is_empty() && !key.starts_with('*') {
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

    // Test connection handler
    let test_connection = move |_: ev::MouseEvent| {
        spawn_local(async move {
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

    // Save settings handler
    let save_settings = move |_: ev::MouseEvent| {
        is_saving.set(true);
        save_status.set("Saving...".to_string());

        let provider = selected_provider.get().to_string_key();
        let api_key_or_host_val = api_key_or_host.get();
        let model = model_name.get();
        let emb_model = embedding_model.get();

        spawn_local(async move {
            let settings = match provider.as_str() {
                "ollama" => LLMSettings {
                    provider: "ollama".to_string(),
                    api_key: None,
                    host: Some(api_key_or_host_val),
                    model,
                    embedding_model: Some(emb_model),
                },
                "claude" => {
                    if !api_key_or_host_val.is_empty() {
                        if let Err(e) =
                            save_api_key("claude".to_string(), api_key_or_host_val.clone()).await
                        {
                            save_status.set(format!("Failed to save API key: {}", e));
                            is_saving.set(false);
                            return;
                        }
                    }
                    LLMSettings {
                        provider: "claude".to_string(),
                        api_key: if api_key_or_host_val.is_empty() {
                            None
                        } else {
                            Some(api_key_or_host_val)
                        },
                        host: None,
                        model,
                        embedding_model: None,
                    }
                }
                "gemini" => {
                    if !api_key_or_host_val.is_empty() {
                        if let Err(e) =
                            save_api_key("gemini".to_string(), api_key_or_host_val.clone()).await
                        {
                            save_status.set(format!("Failed to save API key: {}", e));
                            is_saving.set(false);
                            return;
                        }
                    }
                    LLMSettings {
                        provider: "gemini".to_string(),
                        api_key: if api_key_or_host_val.is_empty() {
                            None
                        } else {
                            Some(api_key_or_host_val)
                        },
                        host: None,
                        model,
                        embedding_model: None,
                    }
                }
                "openai" => {
                    if !api_key_or_host_val.is_empty() {
                        if let Err(e) =
                            save_api_key("openai".to_string(), api_key_or_host_val.clone()).await
                        {
                            save_status.set(format!("Failed to save API key: {}", e));
                            is_saving.set(false);
                            return;
                        }
                    }
                    LLMSettings {
                        provider: "openai".to_string(),
                        api_key: if api_key_or_host_val.is_empty() {
                            None
                        } else {
                            Some(api_key_or_host_val)
                        },
                        host: None,
                        model,
                        embedding_model: None,
                    }
                }
                "openrouter" => {
                    if !api_key_or_host_val.is_empty() {
                        if let Err(e) =
                            save_api_key("openrouter".to_string(), api_key_or_host_val.clone()).await
                        {
                            save_status.set(format!("Failed to save API key: {}", e));
                            is_saving.set(false);
                            return;
                        }
                    }
                    LLMSettings {
                        provider: "openrouter".to_string(),
                        api_key: if api_key_or_host_val.is_empty() {
                            None
                        } else {
                            Some(api_key_or_host_val)
                        },
                        host: None,
                        model,
                        embedding_model: None,
                    }
                }
                other => {
                    // Handle remaining providers (mistral, groq, together, cohere, deepseek)
                    if !api_key_or_host_val.is_empty() {
                        if let Err(e) =
                            save_api_key(other.to_string(), api_key_or_host_val.clone()).await
                        {
                            save_status.set(format!("Failed to save API key: {}", e));
                            is_saving.set(false);
                            return;
                        }
                    }
                    LLMSettings {
                        provider: other.to_string(),
                        api_key: if api_key_or_host_val.is_empty() {
                            None
                        } else {
                            Some(api_key_or_host_val)
                        },
                        host: None,
                        model,
                        embedding_model: None,
                    }
                }
            };

            match configure_llm(settings).await {
                Ok(msg) => {
                    save_status.set(msg);
                    if let Ok(status) = check_llm_health().await {
                        health_status.set(Some(status));
                    }
                }
                Err(e) => {
                    save_status.set(format!("Error: {}", e));
                    is_saving.set(false);
                    return;
                }
            }

            // Save Voice Settings
            let voice_prov = selected_voice_provider.get();
            let voice_val = voice_api_key_or_host.get();
            let voice_mod = voice_model_id.get();
            let voice_id = selected_voice_id.get();

            let voice_config = if voice_prov == "Disabled" {
                VoiceConfig {
                    provider: "Disabled".to_string(),
                    cache_dir: None,
                    default_voice_id: None,
                    elevenlabs: None,
                    fish_audio: None,
                    ollama: None,
                    openai: None,
                }
            } else {
                let mut base = VoiceConfig {
                    provider: voice_prov.clone(),
                    cache_dir: None,
                    default_voice_id: None,
                    elevenlabs: None,
                    fish_audio: None,
                    ollama: None,
                    openai: None,
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

    // Handle provider change
    let on_provider_change = move |val: String| {
        let provider = LLMProvider::from_string(&val);
        selected_provider.set(provider.clone());

        // Reset defaults based on provider
        match provider.clone() {
            LLMProvider::Ollama => {
                api_key_or_host.set("http://localhost:11434".to_string());
                model_name.set("llama3.2".to_string());
                fetch_ollama_models("http://localhost:11434".to_string());
            }
            LLMProvider::ClaudeCode => {
                // No API key needed, just set default model
                api_key_or_host.set(String::new());
                model_name.set(provider.default_model().to_string());
            }
            _ => {
                api_key_or_host.set(String::new());
                model_name.set(provider.default_model().to_string());
                fetch_cloud_models(provider, None);
            }
        }
    };

    // Handle Claude Code login
    let on_claude_code_login = move |_: ev::MouseEvent| {
        is_claude_code_logging_in.set(true);
        spawn_local(async move {
            match claude_code_login().await {
                Ok(()) => {
                    // Refresh status after login
                    if let Ok(status) = get_claude_code_status().await {
                        claude_code_status.set(status);
                    }
                }
                Err(e) => {
                    // Update status to show error
                    let mut status = claude_code_status.get();
                    status.error = Some(format!("Login failed: {}", e));
                    claude_code_status.set(status);
                }
            }
            is_claude_code_logging_in.set(false);
        });
    };

    // Handle voice provider change
    let on_voice_provider_change = move |val: String| {
        selected_voice_provider.set(val.clone());
        available_voices.set(Vec::new());

        match val.as_str() {
            "Ollama" => {
                voice_api_key_or_host.set("http://localhost:11434".to_string());
                voice_model_id.set("bark".to_string());
            }
            "ElevenLabs" => {
                voice_api_key_or_host.set(String::new());
                voice_model_id.set("eleven_multilingual_v2".to_string());
            }
            "OpenAI" => {
                voice_api_key_or_host.set(String::new());
                voice_model_id.set("tts-1".to_string());
                selected_voice_id.set("alloy".to_string());
                fetch_voices("OpenAI".to_string(), None);
            }
            _ => {}
        }
    };

    // Handle theme change
    let on_theme_change = move |val: String| {
        if val == "custom" {
            is_custom_theme.set(true);
            theme_value.set("custom".to_string());
        } else {
            theme_value.set(val.clone());
            theme_state.set_preset(&val);
            is_custom_theme.set(false);
        }
    };

    // Handle reindex
    let handle_reindex = move |_: ev::MouseEvent| {
        is_reindexing.set(true);
        reindex_status.set("Re-indexing...".to_string());
        spawn_local(async move {
            match reindex_library(None).await {
                Ok(msg) => {
                    reindex_status.set(msg);
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
    };

    // Back navigation
    let handle_back = {
        let navigate = navigate.clone();
        move |_: ev::MouseEvent| {
            navigate("/", Default::default());
        }
    };

    // Derived signals for UI
    let show_embedding_model =
        move || matches!(selected_provider.get(), LLMProvider::Ollama);
    let is_ollama = move || matches!(selected_provider.get(), LLMProvider::Ollama);
    let placeholder_text = move || selected_provider.get().placeholder_text();
    let label_text = move || selected_provider.get().label_text();
    let input_type = move || {
        if matches!(selected_provider.get(), LLMProvider::Ollama | LLMProvider::ClaudeCode) {
            "text"
        } else {
            "password"
        }
    };
    let is_claude_code = move || matches!(selected_provider.get(), LLMProvider::ClaudeCode);

    view! {
        <div class="p-8 bg-theme-primary text-theme-primary min-h-screen font-sans transition-colors duration-300">
            <div class="max-w-2xl mx-auto space-y-6">
                // Header with back button and health status
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-4">
                        <button
                            class="text-gray-400 hover:text-white transition-colors"
                            on:click=handle_back
                        >
                            "< Back"
                        </button>
                        <h1 class="text-2xl font-bold">"Settings"</h1>
                    </div>
                    {move || {
                        health_status.get().map(|status| {
                            let variant = if status.healthy {
                                BadgeVariant::Success
                            } else {
                                BadgeVariant::Danger
                            };
                            view! {
                                <Badge variant=variant>
                                    {status.message.clone()}
                                </Badge>
                            }
                        })
                    }}
                </div>

                // LLM Configuration Card
                <Card>
                    <CardHeader>
                        <h2 class="text-lg font-semibold">"LLM Configuration"</h2>
                    </CardHeader>
                    <CardBody class="space-y-4">
                        // Provider Selection
                        <div>
                            <label class="block text-sm font-medium text-theme-secondary mb-1">
                                "Provider"
                            </label>
                            <Select
                                value=provider_select_value.get()
                                on_change=Callback::new(on_provider_change)
                            >
                                <option value="Ollama" selected=move || matches!(selected_provider.get(), LLMProvider::Ollama)>
                                    "Ollama (Local)"
                                </option>
                                <option value="OpenRouter" selected=move || matches!(selected_provider.get(), LLMProvider::OpenRouter)>
                                    "OpenRouter (400+ models)"
                                </option>
                                <option value="Claude" selected=move || matches!(selected_provider.get(), LLMProvider::Claude)>
                                    "Claude (Anthropic)"
                                </option>
                                <option value="ClaudeCode" selected=move || matches!(selected_provider.get(), LLMProvider::ClaudeCode)>
                                    "Claude Code (CLI)"
                                </option>
                                <option value="OpenAI" selected=move || matches!(selected_provider.get(), LLMProvider::OpenAI)>
                                    "OpenAI"
                                </option>
                                <option value="Gemini" selected=move || matches!(selected_provider.get(), LLMProvider::Gemini)>
                                    "Gemini (Google)"
                                </option>
                                <option value="Mistral" selected=move || matches!(selected_provider.get(), LLMProvider::Mistral)>
                                    "Mistral AI"
                                </option>
                                <option value="Groq" selected=move || matches!(selected_provider.get(), LLMProvider::Groq)>
                                    "Groq (Fast)"
                                </option>
                                <option value="Together" selected=move || matches!(selected_provider.get(), LLMProvider::Together)>
                                    "Together AI"
                                </option>
                                <option value="Cohere" selected=move || matches!(selected_provider.get(), LLMProvider::Cohere)>
                                    "Cohere"
                                </option>
                                <option value="DeepSeek" selected=move || matches!(selected_provider.get(), LLMProvider::DeepSeek)>
                                    "DeepSeek"
                                </option>
                            </Select>
                        </div>

                        // Claude Code Status (shown when ClaudeCode selected)
                        <Show when=move || is_claude_code()>
                            <div class="p-3 rounded-lg bg-theme-secondary/20 border border-theme-border">
                                {move || {
                                    let status = claude_code_status.get();
                                    if !status.installed {
                                        view! {
                                            <div class="space-y-2">
                                                <div class="flex items-center gap-2 text-amber-500">
                                                    <span class="text-lg">"!"</span>
                                                    <span class="font-medium">"Claude Code CLI not installed"</span>
                                                </div>
                                                <p class="text-sm text-theme-secondary">
                                                    "Install with: "
                                                    <code class="bg-theme-secondary/30 px-1 rounded">"npm install -g @anthropic-ai/claude-code"</code>
                                                </p>
                                            </div>
                                        }.into_any()
                                    } else if !status.logged_in {
                                        view! {
                                            <div class="space-y-3">
                                                <div class="flex items-center gap-2 text-amber-500">
                                                    <span class="text-lg">"!"</span>
                                                    <span class="font-medium">"Not logged in to Claude Code"</span>
                                                </div>
                                                {status.version.map(|v| view! {
                                                    <p class="text-xs text-theme-secondary">"Version: " {v}</p>
                                                })}
                                                <Button
                                                    variant=ButtonVariant::Primary
                                                    on_click=on_claude_code_login
                                                    disabled=is_claude_code_logging_in.get()
                                                >
                                                    {move || if is_claude_code_logging_in.get() {
                                                        "Logging in..."
                                                    } else {
                                                        "Login with Claude Code"
                                                    }}
                                                </Button>
                                                {status.error.map(|e| view! {
                                                    <p class="text-xs text-red-400">{e}</p>
                                                })}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="space-y-1">
                                                <div class="flex items-center gap-2 text-green-500">
                                                    <span class="text-lg">"*"</span>
                                                    <span class="font-medium">"Connected to Claude Code"</span>
                                                </div>
                                                {status.user_email.map(|email| view! {
                                                    <p class="text-sm text-theme-secondary">"Logged in as: " {email}</p>
                                                })}
                                                {status.version.map(|v| view! {
                                                    <p class="text-xs text-theme-secondary">"Version: " {v}</p>
                                                })}
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        </Show>

                        // API Key / Host (hidden for Claude Code)
                        <Show when=move || !is_claude_code()>
                            <div>
                                <label class="block text-sm font-medium text-theme-secondary mb-1">
                                    {label_text}
                                </label>
                                <Input
                                    value=api_key_or_host
                                    placeholder=placeholder_text()
                                    r#type=input_type()
                                />
                            </div>
                        </Show>

                        // Model Selection
                        <div>
                            <label class="block text-sm font-medium text-theme-secondary mb-1">
                                "Model"
                            </label>
                            <select
                                class="w-full p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none"
                                prop:value=move || model_name.get()
                                on:change=move |e| model_name.set(event_target_value(&e))
                            >
                                {move || {
                                    if is_ollama() {
                                        let models = ollama_models.get();
                                        if models.is_empty() {
                                            view! {
                                                <option value=model_name.get()>
                                                    {model_name.get()}
                                                </option>
                                            }.into_any()
                                        } else {
                                            models.iter().map(|m| {
                                                let name = m.name.clone();
                                                let display = if let Some(ref size) = m.size {
                                                    format!("{} ({})", m.name, size)
                                                } else {
                                                    m.name.clone()
                                                };
                                                view! {
                                                    <option
                                                        value=name.clone()
                                                        selected=move || model_name.get() == name
                                                    >
                                                        {display}
                                                    </option>
                                                }
                                            }).collect_view().into_any()
                                        }
                                    } else {
                                        let models = cloud_models.get();
                                        if models.is_empty() {
                                            view! {
                                                <option value=model_name.get()>
                                                    {model_name.get()}
                                                </option>
                                            }.into_any()
                                        } else {
                                            models.iter().map(|m| {
                                                let id = m.id.clone();
                                                let display = if let Some(ref desc) = m.description {
                                                    format!("{} - {}", m.name, desc)
                                                } else {
                                                    m.name.clone()
                                                };
                                                view! {
                                                    <option
                                                        value=id.clone()
                                                        selected=move || model_name.get() == id
                                                    >
                                                        {display}
                                                    </option>
                                                }
                                            }).collect_view().into_any()
                                        }
                                    }
                                }}
                            </select>
                        </div>

                        // Embedding Model (Ollama only)
                        <Show when=show_embedding_model>
                            <div>
                                <label class="block text-sm font-medium text-theme-secondary mb-1">
                                    "Embedding Model"
                                </label>
                                <Input
                                    value=embedding_model
                                    placeholder="nomic-embed-text".to_string()
                                />
                            </div>
                        </Show>
                    </CardBody>
                </Card>

                // Audio Configuration Card
                <Card>
                    <CardHeader>
                        <h2 class="text-lg font-semibold">"Audio Configuration"</h2>
                    </CardHeader>
                    <CardBody class="space-y-4">
                        // Voice Provider Selection
                        <div>
                            <label class="block text-sm font-medium text-theme-secondary mb-1">
                                "Voice Provider"
                            </label>
                            <Select
                                value=selected_voice_provider.get()
                                on_change=Callback::new(on_voice_provider_change)
                            >
                                <option value="Disabled">"Disabled"</option>
                                <option value="OpenAI">"OpenAI TTS"</option>
                                <option value="ElevenLabs">"ElevenLabs"</option>
                                <option value="Ollama">"Ollama (Local)"</option>
                            </Select>
                        </div>

                        // Voice provider specific options
                        <Show when=move || selected_voice_provider.get() != "Disabled">
                            // API Key / Base URL
                            <div>
                                <label class="block text-sm font-medium text-theme-secondary mb-1">
                                    {move || {
                                        if selected_voice_provider.get() == "Ollama" {
                                            "Base URL"
                                        } else {
                                            "API Key"
                                        }
                                    }}
                                </label>
                                <Input
                                    value=voice_api_key_or_host
                                    placeholder="API Key or Base URL"
                                    r#type="password"
                                />
                            </div>

                            // OpenAI specific options
                            <Show when=move || selected_voice_provider.get() == "OpenAI">
                                // TTS Model selection
                                <div>
                                    <label class="block text-sm font-medium text-theme-secondary mb-1">
                                        "TTS Model"
                                    </label>
                                    <select
                                        class="w-full p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none"
                                        prop:value=move || voice_model_id.get()
                                        on:change=move |e| voice_model_id.set(event_target_value(&e))
                                    >
                                        {move || {
                                            let models = openai_tts_models.get();
                                            if models.is_empty() {
                                                view! {
                                                    <>
                                                        <option value="tts-1">"TTS-1 (Fast)"</option>
                                                        <option value="tts-1-hd">"TTS-1 HD (High Quality)"</option>
                                                    </>
                                                }.into_any()
                                            } else {
                                                models.iter().map(|(id, name)| {
                                                    let id_clone = id.clone();
                                                    view! {
                                                        <option
                                                            value=id.clone()
                                                            selected=move || voice_model_id.get() == id_clone
                                                        >
                                                            {name.clone()}
                                                        </option>
                                                    }
                                                }).collect_view().into_any()
                                            }
                                        }}
                                    </select>
                                </div>

                                // Voice selection
                                <div>
                                    <label class="block text-sm font-medium text-theme-secondary mb-1">
                                        "Voice"
                                    </label>
                                    <select
                                        class="w-full p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none"
                                        prop:value=move || selected_voice_id.get()
                                        on:change=move |e| selected_voice_id.set(event_target_value(&e))
                                    >
                                        {move || {
                                            let voices = available_voices.get();
                                            if voices.is_empty() {
                                                view! {
                                                    <>
                                                        <option value="alloy">"Alloy - Neutral and balanced"</option>
                                                        <option value="echo">"Echo - Warm and clear"</option>
                                                        <option value="fable">"Fable - British accent"</option>
                                                        <option value="onyx">"Onyx - Deep and authoritative"</option>
                                                        <option value="nova">"Nova - Friendly and upbeat"</option>
                                                        <option value="shimmer">"Shimmer - Warm and pleasant"</option>
                                                    </>
                                                }.into_any()
                                            } else {
                                                voices.iter().map(|v| {
                                                    let id = v.id.clone();
                                                    let display = if let Some(ref desc) = v.description {
                                                        format!("{} - {}", v.name, desc)
                                                    } else {
                                                        v.name.clone()
                                                    };
                                                    view! {
                                                        <option
                                                            value=id.clone()
                                                            selected=move || selected_voice_id.get() == id
                                                        >
                                                            {display}
                                                        </option>
                                                    }
                                                }).collect_view().into_any()
                                            }
                                        }}
                                    </select>
                                </div>
                            </Show>

                            // ElevenLabs specific options
                            <Show when=move || selected_voice_provider.get() == "ElevenLabs">
                                <div>
                                    <label class="block text-sm font-medium text-theme-secondary mb-1">
                                        "Model ID"
                                    </label>
                                    <Input
                                        value=voice_model_id
                                        placeholder="eleven_multilingual_v2".to_string()
                                    />
                                </div>

                                // Voice selection if voices loaded
                                <Show when=move || !available_voices.get().is_empty()>
                                    <div>
                                        <label class="block text-sm font-medium text-theme-secondary mb-1">
                                            "Voice"
                                        </label>
                                        <select
                                            class="w-full p-2 rounded bg-gray-700 text-white border border-gray-600 focus:border-purple-500 outline-none"
                                            prop:value=move || selected_voice_id.get()
                                            on:change=move |e| selected_voice_id.set(event_target_value(&e))
                                        >
                                            {move || {
                                                available_voices.get().iter().map(|v| {
                                                    let id = v.id.clone();
                                                    view! {
                                                        <option
                                                            value=id.clone()
                                                            selected=move || selected_voice_id.get() == id
                                                        >
                                                            {v.name.clone()}
                                                        </option>
                                                    }
                                                }).collect_view()
                                            }}
                                        </select>
                                    </div>
                                </Show>
                            </Show>

                            // Ollama specific options
                            <Show when=move || selected_voice_provider.get() == "Ollama">
                                <div>
                                    <label class="block text-sm font-medium text-theme-secondary mb-1">
                                        "Voice Model"
                                    </label>
                                    <Input
                                        value=voice_model_id
                                        placeholder="bark".to_string()
                                    />
                                </div>
                            </Show>
                        </Show>
                    </CardBody>
                </Card>

                // Audio Cache Statistics Card (TASK-005)
                <AudioCacheCard />

                // Appearance Card - Enhanced with Theme Editor
                <Card>
                    <CardHeader>
                        <h2 class="text-lg font-semibold">"Appearance"</h2>
                    </CardHeader>
                    <CardBody class="space-y-4">
                        // Quick theme select
                        <div>
                            <label class="block text-sm font-medium text-theme-secondary mb-1">
                                "Theme Preset"
                            </label>
                            <Select
                                value=theme_value.get()
                                on_change=Callback::new(on_theme_change)
                            >
                                <option value="fantasy">"Fantasy (Default)"</option>
                                <option value="cosmic">"Cosmic Horror"</option>
                                <option value="terminal">"Terminal"</option>
                                <option value="noir">"Noir"</option>
                                <option value="neon">"Neon Cyberpunk"</option>
                                <option value="custom">"Custom Mix"</option>
                            </Select>
                            <p class="text-xs text-theme-secondary mt-1">
                                "Quick select a theme preset. For advanced blending, use the Theme Editor below."
                            </p>
                        </div>

                        // Advanced Theme Editor
                        <details class="group">
                            <summary class="cursor-pointer text-sm font-medium text-[var(--accent)] hover:underline list-none flex items-center gap-2">
                                <span class="group-open:rotate-90 transition-transform">">"</span>
                                "Advanced Theme Blending"
                            </summary>
                            <div class="mt-4 pt-4 border-t border-[var(--border-subtle)]">
                                <crate::components::settings_components::ThemeEditor />
                            </div>
                        </details>
                    </CardBody>
                </Card>

                // Search Engine (Meilisearch) Card
                <Card>
                    <CardHeader>
                        <div class="flex items-center justify-between w-full">
                            <h2 class="text-lg font-semibold">"Search Engine"</h2>
                            {move || {
                                meili_status.get().map(|status| {
                                    let variant = if status.healthy {
                                        BadgeVariant::Success
                                    } else {
                                        BadgeVariant::Danger
                                    };
                                    let text = if status.healthy { "Connected" } else { "Offline" };
                                    view! {
                                        <Badge variant=variant>
                                            {text}
                                        </Badge>
                                    }
                                })
                            }}
                        </div>
                    </CardHeader>
                    <CardBody class="space-y-4">
                        // Host info
                        {move || {
                            meili_status.get().map(|status| {
                                view! {
                                    <div class="text-sm text-theme-secondary">
                                        <span class="font-medium">"Host: "</span>
                                        <span>{status.host.clone()}</span>
                                    </div>
                                }
                            })
                        }}

                        // Document counts
                        {move || {
                            meili_status.get().and_then(|status| {
                                status.document_counts.map(|counts| {
                                    view! {
                                        <div class="space-y-2">
                                            <h3 class="text-sm font-medium text-theme-secondary">
                                                "Index Document Counts"
                                            </h3>
                                            <div class="grid grid-cols-2 gap-2 text-sm">
                                                {counts.iter().map(|(index, count)| {
                                                    view! {
                                                        <div class="flex justify-between px-2 py-1 bg-theme-secondary rounded">
                                                            <span class="font-mono">{index.clone()}</span>
                                                            <span class="text-theme-accent">{count.to_string()}</span>
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    }
                                })
                            })
                        }}

                        // Reindex status message
                        <Show when=move || !reindex_status.get().is_empty()>
                            <div class=move || {
                                if reindex_status.get().contains("Error") {
                                    "text-red-400 text-sm"
                                } else {
                                    "text-green-400 text-sm"
                                }
                            }>
                                {move || reindex_status.get()}
                            </div>
                        </Show>

                        // Reindex button
                        <div class="pt-2">
                            <Button
                                variant=ButtonVariant::Secondary
                                loading=is_reindexing.get()
                                on_click=handle_reindex
                            >
                                "Clear All Indexes"
                            </Button>
                        </div>
                    </CardBody>
                </Card>

                // Audit Logs Card
                <Card>
                    <CardHeader>
                        <h2 class="text-lg font-semibold">"Security & Audit Logs"</h2>
                    </CardHeader>
                    <CardBody class="space-y-4">
                        <p class="text-sm text-theme-secondary">
                            "View security audit logs and analytics dashboards."
                        </p>
                        <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                            // Usage Dashboard link
                            <a
                                href="/analytics/usage"
                                class="p-4 bg-theme-secondary rounded-lg hover:bg-theme-secondary/80 transition-colors cursor-pointer block"
                            >
                                <div class="font-medium text-theme-primary">"Usage & Costs"</div>
                                <div class="text-xs text-theme-secondary mt-1">
                                    "View token usage, costs by provider, and budget status"
                                </div>
                            </a>
                            // Search Analytics link
                            <a
                                href="/analytics/search"
                                class="p-4 bg-theme-secondary rounded-lg hover:bg-theme-secondary/80 transition-colors cursor-pointer block"
                            >
                                <div class="font-medium text-theme-primary">"Search Analytics"</div>
                                <div class="text-xs text-theme-secondary mt-1">
                                    "Popular queries, cache stats, and search performance"
                                </div>
                            </a>
                            // Audit Logs link
                            <a
                                href="/analytics/audit"
                                class="p-4 bg-theme-secondary rounded-lg hover:bg-theme-secondary/80 transition-colors cursor-pointer block"
                            >
                                <div class="font-medium text-theme-primary">"Audit Logs"</div>
                                <div class="text-xs text-theme-secondary mt-1">
                                    "Security events, API key usage, and configuration changes"
                                </div>
                            </a>
                        </div>
                    </CardBody>
                </Card>

                // Actions footer
                <div class="flex justify-end gap-4 pt-4 border-t border-gray-700">
                    <Show when=move || !save_status.get().is_empty()>
                        <span class=move || {
                            let status = save_status.get();
                            if status.contains("Error") || status.contains("Failed") {
                                "text-red-400 self-center mr-auto"
                            } else {
                                "text-green-400 self-center mr-auto"
                            }
                        }>
                            {move || save_status.get()}
                        </span>
                    </Show>

                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=test_connection
                    >
                        "Test Connection"
                    </Button>

                    <Button
                        variant=ButtonVariant::Primary
                        loading=is_saving.get()
                        on_click=save_settings
                    >
                        "Save Changes"
                    </Button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Audio Cache Card Component (TASK-005)
// ============================================================================

/// Audio Cache Statistics and Management Card
///
/// Displays cache statistics including:
/// - Hit/miss rate and counts
/// - Current and max cache size with visual progress bar
/// - Entry counts by format
/// - Actions to clear or prune the cache
#[component]
pub fn AudioCacheCard() -> impl IntoView {
    // Cache stats signal
    let cache_stats: RwSignal<Option<VoiceCacheStats>> = RwSignal::new(None);
    let cache_size: RwSignal<Option<AudioCacheSizeInfo>> = RwSignal::new(None);
    let is_loading = RwSignal::new(false);
    let is_clearing = RwSignal::new(false);
    let is_pruning = RwSignal::new(false);
    let action_status: RwSignal<Option<String>> = RwSignal::new(None);

    // Load cache stats on mount
    Effect::new(move || {
        spawn_local(async move {
            is_loading.set(true);
            if let Ok(stats) = get_audio_cache_stats().await {
                cache_stats.set(Some(stats));
            }
            if let Ok(size) = get_audio_cache_size().await {
                cache_size.set(Some(size));
            }
            is_loading.set(false);
        });
    });

    // Refresh cache stats
    let refresh_stats = move || {
        spawn_local(async move {
            is_loading.set(true);
            if let Ok(stats) = get_audio_cache_stats().await {
                cache_stats.set(Some(stats));
            }
            if let Ok(size) = get_audio_cache_size().await {
                cache_size.set(Some(size));
            }
            is_loading.set(false);
        });
    };

    // Clear all cache
    let clear_cache_action = move |_| {
        spawn_local(async move {
            is_clearing.set(true);
            action_status.set(None);
            match clear_audio_cache().await {
                Ok(()) => {
                    action_status.set(Some("Cache cleared successfully".to_string()));
                    // Refresh stats
                    if let Ok(stats) = get_audio_cache_stats().await {
                        cache_stats.set(Some(stats));
                    }
                    if let Ok(size) = get_audio_cache_size().await {
                        cache_size.set(Some(size));
                    }
                }
                Err(e) => {
                    action_status.set(Some(format!("Failed to clear cache: {}", e)));
                }
            }
            is_clearing.set(false);
        });
    };

    // Prune old entries (older than 7 days)
    let prune_cache_action = move |_| {
        spawn_local(async move {
            is_pruning.set(true);
            action_status.set(None);
            let seven_days_secs = 7 * 24 * 60 * 60;
            match prune_audio_cache(seven_days_secs).await {
                Ok(count) => {
                    action_status.set(Some(format!("Pruned {} old entries", count)));
                    // Refresh stats
                    if let Ok(stats) = get_audio_cache_stats().await {
                        cache_stats.set(Some(stats));
                    }
                    if let Ok(size) = get_audio_cache_size().await {
                        cache_size.set(Some(size));
                    }
                }
                Err(e) => {
                    action_status.set(Some(format!("Failed to prune cache: {}", e)));
                }
            }
            is_pruning.set(false);
        });
    };

    view! {
        <Card>
            <CardHeader>
                <div class="flex items-center justify-between w-full">
                    <h2 class="text-lg font-semibold">"Audio Cache"</h2>
                    <div class="flex items-center gap-2">
                        {move || {
                            cache_size.get().map(|size| {
                                let variant = if size.usage_percent > 90.0 {
                                    BadgeVariant::Danger
                                } else if size.usage_percent > 70.0 {
                                    BadgeVariant::Warning
                                } else {
                                    BadgeVariant::Success
                                };
                                view! {
                                    <Badge variant=variant>
                                        {format!("{:.1}% used", size.usage_percent)}
                                    </Badge>
                                }
                            })
                        }}
                    </div>
                </div>
            </CardHeader>
            <CardBody class="space-y-4">
                // Loading state
                <Show when=move || is_loading.get()>
                    <div class="flex items-center justify-center py-4">
                        <div class="animate-spin rounded-full h-6 w-6 border-b-2 border-[var(--accent)]"></div>
                        <span class="ml-2 text-theme-secondary">"Loading cache statistics..."</span>
                    </div>
                </Show>

                // Cache size progress bar
                <Show when=move || cache_size.get().is_some() && !is_loading.get()>
                    {move || {
                        cache_size.get().map(|size| {
                            view! {
                                <div class="space-y-2">
                                    <div class="flex justify-between text-sm">
                                        <span class="text-theme-secondary">"Storage Used"</span>
                                        <span class="font-mono">
                                            {format!("{} / {}", format_bytes(size.current_size_bytes), format_bytes(size.max_size_bytes))}
                                        </span>
                                    </div>
                                    <div class="w-full bg-[var(--bg-tertiary)] rounded-full h-2.5">
                                        <div
                                            class="h-2.5 rounded-full transition-all duration-300"
                                            style:width=format!("{}%", size.usage_percent.min(100.0))
                                            style:background-color={if size.usage_percent > 90.0 {
                                                "var(--error)"
                                            } else if size.usage_percent > 70.0 {
                                                "var(--warning)"
                                            } else {
                                                "var(--accent)"
                                            }}
                                        ></div>
                                    </div>
                                    <div class="text-xs text-theme-secondary">
                                        {format!("{} entries cached", size.entry_count)}
                                    </div>
                                </div>
                            }
                        })
                    }}
                </Show>

                // Cache statistics
                <Show when=move || cache_stats.get().is_some() && !is_loading.get()>
                    {move || {
                        cache_stats.get().map(|stats| {
                            view! {
                                <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                                    // Hit rate
                                    <div class="bg-[var(--bg-tertiary)] rounded-lg p-3">
                                        <div class="text-xs text-theme-secondary uppercase tracking-wide">"Hit Rate"</div>
                                        <div class="text-xl font-bold text-[var(--accent)]">
                                            {format!("{:.1}%", stats.hit_rate * 100.0)}
                                        </div>
                                    </div>

                                    // Hits
                                    <div class="bg-[var(--bg-tertiary)] rounded-lg p-3">
                                        <div class="text-xs text-theme-secondary uppercase tracking-wide">"Cache Hits"</div>
                                        <div class="text-xl font-bold text-green-400">
                                            {stats.hits.to_string()}
                                        </div>
                                    </div>

                                    // Misses
                                    <div class="bg-[var(--bg-tertiary)] rounded-lg p-3">
                                        <div class="text-xs text-theme-secondary uppercase tracking-wide">"Cache Misses"</div>
                                        <div class="text-xl font-bold text-yellow-400">
                                            {stats.misses.to_string()}
                                        </div>
                                    </div>

                                    // Evictions
                                    <div class="bg-[var(--bg-tertiary)] rounded-lg p-3">
                                        <div class="text-xs text-theme-secondary uppercase tracking-wide">"Evictions"</div>
                                        <div class="text-xl font-bold text-red-400">
                                            {stats.evictions.to_string()}
                                        </div>
                                    </div>
                                </div>

                                // Format breakdown
                                <Show when={
                                    let stats = stats.clone();
                                    move || !stats.entries_by_format.is_empty()
                                }>
                                    <div class="mt-4">
                                        <h3 class="text-sm font-medium text-theme-secondary mb-2">"Entries by Format"</h3>
                                        <div class="flex flex-wrap gap-2">
                                            {stats.entries_by_format.iter().map(|(format, count)| {
                                                view! {
                                                    <span class="px-2 py-1 bg-[var(--bg-tertiary)] rounded text-xs font-mono">
                                                        {format!("{}: {}", format.to_uppercase(), count)}
                                                    </span>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                </Show>

                                // Additional info
                                <div class="mt-4 text-xs text-theme-secondary space-y-1">
                                    <Show when=move || { stats.oldest_entry_age_secs > 0 }>
                                        <div>
                                            "Oldest entry: "
                                            {format_duration(stats.oldest_entry_age_secs)}
                                            " ago"
                                        </div>
                                    </Show>
                                    <Show when=move || { stats.avg_entry_size_bytes > 0 }>
                                        <div>
                                            "Average entry size: "
                                            {format_bytes(stats.avg_entry_size_bytes)}
                                        </div>
                                    </Show>
                                </div>
                            }
                        })
                    }}
                </Show>

                // Action status message
                <Show when=move || action_status.get().is_some()>
                    {move || {
                        action_status.get().map(|status| {
                            let is_error = status.contains("Failed");
                            view! {
                                <div class=move || {
                                    if is_error {
                                        "text-sm text-red-400 bg-red-400/10 rounded px-3 py-2"
                                    } else {
                                        "text-sm text-green-400 bg-green-400/10 rounded px-3 py-2"
                                    }
                                }>
                                    {status}
                                </div>
                            }
                        })
                    }}
                </Show>

                // Action buttons
                <div class="flex flex-wrap gap-2 pt-2">
                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=move |_| refresh_stats()
                        disabled=is_loading.get()
                    >
                        "Refresh"
                    </Button>

                    <Button
                        variant=ButtonVariant::Secondary
                        on_click=prune_cache_action
                        loading=is_pruning.get()
                        disabled=is_loading.get() || is_clearing.get()
                    >
                        "Prune Old (7+ days)"
                    </Button>

                    <Button
                        variant=ButtonVariant::Danger
                        on_click=clear_cache_action
                        loading=is_clearing.get()
                        disabled=is_loading.get() || is_pruning.get()
                    >
                        "Clear All"
                    </Button>
                </div>

                // Help text
                <p class="text-xs text-theme-secondary">
                    "Audio cache stores synthesized speech to avoid re-generating the same audio. "
                    "Pruning removes entries older than 7 days. Clearing removes all cached audio."
                </p>
            </CardBody>
        </Card>
    }
}

/// Format duration in seconds to human-readable string
fn format_duration(secs: i64) -> String {
    if secs < 60 {
        format!("{} seconds", secs)
    } else if secs < 3600 {
        format!("{} minutes", secs / 60)
    } else if secs < 86400 {
        format!("{:.1} hours", secs as f64 / 3600.0)
    } else {
        format!("{:.1} days", secs as f64 / 86400.0)
    }
}
