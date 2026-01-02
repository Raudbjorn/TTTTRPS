//! Campaign Create Modal Component
//!
//! Multi-step wizard for creating new campaigns with system selection,
//! theme configuration, and initial settings.

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::bindings::{
    create_campaign, get_theme_preset, Campaign, ThemeWeights,
};
use crate::components::design_system::{Button, ButtonVariant, Input, Modal};

/// Wizard step
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    Basics,
    System,
    Theme,
    Settings,
    Review,
}

impl WizardStep {
    fn index(&self) -> usize {
        match self {
            Self::Basics => 0,
            Self::System => 1,
            Self::Theme => 2,
            Self::Settings => 3,
            Self::Review => 4,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Basics => "Basics",
            Self::System => "System",
            Self::Theme => "Theme",
            Self::Settings => "Settings",
            Self::Review => "Review",
        }
    }

    fn all() -> Vec<Self> {
        vec![Self::Basics, Self::System, Self::Theme, Self::Settings, Self::Review]
    }
}

/// Game system options
const GAME_SYSTEMS: &[(&str, &str, &str)] = &[
    ("D&D 5e", "Dungeons & Dragons 5th Edition", "High fantasy adventure with iconic classes and monsters"),
    ("Pathfinder 2e", "Pathfinder Second Edition", "Tactical fantasy with deep character customization"),
    ("Call of Cthulhu", "Call of Cthulhu 7e", "Cosmic horror investigation and sanity mechanics"),
    ("Delta Green", "Delta Green", "Modern-day conspiracy horror"),
    ("Mothership", "Mothership 1e", "Sci-fi horror with panic mechanics"),
    ("Cyberpunk Red", "Cyberpunk Red", "Dystopian future with style over substance"),
    ("Shadowrun", "Shadowrun 6e", "Cyberpunk meets fantasy in a corporate dystopia"),
    ("Vampire: The Masquerade", "VtM 5th Edition", "Personal horror in the World of Darkness"),
    ("Blades in the Dark", "Blades in the Dark", "Heist-focused narrative play"),
    ("Fate Core", "Fate Core", "Narrative-driven with aspects and fate points"),
    ("Other", "Custom System", "Define your own game system"),
];

/// Theme presets
const THEME_PRESETS: &[(&str, &str, &str)] = &[
    ("epic", "Epic Adventure", "Grand quests, heroic deeds, and world-changing events"),
    ("dark", "Dark & Gritty", "Moral ambiguity, hard choices, and survival"),
    ("mystery", "Mystery & Intrigue", "Secrets, investigation, and plot twists"),
    ("horror", "Horror", "Fear, dread, and the unknown"),
    ("comedy", "Comedy", "Humor, wit, and lighthearted moments"),
    ("sandbox", "Sandbox", "Player-driven exploration and freedom"),
    ("political", "Political Intrigue", "Power struggles, factions, and diplomacy"),
    ("exploration", "Exploration", "Discovery, travel, and new frontiers"),
];

/// Step progress indicator
#[component]
fn StepIndicator(
    steps: Vec<WizardStep>,
    current_step: WizardStep,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-center gap-2 mb-8">
            {steps.iter().enumerate().map(|(i, step)| {
                let is_current = *step == current_step;
                let is_complete = step.index() < current_step.index();

                let circle_class = if is_current {
                    "bg-purple-600 text-white"
                } else if is_complete {
                    "bg-purple-900 text-purple-300"
                } else {
                    "bg-zinc-800 text-zinc-500"
                };

                let line_class = if step.index() < current_step.index() {
                    "bg-purple-600"
                } else {
                    "bg-zinc-700"
                };

                view! {
                    <>
                        {(i > 0).then(|| view! {
                            <div class=format!("w-12 h-0.5 {}", line_class)></div>
                        })}
                        <div class="flex flex-col items-center gap-1">
                            <div class=format!("w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium {}", circle_class)>
                                {if is_complete { "v" } else { &(i + 1).to_string() }}
                            </div>
                            <span class=format!("text-xs {}", if is_current { "text-white" } else { "text-zinc-500" })>
                                {step.label()}
                            </span>
                        </div>
                    </>
                }
            }).collect_view()}
        </div>
    }
}

