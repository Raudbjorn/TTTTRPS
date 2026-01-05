use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;
use std::collections::HashMap;
use gloo_timers::callback::Timeout;
use crate::bindings::{
    check_llm_health, configure_llm, get_llm_config, list_claude_models, list_gemini_models,
    list_ollama_models, list_openai_models, list_openrouter_models, list_provider_models,
    save_api_key, HealthStatus, LLMSettings, ModelInfo, OllamaModel,
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
        }
    }

    fn label_text(&self) -> &'static str {
        match self {
            LLMProvider::Ollama => "Ollama Host",
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
        }
    }

    fn brand_color(&self) -> &'static str {
        match self {
            LLMProvider::Claude => "text-orange-400", // Anthropic Sienna
            LLMProvider::Gemini => "text-blue-400", // Gemini Blue
            LLMProvider::OpenAI => "text-emerald-400", // OpenAI Green
            LLMProvider::Ollama => "text-white", // Ollama White
            LLMProvider::OpenRouter => "text-violet-400",
            _ => "text-[var(--accent-primary)]",
        }
    }
}

#[component]
pub fn LLMSettingsView() -> impl IntoView {
    // Signals
    let selected_provider = RwSignal::new(LLMProvider::Ollama);
    let api_key_or_host = RwSignal::new("http://localhost:11434".to_string());
    let model_name = RwSignal::new("llama3.2".to_string());
    let embedding_model = RwSignal::new("nomic-embed-text".to_string());
    let save_status = RwSignal::new(String::new());
    let is_saving = RwSignal::new(false);
    let health_status = RwSignal::new(Option::<HealthStatus>::None);
    let initial_load = RwSignal::new(true);
    let timeout_handle = StoredValue::new_local(None::<Timeout>);

    // Models
    let ollama_models = RwSignal::new(Vec::<OllamaModel>::new());
    let cloud_models = RwSignal::new(Vec::<ModelInfo>::new());
    let is_loading_models = RwSignal::new(false);

    // Statuses
    let provider_statuses = RwSignal::new(HashMap::<String, bool>::new());

    // --- Helpers ---

    let fetch_ollama_models = move |host: String| {
        spawn_local(async move {
            is_loading_models.set(true);
            match list_ollama_models(host).await {
                Ok(models) => {
                     ollama_models.set(models);
                     provider_statuses.update(|map| { map.insert("ollama".to_string(), true); });
                },
                Err(_) => {
                    ollama_models.set(Vec::new());
                    provider_statuses.update(|map| { map.insert("ollama".to_string(), false); });
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
            provider_statuses.set(statuses);
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
                 let key_to_save = if provider != LLMProvider::Ollama && !key_or_host.is_empty() {
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
        LLMProvider::Gemini,
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
                                {move || if selected_provider.get() == LLMProvider::Ollama {
                                    "Running locally on your machine."
                                } else {
                                    "Cloud-based inference."
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
                            <Input
                                value=api_key_or_host
                                placeholder=Signal::derive(move || selected_provider.get().placeholder_text().to_string())
                                r#type=Signal::derive(move || if selected_provider.get() == LLMProvider::Ollama { "text".to_string() } else { "password".to_string() })
                            />
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

                         {move || if selected_provider.get() == LLMProvider::Ollama {
                            view! {
                                <div>
                                    <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">"Embedding Model"</label>
                                    <Input value=embedding_model />
                                </div>
                            }.into_any()
                        } else {
                            view! { <span/> }.into_any()
                        }}

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
        </div>
    }
}
