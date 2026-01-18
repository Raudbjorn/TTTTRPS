use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;
use std::collections::HashMap;
use gloo_timers::callback::Timeout;

/// GitHub URL for the Sidecar DM Gemini extension
const SIDECAR_DM_EXTENSION_URL: &str = "https://github.com/Raudbjorn/sidecar-dm-gemini-extension";
use crate::bindings::{
    check_llm_health, configure_llm, get_llm_config, list_claude_models, list_gemini_models,
    list_ollama_models, list_openai_models, list_openrouter_models, list_provider_models,
    save_api_key, HealthStatus, LLMSettings, ModelInfo, OllamaModel,
    // Claude Code CLI
    get_claude_code_status, claude_code_login, claude_code_logout,
    claude_code_install_cli, claude_code_install_skill, ClaudeCodeStatus,
    // Claude Gate OAuth
    claude_gate_get_status, claude_gate_start_oauth, claude_gate_complete_oauth,
    claude_gate_logout, claude_gate_set_storage_backend, claude_gate_list_models, open_url_in_browser,
    ClaudeGateStatus, ClaudeGateStorageBackend, ClaudeGateOAuthStartResponse, ClaudeGateModelInfo,
    // Gemini CLI
    check_gemini_cli_status, launch_gemini_cli_login, check_gemini_cli_extension,
    install_gemini_cli_extension, GeminiCliStatus, GeminiCliExtensionStatus,
    // LLM Proxy
    is_llm_proxy_running, get_llm_proxy_url, list_proxy_providers,
    // Embedding configuration
    list_ollama_embedding_models, setup_ollama_embeddings, OllamaEmbeddingModel,
    list_local_embedding_models, setup_local_embeddings, LocalEmbeddingModel,
};
use crate::components::design_system::{Badge, BadgeVariant, Button, ButtonVariant, Card, Input};
use crate::services::notification_service::{show_error, show_success};

#[derive(Clone, PartialEq, Debug)]
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
    ClaudeCode,
    ClaudeDesktop,
    ClaudeGate,
    GeminiCli,
}

/// Consolidated UI state machine for Claude Gate authentication.
/// This provides a cleaner view of the authentication flow state.
#[derive(Clone, PartialEq, Debug)]
pub enum ClaudeGateUiState {
    /// Initial idle state, not performing any operation
    Idle,
    /// Loading - checking status, starting OAuth, completing auth, or logging out
    Loading,
    /// Waiting for user to input authorization code from browser
    AwaitingCode,
    /// Authenticated successfully
    Authenticated,
    /// Error state with message
    Error(String),
}

impl ClaudeGateUiState {
    /// Derive the UI state from the individual signals.
    /// This consolidates the fragmented state into a single state machine view.
    pub fn derive(
        status: &ClaudeGateStatus,
        is_loading: bool,
        awaiting_code: bool,
    ) -> Self {
        if is_loading {
            return ClaudeGateUiState::Loading;
        }
        if awaiting_code {
            return ClaudeGateUiState::AwaitingCode;
        }
        if let Some(ref error) = status.error {
            return ClaudeGateUiState::Error(error.clone());
        }
        if status.authenticated {
            return ClaudeGateUiState::Authenticated;
        }
        ClaudeGateUiState::Idle
    }

    /// Check if currently in a loading state
    pub fn is_loading(&self) -> bool {
        matches!(self, ClaudeGateUiState::Loading)
    }

    /// Check if authenticated
    pub fn is_authenticated(&self) -> bool {
        matches!(self, ClaudeGateUiState::Authenticated)
    }

    /// Check if awaiting user input
    pub fn is_awaiting_code(&self) -> bool {
        matches!(self, ClaudeGateUiState::AwaitingCode)
    }

    /// Get the display string for the current state
    pub fn display(&self) -> &str {
        match self {
            ClaudeGateUiState::Idle => "Ready to authenticate",
            ClaudeGateUiState::Loading => "Loading...",
            ClaudeGateUiState::AwaitingCode => "Waiting for auth code",
            ClaudeGateUiState::Authenticated => "Authenticated",
            ClaudeGateUiState::Error(_) => "Error",
        }
    }
}

