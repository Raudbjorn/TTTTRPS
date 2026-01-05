use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;
use crate::bindings::{
    configure_voice, get_voice_config, list_elevenlabs_voices, list_openai_tts_models,
    list_openai_voices, check_llm_health,
    ElevenLabsConfig, OllamaConfig, OpenAIVoiceConfig, Voice, VoiceConfig,
};
use crate::components::design_system::{Badge, BadgeVariant, Button, ButtonVariant, Card, Input};
use crate::services::notification_service::{show_error, show_success};

#[component]
pub fn VoiceSettingsView() -> impl IntoView {
    // Signals
    let selected_voice_provider = RwSignal::new("Disabled".to_string());
    let voice_api_key_or_host = RwSignal::new(String::new());
    let voice_model_id = RwSignal::new(String::new());
    let selected_voice_id = RwSignal::new(String::new());

    let available_voices = RwSignal::new(Vec::<Voice>::new());
    let openai_tts_models = RwSignal::new(Vec::<(String, String)>::new());

    let is_loading_voices = RwSignal::new(false);
    let save_status = RwSignal::new(String::new());
    let is_saving = RwSignal::new(false);

    // Helpers
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

    // On Mount
    Effect::new(move |_| {
        spawn_local(async move {
             if let Ok(config) = get_voice_config().await {
                let provider_str = match config.provider.as_str() {
                    "ElevenLabs" => "ElevenLabs",
                    "Ollama" => "Ollama",
                    "FishAudio" => "FishAudio", // FishAudio support exists in backend
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
                                fetch_voices("ElevenLabs".to_string(), Some(c.api_key));
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
                        fetch_voices("OpenAI".to_string(), None);
                    }
                    _ => {}
                }
            }
        });
    });

    // Handlers
    let handle_save = move |_: ev::MouseEvent| {
        is_saving.set(true);
        save_status.set("Saving...".to_string());

        let provider = selected_voice_provider.get();
        let val = voice_api_key_or_host.get();
        let model = voice_model_id.get();
        let voice = selected_voice_id.get();

        spawn_local(async move {
             let voice_config = if provider == "Disabled" {
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
                    provider: provider.clone(),
                    cache_dir: None,
                    default_voice_id: None,
                    elevenlabs: None,
                    fish_audio: None,
                    ollama: None,
                    openai: None,
                };

                match provider.as_str() {
                    "ElevenLabs" => {
                        base.elevenlabs = Some(ElevenLabsConfig {
                            api_key: val,
                            model_id: Some(model),
                        });
                    }
                    "Ollama" => {
                        base.ollama = Some(OllamaConfig {
                            base_url: val,
                            model: model,
                        });
                    }
                    "OpenAI" => {
                        base.openai = Some(OpenAIVoiceConfig {
                            api_key: val,
                            model: model,
                            voice: voice,
                        });
                    }
                    _ => {}
                }
                base
            };

            match configure_voice(voice_config).await {
                Ok(_) => {
                    save_status.set("Voice Settings Saved".to_string());
                    show_success("Voice Configured", Some("Settings saved successfully."));
                }
                Err(e) => {
                     save_status.set(format!("Error: {}", e));
                     show_error("Validation Failed", Some(&e), None);
                }
            }
            is_saving.set(false);
        });
    };

    let handle_provider_change = move |val: String| {
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
            _ => {
                 voice_api_key_or_host.set(String::new());
                 voice_model_id.set(String::new());
            }
        }
    };

    let providers = vec!["Disabled", "Ollama", "ElevenLabs", "OpenAI"]; // FishAudio?

    view! {
         <div class="space-y-8 animate-fade-in pb-20">
            <div class="space-y-2">
                <h3 class="text-xl font-bold text-[var(--text-primary)]">"Voice & Audio"</h3>
                <p class="text-[var(--text-muted)]">"Manage Text-to-Speech engines and voice clones."</p>
            </div>

            <Card class="p-6">
                 <div class="grid grid-cols-1 gap-6">
                    <div>
                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">"Provider"</label>
                        <select
                            class="w-full p-3 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)]"
                            prop:value=selected_voice_provider
                            on:change=move |ev| handle_provider_change(event_target_value(&ev))
                        >
                            {providers.into_iter().map(|p| {
                                view! { <option value=p.to_string()>{p}</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                    </div>

                    {move || match selected_voice_provider.get().as_str() {
                        "Disabled" => view! { <div class="text-[var(--text-muted)] italic">"Voice disabled."</div> }.into_any(),
                        provider => {
                            view! {
                                <div class="space-y-4 border-t border-[var(--border-subtle)] pt-4 animate-fade-in">
                                    // Host / API Key
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">
                                            {if provider == "Ollama" { "Base URL" } else { "API Key" }}
                                        </label>
                                        <Input
                                            value=voice_api_key_or_host
                                            r#type=if provider == "Ollama" { "text".to_string() } else { "password".to_string() }
                                            placeholder=if provider == "Ollama" { "http://localhost:11434" } else { "sk-..." }
                                        />
                                    </div>

                                    // Model Selection
                                    <div>
                                        <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">"Model ID"</label>
                                        <Input value=voice_model_id />
                                    </div>

                                    // Voice Selection (if loaded)
                                    {
                                        let voices = available_voices.get();
                                        if !voices.is_empty() {
                                            view! {
                                                <div>
                                                    <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">"Voice Persona"</label>
                                                    <select
                                                        class="w-full p-3 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] text-[var(--text-primary)] outline-none focus:border-[var(--accent-primary)]"
                                                        prop:value=selected_voice_id
                                                        on:change=move |ev| selected_voice_id.set(event_target_value(&ev))
                                                    >
                                                        {voices.into_iter().map(|v| {
                                                            view! { <option value=v.id.clone()>{v.name}</option> }
                                                        }).collect::<Vec<_>>()}
                                                    </select>
                                                </div>
                                            }.into_any()
                                        } else {
                                           view! { <span/> }.into_any()
                                        }
                                    }
                                </div>
                            }.into_any()
                        }
                    }}

                     <div class="pt-4">
                        <Button
                            variant=ButtonVariant::Primary
                            loading=is_saving
                            on_click=handle_save
                        >
                            "Save Voice Settings"
                        </Button>
                    </div>
                </div>
            </Card>
        </div>
    }
}
