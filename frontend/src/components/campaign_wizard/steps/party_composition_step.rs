//! Party Composition Step - Define or analyze party roles
//!
//! Optional step for defining party composition and getting role analysis.

use leptos::prelude::*;

use crate::services::wizard_state::{
    use_wizard_context, CharacterSummary, LevelRange, PartyCompositionData, PartyRole, StepData,
};

/// Character entry row
#[component]
fn CharacterEntry(
    index: usize,
    character: RwSignal<CharacterSummary>,
    on_remove: Callback<String>,  // Changed to use id instead of index
) -> impl IntoView {
    let char_id = character.get().id.clone();
    let name = RwSignal::new(character.get().name.unwrap_or_default());
    let class = RwSignal::new(character.get().class.unwrap_or_default());
    let role = RwSignal::new(character.get().role);

    // Update parent when fields change
    Effect::new(move |_| {
        character.update(|c| {
            c.name = if name.get().is_empty() { None } else { Some(name.get()) };
            c.class = if class.get().is_empty() { None } else { Some(class.get()) };
            c.role = role.get();
        });
    });

    let remove_id = char_id.clone();

    view! {
        <div class="flex items-center gap-3 p-3 bg-zinc-800/50 rounded-lg">
            // Character number
            <div class="w-8 h-8 rounded-full bg-zinc-700 flex items-center justify-center text-sm text-zinc-400 shrink-0">
                {index + 1}
            </div>

            // Name
            <input
                type="text"
                class="flex-1 px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm
                       placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                placeholder="Character name (optional)"
                prop:value=move || name.get()
                on:input=move |ev| name.set(event_target_value(&ev))
            />

            // Class
            <input
                type="text"
                class="w-32 px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm
                       placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                placeholder="Class"
                prop:value=move || class.get()
                on:input=move |ev| class.set(event_target_value(&ev))
            />

            // Role dropdown - with selected= on each option for proper control
            <select
                class="w-36 px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm
                       focus:border-purple-500 focus:outline-none"
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    role.set(match value.as_str() {
                        "tank" => Some(PartyRole::Tank),
                        "healer" => Some(PartyRole::Healer),
                        "damage" => Some(PartyRole::DamageDealer),
                        "support" => Some(PartyRole::Support),
                        "controller" => Some(PartyRole::Controller),
                        "utility" => Some(PartyRole::Utility),
                        "face" => Some(PartyRole::Face),
                        "scout" => Some(PartyRole::Scout),
                        _ => None,
                    });
                }
            >
                <option value="" selected=move || role.get().is_none()>"Role..."</option>
                <option value="tank" selected=move || role.get() == Some(PartyRole::Tank)>"Tank"</option>
                <option value="healer" selected=move || role.get() == Some(PartyRole::Healer)>"Healer"</option>
                <option value="damage" selected=move || role.get() == Some(PartyRole::DamageDealer)>"Damage Dealer"</option>
                <option value="support" selected=move || role.get() == Some(PartyRole::Support)>"Support"</option>
                <option value="controller" selected=move || role.get() == Some(PartyRole::Controller)>"Controller"</option>
                <option value="utility" selected=move || role.get() == Some(PartyRole::Utility)>"Utility"</option>
                <option value="face" selected=move || role.get() == Some(PartyRole::Face)>"Face"</option>
                <option value="scout" selected=move || role.get() == Some(PartyRole::Scout)>"Scout"</option>
            </select>

            // Remove button - uses id instead of index
            <button
                type="button"
                class="p-2 text-zinc-500 hover:text-red-400 transition-colors"
                on:click=move |_| on_remove.run(remove_id.clone())
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
            </button>
        </div>
    }
}

