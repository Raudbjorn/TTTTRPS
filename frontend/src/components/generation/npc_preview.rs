//! NPC Preview Component
//!
//! Preview component for AI-generated NPCs with importance-based
//! detail display levels.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// NPC importance level determines detail display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NpcImportance {
    /// Walk-on role, minimal details
    Minor,
    /// Recurring character, moderate details
    #[default]
    Supporting,
    /// Major character, full details
    Major,
    /// Antagonist or key ally, maximum detail
    Central,
}

impl NpcImportance {
    /// Get the numeric level for comparison (explicit, not relying on enum order)
    pub fn level(&self) -> u8 {
        match self {
            NpcImportance::Minor => 0,
            NpcImportance::Supporting => 1,
            NpcImportance::Major => 2,
            NpcImportance::Central => 3,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            NpcImportance::Minor => "Minor",
            NpcImportance::Supporting => "Supporting",
            NpcImportance::Major => "Major",
            NpcImportance::Central => "Central",
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            NpcImportance::Minor => "bg-zinc-700 text-zinc-300",
            NpcImportance::Supporting => "bg-blue-900 text-blue-300",
            NpcImportance::Major => "bg-purple-900 text-purple-300",
            NpcImportance::Central => "bg-amber-900 text-amber-300",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            NpcImportance::Minor,
            NpcImportance::Supporting,
            NpcImportance::Major,
            NpcImportance::Central,
        ]
    }
}

/// Generated NPC data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneratedNpc {
    pub name: String,
    pub role: String,
    pub importance: NpcImportance,
    pub appearance: String,
    pub personality_summary: String,
    pub motivations: Vec<String>,
    pub secrets: Vec<String>,
    pub connections: Vec<NpcConnection>,
    pub speech_patterns: Option<SpeechPattern>,
    pub stat_block: Option<String>,
}

/// NPC connection to other entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcConnection {
    pub target_name: String,
    pub relationship: String,
    pub sentiment: Sentiment,
}

/// Sentiment toward another entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
    Complex,
}

impl Sentiment {
    pub fn icon(&self) -> &'static str {
        match self {
            Sentiment::Positive => "+",
            Sentiment::Neutral => "=",
            Sentiment::Negative => "-",
            Sentiment::Complex => "~",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Sentiment::Positive => "text-green-400",
            Sentiment::Neutral => "text-zinc-400",
            Sentiment::Negative => "text-red-400",
            Sentiment::Complex => "text-amber-400",
        }
    }
}

/// Speech patterns for NPC voice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechPattern {
    pub vocabulary_level: String,
    pub common_phrases: Vec<String>,
    pub accent_notes: Option<String>,
    pub sample_dialogue: Option<String>,
}

// ============================================================================
// Components
// ============================================================================

/// Importance badge
#[component]
fn ImportanceBadge(importance: Signal<NpcImportance>) -> impl IntoView {
    view! {
        <span class=move || format!(
            "px-2 py-0.5 text-xs rounded-full font-medium {}",
            importance.get().color_class()
        )>
            {move || importance.get().label()}
        </span>
    }
}

/// Detail section that shows based on importance
#[component]
fn ImportanceSection(
    importance: Signal<NpcImportance>,
    min_importance: NpcImportance,
    label: &'static str,
    children: ChildrenFn,
) -> impl IntoView {
    let should_show = Signal::derive(move || importance.get().level() >= min_importance.level());

    view! {
        <Show when=move || should_show.get()>
            <div class="space-y-2">
                <h5 class="text-sm font-medium text-zinc-400">{label}</h5>
                {children()}
            </div>
        </Show>
    }
}

/// Connection list
#[component]
fn ConnectionList(connections: Signal<Vec<NpcConnection>>) -> impl IntoView {
    view! {
        <div class="space-y-1">
            {move || connections.get().iter().map(|conn| {
                let conn = conn.clone();
                view! {
                    <div class="flex items-center gap-2 text-sm">
                        <span class=format!("font-medium {}", conn.sentiment.color())>
                            {conn.sentiment.icon()}
                        </span>
                        <span class="text-zinc-300">{conn.target_name}</span>
                        <span class="text-zinc-500">" - "{conn.relationship}</span>
                    </div>
                }
            }).collect_view()}
        </div>
    }
}

/// Speech pattern display
#[component]
fn SpeechPatternDisplay(pattern: SpeechPattern) -> impl IntoView {
    view! {
        <div class="space-y-2 p-3 bg-zinc-900/50 rounded-lg">
            <div class="flex items-center gap-2 text-sm">
                <span class="text-zinc-400">"Vocabulary:"</span>
                <span class="text-zinc-300">{pattern.vocabulary_level}</span>
            </div>

            {pattern.accent_notes.map(|notes| view! {
                <div class="text-sm">
                    <span class="text-zinc-400">"Accent: "</span>
                    <span class="text-zinc-300">{notes}</span>
                </div>
            })}

            {(!pattern.common_phrases.is_empty()).then(|| view! {
                <div class="text-sm">
                    <span class="text-zinc-400">"Common phrases: "</span>
                    <span class="text-zinc-300 italic">
                        {pattern.common_phrases.into_iter()
                            .map(|p| format!("\"{}\"", p))
                            .collect::<Vec<_>>()
                            .join(", ")}
                    </span>
                </div>
            })}

            {pattern.sample_dialogue.map(|sample| view! {
                <div class="mt-2 p-2 bg-zinc-800 rounded border-l-2 border-purple-500">
                    <p class="text-sm text-zinc-300 italic">{sample}</p>
                </div>
            })}
        </div>
    }
}