impl std::fmt::Display for LLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProvider::Ollama => write!(f, "Ollama"),
            LLMProvider::Claude => write!(f, "Claude"),
            LLMProvider::Gemini => write!(f, "Gemini"),
            LLMProvider::OpenAI => write!(f, "OpenAI"),
            LLMProvider::OpenRouter => write!(f, "OpenRouter"),
            LLMProvider::Mistral => write!(f, "Mistral"),
            LLMProvider::Groq => write!(f, "Groq"),
            LLMProvider::Together => write!(f, "Together"),
            LLMProvider::Cohere => write!(f, "Cohere"),
            LLMProvider::DeepSeek => write!(f, "DeepSeek"),
            LLMProvider::ClaudeCode => write!(f, "Claude Code"),
            LLMProvider::ClaudeDesktop => write!(f, "Claude Desktop"),
            LLMProvider::ClaudeGate => write!(f, "Claude Gate"),
            LLMProvider::GeminiCli => write!(f, "Gemini CLI"),
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
            LLMProvider::ClaudeCode => "claude-code".to_string(),
            LLMProvider::ClaudeDesktop => "claude-desktop".to_string(),
            LLMProvider::ClaudeGate => "claude-gate".to_string(),
            LLMProvider::GeminiCli => "gemini-cli".to_string(),
        }
    }

    fn from_string(s: &str) -> Self {
        match s {
            "Claude" | "claude" => LLMProvider::Claude,
            "Gemini" | "gemini" => LLMProvider::Gemini,
            "OpenAI" | "openai" => LLMProvider::OpenAI,
            "OpenRouter" | "openrouter" => LLMProvider::OpenRouter,
            "Mistral" | "mistral" => LLMProvider::Mistral,
            "Groq" | "groq" => LLMProvider::Groq,
            "Together" | "together" => LLMProvider::Together,
            "Cohere" | "cohere" => LLMProvider::Cohere,
            "DeepSeek" | "deepseek" => LLMProvider::DeepSeek,
            "ClaudeCode" | "claude-code" => LLMProvider::ClaudeCode,
            "ClaudeDesktop" | "claude-desktop" => LLMProvider::ClaudeDesktop,
            "ClaudeGate" | "claude-gate" => LLMProvider::ClaudeGate,
            "GeminiCli" | "gemini-cli" => LLMProvider::GeminiCli,
            _ => LLMProvider::Ollama,
        }
    }

    fn placeholder_text(&self) -> &'static str {
        match self {
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
            LLMProvider::ClaudeCode => "Uses CLI authentication",
            LLMProvider::ClaudeDesktop => "Uses Desktop authentication",
            LLMProvider::ClaudeGate => "Uses OAuth authentication",
            LLMProvider::GeminiCli => "Uses Google account authentication",
        }
    }

    fn label_text(&self) -> &'static str {
        match self {
            LLMProvider::Ollama => "Ollama Host",
            LLMProvider::ClaudeCode => "Status",
            LLMProvider::ClaudeDesktop => "Status",
            LLMProvider::ClaudeGate => "Status",
            LLMProvider::GeminiCli => "Status",
            _ => "API Key",
        }
    }

    fn default_model(&self) -> &'static str {
        match self {
            LLMProvider::Ollama => "llama3.2",
            LLMProvider::Claude => "claude-3-5-sonnet-20241022",
            LLMProvider::Gemini => "gemini-1.5-pro",
            LLMProvider::OpenAI => "gpt-4o",
            LLMProvider::OpenRouter => "openai/gpt-4o",
            LLMProvider::Mistral => "mistral-large-latest",
            LLMProvider::Groq => "llama-3.3-70b-versatile",
            LLMProvider::Together => "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo",
            LLMProvider::Cohere => "command-r-plus",
            LLMProvider::DeepSeek => "deepseek-chat",
            LLMProvider::ClaudeCode => "claude-sonnet-4-20250514",
            LLMProvider::ClaudeDesktop => "claude-sonnet-4-20250514",
            LLMProvider::ClaudeGate => "claude-sonnet-4-20250514",
            LLMProvider::GeminiCli => "gemini-3-pro-preview",
        }
    }

    fn api_url(&self) -> Option<&'static str> {
        match self {
            LLMProvider::Claude => Some("https://console.anthropic.com/settings/keys"),
            LLMProvider::Gemini => Some("https://aistudio.google.com/app/apikey"),
            LLMProvider::OpenAI => Some("https://platform.openai.com/api-keys"),
            LLMProvider::OpenRouter => Some("https://openrouter.ai/keys"),
            LLMProvider::Mistral => Some("https://console.mistral.ai/api-keys/"),
            LLMProvider::Groq => Some("https://console.groq.com/keys"),
            LLMProvider::Together => Some("https://api.together.xyz/settings/api-keys"),
            LLMProvider::Cohere => Some("https://dashboard.cohere.com/api-keys"),
            LLMProvider::DeepSeek => Some("https://platform.deepseek.com/api_keys"),
            LLMProvider::Ollama => Some("https://ollama.com/download"),
            LLMProvider::ClaudeCode => None, // Uses CLI authentication
            LLMProvider::ClaudeDesktop => None, // Uses Desktop authentication
            LLMProvider::ClaudeGate => None, // Uses OAuth authentication
            LLMProvider::GeminiCli => None, // Uses Google account authentication
        }
    }

    fn brand_color(&self) -> &'static str {
        match self {
            LLMProvider::Claude => "text-orange-400", // Anthropic Sienna
            LLMProvider::Gemini => "text-blue-400", // Gemini Blue
            LLMProvider::OpenAI => "text-emerald-400", // OpenAI Green
            LLMProvider::Ollama => "text-white", // Ollama White
            LLMProvider::OpenRouter => "text-violet-400",
            LLMProvider::ClaudeCode => "text-orange-400", // Anthropic Sienna
            LLMProvider::ClaudeDesktop => "text-orange-400", // Anthropic Sienna
            LLMProvider::ClaudeGate => "text-orange-400", // Anthropic Sienna
            LLMProvider::GeminiCli => "text-blue-400", // Gemini Blue
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
    Local,  // HuggingFace/ONNX - runs locally via Meilisearch
}

impl std::fmt::Display for EmbedderProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedderProvider::Ollama => write!(f, "Ollama"),
            EmbedderProvider::Local => write!(f, "Local (ONNX)"),
        }
    }
}

