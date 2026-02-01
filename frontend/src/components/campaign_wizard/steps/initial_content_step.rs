//! Initial Content Step - Starting locations, NPCs, and plot hooks
//!
//! Optional step for creating initial campaign content.

use leptos::prelude::*;

use crate::services::wizard_state::{
    use_wizard_context, InitialContentData, LocationDraft, NpcDraft, PlotHookDraft, PlotHookType,
    StepData,
};

/// Collapsible section component
#[component]
fn CollapsibleSection(
    title: &'static str,
    count: Signal<usize>,
    #[prop(default = false)]
    default_open: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let is_open = RwSignal::new(default_open);

    view! {
        <div class="border border-zinc-700 rounded-lg overflow-hidden">
            <button
                type="button"
                class="w-full flex items-center justify-between px-4 py-3 bg-zinc-800/50 hover:bg-zinc-800 transition-colors"
                on:click=move |_| is_open.update(|v| *v = !*v)
            >
                <div class="flex items-center gap-2">
                    <span class="font-medium text-white">{title}</span>
                    <span class="px-2 py-0.5 bg-zinc-700 text-zinc-400 text-xs rounded-full">
                        {move || count.get()}
                    </span>
                </div>
                <svg
                    class=move || format!(
                        "w-5 h-5 text-zinc-400 transition-transform {}",
                        if is_open.get() { "rotate-180" } else { "" }
                    )
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                </svg>
            </button>

            <Show when=move || is_open.get()>
                <div class="p-4 border-t border-zinc-700">
                    {children()}
                </div>
            </Show>
        </div>
    }
}

/// Location entry component
#[component]
fn LocationEntry(
    location: RwSignal<LocationDraft>,
    on_remove: Callback<()>,
) -> impl IntoView {
    let name = RwSignal::new(location.get().name);
    let location_type = RwSignal::new(location.get().location_type.unwrap_or_default());
    let description = RwSignal::new(location.get().description.unwrap_or_default());
    let is_starting = RwSignal::new(location.get().is_starting_location);

    // Use the existing ID from the location signal
    let loc_id = location.get_untracked().id.clone();
    Effect::new(move |_| {
        location.set(LocationDraft {
            id: loc_id.clone(),
            name: name.get(),
            location_type: if location_type.get().is_empty() { None } else { Some(location_type.get()) },
            description: if description.get().is_empty() { None } else { Some(description.get()) },
            is_starting_location: is_starting.get(),
        });
    });

    view! {
        <div class="p-3 bg-zinc-900/50 rounded-lg space-y-2">
            <div class="flex items-start gap-2">
                <input
                    type="text"
                    class="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm
                           placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                    placeholder="Location name"
                    prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                />
                <select
                    class="w-32 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm
                           focus:border-purple-500 focus:outline-none"
                    prop:value=move || location_type.get()
                    on:change=move |ev| location_type.set(event_target_value(&ev))
                >
                    <option value="">"Type..."</option>
                    <option value="city">"City"</option>
                    <option value="town">"Town"</option>
                    <option value="village">"Village"</option>
                    <option value="dungeon">"Dungeon"</option>
                    <option value="wilderness">"Wilderness"</option>
                    <option value="building">"Building"</option>
                    <option value="other">"Other"</option>
                </select>
                <button
                    type="button"
                    class="p-2 text-zinc-500 hover:text-red-400 transition-colors"
                    on:click=move |_| on_remove.run(())
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>

            <textarea
                class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm
                       placeholder-zinc-500 focus:border-purple-500 focus:outline-none resize-none"
                rows="2"
                placeholder="Brief description..."
                prop:value=move || description.get()
                on:input=move |ev| description.set(event_target_value(&ev))
            />

            <label class="flex items-center gap-2 text-sm text-zinc-400">
                <input
                    type="checkbox"
                    class="w-4 h-4 rounded bg-zinc-800 border-zinc-700 text-purple-600 focus:ring-purple-500"
                    prop:checked=move || is_starting.get()
                    on:change=move |ev| is_starting.set(event_target_checked(&ev))
                />
                "Starting location"
            </label>
        </div>
    }
}

