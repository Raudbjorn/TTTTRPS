use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;
use std::collections::HashMap;
use gloo_timers::callback::Timeout;

/// GitHub URL for the Sidecar DM Gemini extension
#[allow(dead_code)]
const SIDECAR_DM_EXTENSION_URL: &str = "https://github.com/Raudbjorn/sidecar-dm-gemini-extension";
use crate::bindings::{
    check_llm_health, configure_llm, get_llm_config, list_anthropic_models, list_gemini_models,
    list_ollama_models, list_openai_models, list_openrouter_models, list_provider_models,
    save_api_key, HealthStatus, LLMSettings, ModelInfo, OllamaModel,
    // Claude OAuth
    claude_get_status, claude_list_models, ClaudeStatus,
    // Gemini OAuth
    gemini_list_models, GeminiStatus,
    // Copilot OAuth
    check_copilot_auth, get_copilot_models, CopilotAuthStatus,
    // Embedding configuration
    list_ollama_embedding_models, setup_ollama_embeddings, OllamaEmbeddingModel,
    list_local_embedding_models, setup_local_embeddings, LocalEmbeddingModel,
    setup_copilot_embeddings,
};
use crate::components::design_system::{Badge, BadgeVariant, Button, ButtonVariant, Card, Input};
use crate::services::notification_service::{show_error, show_success};
use super::{ClaudeAuth, CopilotAuth, GeminiAuth};

#[derive(Clone, PartialEq, Debug)]
pub enum LLMProvider {
    Ollama,
    AnthropicAPI,
    Google,      // Google AI Studio API key
    Gemini,  // Gemini via OAuth (Google Cloud Code)
    OpenAI,
    OpenRouter,
    Mistral,
    Groq,
    Together,
    Cohere,
    DeepSeek,
    Claude,
    Copilot,
}

impl std::fmt::Display for LLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProvider::Ollama => write!(f, "Ollama"),
            LLMProvider::AnthropicAPI => write!(f, "Anthropic API"),
            LLMProvider::Google => write!(f, "Google AI"),
            LLMProvider::Gemini => write!(f, "Gemini"),
            LLMProvider::OpenAI => write!(f, "OpenAI"),
            LLMProvider::OpenRouter => write!(f, "OpenRouter"),
            LLMProvider::Mistral => write!(f, "Mistral"),
            LLMProvider::Groq => write!(f, "Groq"),
            LLMProvider::Together => write!(f, "Together"),
            LLMProvider::Cohere => write!(f, "Cohere"),
            LLMProvider::DeepSeek => write!(f, "DeepSeek"),
            LLMProvider::Claude => write!(f, "Claude"),
            LLMProvider::Copilot => write!(f, "Copilot"),
        }
    }
}

impl LLMProvider {
    fn to_string_key(&self) -> String {
        match self {
            LLMProvider::Ollama => "ollama".to_string(),
            LLMProvider::AnthropicAPI => "anthropic".to_string(),
            LLMProvider::Google => "google".to_string(),
            LLMProvider::Gemini => "gemini".to_string(), // Uses gemini backend
            LLMProvider::OpenAI => "openai".to_string(),
            LLMProvider::OpenRouter => "openrouter".to_string(),
            LLMProvider::Mistral => "mistral".to_string(),
            LLMProvider::Groq => "groq".to_string(),
            LLMProvider::Together => "together".to_string(),
            LLMProvider::Cohere => "cohere".to_string(),
            LLMProvider::DeepSeek => "deepseek".to_string(),
            LLMProvider::Claude => "claude".to_string(),
            LLMProvider::Copilot => "copilot".to_string(),
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "Anthropic API" | "anthropic" => LLMProvider::AnthropicAPI,
            "Google AI" | "google" => LLMProvider::Google,
            "Gemini" | "gemini" | "gemini-gate" => LLMProvider::Gemini,
            "OpenAI" | "openai" => LLMProvider::OpenAI,
            "OpenRouter" | "openrouter" => LLMProvider::OpenRouter,
            "Mistral" | "mistral" => LLMProvider::Mistral,
            "Groq" | "groq" => LLMProvider::Groq,
            "Together" | "together" => LLMProvider::Together,
            "Cohere" | "cohere" => LLMProvider::Cohere,
            "DeepSeek" | "deepseek" => LLMProvider::DeepSeek,
            "Claude" | "claude" | "claude-gate" => LLMProvider::Claude,
            "Copilot" | "copilot" => LLMProvider::Copilot,
            _ => LLMProvider::Ollama,
        }
    }