impl EmbedderProvider {
    fn description(&self) -> &'static str {
        match self {
            EmbedderProvider::Ollama => "Uses Ollama for embeddings. Requires Ollama to be running.",
            EmbedderProvider::Local => "Uses HuggingFace models via ONNX. No external service required.",
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
    let claude_code_status = RwSignal::new(ClaudeCodeStatus::default());
    let claude_code_loading = RwSignal::new(false);
    let gemini_cli_status = RwSignal::new(GeminiCliStatus {
        is_installed: false,
        is_authenticated: false,
        message: String::new(),
    });
    let gemini_cli_extension = RwSignal::new(GeminiCliExtensionStatus {
        is_installed: false,
        message: String::new(),
    });
    let gemini_cli_loading = RwSignal::new(false);

    // Claude Gate OAuth status (individual signals for backward compatibility)
    let claude_gate_status = RwSignal::new(ClaudeGateStatus::default());
    let claude_gate_loading = RwSignal::new(false);
    let claude_gate_storage = RwSignal::new(ClaudeGateStorageBackend::Auto);
    let claude_gate_auth_code = RwSignal::new(String::new());
    let claude_gate_awaiting_code = RwSignal::new(false);
    let claude_gate_oauth_url = RwSignal::new(Option::<String>::None);
    let claude_gate_csrf_state = RwSignal::new(Option::<String>::None);

    // Consolidated UI state derived from individual signals.
    // This provides a state machine view for cleaner conditional logic.
    // Usage: claude_gate_ui_state.get().is_loading(), .is_authenticated(), etc.
    // Can also be used in views: {move || claude_gate_ui_state.get().display()}
    // Note: Prefixed with _ to suppress unused warning until UI is fully migrated.
    let _claude_gate_ui_state = Memo::new(move |_| {
        ClaudeGateUiState::derive(
            &claude_gate_status.get(),
            claude_gate_loading.get(),
            claude_gate_awaiting_code.get(),
        )
    });

    // Proxy status
    let proxy_running = RwSignal::new(false);
    let proxy_url = RwSignal::new(String::new());
    let proxy_providers = RwSignal::new(Vec::<String>::new());

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
                LLMProvider::Claude => list_claude_models(api_key).await.unwrap_or_default(),
                LLMProvider::OpenAI => list_openai_models(api_key).await.unwrap_or_default(),
                LLMProvider::Gemini => list_gemini_models(api_key).await.unwrap_or_default(),
                LLMProvider::OpenRouter => list_openrouter_models().await.unwrap_or_default(),
                LLMProvider::ClaudeGate => {
                    // Fetch models from Claude Gate API (OAuth authenticated)
                    match claude_gate_list_models().await {
                        Ok(gate_models) => gate_models
                            .into_iter()
                            .map(|m| ModelInfo { id: m.id.clone(), name: m.name, description: None })
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
            let clouds = vec!["claude", "openai", "gemini", "mistral", "groq", "together", "cohere", "deepseek", "openrouter"];
            for p in clouds {
                if let Ok(Some(key)) = crate::bindings::get_api_key(p.to_string()).await {
                    statuses.insert(p.to_string(), !key.is_empty());
                } else {
                    statuses.insert(p.to_string(), false);
                }
            }
            // Claude Desktop uses Desktop authentication
            statuses.insert("claude-desktop".to_string(), true);

            // Check Claude Code CLI status
            match get_claude_code_status().await {
                Ok(status) => {
                    statuses.insert("claude-code".to_string(), status.installed && status.logged_in);
                    claude_code_status.set(status);
                }
                Err(_) => {
                    statuses.insert("claude-code".to_string(), false);
                }
            }

            // Check Gemini CLI status
            match check_gemini_cli_status().await {
                Ok(status) => {
                    statuses.insert(LLMProvider::GeminiCli.to_string_key(), status.is_installed && status.is_authenticated);
                    gemini_cli_status.set(status);
                }
                Err(_) => {
                    statuses.insert(LLMProvider::GeminiCli.to_string_key(), false);
                }
            }

            // Check Gemini CLI extension status
            if let Ok(ext_status) = check_gemini_cli_extension().await {
                gemini_cli_extension.set(ext_status);
            }

            // Check Claude Gate OAuth status
            match claude_gate_get_status().await {
                Ok(status) => {
                    statuses.insert("claude-gate".to_string(), status.authenticated);
                    // Sync storage backend dropdown with actual backend
                    let backend = match status.storage_backend.to_lowercase().as_str() {
                        "keyring" => ClaudeGateStorageBackend::Keyring,
                        "file" => ClaudeGateStorageBackend::File,
                        _ => ClaudeGateStorageBackend::Auto,
                    };
                    claude_gate_storage.set(backend);
                    claude_gate_status.set(status);
                }
                Err(_) => {
                    statuses.insert("claude-gate".to_string(), false);
                }
            }

            provider_statuses.set(statuses);
        });
    };

    // Refresh Claude Code status
    let refresh_claude_code_status = move || {
        claude_code_loading.set(true);
        spawn_local(async move {
            match get_claude_code_status().await {
                Ok(status) => {
                    let is_ready = status.installed && status.logged_in;
                    provider_statuses.update(|map| { map.insert("claude-code".to_string(), is_ready); });
                    claude_code_status.set(status);
                }
                Err(e) => {
                    show_error("Claude Code Status", Some(&e), None);
                }
            }
            claude_code_loading.set(false);
        });
    };

    // Refresh Gemini CLI status
    let refresh_gemini_cli_status = move || {
        gemini_cli_loading.set(true);
        spawn_local(async move {
            match check_gemini_cli_status().await {
                Ok(status) => {
                    let is_ready = status.is_installed && status.is_authenticated;
                    provider_statuses.update(|map| { map.insert(LLMProvider::GeminiCli.to_string_key(), is_ready); });
                    gemini_cli_status.set(status);
                }
                Err(e) => {
                    show_error("Gemini CLI Status", Some(&e), None);
                }
            }
            // Also check extension status
            match check_gemini_cli_extension().await {
                Ok(ext_status) => gemini_cli_extension.set(ext_status),
                Err(e) => show_error("Gemini Extension Status", Some(&e), None),
            }
            gemini_cli_loading.set(false);
        });
    };

