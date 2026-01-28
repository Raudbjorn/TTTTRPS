//! Basics Step - Campaign name and game system selection
//!
//! First step of the wizard collecting core campaign identity.

use leptos::prelude::*;

use crate::services::wizard_state::{use_wizard_context, BasicsData, StepData};

/// Game system options
const GAME_SYSTEMS: &[(&str, &str, &str)] = &[
    ("dnd5e", "D&D 5th Edition", "High fantasy adventure with iconic classes and monsters"),
    ("pf2e", "Pathfinder 2e", "Tactical fantasy with deep character customization"),
    ("coc7e", "Call of Cthulhu 7e", "Cosmic horror investigation and sanity mechanics"),
    ("delta_green", "Delta Green", "Modern-day conspiracy horror"),
    ("mothership", "Mothership", "Sci-fi horror with panic mechanics"),
    ("cyberpunk_red", "Cyberpunk Red", "Dystopian future with style over substance"),
    ("shadowrun6e", "Shadowrun 6e", "Cyberpunk meets fantasy in a corporate dystopia"),
    ("vtm5e", "Vampire: The Masquerade", "Personal horror in the World of Darkness"),
    ("bitd", "Blades in the Dark", "Heist-focused narrative play"),
    ("fate_core", "Fate Core", "Narrative-driven with aspects and fate points"),
    ("other", "Custom System", "Define your own game system"),
];

/// Basics step component
#[component]
pub fn BasicsStep(
    form_data: RwSignal<Option<StepData>>,
    form_valid: RwSignal<bool>,
) -> impl IntoView {
    let ctx = use_wizard_context();
    let draft = ctx.draft();

    // Local form state
    let name = RwSignal::new(draft.name.unwrap_or_default());
    let system = RwSignal::new(draft.system.unwrap_or_else(|| "dnd5e".to_string()));
    let custom_system = RwSignal::new(String::new());
    let description = RwSignal::new(draft.description.unwrap_or_default());

    // Validation
    let is_valid = Signal::derive(move || {
        let n = name.get();
        let s = system.get();
        let cs = custom_system.get();

        !n.trim().is_empty() && (s != "other" || !cs.trim().is_empty())
    });

    // Update form_valid when validation changes
    Effect::new(move |_| {
        form_valid.set(is_valid.get());
    });

    // Update form_data when inputs change
    Effect::new(move |_| {
        let final_system = if system.get() == "other" {
            custom_system.get()
        } else {
            system.get()
        };

        form_data.set(Some(StepData::Basics(BasicsData {
            name: name.get(),
            system: final_system,
            description: if description.get().is_empty() {
                None
            } else {
                Some(description.get())
            },
        })));
    });

    view! {
        <div class="space-y-8 max-w-2xl mx-auto">
            // Header
            <div class="text-center">
                <h3 class="text-2xl font-bold text-white mb-2">"Name Your Campaign"</h3>
                <p class="text-zinc-400">
                    "Choose a memorable name and select your game system"
                </p>
            </div>

            // Campaign Name
            <div class="space-y-2">
                <label class="block text-sm font-medium text-zinc-300">
                    "Campaign Name"
                    <span class="text-red-400 ml-1">"*"</span>
                </label>
                <input
                    type="text"
                    class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-lg
                           placeholder-zinc-500 focus:border-purple-500 focus:ring-1 focus:ring-purple-500 focus:outline-none"
                    placeholder="e.g., Curse of Strahd, The Lost Mines"
                    prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                    autofocus
                />
                <p class="text-xs text-zinc-500">
                    "This is how your campaign will appear in the dashboard"
                </p>
            </div>

            // Game System Selection
            <div class="space-y-3">
                <label class="block text-sm font-medium text-zinc-300">
                    "Game System"
                    <span class="text-red-400 ml-1">"*"</span>
                </label>

                <div class="grid grid-cols-1 md:grid-cols-2 gap-3 max-h-72 overflow-y-auto pr-2 custom-scrollbar">
                    {GAME_SYSTEMS.iter().map(|(id, label, desc)| {
                        let id_str = id.to_string();
                        let id_for_check = id_str.clone();
                        let id_for_click = id_str.clone();
                        let is_selected = move || system.get() == id_for_check;

                        view! {
                            <button
                                type="button"
                                class=move || format!(
                                    "p-4 rounded-lg border text-left transition-all duration-200 {}",
                                    if is_selected() {
                                        "bg-purple-900/30 border-purple-500 ring-1 ring-purple-500/50"
                                    } else {
                                        "bg-zinc-800/50 border-zinc-700 hover:border-zinc-600 hover:bg-zinc-800"
                                    }
                                )
                                on:click=move |_| system.set(id_for_click.clone())
                            >
                                <div class="font-medium text-white">{*label}</div>
                                <div class="text-xs text-zinc-400 mt-1 line-clamp-2">{*desc}</div>
                            </button>
                        }
                    }).collect_view()}
                </div>

                // Custom system input
                <Show when=move || system.get() == "other">
                    <div class="mt-4 animate-in fade-in slide-in-from-top-2 duration-200">
                        <label class="block text-sm font-medium text-zinc-300 mb-2">
                            "Custom System Name"
                            <span class="text-red-400 ml-1">"*"</span>
                        </label>
                        <input
                            type="text"
                            class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white
                                   placeholder-zinc-500 focus:border-purple-500 focus:ring-1 focus:ring-purple-500 focus:outline-none"
                            placeholder="Enter your game system name..."
                            prop:value=move || custom_system.get()
                            on:input=move |ev| custom_system.set(event_target_value(&ev))
                        />
                    </div>
                </Show>
            </div>

            // Description (optional)
            <div class="space-y-2">
                <label class="block text-sm font-medium text-zinc-300">
                    "Description"
                    <span class="text-zinc-500 ml-2 text-xs font-normal">"(optional)"</span>
                </label>
                <textarea
                    class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white
                           placeholder-zinc-500 focus:border-purple-500 focus:ring-1 focus:ring-purple-500 focus:outline-none resize-none"
                    rows="4"
                    placeholder="A brief description of your campaign's premise, setting, or hook..."
                    prop:value=move || description.get()
                    on:input=move |ev| description.set(event_target_value(&ev))
                />
                <p class="text-xs text-zinc-500">
                    "Help yourself and the AI understand what this campaign is about"
                </p>
            </div>

            // Validation feedback
            <Show when=move || !is_valid.get() && !name.get().is_empty()>
                <div class="p-3 bg-amber-900/20 border border-amber-700/50 rounded-lg">
                    <div class="flex items-center gap-2 text-amber-400 text-sm">
                        <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                        </svg>
                        <span>
                            {move || {
                                if system.get() == "other" && custom_system.get().is_empty() {
                                    "Please enter a custom system name"
                                } else {
                                    "Please fill in all required fields"
                                }
                            }}
                        </span>
                    </div>
                </div>
            </Show>
        </div>
    }
}