    fn placeholder_text(&self) -> &'static str {
        match self {
            LLMProvider::Ollama => "http://localhost:11434",
            LLMProvider::AnthropicAPI => "sk-ant-...",
            LLMProvider::Google => "AIza...",
            LLMProvider::Gemini => "Uses OAuth authentication",
            LLMProvider::OpenAI => "sk-...",
            LLMProvider::OpenRouter => "sk-or-...",
            LLMProvider::Mistral => "API Key",
            LLMProvider::Groq => "gsk_...",
            LLMProvider::Together => "API Key",
            LLMProvider::Cohere => "API Key",
            LLMProvider::DeepSeek => "sk-...",
            LLMProvider::Claude => "Uses OAuth authentication",
            LLMProvider::Copilot => "Uses GitHub OAuth authentication",
        }
    }

    fn label_text(&self) -> &'static str {
        match self {
            LLMProvider::Ollama => "Ollama Host",
            LLMProvider::Claude | LLMProvider::Copilot | LLMProvider::Gemini => "Status",
            _ => "API Key",
        }
    }

    fn default_model(&self) -> &'static str {
        match self {
            LLMProvider::Ollama => "llama3.2",
            LLMProvider::AnthropicAPI => "claude-3-5-sonnet-20241022",
            LLMProvider::Google | LLMProvider::Gemini => "gemini-1.5-pro",
            LLMProvider::OpenAI => "gpt-4o",
            LLMProvider::OpenRouter => "openai/gpt-4o",
            LLMProvider::Mistral => "mistral-large-latest",
            LLMProvider::Groq => "llama-3.3-70b-versatile",
            LLMProvider::Together => "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo",
            LLMProvider::Cohere => "command-r-plus",
            LLMProvider::DeepSeek => "deepseek-chat",
            LLMProvider::Claude => "claude-sonnet-4-20250514",
            LLMProvider::Copilot => "gpt-4o",
        }
    }

    fn api_url(&self) -> Option<&'static str> {
        match self {
            LLMProvider::AnthropicAPI => Some("https://console.anthropic.com/settings/keys"),
            LLMProvider::Google => Some("https://aistudio.google.com/app/apikey"),
            LLMProvider::Gemini => None, // Uses OAuth authentication
            LLMProvider::OpenAI => Some("https://platform.openai.com/api-keys"),
            LLMProvider::OpenRouter => Some("https://openrouter.ai/keys"),
            LLMProvider::Mistral => Some("https://console.mistral.ai/api-keys/"),
            LLMProvider::Groq => Some("https://console.groq.com/keys"),
            LLMProvider::Together => Some("https://api.together.xyz/settings/api-keys"),
            LLMProvider::Cohere => Some("https://dashboard.cohere.com/api-keys"),
            LLMProvider::DeepSeek => Some("https://platform.deepseek.com/api_keys"),
            LLMProvider::Ollama => Some("https://ollama.com/download"),
            LLMProvider::Claude => None, // Uses OAuth authentication
            LLMProvider::Copilot => None, // Uses GitHub OAuth authentication
        }
    }

    fn brand_color(&self) -> &'static str {
        match self {
            // Both AnthropicAPI (API key) and Claude (OAuth) are Anthropic providers, sharing brand color
            LLMProvider::AnthropicAPI | LLMProvider::Claude => "text-orange-400", // Anthropic Sienna
            LLMProvider::Google | LLMProvider::Gemini => "text-blue-400", // Google/Gemini Blue
            LLMProvider::OpenAI => "text-emerald-400", // OpenAI Green
            LLMProvider::Ollama => "text-white", // Ollama White
            LLMProvider::OpenRouter => "text-violet-400",
            LLMProvider::Copilot => "text-[#6e40c9]", // GitHub Purple
            _ => "text-[var(--accent-primary)]",
        }
    }
}

// ============================================================================
// Embedder Provider
// ============================================================================

#[derive(Clone, PartialEq, Debug)]
pub enum EmbedderProvider {
    Ollama,
    Local,   // HuggingFace/ONNX - runs locally via Meilisearch
    Copilot, // GitHub Copilot via local proxy
}

impl std::fmt::Display for EmbedderProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedderProvider::Ollama => write!(f, "Ollama"),
            EmbedderProvider::Local => write!(f, "Local (ONNX)"),
            EmbedderProvider::Copilot => write!(f, "Copilot"),
        }
    }
}

impl EmbedderProvider {
    fn description(&self) -> &'static str {
        match self {
            EmbedderProvider::Ollama => "Uses Ollama for embeddings. Requires Ollama to be running.",
            EmbedderProvider::Local => "Uses HuggingFace models via ONNX. No external service required.",
            EmbedderProvider::Copilot => "Uses GitHub Copilot via OAuth. Requires Copilot authentication.",
        }
    }
}