/// Party composition step component
#[component]
pub fn PartyCompositionStep(
    form_data: RwSignal<Option<StepData>>,
    form_valid: RwSignal<bool>,
) -> impl IntoView {
    let ctx = use_wizard_context();
    let draft = ctx.draft();
    let composition = draft.party_composition.unwrap_or_default();
    let player_count = draft.player_count.unwrap_or(4);

    // Initialize characters from draft or create empty slots
    let initial_chars: Vec<RwSignal<CharacterSummary>> = if composition.characters.is_empty() {
        (0..player_count as usize)
            .map(|_| RwSignal::new(CharacterSummary {
                id: uuid::Uuid::new_v4().to_string(),
                name: None,
                class: None,
                subclass: None,
                level: None,
                role: None,
            }))
            .collect()
    } else {
        composition.characters
            .into_iter()
            .map(|c| RwSignal::new(c))
            .collect()
    };

    let characters = RwSignal::new(initial_chars);

    // Level range
    let start_level = RwSignal::new(composition.level_range.as_ref().map(|r| r.start_level).unwrap_or(1));
    let end_level = RwSignal::new(composition.level_range.as_ref().map(|r| r.end_level).unwrap_or(10));

    // This step is always valid (optional)
    Effect::new(move |_| {
        form_valid.set(true);
    });

    // Update form_data when inputs change
    Effect::new(move |_| {
        let char_data: Vec<CharacterSummary> = characters.get()
            .iter()
            .map(|c| c.get())
            .collect();

        form_data.set(Some(StepData::PartyComposition(PartyCompositionData {
            characters: char_data,
            party_size: Some(characters.get().len() as u8),
            level_range: Some(LevelRange {
                start_level: start_level.get(),
                end_level: end_level.get(),
            }),
        })));
    });

    // Add character handler
    let add_character = move |_| {
        characters.update(|chars| {
            chars.push(RwSignal::new(CharacterSummary {
                id: uuid::Uuid::new_v4().to_string(),
                name: None,
                class: None,
                subclass: None,
                level: None,
                role: None,
            }));
        });
    };

    // Remove character handler - uses id for stable removal
    let remove_character = Callback::new(move |id_to_remove: String| {
        characters.update(|chars| {
            if chars.len() > 1 {
                chars.retain(|c| c.get().id != id_to_remove);
            }
        });
    });

    // Analyze roles
    let role_counts = Signal::derive(move || {
        let chars = characters.get();
        let mut counts = std::collections::HashMap::new();
        for c in chars.iter() {
            if let Some(role) = c.get().role {
                *counts.entry(role).or_insert(0) += 1;
            }
        }
        counts
    });

    view! {
        <div class="space-y-6 max-w-3xl mx-auto">
            // Header
            <div class="text-center">
                <h3 class="text-2xl font-bold text-white mb-2">"Party Composition"</h3>
                <p class="text-zinc-400">
                    "Define your party's characters for encounter balancing and story hooks"
                </p>
                <p class="text-xs text-purple-400 mt-2">
                    "This step is optional - skip if you prefer to define characters later"
                </p>
            </div>

            // Level Range
            <div class="flex items-center gap-6 p-4 bg-zinc-800/50 border border-zinc-700 rounded-lg">
                <div class="flex-1">
                    <label class="block text-sm font-medium text-zinc-300 mb-2">
                        "Starting Level"
                    </label>
                    <input
                        type="number"
                        min="1"
                        max="20"
                        class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white
                               focus:border-purple-500 focus:outline-none"
                        prop:value=move || start_level.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse() {
                                start_level.set(v);
                            }
                        }
                    />
                </div>
                <div class="text-zinc-500 pt-6">"to"</div>
                <div class="flex-1">
                    <label class="block text-sm font-medium text-zinc-300 mb-2">
                        "Target End Level"
                    </label>
                    <input
                        type="number"
                        min="1"
                        max="20"
                        class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white
                               focus:border-purple-500 focus:outline-none"
                        prop:value=move || end_level.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse() {
                                end_level.set(v);
                            }
                        }
                    />
                </div>
            </div>

            // Character list
            <div class="space-y-3">
                <div class="flex items-center justify-between">
                    <label class="block text-sm font-medium text-zinc-300">
                        "Party Members"
                    </label>
                    <button
                        type="button"
                        class="flex items-center gap-1 px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-sm rounded-lg transition-colors"
                        on:click=add_character
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                        </svg>
                        "Add Character"
                    </button>
                </div>

                <div class="space-y-2">
                    {move || {
                        characters.get().iter().enumerate().map(|(i, char_signal)| {
                            view! {
                                <CharacterEntry
                                    index=i
                                    character=*char_signal
                                    on_remove=remove_character
                                />
                            }
                        }).collect_view()
                    }}
                </div>
            </div>

            // Role analysis
            <div class="p-4 bg-zinc-800/50 border border-zinc-700 rounded-lg">
                <h4 class="text-sm font-medium text-zinc-300 mb-3">"Role Coverage"</h4>
                <div class="grid grid-cols-4 gap-2">
                    {PartyRole::all().into_iter().map(|role| {
                        let count = Signal::derive(move || {
                            role_counts.get().get(&role).copied().unwrap_or(0)
                        });
                        let has_role = Signal::derive(move || count.get() > 0);

                        view! {
                            <div class=move || format!(
                                "px-3 py-2 rounded text-center text-sm {}",
                                if has_role.get() {
                                    "bg-purple-900/30 text-purple-300"
                                } else {
                                    "bg-zinc-900 text-zinc-500"
                                }
                            )>
                                <div class="font-medium">{role.label()}</div>
                                <div class="text-xs">
                                    {move || if count.get() > 0 {
                                        format!("{}", count.get())
                                    } else {
                                        "Missing".to_string()
                                    }}
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>

                // Suggestions based on missing roles
                {move || {
                    let roles = role_counts.get();
                    let has_tank = roles.contains_key(&PartyRole::Tank);
                    let has_healer = roles.contains_key(&PartyRole::Healer);
                    let has_damage = roles.contains_key(&PartyRole::DamageDealer);

                    let suggestions: Vec<&str> = [
                        (!has_tank).then_some("Consider adding a tank for frontline protection"),
                        (!has_healer).then_some("A healer will help party sustainability"),
                        (!has_damage).then_some("Damage dealers are important for combat efficiency"),
                    ].into_iter().flatten().collect();

                    (!suggestions.is_empty()).then(|| view! {
                        <div class="mt-3 pt-3 border-t border-zinc-700">
                            <div class="text-xs text-amber-400">
                                <strong>"Suggestions: "</strong>
                                {suggestions.join(". ")}
                            </div>
                        </div>
                    })
                }}
            </div>

            // AI assistance hint
            {move || ctx.ai_assisted.get().then(|| view! {
                <div class="p-4 bg-purple-900/20 border border-purple-700/50 rounded-lg">
                    <div class="flex items-start gap-3">
                        <svg class="w-5 h-5 text-purple-400 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                        </svg>
                        <div>
                            <p class="text-sm text-purple-300">
                                "Ask the AI for class suggestions to fill party gaps, or for encounter recommendations based on your composition."
                            </p>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}
