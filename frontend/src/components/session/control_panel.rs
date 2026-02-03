//! Session Control Panel
//!
//! Two-column dashboard for active session management:
//! - Left: Narrative stream, read-aloud box, story beats
//! - Right: Active mechanics, initiative, quick rules
//!
//! Applies typographic hierarchy for at-a-glance information.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Read-aloud box content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadAloudBox {
    pub title: Option<String>,
    pub content: String,
    pub attribution: Option<String>,
}

/// Story beat for narrative tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryBeat {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub beat_type: BeatType,
    pub is_completed: bool,
    pub is_current: bool,
}

/// Beat type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeatType {
    Setup,
    Revelation,
    Conflict,
    Resolution,
    Milestone,
    Optional,
}

impl BeatType {
    pub fn color(&self) -> &'static str {
        match self {
            BeatType::Setup => "border-blue-500",
            BeatType::Revelation => "border-purple-500",
            BeatType::Conflict => "border-red-500",
            BeatType::Resolution => "border-green-500",
            BeatType::Milestone => "border-amber-500",
            BeatType::Optional => "border-zinc-500",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            BeatType::Setup => "Setup",
            BeatType::Revelation => "Reveal",
            BeatType::Conflict => "Conflict",
            BeatType::Resolution => "Resolution",
            BeatType::Milestone => "Milestone",
            BeatType::Optional => "Optional",
        }
    }
}

/// Quick rule reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickRule {
    pub title: String,
    pub content: String,
    pub source: Option<String>,
    pub is_pinned: bool,
}

/// Pinned table reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedTable {
    pub id: String,
    pub title: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub source: Option<String>,
}

// ============================================================================
// Narrative Panel Components (Left Column)
// ============================================================================

/// Read-aloud box component with dramatic styling
#[component]
fn ReadAloudBoxDisplay(
    content: RwSignal<Option<ReadAloudBox>>,
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || content.get().is_some()>
            <div class="relative p-5 bg-gradient-to-br from-purple-900/30 to-zinc-900 border-l-4 border-purple-500 rounded-r-lg">
                // Close button
                <button
                    type="button"
                    class="absolute top-2 right-2 p-1 text-zinc-500 hover:text-white"
                    on:click=move |_| on_close.run(())
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>

                {move || content.get().map(|box_content| view! {
                    <>
                        {box_content.title.map(|t| view! {
                            <h4 class="text-sm font-semibold text-purple-300 mb-2 uppercase tracking-wider">{t}</h4>
                        })}
                        <p class="text-lg text-white italic leading-relaxed font-serif">
                            "\""{ box_content.content.clone() }"\""
                        </p>
                        {box_content.attribution.map(|a| view! {
                            <p class="text-xs text-zinc-500 mt-3 text-right">"- "{ a }</p>
                        })}
                    </>
                })}
            </div>
        </Show>
    }
}

