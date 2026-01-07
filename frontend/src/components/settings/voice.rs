use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;
use gloo_timers::callback::Timeout;
use crate::bindings::{
    configure_voice, get_voice_config, list_elevenlabs_voices, list_openai_tts_models,
    list_openai_voices, list_all_voices, get_popular_piper_voices, download_piper_voice,
    ElevenLabsConfig, OllamaConfig, OpenAIVoiceConfig, Voice, VoiceConfig,
    PiperConfig, CoquiConfig, PopularPiperVoice,
};
use crate::components::design_system::{Card, Input, SelectRw, SelectOption, Button, ButtonVariant, Slider};
use crate::services::notification_service::{show_error, show_success};

#[component]
pub fn VoiceSettingsView() -> impl IntoView {
    // Signals
    let selected_voice_provider = RwSignal::new("Disabled".to_string());
    let voice_api_key_or_host = RwSignal::new(String::new());
    let piper_models_dir = RwSignal::new(String::new());
    let voice_model_id = RwSignal::new(String::new());
    let selected_voice_id = RwSignal::new(String::new());

    let available_voices = RwSignal::new(Vec::<Voice>::new());
    let openai_tts_models = RwSignal::new(Vec::<(String, String)>::new());

    let is_loading_voices = RwSignal::new(false);
    let save_status = RwSignal::new(String::new());
    let is_saving = RwSignal::new(false);
    let initial_load = RwSignal::new(true);
    let timeout_handle = StoredValue::new_local(None::<Timeout>);

    // Piper advanced settings
    let piper_length_scale = RwSignal::new(1.0_f32);
    let piper_noise_scale = RwSignal::new(0.667_f32);
    let piper_noise_w = RwSignal::new(0.8_f32);
    let piper_sentence_silence = RwSignal::new(0.2_f32);

    // Coqui advanced settings
    let coqui_speed = RwSignal::new(1.0_f32);
    let coqui_temperature = RwSignal::new(0.8_f32);
    let coqui_top_k = RwSignal::new(50.0_f32);
    let coqui_top_p = RwSignal::new(0.95_f32);
    let coqui_repetition_penalty = RwSignal::new(2.0_f32);

    // Show advanced settings toggle
    let show_advanced = RwSignal::new(false);

    // Piper voice download
    let popular_voices = RwSignal::new(Vec::<PopularPiperVoice>::new());
    let downloading_voice = RwSignal::new(Option::<String>::None);
    let download_error = RwSignal::new(Option::<String>::None);

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
                "Piper" | "Coqui" => {
                    if let Ok(voices) = list_all_voices().await {
                        let filtered: Vec<Voice> = voices.into_iter()
                            .filter(|v| v.provider.eq_ignore_ascii_case(provider.as_str()))
                            .collect();
                        available_voices.set(filtered);
                    }
                }
                _ => {
                    available_voices.set(Vec::new());
                }
            }
            is_loading_voices.set(false);
        });
    };

    // Track the saved voice ID from config (not user-changeable)
    let saved_voice_id = RwSignal::new(String::new());

    // On Mount - load existing config
    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(config) = get_voice_config().await {
                let provider_str = match config.provider.as_str() {
                    "ElevenLabs" => "ElevenLabs",
                    "Ollama" => "Ollama",
                    "FishAudio" => "FishAudio",
                    "OpenAI" => "OpenAI",
                    "Piper" => "Piper",
                    "Coqui" => "Coqui",
                    _ => "Disabled",
                };
                selected_voice_provider.set(provider_str.to_string());

                // Restore default voice for all providers
                if let Some(ref default_voice) = config.default_voice_id {
                    saved_voice_id.set(default_voice.clone());
                    selected_voice_id.set(default_voice.clone());
                }

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
                        }
                        fetch_voices("OpenAI".to_string(), None);
                    }
                    "Piper" => {
                        if let Some(c) = config.piper {
                            piper_models_dir.set(c.models_dir.unwrap_or_default());
                            piper_length_scale.set(c.length_scale);
                            piper_noise_scale.set(c.noise_scale);
                            piper_noise_w.set(c.noise_w);
                            piper_sentence_silence.set(c.sentence_silence);
                        }
                        fetch_voices("Piper".to_string(), None);
                        // Load popular voices for download
                        if let Ok(voices) = get_popular_piper_voices().await {
                            popular_voices.set(voices);
                        }
                    }
                    "Coqui" => {
                        if let Some(c) = config.coqui {
                            voice_api_key_or_host.set(format!("{}", c.port));
                            voice_model_id.set(c.model);
                            coqui_speed.set(c.speed);
                            coqui_temperature.set(c.temperature);
                            coqui_top_k.set(c.top_k as f32);
                            coqui_top_p.set(c.top_p);
                            coqui_repetition_penalty.set(c.repetition_penalty);
                        }
                        fetch_voices("Coqui".to_string(), None);
                    }
                    _ => {}
                }
            }
            initial_load.set(false);
        });
    });

    // Sync voice selection when available_voices changes
    // This ensures the saved voice is re-selected after the dropdown re-renders
    Effect::new(move |_| {
        let voices = available_voices.get();
        let saved = saved_voice_id.get_untracked();

        if !voices.is_empty() && !saved.is_empty() {
            // Check if saved voice exists in available voices
            if voices.iter().any(|v| v.id == saved) {
                // Use request_animation_frame to ensure DOM has rendered options first
                request_animation_frame(move || {
                    selected_voice_id.set(saved);
                });
            }
        }
    });

    // Auto-Save Effect
    Effect::new(move |_| {
        let provider = selected_voice_provider.get();
        let val = voice_api_key_or_host.get();
        let piper_dir = piper_models_dir.get();
        let model = voice_model_id.get();
        let voice = selected_voice_id.get();

        // Piper advanced settings (track for reactivity)
        let length_scale = piper_length_scale.get();
        let noise_scale = piper_noise_scale.get();
        let noise_w = piper_noise_w.get();
        let sentence_silence = piper_sentence_silence.get();

        // Coqui advanced settings (track for reactivity)
        let coqui_spd = coqui_speed.get();
        let coqui_temp = coqui_temperature.get();
        let coqui_tk = coqui_top_k.get();
        let coqui_tp = coqui_top_p.get();
        let coqui_rep = coqui_repetition_penalty.get();

        if initial_load.get_untracked() {
            return;
        }

        // Cancel any pending save
        timeout_handle.update_value(|h| { if let Some(t) = h.take() { t.cancel(); } });

        let perform_save = move || {
            is_saving.set(true);
            save_status.set("Saving...".to_string());

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
                        piper: None,
                        coqui: None,
                    }
                } else {
                    let mut base = VoiceConfig {
                        provider: provider.clone(),
                        cache_dir: None,
                        default_voice_id: if !voice.is_empty() { Some(voice.clone()) } else { None },
                        elevenlabs: None,
                        fish_audio: None,
                        ollama: None,
                        openai: None,
                        piper: None,
                        coqui: None,
                    };

                    match provider.as_str() {
                        "ElevenLabs" => {
                            base.elevenlabs = Some(ElevenLabsConfig {
                                api_key: val.clone(),
                                model_id: Some(model.clone()),
                            });
                        }
                        "Ollama" => {
                            base.ollama = Some(OllamaConfig {
                                base_url: val.clone(),
                                model: model.clone(),
                            });
                        }
                        "OpenAI" => {
                            base.openai = Some(OpenAIVoiceConfig {
                                api_key: val.clone(),
                                model: model.clone(),
                                voice: voice.clone(),
                            });
                        }
                        "Piper" => {
                            base.piper = Some(PiperConfig {
                                models_dir: if piper_dir.is_empty() { None } else { Some(piper_dir.clone()) },
                                length_scale: length_scale,
                                noise_scale: noise_scale,
                                noise_w: noise_w,
                                sentence_silence: sentence_silence,
                                speaker_id: 0,
                            });
                        }
                        "Coqui" => {
                            let port: u16 = val.parse().unwrap_or(5002);
                            base.coqui = Some(CoquiConfig {
                                port,
                                model: if model.is_empty() { "tts_models/en/ljspeech/vits".to_string() } else { model.clone() },
                                speaker: None,
                                language: None,
                                speed: coqui_spd,
                                speaker_wav: None,
                                temperature: coqui_temp,
                                top_k: coqui_tk as u32,
                                top_p: coqui_tp,
                                repetition_penalty: coqui_rep,
                            });
                        }
                        _ => {}
                    }
                    base
                };

                match configure_voice(voice_config).await {
                    Ok(_) => {
                        save_status.set("All changes saved".to_string());
                        // Update saved_voice_id to match what was just saved
                        if !voice.is_empty() {
                            saved_voice_id.set(voice.clone());
                        }
                    }
                    Err(e) => {
                        save_status.set(format!("Error: {}", e));
                        show_error("Save Failed", Some(&e), None);
                    }
                }
                is_saving.set(false);
            });
        };

        // Debounce: save after 1 second of no changes
        timeout_handle.set_value(Some(Timeout::new(1000, perform_save)));
    });

    on_cleanup(move || {
        timeout_handle.update_value(|h| { if let Some(t) = h.take() { t.cancel(); } });
    });

    // Provider change handler
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
            "Piper" => {
                piper_models_dir.set(String::new());
                voice_model_id.set(String::new());
                fetch_voices("Piper".to_string(), None);
                // Load popular voices for download
                spawn_local(async move {
                    if let Ok(voices) = get_popular_piper_voices().await {
                        popular_voices.set(voices);
                    }
                });
            }
            "Coqui" => {
                voice_api_key_or_host.set("5002".to_string());
                voice_model_id.set("tts_models/en/ljspeech/vits".to_string());
                fetch_voices("Coqui".to_string(), None);
            }
            _ => {
                voice_api_key_or_host.set(String::new());
                voice_model_id.set(String::new());
                piper_models_dir.set(String::new());
            }
        }
    };

    let providers = vec!["Disabled", "Ollama", "ElevenLabs", "OpenAI", "Piper", "Coqui"];

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
                        <SelectRw
                            value=selected_voice_provider
                            on_change=Callback::new(move |val: String| handle_provider_change(val))
                        >
                            {providers.into_iter().map(|p| {
                                view! { <SelectOption value=p.to_string() /> }
                            }).collect::<Vec<_>>()}
                        </SelectRw>
                    </div>

                    {move || {
                        let provider = selected_voice_provider.get();
                        match provider.as_str() {
                            "Disabled" => view! { <div class="text-[var(--text-muted)] italic">"Voice disabled."</div> }.into_any(),
                            _ => {
                                let provider = provider.clone();
                                let is_piper = provider == "Piper";
                                let is_password = provider == "OpenAI" || provider == "ElevenLabs";
                                let label = match provider.as_str() {
                                    "Ollama" => "Base URL",
                                    "ElevenLabs" | "OpenAI" => "API Key",
                                    "Coqui" => "Port",
                                    _ => "Configuration"
                                };
                                let placeholder = match provider.as_str() {
                                    "Ollama" => "http://localhost:11434",
                                    "OpenAI" | "ElevenLabs" => "sk-...",
                                    "Coqui" => "5002",
                                    _ => ""
                                };
                                view! {
                                <div class="space-y-4 border-t border-[var(--border-subtle)] pt-4 animate-fade-in">
                                    <Show when=move || !is_piper>
                                        <div>
                                            <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">
                                                {label}
                                            </label>
                                            <Input
                                                value=voice_api_key_or_host
                                                r#type=if is_password { "password".to_string() } else { "text".to_string() }
                                                placeholder=placeholder
                                            />
                                        </div>
                                    </Show>

                                    <Show when=move || is_piper>
                                        <div>
                                            <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">"Models Directory (Optional)"</label>
                                            <Input
                                                value=piper_models_dir
                                                placeholder="/path/to/piper/models"
                                            />
                                        </div>
                                    </Show>

                                    <Show when=move || !is_piper>
                                        <div>
                                            <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">"Model ID"</label>
                                            <Input value=voice_model_id />
                                        </div>
                                    </Show>

                                    {
                                        let voices = available_voices.get();
                                        if !voices.is_empty() {
                                            view! {
                                                <div>
                                                    <label class="block text-sm font-medium text-[var(--text-secondary)] mb-2">"Voice Persona"</label>
                                                    <SelectRw
                                                        value=selected_voice_id
                                                    >
                                                        {voices.into_iter().map(|v| {
                                                            view! { <SelectOption value=v.id.clone() label=v.name /> }
                                                        }).collect::<Vec<_>>()}
                                                    </SelectRw>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <span/> }.into_any()
                                        }
                                    }

                                    // Advanced Settings Toggle
                                    <div class="pt-2">
                                        <button
                                            type="button"
                                            class="flex items-center gap-2 text-sm text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors"
                                            on:click=move |_| show_advanced.update(|v| *v = !*v)
                                        >
                                            <span class=move || if show_advanced.get() { "transform rotate-90 transition-transform" } else { "transition-transform" }>
                                                "▶"
                                            </span>
                                            "Advanced Settings"
                                        </button>
                                    </div>

                                    // Piper Advanced Settings
                                    <Show when=move || is_piper && show_advanced.get()>
                                        <div class="space-y-4 p-4 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] animate-fade-in">
                                            <h4 class="text-sm font-semibold text-[var(--accent-primary)]">"Piper Settings"</h4>

                                            <div>
                                                <div class="flex justify-between mb-1">
                                                    <label class="text-sm text-[var(--text-muted)]">"Speed (Length Scale)"</label>
                                                    <span class="text-xs font-mono text-[var(--text-muted)]">{move || format!("{:.2}", piper_length_scale.get())}</span>
                                                </div>
                                                <Slider value=piper_length_scale min=0.5 max=2.0 step=0.01 />
                                                <p class="text-xs text-[var(--text-muted)] mt-1">"1.0 = normal, higher = slower"</p>
                                            </div>

                                            <div>
                                                <div class="flex justify-between mb-1">
                                                    <label class="text-sm text-[var(--text-muted)]">"Noise Scale"</label>
                                                    <span class="text-xs font-mono text-[var(--text-muted)]">{move || format!("{:.2}", piper_noise_scale.get())}</span>
                                                </div>
                                                <Slider value=piper_noise_scale min=0.0 max=1.0 step=0.01 />
                                                <p class="text-xs text-[var(--text-muted)] mt-1">"Phoneme variability"</p>
                                            </div>

                                            <div>
                                                <div class="flex justify-between mb-1">
                                                    <label class="text-sm text-[var(--text-muted)]">"Noise W"</label>
                                                    <span class="text-xs font-mono text-[var(--text-muted)]">{move || format!("{:.2}", piper_noise_w.get())}</span>
                                                </div>
                                                <Slider value=piper_noise_w min=0.0 max=1.0 step=0.01 />
                                                <p class="text-xs text-[var(--text-muted)] mt-1">"Phoneme width variability"</p>
                                            </div>

                                            <div>
                                                <div class="flex justify-between mb-1">
                                                    <label class="text-sm text-[var(--text-muted)]">"Sentence Silence"</label>
                                                    <span class="text-xs font-mono text-[var(--text-muted)]">{move || format!("{:.2}s", piper_sentence_silence.get())}</span>
                                                </div>
                                                <Slider value=piper_sentence_silence min=0.0 max=2.0 step=0.01 />
                                                <p class="text-xs text-[var(--text-muted)] mt-1">"Pause between sentences"</p>
                                            </div>
                                        </div>
                                    </Show>

                                    // Coqui Advanced Settings
                                    <Show when=move || provider == "Coqui" && show_advanced.get()>
                                        <div class="space-y-4 p-4 rounded-lg bg-[var(--bg-deep)] border border-[var(--border-subtle)] animate-fade-in">
                                            <h4 class="text-sm font-semibold text-[var(--accent-primary)]">"Coqui Settings"</h4>

                                            <div>
                                                <div class="flex justify-between mb-1">
                                                    <label class="text-sm text-[var(--text-muted)]">"Speed"</label>
                                                    <span class="text-xs font-mono text-[var(--text-muted)]">{move || format!("{:.2}x", coqui_speed.get())}</span>
                                                </div>
                                                <Slider value=coqui_speed min=0.5 max=2.0 step=0.01 />
                                                <p class="text-xs text-[var(--text-muted)] mt-1">"Playback speed multiplier"</p>
                                            </div>

                                            <div>
                                                <div class="flex justify-between mb-1">
                                                    <label class="text-sm text-[var(--text-muted)]">"Temperature"</label>
                                                    <span class="text-xs font-mono text-[var(--text-muted)]">{move || format!("{:.2}", coqui_temperature.get())}</span>
                                                </div>
                                                <Slider value=coqui_temperature min=0.0 max=2.0 step=0.01 />
                                                <p class="text-xs text-[var(--text-muted)] mt-1">"Generation randomness (XTTS)"</p>
                                            </div>

                                            <div>
                                                <div class="flex justify-between mb-1">
                                                    <label class="text-sm text-[var(--text-muted)]">"Top K"</label>
                                                    <span class="text-xs font-mono text-[var(--text-muted)]">{move || format!("{}", coqui_top_k.get() as u32)}</span>
                                                </div>
                                                <Slider value=coqui_top_k min=1.0 max=100.0 step=1.0 />
                                                <p class="text-xs text-[var(--text-muted)] mt-1">"Limits token selection pool"</p>
                                            </div>

                                            <div>
                                                <div class="flex justify-between mb-1">
                                                    <label class="text-sm text-[var(--text-muted)]">"Top P"</label>
                                                    <span class="text-xs font-mono text-[var(--text-muted)]">{move || format!("{:.2}", coqui_top_p.get())}</span>
                                                </div>
                                                <Slider value=coqui_top_p min=0.0 max=1.0 step=0.01 />
                                                <p class="text-xs text-[var(--text-muted)] mt-1">"Nucleus sampling threshold"</p>
                                            </div>

                                            <div>
                                                <div class="flex justify-between mb-1">
                                                    <label class="text-sm text-[var(--text-muted)]">"Repetition Penalty"</label>
                                                    <span class="text-xs font-mono text-[var(--text-muted)]">{move || format!("{:.1}", coqui_repetition_penalty.get())}</span>
                                                </div>
                                                <Slider value=coqui_repetition_penalty min=1.0 max=5.0 step=0.1 />
                                                <p class="text-xs text-[var(--text-muted)] mt-1">"Reduces repetitive output"</p>
                                            </div>
                                        </div>
                                    </Show>
                                </div>
                            }.into_any()
                        }
                    }}}

                    // Status indicator
                    {move || {
                        let status = save_status.get();
                        if !status.is_empty() {
                            let is_error = status.starts_with("Error");
                            let class = if is_error {
                                "text-sm text-red-400"
                            } else if is_saving.get() {
                                "text-sm text-yellow-400"
                            } else {
                                "text-sm text-green-400"
                            };
                            view! { <div class=class>{status}</div> }.into_any()
                        } else {
                            view! { <span/> }.into_any()
                        }
                    }}
                </div>
            </Card>

            // Piper Voice Download Section
            <Show when=move || selected_voice_provider.get() == "Piper">
                <Card class="p-6">
                    <div class="space-y-4">
                        <div class="flex items-center justify-between">
                            <div>
                                <h4 class="text-lg font-semibold text-[var(--text-primary)]">"Download Voices"</h4>
                                <p class="text-sm text-[var(--text-muted)]">"Download high-quality Piper voices from Hugging Face"</p>
                            </div>
                        </div>

                        // Download error
                        {move || download_error.get().map(|err| view! {
                            <div class="p-3 rounded-lg bg-red-900/30 border border-red-700 text-red-300 text-sm">
                                {err}
                            </div>
                        })}

                        // Popular voices list
                        <div class="space-y-2">
                            {move || {
                                let voices = popular_voices.get();
                                let installed = available_voices.get();
                                let installed_ids: Vec<String> = installed.iter()
                                    .map(|v| v.id.clone())
                                    .collect();

                                if voices.is_empty() {
                                    view! {
                                        <div class="text-[var(--text-muted)] text-sm italic">
                                            "Loading available voices..."
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="grid gap-2">
                                            {voices.into_iter().map(|(key, name, desc)| {
                                                let key_clone = key.clone();
                                                let key_for_check = key.clone();
                                                let key_for_download = key.clone();
                                                let is_installed = installed_ids.iter().any(|id| id.contains(&key_for_check));
                                                let is_downloading = downloading_voice.get().as_ref() == Some(&key_clone);

                                                view! {
                                                    <div class="flex items-center justify-between p-3 rounded-lg bg-[var(--bg-surface)] border border-[var(--border-subtle)]">
                                                        <div class="flex-1 min-w-0">
                                                            <div class="font-medium text-[var(--text-primary)]">{name}</div>
                                                            <div class="text-xs text-[var(--text-muted)] truncate">{desc}</div>
                                                        </div>
                                                        <div class="ml-3 flex-shrink-0">
                                                            {if is_installed {
                                                                view! {
                                                                    <span class="px-2 py-1 text-xs rounded bg-green-900/30 text-green-400 border border-green-700">
                                                                        "Installed"
                                                                    </span>
                                                                }.into_any()
                                                            } else if is_downloading {
                                                                view! {
                                                                    <span class="px-2 py-1 text-xs rounded bg-blue-900/30 text-blue-400 border border-blue-700 animate-pulse">
                                                                        "Downloading..."
                                                                    </span>
                                                                }.into_any()
                                                            } else {
                                                                view! {
                                                                    <Button
                                                                        variant=ButtonVariant::Secondary
                                                                        on_click=move |_: ev::MouseEvent| {
                                                                            let voice_key = key_for_download.clone();
                                                                            downloading_voice.set(Some(voice_key.clone()));
                                                                            download_error.set(None);

                                                                            spawn_local(async move {
                                                                                match download_piper_voice(voice_key.clone(), None).await {
                                                                                    Ok(_path) => {
                                                                                        show_success("Voice Downloaded", Some(&format!("Downloaded {}", voice_key)));
                                                                                        // Refresh voice list
                                                                                        if let Ok(voices) = list_all_voices().await {
                                                                                            let filtered: Vec<Voice> = voices.into_iter()
                                                                                                .filter(|v| v.provider.eq_ignore_ascii_case("piper"))
                                                                                                .collect();
                                                                                            available_voices.set(filtered);
                                                                                        }
                                                                                    }
                                                                                    Err(e) => {
                                                                                        download_error.set(Some(format!("Failed to download: {}", e)));
                                                                                        show_error("Download Failed", Some(&e), None);
                                                                                    }
                                                                                }
                                                                                downloading_voice.set(None);
                                                                            });
                                                                        }
                                                                    >
                                                                        "Download"
                                                                    </Button>
                                                                }.into_any()
                                                            }}
                                                        </div>
                                                    </div>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>
                </Card>
            </Show>
        </div>
    }
}