/// NPC entry component
#[component]
fn NpcEntry(
    npc: RwSignal<NpcDraft>,
    on_remove: Callback<()>,
) -> impl IntoView {
    let name = RwSignal::new(npc.get().name);
    let role = RwSignal::new(npc.get().role.unwrap_or_default());
    let description = RwSignal::new(npc.get().description.unwrap_or_default());

    Effect::new(move |_| {
        npc.update(|n| {
            n.name = name.get();
            n.role = if role.get().is_empty() { None } else { Some(role.get()) };
            n.description = if description.get().is_empty() { None } else { Some(description.get()) };
        });
    });

    view! {
        <div class="p-3 bg-zinc-900/50 rounded-lg space-y-2">
            <div class="flex items-start gap-2">
                <input
                    type="text"
                    class="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm
                           placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                    placeholder="NPC name"
                    prop:value=move || name.get()
                    on:input=move |ev| name.set(event_target_value(&ev))
                />
                <select
                    class="w-32 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm
                           focus:border-purple-500 focus:outline-none"
                    prop:value=move || role.get()
                    on:change=move |ev| role.set(event_target_value(&ev))
                >
                    <option value="">"Role..."</option>
                    <option value="ally">"Ally"</option>
                    <option value="patron">"Patron"</option>
                    <option value="villain">"Villain"</option>
                    <option value="merchant">"Merchant"</option>
                    <option value="informant">"Informant"</option>
                    <option value="neutral">"Neutral"</option>
                </select>
                <button
                    type="button"
                    class="p-2 text-zinc-500 hover:text-red-400 transition-colors"
                    on:click=move |_| on_remove.run(())
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>

            <textarea
                class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm
                       placeholder-zinc-500 focus:border-purple-500 focus:outline-none resize-none"
                rows="2"
                placeholder="Brief description..."
                prop:value=move || description.get()
                on:input=move |ev| description.set(event_target_value(&ev))
            />
        </div>
    }
}

/// Plot hook entry component
#[component]
fn PlotHookEntry(
    hook: RwSignal<PlotHookDraft>,
    on_remove: Callback<()>,
) -> impl IntoView {
    let title = RwSignal::new(hook.get().title);
    let description = RwSignal::new(hook.get().description.unwrap_or_default());
    let hook_type = RwSignal::new(hook.get().hook_type);

    // Use the existing ID from the hook signal
    let hook_id = hook.get_untracked().id.clone();
    Effect::new(move |_| {
        hook.set(PlotHookDraft {
            id: hook_id.clone(),
            title: title.get(),
            description: if description.get().is_empty() { None } else { Some(description.get()) },
            hook_type: hook_type.get(),
        });
    });

    view! {
        <div class="p-3 bg-zinc-900/50 rounded-lg space-y-2">
            <div class="flex items-start gap-2">
                <input
                    type="text"
                    class="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm
                           placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                    placeholder="Plot hook title"
                    prop:value=move || title.get()
                    on:input=move |ev| title.set(event_target_value(&ev))
                />
                <select
                    class="w-40 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm
                           focus:border-purple-500 focus:outline-none"
                    prop:value=move || {
                        match hook_type.get() {
                            Some(PlotHookType::MainQuest) => "main",
                            Some(PlotHookType::SideQuest) => "side",
                            Some(PlotHookType::CharacterTie) => "character",
                            Some(PlotHookType::WorldEvent) => "world",
                            Some(PlotHookType::Mystery) => "mystery",
                            None => "",
                        }
                    }
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        hook_type.set(match value.as_str() {
                            "main" => Some(PlotHookType::MainQuest),
                            "side" => Some(PlotHookType::SideQuest),
                            "character" => Some(PlotHookType::CharacterTie),
                            "world" => Some(PlotHookType::WorldEvent),
                            "mystery" => Some(PlotHookType::Mystery),
                            _ => None,
                        });
                    }
                >
                    <option value="">"Type..."</option>
                    <option value="main">"Main Quest"</option>
                    <option value="side">"Side Quest"</option>
                    <option value="character">"Character Tie"</option>
                    <option value="world">"World Event"</option>
                    <option value="mystery">"Mystery"</option>
                </select>
                <button
                    type="button"
                    class="p-2 text-zinc-500 hover:text-red-400 transition-colors"
                    on:click=move |_| on_remove.run(())
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>

            <textarea
                class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm
                       placeholder-zinc-500 focus:border-purple-500 focus:outline-none resize-none"
                rows="2"
                placeholder="Describe the hook..."
                prop:value=move || description.get()
                on:input=move |ev| description.set(event_target_value(&ev))
            />
        </div>
    }
}