    // Refresh Claude Gate OAuth status
    let refresh_claude_gate_status = move || {
        claude_gate_loading.set(true);
        spawn_local(async move {
            match claude_gate_get_status().await {
                Ok(status) => {
                    let is_ready = status.authenticated;
                    provider_statuses.update(|map| { map.insert("claude-gate".to_string(), is_ready); });
                    // Update storage backend signal from status (case-insensitive match)
                    let backend = match status.storage_backend.to_lowercase().as_str() {
                        "keyring" => ClaudeGateStorageBackend::Keyring,
                        "file" => ClaudeGateStorageBackend::File,
                        _ => ClaudeGateStorageBackend::Auto,
                    };
                    claude_gate_storage.set(backend);
                    claude_gate_status.set(status);
                }
                Err(e) => {
                    show_error("Claude Gate Status", Some(&e), None);
                }
            }
            claude_gate_loading.set(false);
        });
    };

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

    // Check proxy status
    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(running) = is_llm_proxy_running().await {
                proxy_running.set(running);
            }
            if let Ok(url) = get_llm_proxy_url().await {
                proxy_url.set(url);
            }
            if let Ok(providers) = list_proxy_providers().await {
                proxy_providers.set(providers);
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
                 // ClaudeCode, ClaudeDesktop, ClaudeGate, and GeminiCli don't need API keys - they use CLI/Desktop/OAuth/Google auth
                 let needs_api_key = !matches!(
                     provider,
                     LLMProvider::Ollama | LLMProvider::ClaudeCode | LLMProvider::ClaudeDesktop | LLMProvider::ClaudeGate | LLMProvider::GeminiCli
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
            LLMProvider::ClaudeCode | LLMProvider::ClaudeDesktop => {
                 // No API key needed - uses CLI/Desktop authentication
                 api_key_or_host.set(String::new());
                 model_name.set(p.default_model().to_string());
                 cloud_models.set(Vec::new());
            },
            LLMProvider::ClaudeGate => {
                 // No API key needed - uses OAuth authentication
                 api_key_or_host.set(String::new());
                 model_name.set(p.default_model().to_string());
                 // Fetch models from API if authenticated
                 fetch_cloud_models(LLMProvider::ClaudeGate, None);
            },
            LLMProvider::GeminiCli => {
                 // No API key needed - uses Google account authentication
                 api_key_or_host.set(String::new());
                 model_name.set(p.default_model().to_string());
                 cloud_models.set(Vec::new());
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
        LLMProvider::Claude,
        LLMProvider::ClaudeCode,
        LLMProvider::ClaudeGate,
        LLMProvider::Gemini,
        LLMProvider::GeminiCli,
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
                                    LLMProvider::ClaudeCode => "Uses Claude Code CLI authentication.",
                                    LLMProvider::ClaudeDesktop => "Uses Claude Desktop authentication.",
                                    LLMProvider::ClaudeGate => "Uses Anthropic OAuth authentication.",
                                    LLMProvider::GeminiCli => "Uses Google account authentication (free tier).",
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
                                if selected_provider.get() == LLMProvider::ClaudeCode {
                                    // Claude Code status panel
                                    let status = claude_code_status.get();
                                    let is_loading = claude_code_loading.get();
                                    view! {
                                        <div class="p-4 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] space-y-3">
                                            // Status indicators
                                            <div class="flex flex-wrap gap-2">
                                                <div class=move || format!(
                                                    "flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium {}",
                                                    if status.installed { "bg-green-500/20 text-green-400" } else { "bg-red-500/20 text-red-400" }
                                                )>
                                                    <span class=move || format!(
                                                        "w-2 h-2 rounded-full {}",
                                                        if status.installed { "bg-green-400" } else { "bg-red-400" }
                                                    )></span>
                                                    {if status.installed { "CLI Installed" } else { "CLI Not Installed" }}
                                                </div>
                                                <div class=move || format!(
                                                    "flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium {}",
                                                    if status.logged_in { "bg-green-500/20 text-green-400" } else { "bg-yellow-500/20 text-yellow-400" }
                                                )>
                                                    <span class=move || format!(
                                                        "w-2 h-2 rounded-full {}",
                                                        if status.logged_in { "bg-green-400" } else { "bg-yellow-400" }
                                                    )></span>
                                                    {if status.logged_in { "Logged In" } else { "Not Logged In" }}
                                                </div>
                                                {status.version.clone().map(|v| view! {
                                                    <div class="flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium bg-blue-500/20 text-blue-400">
                                                        {format!("v{}", v)}
                                                    </div>
                                                })}
                                            </div>

                                            // Error message if any
                                            {status.error.clone().map(|e| view! {
                                                <p class="text-xs text-red-400">{e}</p>
                                            })}

                                            // Action buttons
                                            <div class="flex flex-wrap gap-2 pt-2">
                                                {move || if !status.installed {
                                                    view! {
                                                        <button
                                                            class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--accent-primary)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
                                                            disabled=is_loading
                                                            on:click=move |_| {
                                                                spawn_local(async move {
                                                                    match claude_code_install_cli().await {
                                                                        Ok(_) => show_success("Installing CLI", Some("Opening terminal...")),
                                                                        Err(e) => show_error("Install Failed", Some(&e), None),
                                                                    }
                                                                });
                                                            }
                                                        >
                                                            "Install CLI"
                                                        </button>
                                                    }.into_any()
                                                } else if !status.logged_in {
                                                    view! {
                                                        <button
                                                            class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--accent-primary)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
                                                            disabled=is_loading
                                                            on:click=move |_| {
                                                                spawn_local(async move {
                                                                    match claude_code_login().await {
                                                                        Ok(_) => show_success("Logging In", Some("Opening terminal...")),
                                                                        Err(e) => show_error("Login Failed", Some(&e), None),
                                                                    }
                                                                });
                                                            }
                                                        >
                                                            "Login"
                                                        </button>
                                                    }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}

                                                <button
                                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--bg-elevated)] text-[var(--text-secondary)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                                                    disabled=is_loading
                                                    on:click=move |_| refresh_claude_code_status()
                                                >
                                                    {if is_loading { "Checking..." } else { "Refresh Status" }}
                                                </button>

                                                {move || if status.installed && status.logged_in && !status.skill_installed {
                                                    view! {
                                                        <button
                                                            class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--bg-elevated)] text-[var(--text-secondary)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                                                            disabled=is_loading
                                                            on:click=move |_| {
                                                                spawn_local(async move {
                                                                    match claude_code_install_skill().await {
                                                                        Ok(_) => {
                                                                            show_success("Skill Installed", None);
                                                                            refresh_claude_code_status();
                                                                        }
                                                                        Err(e) => show_error("Install Failed", Some(&e), None),
                                                                    }
                                                                });
                                                            }
                                                        >
                                                            "Install Bridge Skill"
                                                        </button>
                                                    }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else if selected_provider.get() == LLMProvider::GeminiCli {
                                    // Gemini CLI status panel - capture signals for reactive updates
                                    view! {
                                        <div class="p-4 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] space-y-3">
                                            // Status indicators
                                            <div class="flex flex-wrap gap-2">
                                                <div class=move || {
                                                    let status = gemini_cli_status.get();
                                                    format!(
                                                        "flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium {}",
                                                        if status.is_installed { "bg-green-500/20 text-green-400" } else { "bg-red-500/20 text-red-400" }
                                                    )
                                                }>
                                                    <span class=move || {
                                                        let status = gemini_cli_status.get();
                                                        format!(
                                                            "w-2 h-2 rounded-full {}",
                                                            if status.is_installed { "bg-green-400" } else { "bg-red-400" }
                                                        )
                                                    }></span>
                                                    {move || if gemini_cli_status.get().is_installed { "CLI Installed" } else { "CLI Not Installed" }}
                                                </div>
                                                <div class=move || {
                                                    let status = gemini_cli_status.get();
                                                    format!(
                                                        "flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium {}",
                                                        if status.is_authenticated { "bg-green-500/20 text-green-400" } else { "bg-yellow-500/20 text-yellow-400" }
                                                    )
                                                }>
                                                    <span class=move || {
                                                        let status = gemini_cli_status.get();
                                                        format!(
                                                            "w-2 h-2 rounded-full {}",
                                                            if status.is_authenticated { "bg-green-400" } else { "bg-yellow-400" }
                                                        )
                                                    }></span>
                                                    {move || if gemini_cli_status.get().is_authenticated { "Authenticated" } else { "Not Authenticated" }}
                                                </div>
                                                <div class=move || {
                                                    let ext_status = gemini_cli_extension.get();
                                                    format!(
                                                        "flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium {}",
                                                        if ext_status.is_installed { "bg-green-500/20 text-green-400" } else { "bg-gray-500/20 text-gray-400" }
                                                    )
                                                }>
                                                    <span class=move || {
                                                        let ext_status = gemini_cli_extension.get();
                                                        format!(
                                                            "w-2 h-2 rounded-full {}",
                                                            if ext_status.is_installed { "bg-green-400" } else { "bg-gray-400" }
                                                        )
                                                    }></span>
                                                    {move || if gemini_cli_extension.get().is_installed { "Extension Installed" } else { "Extension Not Installed" }}
                                                </div>
                                            </div>

                                            // Status message
                                            {move || {
                                                let status = gemini_cli_status.get();
                                                (!status.message.is_empty()).then(|| view! {
                                                    <p class="text-xs text-[var(--text-muted)]">{status.message.clone()}</p>
                                                })
                                            }}

                                            // Action buttons
                                            <div class="flex flex-wrap gap-2 pt-2">
                                                {move || {
                                                    let status = gemini_cli_status.get();
                                                    let is_loading = gemini_cli_loading.get();
                                                    if !status.is_installed {
                                                        view! {
                                                            <a
                                                                href="https://github.com/google-gemini/gemini-cli"
                                                                target="_blank"
                                                                rel="noopener noreferrer"
                                                                class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--accent-primary)] text-white hover:opacity-90 transition-opacity"
                                                            >
                                                                "Install Gemini CLI"
                                                            </a>
                                                        }.into_any()
                                                    } else if !status.is_authenticated {
                                                        view! {
                                                            <button
                                                                class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--accent-primary)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
                                                                disabled=is_loading
                                                                on:click=move |_| {
                                                                    spawn_local(async move {
                                                                        match launch_gemini_cli_login().await {
                                                                            Ok(_) => show_success("Launching Gemini CLI", Some("Opening terminal for authentication...")),
                                                                            Err(e) => show_error("Launch Failed", Some(&e), None),
                                                                        }
                                                                    });
                                                                }
                                                            >
                                                                "Login with Google"
                                                            </button>
                                                        }.into_any()
                                                    } else {
                                                        view! { <span></span> }.into_any()
                                                    }
                                                }}

                                                <button
                                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--bg-elevated)] text-[var(--text-secondary)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                                                    disabled=move || gemini_cli_loading.get()
                                                    on:click=move |_| refresh_gemini_cli_status()
                                                >
                                                    {move || if gemini_cli_loading.get() { "Checking..." } else { "Refresh Status" }}
                                                </button>

                                                {move || {
                                                    let status = gemini_cli_status.get();
                                                    let ext_status = gemini_cli_extension.get();
                                                    let is_loading = gemini_cli_loading.get();
                                                    if status.is_installed && status.is_authenticated && !ext_status.is_installed {
                                                        view! {
                                                            <button
                                                                class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--bg-elevated)] text-[var(--text-secondary)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                                                                disabled=is_loading
                                                                on:click=move |_| {
                                                                    spawn_local(async move {
                                                                        // Install extension from GitHub repository
                                                                        match install_gemini_cli_extension(SIDECAR_DM_EXTENSION_URL.to_string()).await {
                                                                            Ok(msg) => {
                                                                                show_success("Extension Installed", Some(&msg));
                                                                                refresh_gemini_cli_status();
                                                                            }
                                                                            Err(e) => show_error("Install Failed", Some(&e), None),
                                                                        }
                                                                    });
                                                                }
                                                            >
                                                                "Install Sidecar DM Extension"
                                                            </button>
                                                        }.into_any()
                                                    } else {
                                                        view! { <span></span> }.into_any()
                                                    }
                                                }}
                                            </div>
                                        </div>
                                    }.into_any()
                                } else if selected_provider.get() == LLMProvider::ClaudeGate {
                                    // Claude Gate OAuth status panel
                                    // Note: Signals are accessed directly in closures for reactivity
                                    let _status = claude_gate_status.get();
                                    let _is_loading = claude_gate_loading.get();
                                    view! {
                                        <div class="p-4 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] space-y-3">
                                            // Status indicators
                                            <div class="flex flex-wrap gap-2">
                                                <div class=move || {
                                                    let s = claude_gate_status.get();
                                                    format!(
                                                        "flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium {}",
                                                        if s.authenticated { "bg-green-500/20 text-green-400" } else { "bg-yellow-500/20 text-yellow-400" }
                                                    )
                                                }>
                                                    <span class=move || {
                                                        let s = claude_gate_status.get();
                                                        format!(
                                                            "w-2 h-2 rounded-full {}",
                                                            if s.authenticated { "bg-green-400" } else { "bg-yellow-400" }
                                                        )
                                                    }></span>
                                                    {move || if claude_gate_status.get().authenticated { "Authenticated" } else { "Not Authenticated" }}
                                                </div>
                                                <div class="flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium bg-blue-500/20 text-blue-400">
                                                    {move || format!("Storage: {}", claude_gate_status.get().storage_backend)}
                                                </div>
                                                {move || claude_gate_status.get().expiration_display.map(|exp| view! {
                                                    <div class="flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-medium bg-gray-500/20 text-gray-400">
                                                        {format!("Expires: {}", exp)}
                                                    </div>
                                                })}
                                            </div>

                                            // Error message if any
                                            {move || claude_gate_status.get().error.map(|e| view! {
                                                <p class="text-xs text-red-400">{e}</p>
                                            })}

                                            // Storage backend selector
                                            <div class="pt-2">
                                                <label class="block text-xs text-[var(--text-muted)] mb-1">"Token Storage"</label>
                                                <select
                                                    class="w-full p-2 rounded-lg bg-[var(--bg-elevated)] border border-[var(--border-subtle)] text-[var(--text-primary)] text-xs outline-none focus:border-[var(--accent-primary)]"
                                                    style="color-scheme: dark;"
                                                    prop:value=move || claude_gate_storage.get().to_string()
                                                    on:change=move |ev| {
                                                        let val = event_target_value(&ev);
                                                        let new_backend = match val.as_str() {
                                                            "Keyring" => ClaudeGateStorageBackend::Keyring,
                                                            "File" => ClaudeGateStorageBackend::File,
                                                            _ => ClaudeGateStorageBackend::Auto,
                                                        };
                                                        // Capture previous value for rollback on failure
                                                        let previous_backend = claude_gate_storage.get();
                                                        // Optimistic update
                                                        claude_gate_storage.set(new_backend.clone());
                                                        spawn_local(async move {
                                                            if let Err(e) = claude_gate_set_storage_backend(new_backend).await {
                                                                // Rollback to previous value on failure
                                                                claude_gate_storage.set(previous_backend);
                                                                show_error("Storage Change Failed", Some(&e), None);
                                                            }
                                                        });
                                                    }
                                                >
                                                    <option value="Auto" class="bg-[var(--bg-elevated)] text-[var(--text-primary)]">"Auto (recommended)"</option>
                                                    <option value="Keyring" class="bg-[var(--bg-elevated)] text-[var(--text-primary)]">"System Keyring"</option>
                                                    <option value="File" class="bg-[var(--bg-elevated)] text-[var(--text-primary)]">"File (~/.config/cld/auth.json)"</option>
                                                </select>
                                            </div>

                                            // Action buttons and auth code input
                                            <div class="flex flex-col gap-3 pt-2">
                                                // Auth code input (shown when awaiting code)
                                                {move || {
                                                    if claude_gate_awaiting_code.get() {
                                                        view! {
                                                            <div class="flex flex-col gap-2 p-3 rounded-lg bg-[var(--bg-elevated)] border border-[var(--border-subtle)]">
                                                                // Show OAuth URL if available (for manual copy when popup blocked)
                                                                {move || {
                                                                    if let Some(url) = claude_gate_oauth_url.get() {
                                                                        view! {
                                                                            <div class="flex flex-col gap-1">
                                                                                <p class="text-xs text-[var(--text-secondary)]">
                                                                                    "If the browser didn't open, copy this URL:"
                                                                                </p>
                                                                                <div class="flex gap-2 items-center">
                                                                                    <input
                                                                                        type="text"
                                                                                        readonly=true
                                                                                        class="flex-1 px-2 py-1 text-xs rounded bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-muted)] font-mono truncate"
                                                                                        prop:value=url.clone()
                                                                                    />
                                                                                    <button
                                                                                        class="px-2 py-1 text-xs rounded bg-[var(--accent-primary)]/20 text-[var(--accent-primary)] hover:bg-[var(--accent-primary)]/30"
                                                                                        on:click={
                                                                                            let url_copy = url.clone();
                                                                                            move |_| {
                                                                                                if let Some(window) = web_sys::window() {
                                                                                                    let clipboard = window.navigator().clipboard();
                                                                                                    let url_to_copy = url_copy.clone();
                                                                                                    spawn_local(async move {
                                                                                                        let _ = wasm_bindgen_futures::JsFuture::from(
                                                                                                            clipboard.write_text(&url_to_copy)
                                                                                                        ).await;
                                                                                                        show_success("Copied", Some("URL copied to clipboard"));
                                                                                                    });
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    >
                                                                                        "Copy"
                                                                                    </button>
                                                                                </div>
                                                                            </div>
                                                                        }.into_any()
                                                                    } else {
                                                                        view! { <div></div> }.into_any()
                                                                    }
                                                                }}
                                                                <p class="text-xs text-[var(--text-secondary)]">
                                                                    "After authorizing in your browser, paste the authorization code here:"
                                                                </p>
                                                                <div class="flex gap-2">
                                                                    <input
                                                                        type="text"
                                                                        placeholder="Paste authorization code..."
                                                                        class="flex-1 px-3 py-1.5 text-sm rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:outline-none focus:border-[var(--accent-primary)]"
                                                                        prop:value=move || claude_gate_auth_code.get()
                                                                        on:input=move |ev| {
                                                                            claude_gate_auth_code.set(event_target_value(&ev));
                                                                        }
                                                                    />
                                                                    <button
                                                                        class="px-3 py-1.5 text-xs font-medium rounded-lg bg-green-500/20 text-green-400 hover:bg-green-500/30 transition-colors disabled:opacity-50"
                                                                        disabled=move || claude_gate_loading.get() || claude_gate_auth_code.get().is_empty()
                                                                        on:click=move |_| {
                                                                            let code = claude_gate_auth_code.get();
                                                                            let csrf_state = claude_gate_csrf_state.get();
                                                                            spawn_local(async move {
                                                                                claude_gate_loading.set(true);
                                                                                match claude_gate_complete_oauth(code, csrf_state).await {
                                                                                    Ok(result) => {
                                                                                        if result.success {
                                                                                            show_success("Login Complete", Some("Successfully authenticated with Claude"));
                                                                                            claude_gate_awaiting_code.set(false);
                                                                                            claude_gate_auth_code.set(String::new());
                                                                                            claude_gate_oauth_url.set(None);
                                                                                            claude_gate_csrf_state.set(None);
                                                                                            refresh_claude_gate_status();
                                                                                        } else {
                                                                                            show_error("OAuth Failed", result.error.as_deref(), None);
                                                                                        }
                                                                                    }
                                                                                    Err(e) => show_error("OAuth Failed", Some(&e), None),
                                                                                }
                                                                                claude_gate_loading.set(false);
                                                                            });
                                                                        }
                                                                    >
                                                                        "Complete Login"
                                                                    </button>
                                                                    <button
                                                                        class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--bg-surface)] text-[var(--text-muted)] hover:bg-[var(--bg-elevated)] transition-colors"
                                                                        on:click=move |_| {
                                                                            claude_gate_awaiting_code.set(false);
                                                                            claude_gate_auth_code.set(String::new());
                                                                            claude_gate_oauth_url.set(None);
                                                                            claude_gate_csrf_state.set(None);
                                                                        }
                                                                    >
                                                                        "Cancel"
                                                                    </button>
                                                                </div>
                                                            </div>
                                                        }.into_any()
                                                    } else {
                                                        view! { <span /> }.into_any()
                                                    }
                                                }}

                                                // Main action buttons
                                                <div class="flex flex-wrap gap-2">
                                                    {move || {
                                                        let s = claude_gate_status.get();
                                                        let loading = claude_gate_loading.get();
                                                        let awaiting = claude_gate_awaiting_code.get();
                                                        if !s.authenticated && !awaiting {
                                                            view! {
                                                                <button
                                                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--accent-primary)] text-white hover:opacity-90 transition-opacity disabled:opacity-50"
                                                                    disabled=loading
                                                                    on:click=move |_| {
                                                                        spawn_local(async move {
                                                                            claude_gate_loading.set(true);
                                                                            match claude_gate_start_oauth().await {
                                                                                Ok(response) => {
                                                                                    // Store URL for display if browser fails to open
                                                                                    claude_gate_oauth_url.set(Some(response.auth_url.clone()));
                                                                                    // Store CSRF state for verification
                                                                                    claude_gate_csrf_state.set(Some(response.state));
                                                                                    // Open URL using Tauri's shell plugin
                                                                                    match open_url_in_browser(response.auth_url).await {
                                                                                        Ok(_) => {
                                                                                            show_success("Login Started", Some("Complete authentication in your browser, then paste the code below"));
                                                                                            claude_gate_awaiting_code.set(true);
                                                                                        }
                                                                                        Err(e) => {
                                                                                            show_error("Browser Open Failed", Some(&format!("{}. Copy the URL shown below.", e)), None);
                                                                                            claude_gate_awaiting_code.set(true);
                                                                                        }
                                                                                    }
                                                                                }
                                                                                Err(e) => show_error("OAuth Failed", Some(&e), None),
                                                                            }
                                                                            claude_gate_loading.set(false);
                                                                        });
                                                                    }
                                                                >
                                                                    "Login with Claude"
                                                                </button>
                                                            }.into_any()
                                                        } else if s.authenticated {
                                                            view! {
                                                                <button
                                                                    class="px-3 py-1.5 text-xs font-medium rounded-lg bg-red-500/20 text-red-400 hover:bg-red-500/30 transition-colors disabled:opacity-50"
                                                                    disabled=loading
                                                                    on:click=move |_| {
                                                                        spawn_local(async move {
                                                                            claude_gate_loading.set(true);
                                                                            match claude_gate_logout().await {
                                                                                Ok(_) => {
                                                                                    show_success("Logged Out", None);
                                                                                    refresh_claude_gate_status();
                                                                                }
                                                                                Err(e) => show_error("Logout Failed", Some(&e), None),
                                                                            }
                                                                            claude_gate_loading.set(false);
                                                                        });
                                                                    }
                                                                >
                                                                    "Logout"
                                                                </button>
                                                            }.into_any()
                                                        } else {
                                                            view! { <span /> }.into_any()
                                                        }
                                                    }}

                                                    <button
                                                        class="px-3 py-1.5 text-xs font-medium rounded-lg bg-[var(--bg-elevated)] text-[var(--text-secondary)] hover:bg-[var(--bg-surface)] transition-colors disabled:opacity-50"
                                                        disabled=move || claude_gate_loading.get()
                                                        on:click=move |_| refresh_claude_gate_status()
                                                    >
                                                        {move || if claude_gate_loading.get() { "Checking..." } else { "Refresh Status" }}
                                                    </button>
                                                </div>
                                            </div>

                                            // Link to extraction settings
                                            <div class="pt-2 border-t border-[var(--border-subtle)]">
                                                <p class="text-xs text-[var(--text-muted)]">
                                                    "Claude Gate can also be used for document extraction. Configure in "
                                                    <span class="text-[var(--accent-primary)]">"Extraction Settings"</span>
                                                    "."
                                                </p>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    // Regular input for other providers
                                    view! {
                                        <Input
                                            value=api_key_or_host
                                            placeholder=Signal::derive(move || selected_provider.get().placeholder_text().to_string())
                                            r#type=Signal::derive(move || if matches!(selected_provider.get(), LLMProvider::Ollama | LLMProvider::ClaudeDesktop) { "text".to_string() } else { "password".to_string() })
                                            disabled=Signal::derive(move || matches!(selected_provider.get(), LLMProvider::ClaudeDesktop))
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
                                                    view! { <option value=m.id.clone() class="bg-[var(--bg-elevated)] text-[var(--text-primary)]">{m.id.clone()}</option> }
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
                            } else {
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
                                    let result = if provider == EmbedderProvider::Ollama {
                                        let host = api_key_or_host.get_untracked();
                                        let host = if host.is_empty() { "http://localhost:11434".to_string() } else { host };
                                        setup_ollama_embeddings(host, model.clone()).await
                                    } else {
                                        setup_local_embeddings(model.clone()).await
                                    };

                                    match result {
                                        Ok(result) => {
                                            embeddings_status.set(format!(
                                                "✓ Configured {} indexes with {} ({}D)",
                                                result.indexes_configured.len(),
                                                result.model,
                                                result.dimensions
                                            ));
                                            show_success(
                                                "Embeddings Configured",
                                                Some(&format!(
                                                    "AI-powered search enabled on {} indexes using {}",
                                                    result.indexes_configured.len(),
                                                    result.model
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

            // Proxy Status Card
            <Card class="p-4 mt-4">
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-3">
                        <div class=move || {
                            if proxy_running.get() {
                                "w-2 h-2 rounded-full bg-green-500"
                            } else {
                                "w-2 h-2 rounded-full bg-red-500"
                            }
                        }></div>
                        <div>
                            <span class="text-sm font-medium text-[var(--text-primary)]">"LLM Proxy"</span>
                            <span class="text-xs text-[var(--text-muted)] ml-2">
                                {move || if proxy_running.get() { "Running" } else { "Stopped" }}
                            </span>
                        </div>
                    </div>
                    <div class="text-xs text-[var(--text-muted)]">
                        {move || proxy_url.get()}
                    </div>
                </div>
                {move || {
                    let providers = proxy_providers.get();
                    if !providers.is_empty() {
                        view! {
                            <div class="mt-2 pt-2 border-t border-[var(--border-subtle)]">
                                <span class="text-xs text-[var(--text-muted)]">"Registered: "</span>
                                <span class="text-xs text-[var(--text-secondary)]">
                                    {providers.join(", ")}
                                </span>
                            </div>
                        }.into_any()
                    } else {
                        view! { <span/> }.into_any()
                    }
                }}
            </Card>
        </div>
    }
}