/// Basics step - name and description
#[component]
fn StepBasics(
    name: RwSignal<String>,
    description: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="text-center mb-8">
                <h3 class="text-xl font-bold text-white mb-2">"Name Your Campaign"</h3>
                <p class="text-zinc-400">"Choose a memorable name for your adventure"</p>
            </div>

            <div>
                <label class="block text-sm font-medium text-zinc-400 mb-2">
                    "Campaign Name"
                </label>
                <input
                    type="text"
                    class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-lg placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                    placeholder="e.g., Curse of Strahd, The Lost Mines"
                    prop:value=move || name.get()
                    on:input=move |evt| name.set(event_target_value(&evt))
                />
            </div>

            <div>
                <label class="block text-sm font-medium text-zinc-400 mb-2">
                    "Description "
                    <span class="text-zinc-500">"(optional)"</span>
                </label>
                <textarea
                    class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:border-purple-500 focus:outline-none resize-none"
                    rows="4"
                    placeholder="A brief description of your campaign's premise..."
                    prop:value=move || description.get()
                    on:input=move |evt| description.set(event_target_value(&evt))
                />
            </div>
        </div>
    }
}

/// System selection step
#[component]
fn StepSystem(
    selected_system: RwSignal<String>,
    custom_system: RwSignal<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="text-center mb-8">
                <h3 class="text-xl font-bold text-white mb-2">"Choose Your System"</h3>
                <p class="text-zinc-400">"Select the game system for this campaign"</p>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-3 max-h-80 overflow-y-auto pr-2">
                {GAME_SYSTEMS.iter().map(|(id, name, desc)| {
                    let id_str = id.to_string();
                    let id_for_click = id_str.clone();
                    let is_selected = move || selected_system.get() == id_str;

                    view! {
                        <button
                            class=move || format!(
                                "p-4 rounded-lg border text-left transition-colors {}",
                                if is_selected() {
                                    "bg-purple-900/30 border-purple-500"
                                } else {
                                    "bg-zinc-800/50 border-zinc-700 hover:border-zinc-600"
                                }
                            )
                            on:click=move |_| selected_system.set(id_for_click.clone())
                        >
                            <div class="font-medium text-white">{*name}</div>
                            <div class="text-xs text-zinc-400 mt-1">{*desc}</div>
                        </button>
                    }
                }).collect_view()}
            </div>

            // Custom system input
            {move || (selected_system.get() == "Other").then(|| view! {
                <div class="mt-4">
                    <label class="block text-sm font-medium text-zinc-400 mb-2">
                        "Custom System Name"
                    </label>
                    <input
                        type="text"
                        class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                        placeholder="Enter your game system name..."
                        prop:value=move || custom_system.get()
                        on:input=move |evt| custom_system.set(event_target_value(&evt))
                    />
                </div>
            })}
        </div>
    }
}

