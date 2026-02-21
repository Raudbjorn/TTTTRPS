//! Character Background Preview
//!
//! Preview component for AI-generated character backgrounds
//! with editable fields.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Character background data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterBackground {
    pub personality_traits: Vec<String>,
    pub ideals: Vec<String>,
    pub bonds: Vec<String>,
    pub flaws: Vec<String>,
    pub backstory_summary: String,
    pub key_events: Vec<BackgroundEvent>,
    pub connections: Vec<BackgroundConnection>,
}

/// Key event in character's past
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackgroundEvent {
    pub name: String,
    pub description: String,
    /// Age at event (u16 to support ages up to 65535, e.g., for long-lived races like elves)
    pub age_at_event: Option<u16>,
}

/// Connection to other entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundConnection {
    pub name: String,
    pub relationship: String,
    pub status: ConnectionStatus,
}

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Alive,
    Dead,
    Missing,
    Estranged,
    Unknown,
}

impl ConnectionStatus {
    pub fn label(&self) -> &'static str {
        match self {
            ConnectionStatus::Alive => "Alive",
            ConnectionStatus::Dead => "Deceased",
            ConnectionStatus::Missing => "Missing",
            ConnectionStatus::Estranged => "Estranged",
            ConnectionStatus::Unknown => "Unknown",
        }
    }
}

// ============================================================================
// Components
// ============================================================================

/// Local wrapper for traits with stable IDs for safe removal
#[derive(Debug, Clone)]
struct TraitEntry {
    id: String,
    text: RwSignal<String>,
}

impl TraitEntry {
    fn new(text: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            text: RwSignal::new(text),
        }
    }
}

