#![allow(non_snake_case)]
use dioxus::prelude::*;

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

#[component]
pub fn Settings() -> Element {
    let mut selected_provider = use_signal(|| LLMProvider::Ollama);
    let mut api_key_or_host = use_signal(|| "http://localhost:11434".to_string());
    let mut model_name = use_signal(|| "llama3.2".to_string());
    let mut save_status = use_signal(|| String::new());

    let save_settings = move |_: MouseEvent| {
        // TODO: Call Tauri backend to save settings securely
        spawn(async move {
            // let result = invoke("save_llm_settings", settings).await;
            save_status.set("Settings saved! (Placeholder)".to_string());
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
                                        selected_provider.set(match value.as_str() {
                                            "Claude" => LLMProvider::Claude,
                                            "Gemini" => LLMProvider::Gemini,
                                            _ => LLMProvider::Ollama,
                                        });
                                    },
                                    option { value: "Ollama", "Ollama (Local)" }
                                    option { value: "Claude", "Claude (Anthropic)" }
                                    option { value: "Gemini", "Gemini (Google)" }
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
                                    placeholder: "llama3.2 / claude-3-sonnet / gemini-pro",
                                    value: "{model_name}",
                                    oninput: move |e| model_name.set(e.value())
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
                        span { class: "text-sm text-green-400", "{save_status}" }
                        button {
                            class: "px-6 py-2 bg-green-600 rounded hover:bg-green-500 transition-colors font-medium",
                            onclick: save_settings,
                            "Save Changes"
                        }
                    }
                }
            }
        }
    }
}