/// Theme configuration step
#[component]
fn StepTheme(
    selected_themes: RwSignal<Vec<String>>,
) -> impl IntoView {
    let toggle_theme = move |theme: String| {
        selected_themes.update(|themes| {
            if themes.contains(&theme) {
                themes.retain(|t| t != &theme);
            } else if themes.len() < 3 {
                themes.push(theme);
            }
        });
    };

    view! {
        <div class="space-y-6">
            <div class="text-center mb-8">
                <h3 class="text-xl font-bold text-white mb-2">"Set the Tone"</h3>
                <p class="text-zinc-400">"Select up to 3 themes that define your campaign"</p>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
                {THEME_PRESETS.iter().map(|(id, name, desc)| {
                    let id_str = id.to_string();
                    let id_for_check = id_str.clone();
                    let id_for_click = id_str.clone();
                    let is_selected = move || selected_themes.get().contains(&id_for_check);

                    view! {
                        <button
                            class=move || format!(
                                "p-4 rounded-lg border text-left transition-colors {}",
                                if is_selected() {
                                    "bg-purple-900/30 border-purple-500"
                                } else {
                                    "bg-zinc-800/50 border-zinc-700 hover:border-zinc-600"
                                }
                            )
                            on:click=move |_| toggle_theme(id_for_click.clone())
                        >
                            <div class="flex items-center justify-between">
                                <div class="font-medium text-white">{*name}</div>
                                {is_selected().then(|| view! {
                                    <span class="text-purple-400">"v"</span>
                                })}
                            </div>
                            <div class="text-xs text-zinc-400 mt-1">{*desc}</div>
                        </button>
                    }
                }).collect_view()}
            </div>

            <div class="text-center text-sm text-zinc-500">
                {move || format!("{}/3 themes selected", selected_themes.get().len())}
            </div>
        </div>
    }
}