/// Editable trait list
#[component]
fn TraitList(
    label: &'static str,
    traits: RwSignal<Vec<String>>,
    is_editing: Signal<bool>,
) -> impl IntoView {
    // Local state with IDs for safe removal
    let local_traits: RwSignal<Vec<TraitEntry>> = RwSignal::new(
        traits
            .get_untracked()
            .into_iter()
            .map(TraitEntry::new)
            .collect(),
    );

    // Sync local -> parent when local changes
    Effect::new(move |_| {
        let strings: Vec<String> = local_traits.get().iter().map(|e| e.text.get()).collect();
        traits.set(strings);
    });

    let add_trait = move |_| {
        local_traits.update(|t| t.push(TraitEntry::new(String::new())));
    };

    view! {
        <div class="space-y-2">
            <div class="flex items-center justify-between">
                <label class="text-sm font-medium text-zinc-300">{label}</label>
                <Show when=move || is_editing.get()>
                    <button
                        type="button"
                        class="text-xs text-purple-400 hover:text-purple-300"
                        on:click=add_trait
                    >
                        "+ Add"
                    </button>
                </Show>
            </div>

            <div class="space-y-1">
                {move || {
                    let trait_list = local_traits.get();
                    if trait_list.is_empty() {
                        view! {
                            <p class="text-sm text-zinc-500 italic">"None defined"</p>
                        }.into_any()
                    } else {
                        trait_list.iter().map(|entry| {
                            let entry_id = entry.id.clone();
                            let entry_text = entry.text;
                            let remove_id = entry_id.clone();
                            view! {
                                <div class="flex items-center gap-2">
                                    {move || if is_editing.get() {
                                        let remove_id = remove_id.clone();
                                        view! {
                                            <input
                                                type="text"
                                                class="flex-1 px-2 py-1 bg-zinc-900 border border-zinc-700 rounded text-white text-sm"
                                                prop:value=move || entry_text.get()
                                                on:input=move |ev| {
                                                    entry_text.set(event_target_value(&ev));
                                                }
                                            />
                                            <button
                                                type="button"
                                                class="p-1 text-zinc-500 hover:text-red-400"
                                                on:click=move |_| {
                                                    let id_to_remove = remove_id.clone();
                                                    local_traits.update(|t| t.retain(|e| e.id != id_to_remove));
                                                }
                                            >
                                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                                </svg>
                                            </button>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <span class="text-sm text-zinc-300">"- "{move || entry_text.get()}</span>
                                        }.into_any()
                                    }}
                                </div>
                            }
                        }).collect_view().into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Event entry component
#[component]
fn EventEntry(
    event: RwSignal<BackgroundEvent>,
    is_editing: Signal<bool>,
    on_remove: Callback<()>,
) -> impl IntoView {
    let name = RwSignal::new(event.get().name);
    let description = RwSignal::new(event.get().description);

    // Use get_untracked() to read signals without subscribing, avoiding infinite loop
    // The Effect::watch pattern: watch specific signals and handle changes in the handler
    Effect::watch(
        move || (name.get(), description.get()),
        move |(new_name, new_desc), _, _| {
            event.update(|e| {
                e.name = new_name.clone();
                e.description = new_desc.clone();
            });
        },
        false,
    );

    view! {
        <div class="p-3 bg-zinc-900/50 rounded-lg">
            {move || if is_editing.get() {
                view! {
                    <div class="space-y-2">
                        <div class="flex items-center gap-2">
                            <input
                                type="text"
                                class="flex-1 px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-white text-sm"
                                placeholder="Event name"
                                prop:value=move || name.get()
                                on:input=move |ev| name.set(event_target_value(&ev))
                            />
                            <button
                                type="button"
                                class="p-1 text-zinc-500 hover:text-red-400"
                                on:click=move |_| on_remove.run(())
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                </svg>
                            </button>
                        </div>
                        <textarea
                            class="w-full px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-white text-sm resize-none"
                            rows="2"
                            placeholder="What happened..."
                            prop:value=move || description.get()
                            on:input=move |ev| description.set(event_target_value(&ev))
                        />
                    </div>
                }.into_any()
            } else {
                view! {
                    <div>
                        <h5 class="font-medium text-white text-sm">{move || name.get()}</h5>
                        <p class="text-sm text-zinc-400 mt-1">{move || description.get()}</p>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

/// Character background preview component
#[component]
pub fn CharacterBackgroundPreview(
    /// The background data
    background: RwSignal<CharacterBackground>,
    /// Whether in edit mode
    #[prop(default = RwSignal::new(false))]
    is_editing: RwSignal<bool>,
) -> impl IntoView {
    let personality_traits = RwSignal::new(background.get().personality_traits);
    let ideals = RwSignal::new(background.get().ideals);
    let bonds = RwSignal::new(background.get().bonds);
    let flaws = RwSignal::new(background.get().flaws);
    let backstory = RwSignal::new(background.get().backstory_summary);
    let events: RwSignal<Vec<RwSignal<BackgroundEvent>>> = RwSignal::new(
        background
            .get()
            .key_events
            .into_iter()
            .map(RwSignal::new)
            .collect(),
    );

    let is_editing_signal = Signal::derive(move || is_editing.get());

    // Sync back to parent
    Effect::new(move |_| {
        background.update(|b| {
            b.personality_traits = personality_traits.get();
            b.ideals = ideals.get();
            b.bonds = bonds.get();
            b.flaws = flaws.get();
            b.backstory_summary = backstory.get();
            b.key_events = events.get().iter().map(|e| e.get()).collect();
        });
    });

    let add_event = move |_| {
        events.update(|e| {
            e.push(RwSignal::new(BackgroundEvent {
                name: String::new(),
                description: String::new(),
                age_at_event: None,
            }));
        });
    };

    view! {
        <div class="space-y-4">
            // Backstory
            <div>
                <label class="block text-sm font-medium text-zinc-300 mb-2">"Backstory"</label>
                {move || if is_editing.get() {
                    view! {
                        <textarea
                            class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm resize-none"
                            rows="4"
                            prop:value=move || backstory.get()
                            on:input=move |ev| backstory.set(event_target_value(&ev))
                        />
                    }.into_any()
                } else {
                    view! {
                        <p class="text-sm text-zinc-300 whitespace-pre-wrap">{move || backstory.get()}</p>
                    }.into_any()
                }}
            </div>

            // Traits grid
            <div class="grid grid-cols-2 gap-4">
                <TraitList label="Personality Traits" traits=personality_traits is_editing=is_editing_signal />
                <TraitList label="Ideals" traits=ideals is_editing=is_editing_signal />
                <TraitList label="Bonds" traits=bonds is_editing=is_editing_signal />
                <TraitList label="Flaws" traits=flaws is_editing=is_editing_signal />
            </div>

            // Key Events
            <div>
                <div class="flex items-center justify-between mb-2">
                    <label class="text-sm font-medium text-zinc-300">"Key Events"</label>
                    <Show when=move || is_editing.get()>
                        <button
                            type="button"
                            class="text-xs text-purple-400 hover:text-purple-300"
                            on:click=add_event
                        >
                            "+ Add Event"
                        </button>
                    </Show>
                </div>
                <div class="space-y-2">
                    {move || {
                        events.get().iter().map(|event| {
                            // Capture event data for content-based removal
                            // (avoids index shift issues between render and callback)
                            let event_to_remove = event.clone();
                            let remove_cb = Callback::new(move |_: ()| {
                                let target = event_to_remove.clone();
                                events.update(|e| {
                                    e.retain(|item| item != &target);
                                });
                            });
                            view! {
                                <EventEntry
                                    event=*event
                                    is_editing=is_editing_signal
                                    on_remove=remove_cb
                                />
                            }
                        }).collect_view()
                    }}
                </div>
            </div>
        </div>
    }
}
