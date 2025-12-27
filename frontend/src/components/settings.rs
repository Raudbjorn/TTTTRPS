#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::bindings::{configure_llm, get_llm_config, save_api_key, check_llm_health, LLMSettings, HealthStatus};

#[derive(Clone, PartialEq)]
pub enum LLMProvider {
    Ollama,
    Claude,
    Gemini,
}

impl std::fmt::Display for LLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProvider::Ollama => write!(f, "Ollama"),
            LLMProvider::Claude => write!(f, "Claude"),
            LLMProvider::Gemini => write!(f, "Gemini"),
        }
    }
}

impl LLMProvider {
    fn to_string_key(&self) -> String {
        match self {
            LLMProvider::Ollama => "ollama".to_string(),
            LLMProvider::Claude => "claude".to_string(),
            LLMProvider::Gemini => "gemini".to_string(),
        }
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

    // Load existing config on mount
    use_effect(move || {
        spawn(async move {
            if let Ok(Some(config)) = get_llm_config().await {
                match config.provider.as_str() {
                    "ollama" => {
                        selected_provider.set(LLMProvider::Ollama);
                        api_key_or_host.set(config.host.unwrap_or_default());
                    }
                    "claude" => {
                        selected_provider.set(LLMProvider::Claude);
                        api_key_or_host.set(String::new()); // Don't show masked key
                    }
                    "gemini" => {
                        selected_provider.set(LLMProvider::Gemini);
                        api_key_or_host.set(String::new());
                    }
                    _ => {}
                }
                model_name.set(config.model);
                if let Some(emb) = config.embedding_model {
                    embedding_model.set(emb);
                }
            }

            // Check health
            if let Ok(status) = check_llm_health().await {
                health_status.set(Some(status));
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
        });
    };

    let placeholder_text = match *selected_provider.read() {
        LLMProvider::Ollama => "http://localhost:11434",
        LLMProvider::Claude => "sk-ant-...",
        LLMProvider::Gemini => "AIza...",
    };

    let label_text = match *selected_provider.read() {
        LLMProvider::Ollama => "Ollama Host",
        LLMProvider::Claude => "Claude API Key",
        LLMProvider::Gemini => "Gemini API Key",
    };

    let show_embedding_model = matches!(*selected_provider.read(), LLMProvider::Ollama);

    rsx! {
        div {
            class: "p-8 bg-gray-900 text-white min-h-screen font-sans",
            div {
                class: "max-w-2xl mx-auto",
                div {
                    class: "flex items-center mb-8",
                    Link { to: crate::Route::Chat {}, class: "mr-4 text-gray-400 hover:text-white", "← Back to Chat" }
                    h1 { class: "text-2xl font-bold", "Settings" }
                }

                div {
                    class: "bg-gray-800 rounded-lg p-6 space-y-6",

                    // Connection Status
                    if let Some(status) = health_status.read().as_ref() {
                        div {
                            class: if status.healthy { "p-3 bg-green-900 border border-green-700 rounded-lg" } else { "p-3 bg-yellow-900 border border-yellow-700 rounded-lg" },
                            div { class: "flex items-center gap-2",
                                div {
                                    class: if status.healthy { "w-3 h-3 bg-green-500 rounded-full" } else { "w-3 h-3 bg-yellow-500 rounded-full" }
                                }
                                span {
                                    class: if status.healthy { "text-green-300" } else { "text-yellow-300" },
                                    "{status.message}"
                                }
                            }
                        }
                    }

                    div {
                        h2 { class: "text-xl font-semibold mb-4", "LLM Configuration" }

                        div {
                            class: "space-y-4",
                            // Provider Selection
                            div {
                                label { class: "block text-sm font-medium text-gray-400 mb-1", "Provider" }
                                select {
                                    class: "block w-full p-2 bg-gray-700 border border-gray-600 rounded text-white focus:border-blue-500 outline-none",
                                    onchange: move |e| {
                                        let value = e.value();
                                        let provider = match value.as_str() {
                                            "Claude" => LLMProvider::Claude,
                                            "Gemini" => LLMProvider::Gemini,
                                            _ => LLMProvider::Ollama,
                                        };
                                        selected_provider.set(provider.clone());
                                        // Reset the input based on provider
                                        match provider {
                                            LLMProvider::Ollama => {
                                                api_key_or_host.set("http://localhost:11434".to_string());
                                                model_name.set("llama3.2".to_string());
                                            }
                                            LLMProvider::Claude => {
                                                api_key_or_host.set(String::new());
                                                model_name.set("claude-3-5-sonnet-20241022".to_string());
                                            }
                                            LLMProvider::Gemini => {
                                                api_key_or_host.set(String::new());
                                                model_name.set("gemini-pro".to_string());
                                            }
                                        }
                                    },
                                    option { value: "Ollama", selected: matches!(*selected_provider.read(), LLMProvider::Ollama), "Ollama (Local)" }
                                    option { value: "Claude", selected: matches!(*selected_provider.read(), LLMProvider::Claude), "Claude (Anthropic)" }
                                    option { value: "Gemini", selected: matches!(*selected_provider.read(), LLMProvider::Gemini), "Gemini (Google)" }
                                }
                            }

                            // API Key / Host
                            div {
                                label { class: "block text-sm font-medium text-gray-400 mb-1", "{label_text}" }
                                input {
                                    class: "block w-full p-2 bg-gray-700 border border-gray-600 rounded text-white focus:border-blue-500 outline-none",
                                    r#type: if matches!(*selected_provider.read(), LLMProvider::Ollama) { "text" } else { "password" },
                                    placeholder: "{placeholder_text}",
                                    value: "{api_key_or_host}",
                                    oninput: move |e| api_key_or_host.set(e.value())
                                }
                            }

                            // Model Name
                            div {
                                label { class: "block text-sm font-medium text-gray-400 mb-1", "Model" }
                                input {
                                    class: "block w-full p-2 bg-gray-700 border border-gray-600 rounded text-white focus:border-blue-500 outline-none",
                                    placeholder: "llama3.2 / claude-3-5-sonnet / gemini-pro",
                                    value: "{model_name}",
                                    oninput: move |e| model_name.set(e.value())
                                }
                            }

                            // Embedding Model (Ollama only)
                            if show_embedding_model {
                                div {
                                    label { class: "block text-sm font-medium text-gray-400 mb-1", "Embedding Model" }
                                    input {
                                        class: "block w-full p-2 bg-gray-700 border border-gray-600 rounded text-white focus:border-blue-500 outline-none",
                                        placeholder: "nomic-embed-text",
                                        value: "{embedding_model}",
                                        oninput: move |e| embedding_model.set(e.value())
                                    }
                                }
                            }
                        }
                    }

                    // Voice Settings Section
                    div {
                        h2 { class: "text-xl font-semibold mb-4", "Voice Settings" }
                        div {
                            class: "p-4 bg-gray-700 rounded text-center text-gray-400",
                            "Voice configuration coming soon..."
                        }
                    }

                    // Save Button
                    div {
                        class: "pt-4 border-t border-gray-700 flex justify-between items-center",
                        div {
                            class: "flex-1",
                            if !save_status.read().is_empty() {
                                p {
                                    class: if save_status.read().contains("Error") || save_status.read().contains("failed") {
                                        "text-sm text-red-400"
                                    } else {
                                        "text-sm text-green-400"
                                    },
                                    "{save_status}"
                                }
                            }
                        }
                        div {
                            class: "flex gap-2",
                            button {
                                class: "px-4 py-2 bg-gray-600 rounded hover:bg-gray-500 transition-colors",
                                onclick: test_connection,
                                "Test Connection"
                            }
                            button {
                                class: if *is_saving.read() { "px-6 py-2 bg-gray-600 rounded cursor-not-allowed font-medium" } else { "px-6 py-2 bg-green-600 rounded hover:bg-green-500 transition-colors font-medium" },
                                onclick: save_settings,
                                disabled: *is_saving.read(),
                                if *is_saving.read() { "Saving..." } else { "Save Changes" }
                            }
                        }
                    }
                }
            }
        }
    }
}