/// Main NPC preview component
#[component]
pub fn NpcPreview(
    /// The NPC data to display
    npc: RwSignal<GeneratedNpc>,
    /// Whether in edit mode
    #[prop(default = RwSignal::new(false))]
    is_editing: RwSignal<bool>,
) -> impl IntoView {
    let importance = Signal::derive(move || npc.get().importance);
    let connections = Signal::derive(move || npc.get().connections);

    view! {
        <div class="space-y-4">
            // Header
            <div class="flex items-start justify-between">
                <div>
                    {move || if is_editing.get() {
                        view! {
                            <input
                                type="text"
                                class="text-lg font-bold bg-zinc-900 border border-zinc-700 rounded px-2 py-1 text-white"
                                prop:value=move || npc.get().name
                                on:input=move |ev| npc.update(|n| n.name = event_target_value(&ev))
                            />
                        }.into_any()
                    } else {
                        view! {
                            <h4 class="text-lg font-bold text-white">{move || npc.get().name}</h4>
                        }.into_any()
                    }}
                    <p class="text-sm text-zinc-400">{move || npc.get().role}</p>
                </div>
                <ImportanceBadge importance=importance />
            </div>

            // Importance selector (when editing)
            <Show when=move || is_editing.get()>
                <div class="flex items-center gap-2">
                    <span class="text-sm text-zinc-400">"Importance:"</span>
                    {NpcImportance::all().into_iter().map(|imp| {
                        let is_selected = Signal::derive(move || npc.get().importance == imp);
                        view! {
                            <button
                                type="button"
                                class=move || format!(
                                    "px-2 py-1 text-xs rounded {}",
                                    if is_selected.get() { imp.color_class() } else { "bg-zinc-800 text-zinc-400" }
                                )
                                on:click=move |_| npc.update(|n| n.importance = imp)
                            >
                                {imp.label()}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </Show>

            // Appearance (always shown)
            <div>
                <h5 class="text-sm font-medium text-zinc-400 mb-1">"Appearance"</h5>
                {move || if is_editing.get() {
                    view! {
                        <textarea
                            class="w-full px-2 py-1 bg-zinc-900 border border-zinc-700 rounded text-white text-sm resize-none"
                            rows="2"
                            prop:value=move || npc.get().appearance
                            on:input=move |ev| npc.update(|n| n.appearance = event_target_value(&ev))
                        />
                    }.into_any()
                } else {
                    view! {
                        <p class="text-sm text-zinc-300">{move || npc.get().appearance}</p>
                    }.into_any()
                }}
            </div>

            // Personality (always shown)
            <div>
                <h5 class="text-sm font-medium text-zinc-400 mb-1">"Personality"</h5>
                {move || if is_editing.get() {
                    view! {
                        <textarea
                            class="w-full px-2 py-1 bg-zinc-900 border border-zinc-700 rounded text-white text-sm resize-none"
                            rows="2"
                            prop:value=move || npc.get().personality_summary
                            on:input=move |ev| npc.update(|n| n.personality_summary = event_target_value(&ev))
                        />
                    }.into_any()
                } else {
                    view! {
                        <p class="text-sm text-zinc-300">{move || npc.get().personality_summary}</p>
                    }.into_any()
                }}
            </div>

            // Motivations (Supporting+)
            <ImportanceSection importance=importance min_importance=NpcImportance::Supporting label="Motivations">
                <ul class="text-sm text-zinc-300 list-disc list-inside">
                    {move || npc.get().motivations.iter().map(|m| view! {
                        <li>{m.clone()}</li>
                    }).collect_view()}
                </ul>
            </ImportanceSection>

            // Connections (Supporting+)
            <ImportanceSection importance=importance min_importance=NpcImportance::Supporting label="Connections">
                <ConnectionList connections=connections />
            </ImportanceSection>

            // Secrets (Major+)
            <ImportanceSection importance=importance min_importance=NpcImportance::Major label="Secrets">
                <ul class="text-sm text-amber-300/80 list-disc list-inside">
                    {move || npc.get().secrets.iter().map(|s| view! {
                        <li>{s.clone()}</li>
                    }).collect_view()}
                </ul>
            </ImportanceSection>

            // Speech Patterns (Major+)
            <ImportanceSection importance=importance min_importance=NpcImportance::Major label="Speech Patterns">
                {move || npc.get().speech_patterns.clone().map(|sp| view! {
                    <SpeechPatternDisplay pattern=sp />
                })}
            </ImportanceSection>

            // Stat Block (Central only)
            <ImportanceSection importance=importance min_importance=NpcImportance::Central label="Stat Block">
                {move || npc.get().stat_block.clone().map(|stats| view! {
                    <pre class="text-xs text-zinc-400 bg-zinc-900 p-2 rounded overflow-x-auto">{stats}</pre>
                })}
            </ImportanceSection>
        </div>
    }
}