/// Settings step
#[component]
fn StepSettings(
    voice_enabled: RwSignal<bool>,
    auto_transcribe: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="text-center mb-8">
                <h3 class="text-xl font-bold text-white mb-2">"Campaign Settings"</h3>
                <p class="text-zinc-400">"Configure optional features"</p>
            </div>

            <div class="space-y-4">
                // Voice enabled toggle
                <div class="flex items-center justify-between p-4 bg-zinc-800/50 rounded-lg border border-zinc-700">
                    <div>
                        <div class="font-medium text-white">"AI Voice Output"</div>
                        <div class="text-sm text-zinc-400">"Enable text-to-speech for NPC dialogue"</div>
                    </div>
                    <button
                        class=move || format!(
                            "w-12 h-6 rounded-full transition-colors {}",
                            if voice_enabled.get() { "bg-purple-600" } else { "bg-zinc-600" }
                        )
                        on:click=move |_| voice_enabled.update(|v| *v = !*v)
                    >
                        <div
                            class=move || format!(
                                "w-5 h-5 rounded-full bg-white shadow transform transition-transform {}",
                                if voice_enabled.get() { "translate-x-6" } else { "translate-x-0.5" }
                            )
                        />
                    </button>
                </div>

                // Auto transcribe toggle
                <div class="flex items-center justify-between p-4 bg-zinc-800/50 rounded-lg border border-zinc-700">
                    <div>
                        <div class="font-medium text-white">"Auto-Transcribe Sessions"</div>
                        <div class="text-sm text-zinc-400">"Automatically generate session notes"</div>
                    </div>
                    <button
                        class=move || format!(
                            "w-12 h-6 rounded-full transition-colors {}",
                            if auto_transcribe.get() { "bg-purple-600" } else { "bg-zinc-600" }
                        )
                        on:click=move |_| auto_transcribe.update(|v| *v = !*v)
                    >
                        <div
                            class=move || format!(
                                "w-5 h-5 rounded-full bg-white shadow transform transition-transform {}",
                                if auto_transcribe.get() { "translate-x-6" } else { "translate-x-0.5" }
                            )
                        />
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Review step
#[component]
fn StepReview(
    name: RwSignal<String>,
    description: RwSignal<String>,
    system: RwSignal<String>,
    custom_system: RwSignal<String>,
    themes: RwSignal<Vec<String>>,
    voice_enabled: RwSignal<bool>,
    auto_transcribe: RwSignal<bool>,
) -> impl IntoView {
    let display_system = move || {
        let sys = system.get();
        if sys == "Other" {
            custom_system.get()
        } else {
            sys
        }
    };

    view! {
        <div class="space-y-6">
            <div class="text-center mb-8">
                <h3 class="text-xl font-bold text-white mb-2">"Review Your Campaign"</h3>
                <p class="text-zinc-400">"Confirm your choices before creating"</p>
            </div>

            <div class="bg-zinc-800/50 rounded-lg border border-zinc-700 p-6 space-y-4">
                <div>
                    <div class="text-xs text-zinc-500 uppercase tracking-wider">"Name"</div>
                    <div class="text-lg font-medium text-white">{move || name.get()}</div>
                </div>

                {move || (!description.get().is_empty()).then(|| view! {
                    <div>
                        <div class="text-xs text-zinc-500 uppercase tracking-wider">"Description"</div>
                        <div class="text-sm text-zinc-300">{description.get()}</div>
                    </div>
                })}

                <div>
                    <div class="text-xs text-zinc-500 uppercase tracking-wider">"Game System"</div>
                    <div class="text-white">{display_system}</div>
                </div>

                <div>
                    <div class="text-xs text-zinc-500 uppercase tracking-wider">"Themes"</div>
                    <div class="flex flex-wrap gap-2 mt-1">
                        {move || themes.get().into_iter().map(|theme| {
                            view! {
                                <span class="px-2 py-1 bg-purple-900/30 text-purple-300 text-sm rounded">
                                    {theme}
                                </span>
                            }
                        }).collect_view()}
                        {move || themes.get().is_empty().then(|| view! {
                            <span class="text-zinc-500 text-sm">"No themes selected"</span>
                        })}
                    </div>
                </div>

                <div>
                    <div class="text-xs text-zinc-500 uppercase tracking-wider">"Features"</div>
                    <div class="flex flex-wrap gap-2 mt-1">
                        {move || voice_enabled.get().then(|| view! {
                            <span class="px-2 py-1 bg-zinc-700 text-zinc-300 text-sm rounded">"Voice Output"</span>
                        })}
                        {move || auto_transcribe.get().then(|| view! {
                            <span class="px-2 py-1 bg-zinc-700 text-zinc-300 text-sm rounded">"Auto-Transcribe"</span>
                        })}
                        {move || (!voice_enabled.get() && !auto_transcribe.get()).then(|| view! {
                            <span class="text-zinc-500 text-sm">"Default settings"</span>
                        })}
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Main campaign create modal component
#[component]
pub fn CampaignCreateModal(
    /// Whether the modal is open
    is_open: RwSignal<bool>,
    /// Callback when campaign is created
    on_create: Callback<Campaign>,
) -> impl IntoView {
    // Wizard state
    let current_step = RwSignal::new(WizardStep::Basics);

    // Form state
    let name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let selected_system = RwSignal::new("D&D 5e".to_string());
    let custom_system = RwSignal::new(String::new());
    let selected_themes = RwSignal::new(Vec::<String>::new());
    let voice_enabled = RwSignal::new(false);
    let auto_transcribe = RwSignal::new(false);

    // Creating state
    let is_creating = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    // Can proceed to next step
    let can_proceed = Signal::derive(move || {
        match current_step.get() {
            WizardStep::Basics => !name.get().trim().is_empty(),
            WizardStep::System => {
                let sys = selected_system.get();
                sys != "Other" || !custom_system.get().trim().is_empty()
            },
            WizardStep::Theme => true, // Optional
            WizardStep::Settings => true,
            WizardStep::Review => true,
        }
    });

    let handle_next = move |_: ev::MouseEvent| {
        match current_step.get() {
            WizardStep::Basics => current_step.set(WizardStep::System),
            WizardStep::System => current_step.set(WizardStep::Theme),
            WizardStep::Theme => current_step.set(WizardStep::Settings),
            WizardStep::Settings => current_step.set(WizardStep::Review),
            WizardStep::Review => {
                // Create campaign
                is_creating.set(true);
                error.set(None);

                let campaign_name = name.get();
                let system = if selected_system.get() == "Other" {
                    custom_system.get()
                } else {
                    selected_system.get()
                };

                spawn_local(async move {
                    match create_campaign(campaign_name, system).await {
                        Ok(campaign) => {
                            on_create.run(campaign);
                            // Reset form
                            name.set(String::new());
                            description.set(String::new());
                            selected_system.set("D&D 5e".to_string());
                            custom_system.set(String::new());
                            selected_themes.set(vec![]);
                            voice_enabled.set(false);
                            auto_transcribe.set(false);
                            current_step.set(WizardStep::Basics);
                            is_open.set(false);
                        }
                        Err(e) => {
                            error.set(Some(e));
                        }
                    }
                    is_creating.set(false);
                });
            }
        }
    };

    let handle_back = move |_: ev::MouseEvent| {
        match current_step.get() {
            WizardStep::Basics => {},
            WizardStep::System => current_step.set(WizardStep::Basics),
            WizardStep::Theme => current_step.set(WizardStep::System),
            WizardStep::Settings => current_step.set(WizardStep::Theme),
            WizardStep::Review => current_step.set(WizardStep::Settings),
        }
    };

    let handle_close = move |_: ev::MouseEvent| {
        is_open.set(false);
        // Reset to first step on close
        current_step.set(WizardStep::Basics);
    };

    let is_first_step = Signal::derive(move || current_step.get() == WizardStep::Basics);
    let is_last_step = Signal::derive(move || current_step.get() == WizardStep::Review);

    view! {
        <Show when=move || is_open.get()>
            <div
                class="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 flex items-center justify-center p-4"
                on:click=handle_close.clone()
            >
                <div
                    class="bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] overflow-hidden flex flex-col"
                    on:click=move |evt: ev::MouseEvent| evt.stop_propagation()
                >
                    // Header
                    <div class="px-6 py-4 border-b border-zinc-800 flex items-center justify-between shrink-0">
                        <h2 class="text-xl font-bold text-white">"Create New Campaign"</h2>
                        <button
                            class="p-2 text-zinc-400 hover:text-white transition-colors"
                            on:click=handle_close
                        >
                            "X"
                        </button>
                    </div>

                    // Progress indicator
                    <div class="px-6 pt-6 shrink-0">
                        <StepIndicator
                            steps=WizardStep::all()
                            current_step=current_step.get()
                        />
                    </div>

                    // Content
                    <div class="flex-1 overflow-y-auto px-6 pb-6">
                        // Error message
                        {move || error.get().map(|e| view! {
                            <div class="mb-4 p-4 bg-red-900/30 border border-red-800 rounded-lg text-red-400 text-sm">
                                {e}
                            </div>
                        })}

                        // Step content
                        {move || match current_step.get() {
                            WizardStep::Basics => view! {
                                <StepBasics name=name description=description />
                            }.into_any(),
                            WizardStep::System => view! {
                                <StepSystem selected_system=selected_system custom_system=custom_system />
                            }.into_any(),
                            WizardStep::Theme => view! {
                                <StepTheme selected_themes=selected_themes />
                            }.into_any(),
                            WizardStep::Settings => view! {
                                <StepSettings voice_enabled=voice_enabled auto_transcribe=auto_transcribe />
                            }.into_any(),
                            WizardStep::Review => view! {
                                <StepReview
                                    name=name
                                    description=description
                                    system=selected_system
                                    custom_system=custom_system
                                    themes=selected_themes
                                    voice_enabled=voice_enabled
                                    auto_transcribe=auto_transcribe
                                />
                            }.into_any(),
                        }}
                    </div>

                    // Footer
                    <div class="px-6 py-4 border-t border-zinc-800 flex justify-between shrink-0">
                        <button
                            class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            disabled=move || is_first_step.get()
                            on:click=handle_back
                        >
                            "Back"
                        </button>

                        <button
                            class="px-6 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                            disabled=move || !can_proceed.get() || is_creating.get()
                            on:click=handle_next
                        >
                            {move || {
                                if is_creating.get() {
                                    "Creating..."
                                } else if is_last_step.get() {
                                    "Create Campaign"
                                } else {
                                    "Next"
                                }
                            }}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
