#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::bindings::{
    configure_llm, get_llm_config, save_api_key, check_llm_health, LLMSettings, HealthStatus,
    configure_voice, get_voice_config, VoiceConfig, ElevenLabsConfig, OllamaConfig,
    check_meilisearch_health, reindex_library, MeilisearchStatus,
    list_ollama_models, OllamaModel,
    list_claude_models, list_openai_models, list_gemini_models,
    list_openrouter_models, list_provider_models, ModelInfo
};
use crate::components::design_system::{Button, ButtonVariant, Input, Select, Card, CardHeader, CardBody, Badge, BadgeVariant};

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

    // Meilisearch Signals
    let mut meili_status = use_signal(|| Option::<MeilisearchStatus>::None);
    let mut is_reindexing = use_signal(|| false);
    let mut reindex_status = use_signal(|| String::new());

    // Ollama models list
    let mut ollama_models = use_signal(|| Vec::<OllamaModel>::new());
    // Cloud provider models list
    let mut cloud_models = use_signal(|| Vec::<ModelInfo>::new());
    let mut is_loading_models = use_signal(|| false);

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
                LLMProvider::Claude => list_claude_models(api_key).await.unwrap_or_default(),
                LLMProvider::OpenAI => list_openai_models(api_key).await.unwrap_or_default(),
                LLMProvider::Gemini => list_gemini_models(api_key).await.unwrap_or_default(),
                LLMProvider::OpenRouter => list_openrouter_models().await.unwrap_or_default(),
                LLMProvider::Mistral => list_provider_models("mistral".to_string()).await.unwrap_or_default(),
                LLMProvider::Groq => list_provider_models("groq".to_string()).await.unwrap_or_default(),
                LLMProvider::Together => list_provider_models("together".to_string()).await.unwrap_or_default(),
                LLMProvider::Cohere => list_provider_models("cohere".to_string()).await.unwrap_or_default(),
                LLMProvider::DeepSeek => list_provider_models("deepseek".to_string()).await.unwrap_or_default(),
                _ => Vec::new(),
            };
            cloud_models.set(models);
            is_loading_models.set(false);
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
                // Determine provider string from enum-like handling or explicit fields
                // Backend returns "Disabled", "ElevenLabs", "Ollama", etc in provider field (string) because we mapped it?
                // Wait, backend returns VoiceConfig struct where provider is VoiceProviderType.
                // In bindings.rs, VoiceConfig provider is String.
                // It seems serialization of enum uses variant name by default.

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
                            voice_api_key_or_host.set(c.api_key); // Might be masked
                            voice_model_id.set(c.model_id.unwrap_or_default());
                        }
                    }
                    "Ollama" => {
                        if let Some(c) = config.ollama {
                            voice_api_key_or_host.set(c.base_url);
                            voice_model_id.set(c.model);
                        }
                    }
                    // Handle others if needed
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

    // Consume theme context
    let mut theme_sig = use_context::<crate::ThemeSignal>();

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
                    // Add OpenAI/FishAudio implementations later if needed
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
                    CardHeader { h2 { class: "text-lg font-semibold", "Audio Configuration" } }
                    CardBody {
                        class: "space-y-4",
                         div {
                            label { class: "block text-sm font-medium text-theme-secondary mb-1", "Voice Provider" }
                            Select {
                                value: selected_voice_provider.read().clone(),
                                onchange: move |val: String| {
                                    selected_voice_provider.set(val.clone());
                                    // Reset fields based on provider defaults if needed
                                    if val == "Ollama" {
                                        voice_api_key_or_host.set("http://localhost:11434".to_string());
                                        voice_model_id.set("bark".to_string());
                                    } else if val == "ElevenLabs" {
                                        voice_api_key_or_host.set(String::new());
                                        voice_model_id.set("eleven_multilingual_v2".to_string());
                                    }
                                },
                                option { value: "Disabled", "Disabled" }
                                option { value: "ElevenLabs", "ElevenLabs" }
                                option { value: "Ollama", "Ollama (Local)" }
                            }
                        }

                        if *selected_voice_provider.read() != "Disabled" {
                            div {
                                label { class: "block text-sm font-medium text-theme-secondary mb-1",
                                    if *selected_voice_provider.read() == "Ollama" { "Base URL" } else { "API Key" }
                                }
                                Input {
                                    r#type: if *selected_voice_provider.read() == "Ollama" { "text" } else { "password" },
                                    placeholder: if *selected_voice_provider.read() == "Ollama" { "http://localhost:11434" } else { "sk-..." },
                                    value: "{voice_api_key_or_host}",
                                    oninput: move |val| voice_api_key_or_host.set(val)
                                }
                            }

                             div {
                                label { class: "block text-sm font-medium text-theme-secondary mb-1", "Model ID / Voice Model" }
                                Input {
                                    placeholder: "e.g. eleven_multilingual_v2",
                                    value: "{voice_model_id}",
                                    oninput: move |val| voice_model_id.set(val)
                                }
                            }
                        }
                    }
                }

                Card {
                    CardHeader { h2 { class: "text-lg font-semibold", "Appearance" } }
                    CardBody {
                        div {
                            label { class: "block text-sm font-medium text-theme-secondary mb-1", "Theme" }
                            Select {
                                value: "{theme_sig}",
                                onchange: move |val| theme_sig.set(val),
                                option { value: "fantasy", "Fantasy (Default)" }
                                option { value: "scifi", "Sci-Fi" }
                                option { value: "horror", "Horror" }
                                option { value: "cyberpunk", "Cyberpunk" }
                            }
                        }
                    }
                }

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