/// Story beats tracker
#[component]
fn StoryBeatsTracker(
    beats: RwSignal<Vec<StoryBeat>>,
    on_toggle_beat: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="space-y-3">
            <h3 class="text-sm font-semibold text-zinc-300 uppercase tracking-wider">"Story Beats"</h3>
            <div class="space-y-2">
                {move || beats.get().iter().map(|beat| {
                    let beat_id = beat.id.clone();
                    let is_current = beat.is_current;
                    let is_completed = beat.is_completed;
                    let beat_type = beat.beat_type;

                    view! {
                        <div
                            class=format!(
                                "p-3 rounded-lg border-l-4 transition-all cursor-pointer {} {}",
                                beat_type.color(),
                                if is_current {
                                    "bg-zinc-800 ring-1 ring-purple-500/50"
                                } else if is_completed {
                                    "bg-zinc-800/50 opacity-60"
                                } else {
                                    "bg-zinc-800/30 hover:bg-zinc-800/50"
                                }
                            )
                            on:click={
                                let id = beat_id.clone();
                                move |_| on_toggle_beat.run(id.clone())
                            }
                        >
                            <div class="flex items-center justify-between">
                                <div class="flex items-center gap-2">
                                    // Completion checkbox
                                    <div class=format!(
                                        "w-4 h-4 rounded border flex items-center justify-center {}",
                                        if is_completed { "bg-green-600 border-green-600" } else { "border-zinc-600" }
                                    )>
                                        {is_completed.then(|| view! {
                                            <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
                                            </svg>
                                        })}
                                    </div>
                                    <span class=format!(
                                        "font-medium {}",
                                        if is_completed { "text-zinc-500 line-through" } else { "text-white" }
                                    )>
                                        {beat.title.clone()}
                                    </span>
                                </div>
                                <span class="text-xs text-zinc-500">{beat_type.label()}</span>
                            </div>
                            {beat.description.clone().map(|d| view! {
                                <p class="text-sm text-zinc-400 mt-1 ml-6">{d}</p>
                            })}
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Narrative stream (recent events/notes)
#[component]
fn NarrativeStream(
    entries: RwSignal<Vec<String>>,
    on_add_entry: Callback<String>,
) -> impl IntoView {
    let new_entry = RwSignal::new(String::new());

    let handle_add = move |_| {
        let entry = new_entry.get();
        if !entry.trim().is_empty() {
            on_add_entry.run(entry);
            new_entry.set(String::new());
        }
    };

    view! {
        <div class="space-y-3">
            <h3 class="text-sm font-semibold text-zinc-300 uppercase tracking-wider">"Session Log"</h3>

            // Entry list
            <div class="space-y-2 max-h-48 overflow-y-auto">
                {move || {
                    let entries_list = entries.get();
                    let total = entries_list.len();
                    entries_list.iter().rev().enumerate().map(|(i, entry)| view! {
                        <div class="text-sm text-zinc-400 py-1 border-b border-zinc-800">
                            <span class="text-zinc-600 mr-2">{format!("{}.", total - i)}</span>
                            {entry.clone()}
                        </div>
                    }).collect_view()
                }}
            </div>

            // Quick add
            <div class="flex gap-2">
                <input
                    type="text"
                    class="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-white text-sm
                           placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                    placeholder="Add note..."
                    prop:value=move || new_entry.get()
                    on:input=move |ev| new_entry.set(event_target_value(&ev))
                    on:keypress=move |ev| {
                        if ev.key() == "Enter" {
                            handle_add(());
                        }
                    }
                />
                <button
                    type="button"
                    class="px-3 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded text-sm"
                    on:click=move |_| handle_add(())
                >
                    "+"
                </button>
            </div>
        </div>
    }
}

// ============================================================================
// Mechanics Panel Components (Right Column)
// ============================================================================

/// Initiative tracker summary (compact)
#[component]
fn InitiativeSummary(
    combatants: Signal<Vec<(String, i32, bool)>>,
    current_turn: Signal<usize>,
) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <h3 class="text-sm font-semibold text-zinc-300 uppercase tracking-wider">"Initiative Order"</h3>
            <div class="space-y-1">
                {move || combatants.get().iter().enumerate().map(|(i, (name, init, is_npc))| {
                    let is_current = i == current_turn.get();
                    view! {
                        <div class=format!(
                            "flex items-center justify-between px-2 py-1 rounded {}",
                            if is_current { "bg-purple-900/50 ring-1 ring-purple-500" } else { "" }
                        )>
                            <div class="flex items-center gap-2">
                                {is_current.then(|| view! {
                                    <span class="text-purple-400">">"</span>
                                })}
                                <span class=format!(
                                    "text-sm {}",
                                    if *is_npc { "text-red-400" } else { "text-green-400" }
                                )>
                                    {name.clone()}
                                </span>
                            </div>
                            <span class="text-xs text-zinc-500 font-mono">{*init}</span>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Quick rules reference
#[component]
fn QuickRules(rules: RwSignal<Vec<QuickRule>>, on_toggle_pin: Callback<usize>) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <h3 class="text-sm font-semibold text-zinc-300 uppercase tracking-wider">"Quick Rules"</h3>
            <div class="space-y-2 max-h-48 overflow-y-auto">
                {move || rules.get().iter().enumerate().map(|(i, rule)| view! {
                    <div class="p-2 bg-zinc-800/50 rounded text-sm">
                        <div class="flex items-center justify-between mb-1">
                            <span class="font-medium text-white">{rule.title.clone()}</span>
                            <button
                                type="button"
                                class=format!(
                                    "p-1 {}",
                                    if rule.is_pinned { "text-amber-400" } else { "text-zinc-600 hover:text-zinc-400" }
                                )
                                on:click=move |_| on_toggle_pin.run(i)
                            >
                                <svg class="w-3 h-3" fill=if rule.is_pinned { "currentColor" } else { "none" } stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M5 5a2 2 0 012-2h10a2 2 0 012 2v16l-7-3.5L5 21V5z" />
                                </svg>
                            </button>
                        </div>
                        <p class="text-zinc-400">{rule.content.clone()}</p>
                        {rule.source.clone().map(|s| view! {
                            <p class="text-xs text-zinc-600 mt-1">"Source: "{s}</p>
                        })}
                    </div>
                }).collect_view()}
            </div>
        </div>
    }
}

/// Pinned tables widget
#[component]
fn PinnedTablesWidget(
    tables: RwSignal<Vec<PinnedTable>>,
    selected_table: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <h3 class="text-sm font-semibold text-zinc-300 uppercase tracking-wider">"Pinned Tables"</h3>

            // Table selector tabs
            <div class="flex gap-1 overflow-x-auto">
                {move || tables.get().iter().map(|table| {
                    let id = table.id.clone();
                    let is_selected = selected_table.get().as_ref() == Some(&id);

                    view! {
                        <button
                            type="button"
                            class=format!(
                                "px-3 py-1 text-xs rounded whitespace-nowrap {}",
                                if is_selected {
                                    "bg-purple-600 text-white"
                                } else {
                                    "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
                                }
                            )
                            on:click={
                                let table_id = id.clone();
                                move |_| selected_table.set(Some(table_id.clone()))
                            }
                        >
                            {table.title.clone()}
                        </button>
                    }
                }).collect_view()}
            </div>

            // Selected table display
            {move || {
                let sel = selected_table.get();
                let table_list = tables.get();
                if let Some(table) = sel.and_then(|id| table_list.iter().find(|t| t.id == id).cloned()) {
                    view! {
                        <div class="overflow-x-auto">
                            <table class="w-full text-xs">
                                <thead>
                                    <tr class="border-b border-zinc-700">
                                        {table.headers.iter().map(|h| view! {
                                            <th class="px-2 py-1 text-left text-zinc-400 font-medium">{h.clone()}</th>
                                        }).collect_view()}
                                    </tr>
                                </thead>
                                <tbody>
                                    {table.rows.iter().map(|row| view! {
                                        <tr class="border-b border-zinc-800">
                                            {row.iter().map(|cell| view! {
                                                <td class="px-2 py-1 text-zinc-300">{cell.clone()}</td>
                                            }).collect_view()}
                                        </tr>
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }.into_any()
                } else {
                    view! { <p class="text-xs text-zinc-500">"No tables pinned"</p> }.into_any()
                }
            }}
        </div>
    }
}

// ============================================================================
// Main Control Panel Component
// ============================================================================

/// Main session control panel with two-column layout
#[component]
pub fn ControlPanel(
    /// Session ID for context (for future API integration)
    #[prop(into)]
    _session_id: String,
    /// Read-aloud box content
    #[prop(default = RwSignal::new(None))]
    read_aloud: RwSignal<Option<ReadAloudBox>>,
    /// Story beats
    #[prop(default = RwSignal::new(vec![]))]
    story_beats: RwSignal<Vec<StoryBeat>>,
    /// Narrative log entries
    #[prop(default = RwSignal::new(vec![]))]
    narrative_entries: RwSignal<Vec<String>>,
    /// Combatants for initiative (name, initiative, is_npc)
    #[prop(default = Signal::derive(|| vec![]))]
    combatants: Signal<Vec<(String, i32, bool)>>,
    /// Current turn index
    #[prop(default = Signal::derive(|| 0))]
    current_turn: Signal<usize>,
    /// Quick rules
    #[prop(default = RwSignal::new(vec![]))]
    quick_rules: RwSignal<Vec<QuickRule>>,
    /// Pinned tables
    #[prop(default = RwSignal::new(vec![]))]
    pinned_tables: RwSignal<Vec<PinnedTable>>,
) -> impl IntoView {
    let selected_table = RwSignal::new(pinned_tables.get().first().map(|t| t.id.clone()));

    // Handlers
    let on_close_read_aloud = Callback::new(move |_: ()| {
        read_aloud.set(None);
    });

    let on_toggle_beat = Callback::new(move |id: String| {
        story_beats.update(|beats| {
            if let Some(beat) = beats.iter_mut().find(|b| b.id == id) {
                beat.is_completed = !beat.is_completed;
            }
        });
    });

    let on_add_entry = Callback::new(move |entry: String| {
        narrative_entries.update(|entries| entries.push(entry));
    });

    let on_toggle_rule_pin = Callback::new(move |index: usize| {
        quick_rules.update(|rules| {
            if let Some(rule) = rules.get_mut(index) {
                rule.is_pinned = !rule.is_pinned;
            }
        });
    });

    view! {
        <div class="grid grid-cols-2 gap-6 h-full">
            // Left Column: Narrative
            <div class="flex flex-col gap-4 overflow-y-auto">
                <h2 class="text-lg font-bold text-white">
                    "Narrative"
                </h2>

                // Read-Aloud Box (prominent when present)
                <ReadAloudBoxDisplay
                    content=read_aloud
                    on_close=on_close_read_aloud
                />

                // Story Beats
                <StoryBeatsTracker
                    beats=story_beats
                    on_toggle_beat=on_toggle_beat
                />

                // Narrative Stream
                <NarrativeStream
                    entries=narrative_entries
                    on_add_entry=on_add_entry
                />
            </div>

            // Right Column: Mechanics
            <div class="flex flex-col gap-4 overflow-y-auto">
                <h2 class="text-lg font-bold text-white">
                    "Mechanics"
                </h2>

                // Initiative (if combat active)
                <Show when=move || !combatants.get().is_empty()>
                    <InitiativeSummary
                        combatants=combatants
                        current_turn=current_turn
                    />
                </Show>

                // Quick Rules
                <QuickRules
                    rules=quick_rules
                    on_toggle_pin=on_toggle_rule_pin
                />

                // Pinned Tables
                <PinnedTablesWidget
                    tables=pinned_tables
                    selected_table=selected_table
                />
            </div>
        </div>
    }
}
