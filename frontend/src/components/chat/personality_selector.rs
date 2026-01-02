//! Personality Selector Component
//!
//! A component for selecting and previewing personalities in the chat interface.
//! Features:
//! - Dropdown selector for available personalities
//! - Live preview of personality effect
//! - Quick toggle between personalities
//! - Settings for narrative style (tone, vocabulary, verbosity, etc.)

use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    list_personalities, preview_personality, set_active_personality,
    get_active_personality, PersonalityPreview, SetActivePersonalityRequest,
    set_personality_settings, PersonalitySettingsRequest,
};
use crate::components::design_system::{Button, ButtonVariant};



/// Personality Selector component for the chat interface
#[component]
pub fn PersonalitySelector(
    /// Current session ID
    #[prop(into)]
    session_id: RwSignal<Option<String>>,
    /// Current campaign ID
    #[prop(into)]
    campaign_id: RwSignal<Option<String>>,
    /// Callback when personality changes
    #[prop(optional)]
    on_personality_change: Option<Callback<Option<String>>>,
) -> impl IntoView {
    // State
    let personalities = RwSignal::new(Vec::<PersonalityPreview>::new());
    let selected_personality_id = RwSignal::new(Option::<String>::None);
    let selected_preview = RwSignal::new(Option::<PersonalityPreview>::None);
    let is_loading = RwSignal::new(false);
    let is_expanded = RwSignal::new(false);
    let show_settings = RwSignal::new(false);
    let error_message = RwSignal::new(Option::<String>::None);

    // Settings state
    let tone = RwSignal::new("neutral".to_string());
    let vocabulary = RwSignal::new("standard".to_string());
    let narrative_style = RwSignal::new("third_person_limited".to_string());
    let verbosity = RwSignal::new("standard".to_string());
    let genre = RwSignal::new("high_fantasy".to_string());

    // Load personalities on mount
    Effect::new(move |_| {
        spawn_local(async move {
            is_loading.set(true);
            match list_personalities().await {
                Ok(list) => {
                    personalities.set(list);
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to load personalities: {}", e)));
                }
            }
            is_loading.set(false);
        });
    });

    // Load active personality when session/campaign changes
    Effect::new(move |_| {
        let sess = session_id.get();
        let camp = campaign_id.get();

        if let (Some(s), Some(c)) = (sess, camp) {
            spawn_local(async move {
                match get_active_personality(s, c).await {
                    Ok(pid) => {
                        selected_personality_id.set(pid.clone());
                        // Load preview if we have a personality
                        if let Some(id) = pid {
                            match preview_personality(id).await {
                                Ok(preview) => {
                                    selected_preview.set(Some(preview));
                                }
                                Err(_) => {
                                    selected_preview.set(None);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        selected_personality_id.set(None);
                    }
                }
            });
        }
    });

    // Handle personality selection
    let on_select = move |personality_id: Option<String>| {
        let sess = session_id.get();
        let camp = campaign_id.get();

        if let (Some(s), Some(c)) = (sess.clone(), camp.clone()) {
            let pid = personality_id.clone();
            selected_personality_id.set(pid.clone());

            spawn_local(async move {
                let request = SetActivePersonalityRequest {
                    session_id: s,
                    personality_id: pid.clone(),
                    campaign_id: c,
                };

                match set_active_personality(request).await {
                    Ok(_) => {
                        // Load preview
                        if let Some(id) = pid {
                            match preview_personality(id).await {
                                Ok(preview) => {
                                    selected_preview.set(Some(preview));
                                }
                                Err(_) => {
                                    selected_preview.set(None);
                                }
                            }
                        } else {
                            selected_preview.set(None);
                        }
                    }
                    Err(e) => {
                        error_message.set(Some(format!("Failed to set personality: {}", e)));
                    }
                }
            });

            // Notify parent
            if let Some(cb) = on_personality_change {
                cb.run(personality_id);
            }
        }
    };

    // Handle settings save
    let save_settings = move |_: ev::MouseEvent| {
        let camp = campaign_id.get();

        if let Some(c) = camp {
            let request = PersonalitySettingsRequest {
                campaign_id: c,
                tone: Some(tone.get()),
                vocabulary: Some(vocabulary.get()),
                narrative_style: Some(narrative_style.get()),
                verbosity: Some(verbosity.get()),
                genre: Some(genre.get()),
                custom_patterns: None,
                use_dialect: None,
                dialect: None,
            };

            spawn_local(async move {
                match set_personality_settings(request).await {
                    Ok(_) => {
                        show_settings.set(false);
                    }
                    Err(e) => {
                        error_message.set(Some(format!("Failed to save settings: {}", e)));
                    }
                }
            });
        }
    };

    view! {
        <div class="relative">
            // Main selector button
            <Button
                variant=ButtonVariant::Secondary
                class="flex items-center gap-2 text-sm"
                on_click=move |_: ev::MouseEvent| {
                    is_expanded.update(|v| *v = !*v);
                }
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                </svg>
                {move || {
                    match selected_preview.get() {
                        Some(preview) => preview.personality_name.clone(),
                        None => "No Personality".to_string(),
                    }
                }}
                <svg class=move || {
                    if is_expanded.get() { "w-4 h-4 transform rotate-180" } else { "w-4 h-4" }
                } fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                </svg>
            </Button>

            // Dropdown panel
            {move || {
                if is_expanded.get() {
                    Some(view! {
                        <div class="absolute top-full left-0 mt-2 w-80 bg-theme-secondary border border-theme rounded-lg shadow-xl z-50">
                            // Header
                            <div class="p-3 border-b border-theme flex justify-between items-center">
                                <h3 class="font-semibold text-theme-primary">"Personality"</h3>
                                <div class="flex gap-2">
                                    <button
                                        class="text-xs text-theme-secondary hover:text-theme-primary"
                                        on:click=move |_| show_settings.update(|v| *v = !*v)
                                    >
                                        "Settings"
                                    </button>
                                    <button
                                        class="text-theme-secondary hover:text-theme-primary"
                                        on:click=move |_| is_expanded.set(false)
                                    >
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                        </svg>
                                    </button>
                                </div>
                            </div>

                            // Error message
                            {move || error_message.get().map(|e| view! {
                                <div class="p-2 bg-red-900/30 text-red-400 text-xs">{e}</div>
                            })}

                            // Loading state
                            {move || {
                                if is_loading.get() {
                                    Some(view! {
                                        <div class="p-4 text-center text-theme-secondary">
                                            <div class="animate-spin w-6 h-6 border-2 border-theme-primary border-t-transparent rounded-full mx-auto"></div>
                                            <p class="mt-2 text-sm">"Loading personalities..."</p>
                                        </div>
                                    })
                                } else {
                                    None
                                }
                            }}

                            // Personality list
                            {move || {
                                if !is_loading.get() {
                                    Some(view! {
                                        <div class="max-h-64 overflow-y-auto">
                                            // None option
                                            <button
                                                class=move || {
                                                    if selected_personality_id.get().is_none() {
                                                        "w-full text-left p-3 hover:bg-theme-primary/5 border-l-2 border-blue-500 bg-blue-900/20"
                                                    } else {
                                                        "w-full text-left p-3 hover:bg-theme-primary/5 border-l-2 border-transparent"
                                                    }
                                                }
                                                on:click=move |_| {
                                                    on_select(None);
                                                    is_expanded.set(false);
                                                }
                                            >
                                                <div class="font-medium text-theme-primary">"No Personality"</div>
                                                <div class="text-xs text-theme-secondary mt-1">"Use default AI behavior"</div>
                                            </button>

                                            // Personality options
                                            <For
                                                each=move || personalities.get()
                                                key=|p| p.personality_id.clone()
                                                children=move |personality| {
                                                    let pid = personality.personality_id.clone();
                                                    let pid_for_check = pid.clone();
                                                    let pid_for_click = pid.clone();
                                                    let name = personality.personality_name.clone();
                                                    let traits: Vec<String> = personality.characteristics.iter().take(2).cloned().collect();

                                                    view! {
                                                        <button
                                                            class=move || {
                                                                if selected_personality_id.get() == Some(pid_for_check.clone()) {
                                                                    "w-full text-left p-3 hover:bg-theme-primary/5 border-l-2 border-blue-500 bg-blue-900/20"
                                                                } else {
                                                                    "w-full text-left p-3 hover:bg-theme-primary/5 border-l-2 border-transparent"
                                                                }
                                                            }
                                                            on:click=move |_| {
                                                                on_select(Some(pid_for_click.clone()));
                                                                is_expanded.set(false);
                                                            }
                                                        >
                                                            <div class="font-medium text-theme-primary">{name}</div>
                                                            <div class="text-xs text-theme-secondary mt-1 flex flex-wrap gap-1">
                                                                {traits.into_iter().map(|t| view! {
                                                                    <span class="bg-theme-primary/10 px-1 py-0.5 rounded">{t}</span>
                                                                }).collect::<Vec<_>>()}
                                                            </div>
                                                        </button>
                                                    }
                                                }
                                            />
                                        </div>
                                    })
                                } else {
                                    None
                                }
                            }}

                            // Preview section
                            {move || {
                                selected_preview.get().map(|preview| view! {
                                    <div class="p-3 border-t border-theme bg-theme-primary/5">
                                        <h4 class="text-xs font-semibold text-theme-secondary mb-2">"Preview"</h4>
                                        <div class="text-sm text-theme-primary">
                                            {preview.sample_greetings.first().cloned().unwrap_or_else(|| "No sample available".to_string())}
                                        </div>
                                        <div class="mt-2 flex flex-wrap gap-1">
                                            {preview.characteristics.into_iter().map(|c| view! {
                                                <span class="text-xs bg-theme-secondary px-2 py-0.5 rounded">{c}</span>
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    </div>
                                })
                            }}

                            // Settings panel
                            {move || {
                                if show_settings.get() {
                                    Some(view! {
                                        <div class="p-3 border-t border-theme">
                                            <h4 class="text-xs font-semibold text-theme-secondary mb-3">"Narrative Settings"</h4>

                                            // Tone selector
                                            <div class="mb-3">
                                                <label class="block text-xs text-theme-secondary mb-1">"Tone"</label>
                                                <select
                                                    class="w-full bg-theme-primary text-theme-primary border border-theme rounded px-2 py-1 text-sm"
                                                    on:change=move |e| {
                                                        use wasm_bindgen::JsCast;
                                                        let target = e.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
                                                        tone.set(target.value());
                                                    }
                                                >
                                                    <option value="neutral" selected=move || tone.get() == "neutral">"Neutral"</option>
                                                    <option value="dramatic" selected=move || tone.get() == "dramatic">"Dramatic"</option>
                                                    <option value="casual" selected=move || tone.get() == "casual">"Casual"</option>
                                                    <option value="mysterious" selected=move || tone.get() == "mysterious">"Mysterious"</option>
                                                    <option value="humorous" selected=move || tone.get() == "humorous">"Humorous"</option>
                                                    <option value="epic" selected=move || tone.get() == "epic">"Epic"</option>
                                                    <option value="gritty" selected=move || tone.get() == "gritty">"Gritty"</option>
                                                    <option value="horror" selected=move || tone.get() == "horror">"Horror"</option>
                                                </select>
                                            </div>

                                            // Vocabulary selector
                                            <div class="mb-3">
                                                <label class="block text-xs text-theme-secondary mb-1">"Vocabulary"</label>
                                                <select
                                                    class="w-full bg-theme-primary text-theme-primary border border-theme rounded px-2 py-1 text-sm"
                                                    on:change=move |e| {
                                                        use wasm_bindgen::JsCast;
                                                        let target = e.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
                                                        vocabulary.set(target.value());
                                                    }
                                                >
                                                    <option value="simple" selected=move || vocabulary.get() == "simple">"Simple"</option>
                                                    <option value="standard" selected=move || vocabulary.get() == "standard">"Standard"</option>
                                                    <option value="elevated" selected=move || vocabulary.get() == "elevated">"Elevated"</option>
                                                    <option value="archaic" selected=move || vocabulary.get() == "archaic">"Archaic"</option>
                                                    <option value="technical" selected=move || vocabulary.get() == "technical">"Technical"</option>
                                                </select>
                                            </div>

                                            // Verbosity selector
                                            <div class="mb-3">
                                                <label class="block text-xs text-theme-secondary mb-1">"Verbosity"</label>
                                                <select
                                                    class="w-full bg-theme-primary text-theme-primary border border-theme rounded px-2 py-1 text-sm"
                                                    on:change=move |e| {
                                                        use wasm_bindgen::JsCast;
                                                        let target = e.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
                                                        verbosity.set(target.value());
                                                    }
                                                >
                                                    <option value="terse" selected=move || verbosity.get() == "terse">"Terse"</option>
                                                    <option value="standard" selected=move || verbosity.get() == "standard">"Standard"</option>
                                                    <option value="verbose" selected=move || verbosity.get() == "verbose">"Verbose"</option>
                                                    <option value="elaborate" selected=move || verbosity.get() == "elaborate">"Elaborate"</option>
                                                </select>
                                            </div>

                                            // Genre selector
                                            <div class="mb-3">
                                                <label class="block text-xs text-theme-secondary mb-1">"Genre"</label>
                                                <select
                                                    class="w-full bg-theme-primary text-theme-primary border border-theme rounded px-2 py-1 text-sm"
                                                    on:change=move |e| {
                                                        use wasm_bindgen::JsCast;
                                                        let target = e.target().unwrap().unchecked_into::<web_sys::HtmlSelectElement>();
                                                        genre.set(target.value());
                                                    }
                                                >
                                                    <option value="high_fantasy" selected=move || genre.get() == "high_fantasy">"High Fantasy"</option>
                                                    <option value="dark_fantasy" selected=move || genre.get() == "dark_fantasy">"Dark Fantasy"</option>
                                                    <option value="sword_and_sorcery" selected=move || genre.get() == "sword_and_sorcery">"Sword & Sorcery"</option>
                                                    <option value="urban_fantasy" selected=move || genre.get() == "urban_fantasy">"Urban Fantasy"</option>
                                                    <option value="sci_fi" selected=move || genre.get() == "sci_fi">"Sci-Fi"</option>
                                                    <option value="horror" selected=move || genre.get() == "horror">"Horror"</option>
                                                    <option value="steampunk" selected=move || genre.get() == "steampunk">"Steampunk"</option>
                                                    <option value="cyberpunk" selected=move || genre.get() == "cyberpunk">"Cyberpunk"</option>
                                                    <option value="western" selected=move || genre.get() == "western">"Western"</option>
                                                    <option value="noir" selected=move || genre.get() == "noir">"Noir"</option>
                                                </select>
                                            </div>

                                            // Save button
                                            <Button
                                                variant=ButtonVariant::Primary
                                                class="w-full"
                                                on_click=save_settings
                                            >
                                                "Save Settings"
                                            </Button>
                                        </div>
                                    })
                                } else {
                                    None
                                }
                            }}
                        </div>
                    })
                } else {
                    None
                }
            }}
        </div>
    }
}

/// Compact personality indicator for chat messages
#[component]
pub fn PersonalityIndicator(
    /// Personality ID
    #[prop(into)]
    personality_id: Option<String>,
) -> impl IntoView {
    let preview = RwSignal::new(Option::<PersonalityPreview>::None);

    // Load preview when personality_id changes
    Effect::new(move |_| {
        if let Some(pid) = personality_id.clone() {
            spawn_local(async move {
                match preview_personality(pid).await {
                    Ok(p) => preview.set(Some(p)),
                    Err(_) => preview.set(None),
                }
            });
        } else {
            preview.set(None);
        }
    });

    view! {
        {move || preview.get().map(|p| view! {
            <span class="inline-flex items-center gap-1 text-xs text-theme-secondary bg-theme-secondary px-2 py-0.5 rounded">
                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
                </svg>
                {p.personality_name}
            </span>
        })}
    }
}