#[component]
pub fn LLMSettingsView() -> impl IntoView {
    // Signals
    let selected_provider = RwSignal::new(LLMProvider::Ollama);
    let api_key_or_host = RwSignal::new("http://localhost:11434".to_string());
    let model_name = RwSignal::new("llama3.2".to_string());
    let save_status = RwSignal::new(String::new());
    let is_saving = RwSignal::new(false);
    let health_status = RwSignal::new(Option::<HealthStatus>::None);
    let initial_load = RwSignal::new(true);
    let timeout_handle = StoredValue::new_local(None::<Timeout>);

    // Models
    let ollama_models = RwSignal::new(Vec::<OllamaModel>::new());
    let cloud_models = RwSignal::new(Vec::<ModelInfo>::new());
    let is_loading_models = RwSignal::new(false);

    // Embedding configuration (separated from LLM provider)
    let embedder_provider = RwSignal::new(EmbedderProvider::Ollama);
    let embedding_model = RwSignal::new("nomic-embed-text".to_string());
    let embedding_models = RwSignal::new(Vec::<OllamaEmbeddingModel>::new());
    let local_embedding_models = RwSignal::new(Vec::<LocalEmbeddingModel>::new());
    let is_setting_up_embeddings = RwSignal::new(false);
    let embeddings_status = RwSignal::new(String::new());

    // Statuses
    let provider_statuses = RwSignal::new(HashMap::<String, bool>::new());

    // Claude Gate OAuth status (for badge display and model fetching)
    let claude_status = RwSignal::new(ClaudeStatus::default());

    // Copilot OAuth status (for badge display and model fetching)
    let copilot_status = RwSignal::new(CopilotAuthStatus::default());

    // Gemini OAuth status (for badge display and model fetching)
    let gemini_status = RwSignal::new(GeminiStatus::default());

    // --- Helpers ---

    let fetch_ollama_models = move |host: String| {
        let host_clone = host.clone();
        spawn_local(async move {
            is_loading_models.set(true);
            match list_ollama_models(host.clone()).await {
                Ok(models) => {
                     ollama_models.set(models);
                     provider_statuses.update(|map| { map.insert("ollama".to_string(), true); });
                },
                Err(_) => {
                    ollama_models.set(Vec::new());
                    provider_statuses.update(|map| { map.insert("ollama".to_string(), false); });
                }
            }
            // Also fetch embedding models
            match list_ollama_embedding_models(host_clone).await {
                Ok(models) => {
                    embedding_models.set(models);
                }
                Err(_) => {
                    // Use default embedding models if fetch fails
                    embedding_models.set(vec![
                        OllamaEmbeddingModel { name: "nomic-embed-text".to_string(), size: "274 MB".to_string(), dimensions: 768 },
                        OllamaEmbeddingModel { name: "mxbai-embed-large".to_string(), size: "669 MB".to_string(), dimensions: 1024 },
                        OllamaEmbeddingModel { name: "all-minilm".to_string(), size: "46 MB".to_string(), dimensions: 384 },
                    ]);
                }
            }
            // Also fetch local embedding models (always available)
            match list_local_embedding_models().await {
                Ok(models) => {
                    local_embedding_models.set(models);
                }
                Err(_) => {
                    local_embedding_models.set(Vec::new());
                }
            }
            is_loading_models.set(false);
        });
    };

    let fetch_cloud_models = move |provider: LLMProvider, api_key: Option<String>| {
        spawn_local(async move {
            is_loading_models.set(true);
            let models = match provider {
                LLMProvider::AnthropicAPI => list_anthropic_models(api_key).await.unwrap_or_default(),
                LLMProvider::OpenAI => list_openai_models(api_key).await.unwrap_or_default(),
                LLMProvider::Google => list_gemini_models(api_key).await.unwrap_or_default(),
                LLMProvider::Gemini => {
                    // Gemini uses OAuth - fetch models from the Cloud Code API
                    match gemini_list_models().await {
                        Ok(gate_models) => gate_models
                            .into_iter()
                            .map(|m| ModelInfo { id: m.id.clone(), name: m.name, description: m.description })
                            .collect(),
                        Err(_) => {
                            // Fall back to default models list if not authenticated
                            list_gemini_models(None).await.unwrap_or_default()
                        }
                    }
                }
                LLMProvider::OpenRouter => list_openrouter_models().await.unwrap_or_default(),
                LLMProvider::Claude => {
                    // Fetch models from Claude API (OAuth authenticated)
                    match claude_list_models().await {
                        Ok(gate_models) if !gate_models.is_empty() => gate_models
                            .into_iter()
                            .map(|m| ModelInfo { id: m.id.clone(), name: m.name, description: None })
                            .collect(),
                        _ => {
                            // Fall back to default models list if not authenticated or empty
                            list_anthropic_models(None).await.unwrap_or_default()
                        }
                    }
                }
                LLMProvider::Copilot => {
                    // Fetch models from Copilot API (OAuth authenticated)
                    match get_copilot_models().await {
                        Ok(models) => models
                            .into_iter()
                            .filter(|m| m.supports_chat)
                            .map(|m| ModelInfo {
                                id: m.id.clone(),
                                name: m.id.clone(),
                                description: Some(format!("by {} ({})", m.owned_by, if m.preview { "preview" } else { "stable" })),
                            })
                            .collect(),
                        Err(_) => Vec::new(),
                    }
                }
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

    let check_providers = move || {
        spawn_local(async move {
            let mut statuses = HashMap::new();
            let ollama_host = if let Ok(Some(config)) = get_llm_config().await {
                config.host.unwrap_or_else(|| "http://localhost:11434".to_string())
            } else {
                "http://localhost:11434".to_string()
            };

            if let Ok(models) = list_ollama_models(ollama_host).await {
                 statuses.insert("ollama".to_string(), !models.is_empty());
            } else {
                 statuses.insert("ollama".to_string(), false);
            }
            let clouds = vec!["anthropic", "openai", "gemini", "mistral", "groq", "together", "cohere", "deepseek", "openrouter"];
            for p in clouds {
                if let Ok(Some(key)) = crate::bindings::get_api_key(p.to_string()).await {
                    statuses.insert(p.to_string(), !key.is_empty());
                } else {
                    statuses.insert(p.to_string(), false);
                }
            }

            // Check Claude OAuth status
            match claude_get_status().await {
                Ok(status) => {
                    statuses.insert("claude".to_string(), status.authenticated);
                    claude_status.set(status);
                }
                Err(_) => {
                    statuses.insert("claude".to_string(), false);
                }
            }

            // Check Copilot OAuth status
            match check_copilot_auth().await {
                Ok(status) => {
                    statuses.insert("copilot".to_string(), status.authenticated);
                    copilot_status.set(status);
                }
                Err(_) => {
                    statuses.insert("copilot".to_string(), false);
                }
            }

            provider_statuses.set(statuses);
        });
    };

    // Refresh Claude Code status


    // --- On Mount ---
    Effect::new(move |_| {
        check_providers();
        spawn_local(async move {
            if let Ok(Some(config)) = get_llm_config().await {
                let provider = LLMProvider::from_string(&config.provider);
                selected_provider.set(provider.clone());

                match provider {
                    LLMProvider::Ollama => {
                        let host = config.host.unwrap_or_else(|| "http://localhost:11434".to_string());
                        api_key_or_host.set(host.clone());
                        fetch_ollama_models(host);
                    }
                    _ => {
                        api_key_or_host.set(String::new()); // Security: don't show key by default
                        api_key_or_host.set(String::new()); // Security: don't show key by default
                        // Can't fetch models without key if we don't show it,
                        // but maybe we can fetch with stored key if we had a backend command for it?
                        // For now keep behavior same as `settings.rs`
                         fetch_cloud_models(provider, None);
                    }
                }
                model_name.set(config.model);
            } else {
                fetch_ollama_models("http://localhost:11434".to_string());
            }
            initial_load.set(false);

            if let Ok(status) = check_llm_health().await {
                health_status.set(Some(status));
            }
        });
    });

    // --- Auto-Save Effect ---
    Effect::new(move |_| {
        // Track dependencies
        let provider = selected_provider.get();
        let key_or_host = api_key_or_host.get();
        let model = model_name.get();
        let emb = embedding_model.get();

        if initial_load.get_untracked() {
            return;
        }

        // Debounce logic
        timeout_handle.update_value(|h| { if let Some(t) = h.take() { t.cancel(); } });

        let perform_save = move || {
             is_saving.set(true);
             save_status.set("Saving...".to_string());
             spawn_local(async move {
                 // OAuth providers don't need API keys - they use OAuth authentication
                 let needs_api_key = !matches!(
                     provider,
                     LLMProvider::Ollama | LLMProvider::Claude | LLMProvider::Copilot
                 );
                 let key_to_save = if needs_api_key && !key_or_host.is_empty() {
                      match save_api_key(provider.to_string_key(), key_or_host.clone()).await {
                         Ok(_) => Some(key_or_host.clone()),
                         Err(e) => {
                             show_error("Key Save Failed", Some(&e), None);
                             is_saving.set(false);
                             return;
                         }
                      }
                 } else {
                     None
                 };

                 let settings = LLMSettings {
                     provider: provider.to_string_key(),
                     api_key: key_to_save,
                     host: if provider == LLMProvider::Ollama { Some(key_or_host.clone()) } else { None },
                     model: model.clone(),
                     embedding_model: if provider == LLMProvider::Ollama { Some(emb.clone()) } else { None },
                     storage_backend: if provider == LLMProvider::Claude {
                         Some(claude_status.get_untracked().storage_backend)
                     } else {
                         None
                     },
                 };

                 match configure_llm(settings).await {
                     Ok(_) => {
                         save_status.set("All changes saved".to_string());
                         if let Ok(status) = check_llm_health().await {
                             health_status.set(Some(status));
                         }
                         check_providers();
                     }
                     Err(e) => {
                         show_error("Save Failed", Some(&e), None);
                         save_status.set("Error saving".to_string());
                     }
                 }
                 is_saving.set(false);
             });
        };

        timeout_handle.set_value(Some(Timeout::new(1000, perform_save)));
    });

    on_cleanup(move || {
        timeout_handle.update_value(|h| { if let Some(t) = h.take() { t.cancel(); } });
    });

    // --- Handlers ---

    let handle_provider_click = move |p: LLMProvider| {
        selected_provider.set(p.clone());
        match p {
            LLMProvider::Ollama => {
                 api_key_or_host.set("http://localhost:11434".to_string());
                 model_name.set("llama3.2".to_string());
                 fetch_ollama_models("http://localhost:11434".to_string());
            },
            LLMProvider::Claude => {
                 // No API key needed - uses OAuth authentication
                 api_key_or_host.set(String::new());
                 model_name.set(p.default_model().to_string());
                 // Fetch models from API if authenticated
                 fetch_cloud_models(LLMProvider::Claude, None);
            },
            LLMProvider::Copilot => {
                 // No API key needed - uses GitHub OAuth authentication
                 api_key_or_host.set(String::new());
                 model_name.set(p.default_model().to_string());
                 // Fetch models from Copilot API if authenticated
                 fetch_cloud_models(LLMProvider::Copilot, None);
            },
            _ => {
                 api_key_or_host.set(String::new());
                 model_name.set(p.default_model().to_string());
                 // Try to fetch with *no* key (gets stored key in backend?) Or just cleared?
                 // Standard flow requires re-entry or just trusting stored key.
                 cloud_models.set(Vec::new());
            }
        }
    };



    // --- UI Helpers ---
    let providers_list = vec![
        LLMProvider::Ollama,
        LLMProvider::OpenAI,
        LLMProvider::AnthropicAPI,
        LLMProvider::Claude,
        LLMProvider::Copilot,
        LLMProvider::Google,       // Google AI Studio (API key)
        LLMProvider::Gemini,   // Gemini (OAuth)
        LLMProvider::OpenRouter,
        LLMProvider::Mistral,
        LLMProvider::Groq,
        LLMProvider::DeepSeek,
    ];

    view! {
        <div class="space-y-8 animate-fade-in pb-20">
            <div class="flex justify-between items-start">
                <div class="space-y-2">
                    <h3 class="text-xl font-bold text-[var(--text-primary)]">"Artificial Intelligence"</h3>
                    <p class="text-[var(--text-muted)]">"Configure the brains behind your assistant."</p>
                </div>
                 {move || health_status.get().map(|s| {
                    if s.healthy {
                        view! { <Badge variant=BadgeVariant::Success>"System Online"</Badge> }
                    } else {
                        view! { <Badge variant=BadgeVariant::Danger>"System Offline"</Badge> }
                    }
                })}
            </div>

            // Active Provider Config
            <Card class="p-6 border-[var(--accent-primary)] border relative overflow-hidden transition-all duration-300">
                // Background Glow
                <div class="absolute -top-20 -right-20 w-64 h-64 bg-[var(--accent-primary)] opacity-5 blur-[100px] pointer-events-none"></div>

                <div class="flex flex-col md:flex-row gap-8 relative z-10">
                    // Left Column: Selection
                    <div class="flex-1 space-y-6">
                        <div>
                            <label class=move || format!("text-xs font-bold uppercase tracking-wider mb-2 block {}", selected_provider.get().brand_color())>
                                "Selected Provider"
                            </label>
                            <h2 class="text-3xl font-bold mb-1">{move || selected_provider.get().to_string()}</h2>
                            <p class="text-sm text-[var(--text-muted)]">
                                {move || match selected_provider.get() {
                                    LLMProvider::Ollama => "Running locally on your machine.",
                                    LLMProvider::Claude => "Uses Anthropic OAuth authentication.",
                                    LLMProvider::Copilot => "Uses GitHub OAuth authentication.",
                                    _ => "Cloud-based inference.",
                                }}
                            </p>
                        </div>

                        <div>
                             <div class="flex justify-between items-center mb-2">
                                <label class="block text-sm font-medium text-[var(--text-secondary)]">
                                    {move || selected_provider.get().label_text()}
                                </label>
                                {move || selected_provider.get().api_url().map(|url| {
                                    view! {
                                        <a
                                            href=url
                                            target="_blank"
                                            rel="noopener noreferrer"
                                            class="text-xs text-[var(--accent-primary)] hover:underline flex items-center gap-1"
                                        >
                                            "Get Key"
                                            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-external-link"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                                        </a>
                                    }
                                })}
                            </div>
                            {move || {
                                let provider = selected_provider.get();
                                if provider == LLMProvider::Claude {
                                    // Claude OAuth panel - uses shared component
                                    view! {
                                        <div class="space-y-3">
                                            <ClaudeAuth
                                                show_card=false
                                                on_status_change=Callback::new(move |status: ClaudeStatus| {
                                                    let is_ready = status.authenticated;
                                                    provider_statuses.update(|map| { map.insert("claude".to_string(), is_ready); });
                                                    claude_status.set(status);
                                                    // Trigger health check and model fetch after auth status changes
                                                    if is_ready {
                                                        spawn_local(async move {
                                                            // Refresh health status
                                                            if let Ok(h) = check_llm_health().await {
                                                                health_status.set(Some(h));
                                                            }
                                                            // Fetch models now that we're authenticated
                                                            fetch_cloud_models(LLMProvider::Claude, None);
                                                        });
                                                    }
                                                })
                                            />
                                            // Link to extraction settings
                                            <div class="pt-2 border-t border-[var(--border-subtle)]">
                                                <p class="text-xs text-[var(--text-muted)]">
                                                    "Claude can also be used for document extraction. Configure in "
                                                    <span class="text-[var(--accent-primary)]">"Extraction Settings"</span>
                                                    "."
                                                </p>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else if provider == LLMProvider::Copilot {
                                    // Copilot OAuth panel - uses shared component
                                    view! {
                                        <div class="space-y-3">
                                            <CopilotAuth
                                                show_card=false
                                                on_status_change=Callback::new(move |status: CopilotAuthStatus| {
                                                    let is_ready = status.authenticated;
                                                    provider_statuses.update(|map| { map.insert("copilot".to_string(), is_ready); });
                                                    copilot_status.set(status);
                                                    // Trigger health check and model fetch after auth status changes
                                                    if is_ready {
                                                        spawn_local(async move {
                                                            // Refresh health status
                                                            if let Ok(h) = check_llm_health().await {
                                                                health_status.set(Some(h));
                                                            }
                                                            // Fetch models now that we're authenticated
                                                            fetch_cloud_models(LLMProvider::Copilot, None);
                                                        });
                                                    }
                                                })
                                            />
                                        </div>
                                    }.into_any()
                                } else if provider == LLMProvider::Gemini {
                                    // Gemini OAuth panel - uses shared component
                                    view! {
                                        <div class="space-y-3">
                                            <GeminiAuth
                                                show_card=false
                                                on_status_change=Callback::new(move |status: GeminiStatus| {
                                                    let is_ready = status.authenticated;
                                                    provider_statuses.update(|map| { map.insert("gemini".to_string(), is_ready); });
                                                    gemini_status.set(status);
                                                    // Trigger health check and model fetch after auth status changes
                                                    if is_ready {
                                                        spawn_local(async move {
                                                            // Refresh health status
                                                            if let Ok(h) = check_llm_health().await {
                                                                health_status.set(Some(h));
                                                            }
                                                            // Fetch models now that we're authenticated
                                                            fetch_cloud_models(LLMProvider::Gemini, None);
                                                        });
                                                    }
                                                })
                                            />
                                        </div>
                                    }.into_any()
                                } else {
                                    // Regular input for other providers
                                    view! {
                                        <Input
                                            value=api_key_or_host
                                            placeholder=Signal::derive(move || selected_provider.get().placeholder_text().to_string())
                                            r#type=Signal::derive(move || if matches!(selected_provider.get(), LLMProvider::Ollama) { "text".to_string() } else { "password".to_string() })
                                        />
                                    }.into_any()
                                }
                            }}
                        </div>

                        <div>
                            <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">"Model"</label>
                             {move || {
                                if selected_provider.get() == LLMProvider::Ollama {
                                    view! {
                                        <select
                                            class="w-full p-3 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)] transition-colors"
                                            style="color-scheme: dark;"
                                            prop:value=model_name
                                            on:change=move |ev| model_name.set(event_target_value(&ev))
                                        >
                                            {ollama_models.get().into_iter().map(|m| {
                                                view! { <option value=m.name.clone() class="bg-[var(--bg-elevated)] text-[var(--text-primary)]">{m.name.clone()}</option> }
                                            }).collect::<Vec<_>>()}
                                        </select>
                                    }.into_any()
                                } else {
                                    // Cloud models dropdown or input
                                    // If we have models loaded, show them, else text input for fallback
                                    let models = cloud_models.get();
                                    if !models.is_empty() {
                                         view! {
                                            <select
                                                class="w-full p-3 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)]"
                                                style="color-scheme: dark;"
                                                prop:value=model_name
                                                on:change=move |ev| model_name.set(event_target_value(&ev))
                                            >
                                                {models.into_iter().map(|m| {
                                                    let display_name = if m.name.is_empty() || m.name == m.id {
                                                        m.id.clone()
                                                    } else {
                                                        m.name.clone()
                                                    };
                                                    view! { <option value=m.id.clone() class="bg-[var(--bg-elevated)] text-[var(--text-primary)]">{display_name}</option> }
                                                }).collect::<Vec<_>>()}
                                            </select>
                                        }.into_any()
                                    } else {
                                         view! {
                                            <Input value=model_name />
                                        }.into_any()
                                    }
                                }
                            }}
                        </div>

                         <div class="pt-4 h-10 flex items-center">
                             <div class="text-sm text-[var(--accent-primary)] font-medium italic animate-pulse">
                                 {move || {
                                      if is_saving.get() {
                                          "Saving changes...".to_string()
                                      } else {
                                          save_status.get()
                                      }
                                 }}
                             </div>
                         </div>
                    </div>

                    // Right Column: Provider Switcher
                    <div class="w-full md:w-64 flex-shrink-0 space-y-3 border-t md:border-t-0 md:border-l border-[var(--border-subtle)] pt-6 md:pt-0 md:pl-6">
                        <label class="text-xs font-bold text-[var(--text-muted)] uppercase tracking-wider block mb-2">
                            "Switch Provider"
                        </label>
                        {providers_list.into_iter().map(|p| {
                            let p_clone = p.clone();
                            let p_active = p.clone();
                            let p_status = p.clone();
                            let is_active = move || selected_provider.get() == p_active;
                            let status = move || provider_statuses.get().get(&p_status.to_string_key()).copied().unwrap_or(false);

                            view! {
                                <button
                                    class=move || format!(
                                        "w-full flex items-center justify-between p-3 rounded-lg text-sm transition-all {}",
                                        if is_active() {
                                            "bg-[var(--accent-primary)] text-[var(--bg-deep)] shadow-md font-bold"
                                        } else {
                                            "bg-[var(--bg-surface)] text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)]"
                                        }
                                    )
                                    on:click=move |_| handle_provider_click(p_clone.clone())
                                >
                                    <span>{p.to_string()}</span>
                                    {move || if status() {
                                        view! { <div class="w-2 h-2 rounded-full bg-green-400 shadow-lg shadow-green-400/50"></div> }
                                    } else {
                                        view! { <div class="w-2 h-2 rounded-full bg-gray-600"></div> }
                                    }}
                                </button>
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                </div>

                // Token Usage Toggle
                <div class="mt-6 pt-6 border-t border-[var(--border-subtle)]">
                    {
                        let layout_state = crate::services::layout_service::use_layout_state();
                        let show_tokens = layout_state.show_token_usage;

                        view! {
                            <div class="flex items-center justify-between">
                                <div>
                                    <h4 class="font-semibold text-[var(--text-secondary)]">"Show Token Usage"</h4>
                                    <p class="text-sm text-[var(--text-muted)]">"Display token counts as a tooltip when hovering over chat messages."</p>
                                </div>
                                <button
                                    class=move || format!(
                                        "h-6 w-11 rounded-full border transition-colors duration-200 relative focus:outline-none focus:ring-2 focus:ring-[var(--accent-primary)] {}",
                                        if show_tokens.get() {
                                            "bg-[var(--accent-primary)] border-[var(--accent-primary)]"
                                        } else {
                                            "bg-[var(--bg-surface)] border-[var(--border-subtle)]"
                                        }
                                    )
                                    on:click=move |_| show_tokens.update(|v| *v = !*v)
                                    role="switch"
                                    aria-checked=move || show_tokens.get().to_string()
                                >
                                    <div
                                        class=move || format!(
                                            "absolute top-1 left-1 h-4 w-4 rounded-full bg-white shadow-sm transition-transform duration-200 {}",
                                            if show_tokens.get() { "translate-x-5" } else { "translate-x-0" }
                                        )
                                    />
                                </button>
                            </div>
                        }
                    }
                </div>
            </Card>

            // Embedding Configuration Card
            <Card class="p-6">
                <div class="space-y-6">
                    <div>
                        <h4 class="text-lg font-bold text-[var(--text-primary)]">"Embedding Configuration"</h4>
                        <p class="text-sm text-[var(--text-muted)]">"Configure the embedding model for AI-powered semantic search."</p>
                    </div>

                    // Embedder Provider Selector
                    <div class="flex gap-4">
                        <button
                            class=move || format!(
                                "flex-1 p-3 rounded-lg text-sm font-medium transition-all {}",
                                if embedder_provider.get() == EmbedderProvider::Ollama {
                                    "bg-[var(--accent-primary)] text-[var(--bg-deep)]"
                                } else {
                                    "bg-[var(--bg-surface)] text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)]"
                                }
                            )
                            on:click=move |_| embedder_provider.set(EmbedderProvider::Ollama)
                        >
                            <div class="flex items-center justify-center gap-2">
                                <span>"Ollama"</span>
                                {move || {
                                    let ollama_ok = provider_statuses.get().get("ollama").copied().unwrap_or(false);
                                    if ollama_ok {
                                        view! { <div class="w-2 h-2 rounded-full bg-green-400"></div> }.into_any()
                                    } else {
                                        view! { <div class="w-2 h-2 rounded-full bg-red-400"></div> }.into_any()
                                    }
                                }}
                            </div>
                        </button>
                        <button
                            class=move || format!(
                                "flex-1 p-3 rounded-lg text-sm font-medium transition-all {}",
                                if embedder_provider.get() == EmbedderProvider::Local {
                                    "bg-[var(--accent-primary)] text-[var(--bg-deep)]"
                                } else {
                                    "bg-[var(--bg-surface)] text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)]"
                                }
                            )
                            on:click=move |_| embedder_provider.set(EmbedderProvider::Local)
                        >
                            "Local (ONNX)"
                        </button>
                        <button
                            class=move || format!(
                                "flex-1 p-3 rounded-lg text-sm font-medium transition-all {}",
                                if embedder_provider.get() == EmbedderProvider::Copilot {
                                    "bg-[#6e40c9] text-white"
                                } else {
                                    "bg-[var(--bg-surface)] text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)]"
                                }
                            )
                            on:click=move |_| embedder_provider.set(EmbedderProvider::Copilot)
                        >
                            <div class="flex items-center justify-center gap-2">
                                <span>"Copilot"</span>
                                {move || {
                                    let copilot_ok = copilot_status.get().authenticated;
                                    if copilot_ok {
                                        view! { <div class="w-2 h-2 rounded-full bg-green-400"></div> }.into_any()
                                    } else {
                                        view! { <div class="w-2 h-2 rounded-full bg-red-400"></div> }.into_any()
                                    }
                                }}
                            </div>
                        </button>
                    </div>

                    <p class="text-xs text-[var(--text-muted)]">
                        {move || embedder_provider.get().description()}
                    </p>

                    // Model selection based on provider
                    <div>
                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">"Embedding Model"</label>
                        {move || {
                            if embedder_provider.get() == EmbedderProvider::Ollama {
                                let models = embedding_models.get();
                                let current_model = embedding_model.get();
                                if !models.is_empty() {
                                    view! {
                                        <select
                                            class="w-full p-3 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)] transition-colors"
                                            style="color-scheme: dark;"
                                            on:change=move |ev| {
                                                let val = event_target_value(&ev);
                                                embedding_model.set(val);
                                            }
                                        >
                                            {models.into_iter().map(|m| {
                                                let is_selected = m.name == current_model;
                                                let label = format!("{} ({}D, {})", m.name, m.dimensions, m.size);
                                                view! {
                                                    <option
                                                        value=m.name.clone()
                                                        selected=is_selected
                                                        class="bg-[var(--bg-elevated)] text-[var(--text-primary)]"
                                                    >
                                                        {label}
                                                    </option>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </select>
                                    }.into_any()
                                } else {
                                    view! {
                                        <Input value=embedding_model />
                                    }.into_any()
                                }
                            } else if embedder_provider.get() == EmbedderProvider::Local {
                                // Local ONNX models
                                let models = local_embedding_models.get();
                                let current_model = embedding_model.get();
                                if !models.is_empty() {
                                    view! {
                                        <select
                                            class="w-full p-3 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)] transition-colors"
                                            style="color-scheme: dark;"
                                            on:change=move |ev| {
                                                let val = event_target_value(&ev);
                                                embedding_model.set(val);
                                            }
                                        >
                                            {models.into_iter().map(|m| {
                                                let is_selected = m.id == current_model;
                                                let label = format!("{} ({}D)", m.name, m.dimensions);
                                                view! {
                                                    <option
                                                        value=m.id.clone()
                                                        selected=is_selected
                                                        class="bg-[var(--bg-elevated)] text-[var(--text-primary)]"
                                                    >
                                                        {label}
                                                    </option>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </select>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="p-3 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-muted)] text-sm">
                                            "No local models available. Install kreuzberg with embeddings feature."
                                        </div>
                                    }.into_any()
                                }
                            } else {
                                // Copilot embedding models (predefined OpenAI-compatible options)
                                let current_model = embedding_model.get();
                                let copilot_models = vec![
                                    ("text-embedding-3-small", "text-embedding-3-small (1536D, recommended)"),
                                    ("text-embedding-3-large", "text-embedding-3-large (3072D, higher quality)"),
                                    ("text-embedding-ada-002", "text-embedding-ada-002 (1536D, legacy)"),
                                ];
                                view! {
                                    <select
                                        class="w-full p-3 rounded-lg bg-[var(--bg-deep)] border border-[#6e40c9]/30 text-[var(--text-primary)] outline-none focus:border-[#6e40c9] transition-colors"
                                        style="color-scheme: dark;"
                                        on:change=move |ev| {
                                            let val = event_target_value(&ev);
                                            embedding_model.set(val);
                                        }
                                    >
                                        {copilot_models.into_iter().map(|(id, label)| {
                                            let is_selected = id == current_model;
                                            view! {
                                                <option
                                                    value=id
                                                    selected=is_selected
                                                    class="bg-[var(--bg-elevated)] text-[var(--text-primary)]"
                                                >
                                                    {label}
                                                </option>
                                            }
                                        }).collect::<Vec<_>>()}
                                    </select>
                                }.into_any()
                            }
                        }}
                    </div>

                    // Setup button
                    <div class="flex items-center gap-3">
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=move |_: ev::MouseEvent| {
                                let provider = embedder_provider.get();
                                let model = embedding_model.get();
                                is_setting_up_embeddings.set(true);
                                embeddings_status.set("Setting up embeddings...".to_string());

                                spawn_local(async move {
                                    // Handle each provider separately due to different return types
                                    let setup_result: Result<(usize, String, u32), String> = match provider {
                                        EmbedderProvider::Ollama => {
                                            let host = api_key_or_host.get_untracked();
                                            let host = if host.is_empty() { "http://localhost:11434".to_string() } else { host };
                                            setup_ollama_embeddings(host, model.clone()).await
                                                .map(|r| (r.indexes_configured.len(), r.model, r.dimensions))
                                        }
                                        EmbedderProvider::Local => {
                                            setup_local_embeddings(model.clone()).await
                                                .map(|r| (r.indexes_configured.len(), r.model, r.dimensions))
                                        }
                                        EmbedderProvider::Copilot => {
                                            setup_copilot_embeddings(model.clone(), None).await
                                                .map(|r| (r.indexes_configured.len(), r.model, r.dimensions))
                                        }
                                    };

                                    match setup_result {
                                        Ok((count, model_name, dims)) => {
                                            embeddings_status.set(format!(
                                                "✓ Configured {} indexes with {} ({}D)",
                                                count,
                                                model_name,
                                                dims
                                            ));
                                            show_success(
                                                "Embeddings Configured",
                                                Some(&format!(
                                                    "AI-powered search enabled on {} indexes using {}",
                                                    count,
                                                    model_name
                                                ))
                                            );
                                        }
                                        Err(e) => {
                                            embeddings_status.set(format!("✗ Failed: {}", e));
                                            show_error("Embeddings Setup Failed", Some(&e), None);
                                        }
                                    }
                                    is_setting_up_embeddings.set(false);
                                });
                            }
                            disabled=is_setting_up_embeddings.get()
                            loading=is_setting_up_embeddings.get()
                        >
                            "Setup AI Search"
                        </Button>
                        <span class="text-xs text-[var(--text-muted)]">
                            {move || embeddings_status.get()}
                        </span>
                    </div>
                </div>
            </Card>
        </div>
    }
}