/// Initial content step component
#[component]
pub fn InitialContentStep(
    form_data: RwSignal<Option<StepData>>,
    form_valid: RwSignal<bool>,
) -> impl IntoView {
    let ctx = use_wizard_context();
    let draft = ctx.draft();
    let content = draft.initial_content.unwrap_or_default();

    // Local form state
    let locations: RwSignal<Vec<RwSignal<LocationDraft>>> = RwSignal::new(
        content.locations.into_iter().map(|l| RwSignal::new(l)).collect()
    );
    let npcs: RwSignal<Vec<RwSignal<NpcDraft>>> = RwSignal::new(
        content.npcs.into_iter().map(|n| RwSignal::new(n)).collect()
    );
    let plot_hooks: RwSignal<Vec<RwSignal<PlotHookDraft>>> = RwSignal::new(
        content.plot_hooks.into_iter().map(|h| RwSignal::new(h)).collect()
    );

    // This step is always valid (optional)
    Effect::new(move |_| {
        form_valid.set(true);
    });

    // Update form_data when inputs change
    Effect::new(move |_| {
        form_data.set(Some(StepData::InitialContent(InitialContentData {
            locations: locations.get().iter().map(|l| l.get()).collect(),
            npcs: npcs.get().iter().map(|n| n.get()).collect(),
            plot_hooks: plot_hooks.get().iter().map(|h| h.get()).collect(),
        })));
    });

    // Add handlers
    let add_location = move |_| {
        locations.update(|locs| {
            locs.push(RwSignal::new(LocationDraft {
                id: uuid::Uuid::new_v4().to_string(),
                name: String::new(),
                location_type: None,
                description: None,
                is_starting_location: locs.is_empty(),
            }));
        });
    };

    let add_npc = move |_| {
        npcs.update(|n| {
            n.push(RwSignal::new(NpcDraft {
                id: uuid::Uuid::new_v4().to_string(),
                name: String::new(),
                role: None,
                description: None,
                location: None,
            }));
        });
    };

    let add_hook = move |_| {
        plot_hooks.update(|h| {
            h.push(RwSignal::new(PlotHookDraft {
                id: uuid::Uuid::new_v4().to_string(),
                title: String::new(),
                description: None,
                hook_type: None,
            }));
        });
    };

    // Counts for section headers
    let location_count = Signal::derive(move || locations.get().len());
    let npc_count = Signal::derive(move || npcs.get().len());
    let hook_count = Signal::derive(move || plot_hooks.get().len());

    view! {
        <div class="space-y-6 max-w-3xl mx-auto">
            // Header
            <div class="text-center">
                <h3 class="text-2xl font-bold text-white mb-2">"Initial Content"</h3>
                <p class="text-zinc-400">
                    "Define starting locations, NPCs, and plot hooks for your campaign"
                </p>
                <p class="text-xs text-purple-400 mt-2">
                    "This step is optional - you can add content later"
                </p>
            </div>

            // Locations Section
            <CollapsibleSection title="Locations" count=location_count default_open=true>
                <div class="space-y-3">
                    {move || locations.get().iter().map(|loc| {
                        // Capture the ID for stable removal
                        let loc_id = loc.get_untracked().id.clone();
                        let remove_cb = Callback::new(move |_: ()| {
                            let id_to_remove = loc_id.clone();
                            locations.update(|locs| {
                                locs.retain(|l| l.get_untracked().id != id_to_remove);
                            });
                        });
                        view! {
                            <LocationEntry location=*loc on_remove=remove_cb />
                        }
                    }).collect_view()}

                    <button
                        type="button"
                        class="w-full py-2 border border-dashed border-zinc-700 rounded-lg text-zinc-400 text-sm hover:border-zinc-600 hover:text-zinc-300 transition-colors"
                        on:click=add_location
                    >
                        "+ Add Location"
                    </button>
                </div>
            </CollapsibleSection>

            // NPCs Section
            <CollapsibleSection title="NPCs" count=npc_count>
                <div class="space-y-3">
                    {move || npcs.get().iter().map(|npc| {
                        // Capture the ID for stable removal
                        let npc_id = npc.get_untracked().id.clone();
                        let remove_cb = Callback::new(move |_: ()| {
                            let id_to_remove = npc_id.clone();
                            npcs.update(|n| {
                                n.retain(|npc| npc.get_untracked().id != id_to_remove);
                            });
                        });
                        view! {
                            <NpcEntry npc=*npc on_remove=remove_cb />
                        }
                    }).collect_view()}

                    <button
                        type="button"
                        class="w-full py-2 border border-dashed border-zinc-700 rounded-lg text-zinc-400 text-sm hover:border-zinc-600 hover:text-zinc-300 transition-colors"
                        on:click=add_npc
                    >
                        "+ Add NPC"
                    </button>
                </div>
            </CollapsibleSection>

            // Plot Hooks Section
            <CollapsibleSection title="Plot Hooks" count=hook_count>
                <div class="space-y-3">
                    {move || plot_hooks.get().iter().map(|hook| {
                        // Capture the ID for stable removal
                        let hook_id = hook.get_untracked().id.clone();
                        let remove_cb = Callback::new(move |_: ()| {
                            let id_to_remove = hook_id.clone();
                            plot_hooks.update(|h| {
                                h.retain(|hook| hook.get_untracked().id != id_to_remove);
                            });
                        });
                        view! {
                            <PlotHookEntry hook=*hook on_remove=remove_cb />
                        }
                    }).collect_view()}

                    <button
                        type="button"
                        class="w-full py-2 border border-dashed border-zinc-700 rounded-lg text-zinc-400 text-sm hover:border-zinc-600 hover:text-zinc-300 transition-colors"
                        on:click=add_hook
                    >
                        "+ Add Plot Hook"
                    </button>
                </div>
            </CollapsibleSection>

            // AI hint
            {move || ctx.ai_assisted.get().then(|| view! {
                <div class="p-4 bg-purple-900/20 border border-purple-700/50 rounded-lg">
                    <div class="flex items-start gap-3">
                        <svg class="w-5 h-5 text-purple-400 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                        </svg>
                        <div>
                            <p class="text-sm text-purple-300">
                                "Ask the AI to generate starting locations, memorable NPCs, or compelling plot hooks based on your campaign themes and vision."
                            </p>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}
