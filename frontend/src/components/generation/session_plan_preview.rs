//! Session Plan Preview Component
//!
//! Preview component for AI-generated session plans with scene timeline.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Session plan data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionPlan {
    pub title: String,
    pub session_number: u32,
    pub estimated_duration_minutes: u32,
    pub scenes: Vec<PlannedScene>,
    pub npc_appearances: Vec<NpcAppearance>,
    pub potential_loot: Vec<LootEntry>,
    pub notes: Option<String>,
}

/// A planned scene in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedScene {
    pub name: String,
    pub scene_type: SceneType,
    pub location: Option<String>,
    pub description: String,
    pub estimated_minutes: u32,
    pub tension_level: TensionLevel,
    pub key_elements: Vec<String>,
}

/// Scene type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneType {
    Roleplay,
    Combat,
    Exploration,
    Puzzle,
    Social,
    Downtime,
    Transition,
}

impl SceneType {
    pub fn label(&self) -> &'static str {
        match self {
            SceneType::Roleplay => "Roleplay",
            SceneType::Combat => "Combat",
            SceneType::Exploration => "Exploration",
            SceneType::Puzzle => "Puzzle",
            SceneType::Social => "Social",
            SceneType::Downtime => "Downtime",
            SceneType::Transition => "Transition",
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            SceneType::Roleplay => "bg-blue-900 text-blue-300",
            SceneType::Combat => "bg-red-900 text-red-300",
            SceneType::Exploration => "bg-green-900 text-green-300",
            SceneType::Puzzle => "bg-purple-900 text-purple-300",
            SceneType::Social => "bg-amber-900 text-amber-300",
            SceneType::Downtime => "bg-zinc-700 text-zinc-300",
            SceneType::Transition => "bg-zinc-800 text-zinc-400",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SceneType::Roleplay => "RP",
            SceneType::Combat => "CB",
            SceneType::Exploration => "EX",
            SceneType::Puzzle => "PZ",
            SceneType::Social => "SC",
            SceneType::Downtime => "DT",
            SceneType::Transition => "TR",
        }
    }
}

/// Tension level for pacing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TensionLevel {
    Low,
    Medium,
    High,
    Climax,
}

impl TensionLevel {
    pub fn value(&self) -> u8 {
        match self {
            TensionLevel::Low => 1,
            TensionLevel::Medium => 2,
            TensionLevel::High => 3,
            TensionLevel::Climax => 4,
        }
    }
}

/// NPC appearance in session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcAppearance {
    pub npc_name: String,
    pub scene_indices: Vec<usize>,
    pub purpose: String,
}

/// Loot entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LootEntry {
    pub name: String,
    pub quantity: u32,
    pub value: Option<String>,
    pub rarity: Option<String>,
}

// ============================================================================
// Components
// ============================================================================

/// Scene card in the timeline
#[component]
fn SceneCard(
    scene: PlannedScene,
    index: usize,
    is_editing: Signal<bool>,
    on_update: Callback<(usize, PlannedScene)>,
) -> impl IntoView {
    let scene_for_display = scene.clone();
    let scene_for_fallback_name = scene.clone();
    let scene_for_fallback_desc = scene.clone();
    let scene_template = scene.clone();
    let scene_name = RwSignal::new(scene.name.clone());
    let scene_description = RwSignal::new(scene.description.clone());

    view! {
        <div class="relative pl-8">
            // Timeline connector
            <div class="absolute left-3 top-0 bottom-0 w-0.5 bg-zinc-700" />

            // Timeline dot with tension indicator
            <div
                class="absolute left-1 top-3 w-5 h-5 rounded-full border-2 border-zinc-600 flex items-center justify-center text-[10px]"
                style=move || format!(
                    "background: linear-gradient(to top, {} {}%, transparent {}%)",
                    match scene_for_display.tension_level {
                        TensionLevel::Low => "#3f3f46",
                        TensionLevel::Medium => "#1e40af",
                        TensionLevel::High => "#9333ea",
                        TensionLevel::Climax => "#dc2626",
                    },
                    scene_for_display.tension_level.value() as u32 * 25,
                    scene_for_display.tension_level.value() as u32 * 25
                )
            >
                {index + 1}
            </div>

            // Scene content
            <div class="p-3 bg-zinc-800/50 rounded-lg mb-3">
                <div class="flex items-start justify-between mb-2">
                    <div class="flex items-center gap-2">
                        <span class=format!(
                            "px-2 py-0.5 text-xs rounded font-mono {}",
                            scene_for_display.scene_type.color_class()
                        )>
                            {scene_for_display.scene_type.icon()}
                        </span>
                        <Show
                            when=move || is_editing.get()
                            fallback=move || view! {
                                <h5 class="font-medium text-white">{scene_for_fallback_name.name.clone()}</h5>
                            }
                        >
                            <input
                                type="text"
                                class="flex-1 px-2 py-1 bg-zinc-900 border border-zinc-700 rounded text-white text-sm
                                       focus:border-purple-500 focus:outline-none"
                                prop:value=move || scene_name.get()
                                on:input=move |ev| scene_name.set(event_target_value(&ev))
                            />
                        </Show>
                    </div>
                    <span class="text-xs text-zinc-500">
                        {scene_for_display.estimated_minutes}" min"
                    </span>
                </div>

                {scene_for_display.location.clone().map(|loc| view! {
                    <p class="text-xs text-zinc-500 mb-2">
                        <span class="text-zinc-400">"Location: "</span>{loc}
                    </p>
                })}

                <Show
                    when=move || is_editing.get()
                    fallback=move || view! {
                        <p class="text-sm text-zinc-300 mb-2">{scene_for_fallback_desc.description.clone()}</p>
                    }
                >
                    {
                        let scene_template = scene_template.clone();
                        view! {
                            <textarea
                                class="w-full px-2 py-1 bg-zinc-900 border border-zinc-700 rounded text-zinc-300 text-sm mb-2 resize-none
                                       focus:border-purple-500 focus:outline-none"
                                rows=2
                                prop:value=move || scene_description.get()
                                on:input=move |ev| scene_description.set(event_target_value(&ev))
                            />
                            <button
                                type="button"
                                class="px-2 py-1 bg-purple-600 hover:bg-purple-500 text-white text-xs rounded transition-colors"
                                on:click=move |_| {
                                    let updated_scene = PlannedScene {
                                        name: scene_name.get(),
                                        description: scene_description.get(),
                                        ..scene_template.clone()
                                    };
                                    on_update.run((index, updated_scene));
                                }
                            >
                                "Save"
                            </button>
                        }
                    }
                </Show>

                {(!scene_for_display.key_elements.is_empty()).then(|| view! {
                    <div class="flex flex-wrap gap-1">
                        {scene_for_display.key_elements.iter().map(|el| view! {
                            <span class="px-2 py-0.5 bg-zinc-900 text-zinc-400 text-xs rounded">
                                {el.clone()}
                            </span>
                        }).collect_view()}
                    </div>
                })}
            </div>
        </div>
    }
}

/// Tension curve visualization
#[component]
fn TensionCurve(scenes: Signal<Vec<PlannedScene>>) -> impl IntoView {
    view! {
        <div class="h-16 flex items-end gap-1">
            {move || {
                let scene_list = scenes.get();
                if scene_list.is_empty() {
                    return view! { <div /> }.into_any();
                }

                scene_list.iter().map(|scene| {
                    let height_pct = scene.tension_level.value() as u32 * 25;
                    let color = match scene.tension_level {
                        TensionLevel::Low => "bg-zinc-600",
                        TensionLevel::Medium => "bg-blue-600",
                        TensionLevel::High => "bg-purple-600",
                        TensionLevel::Climax => "bg-red-600",
                    };

                    view! {
                        <div
                            class=format!("flex-1 {} rounded-t transition-all duration-300", color)
                            style=format!("height: {}%", height_pct)
                            title=format!("{}: {} tension", scene.name, match scene.tension_level {
                                TensionLevel::Low => "Low",
                                TensionLevel::Medium => "Medium",
                                TensionLevel::High => "High",
                                TensionLevel::Climax => "Climax",
                            })
                        />
                    }
                }).collect_view().into_any()
            }}
        </div>
    }
}

/// NPC appearances sidebar
#[component]
fn NpcAppearances(appearances: Signal<Vec<NpcAppearance>>) -> impl IntoView {
    view! {
        <div class="p-3 bg-zinc-800/50 rounded-lg">
            <h5 class="text-sm font-medium text-zinc-300 mb-2">"NPC Appearances"</h5>
            <div class="space-y-2">
                {move || {
                    let app_list = appearances.get();
                    if app_list.is_empty() {
                        view! { <p class="text-xs text-zinc-500">"No NPCs scheduled"</p> }.into_any()
                    } else {
                        app_list.iter().map(|npc| view! {
                            <div class="text-sm">
                                <span class="text-white">{npc.npc_name.clone()}</span>
                                <span class="text-zinc-500">" - "{npc.purpose.clone()}</span>
                                <span class="text-zinc-600 text-xs ml-1">
                                    "(scenes: "{npc.scene_indices.iter().map(|i| (i + 1).to_string()).collect::<Vec<_>>().join(", ")}")"
                                </span>
                            </div>
                        }).collect_view().into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Loot summary
#[component]
fn LootSummary(loot: Signal<Vec<LootEntry>>) -> impl IntoView {
    view! {
        <div class="p-3 bg-zinc-800/50 rounded-lg">
            <h5 class="text-sm font-medium text-zinc-300 mb-2">"Potential Loot"</h5>
            <div class="space-y-1">
                {move || {
                    let loot_list = loot.get();
                    if loot_list.is_empty() {
                        view! { <p class="text-xs text-zinc-500">"No loot planned"</p> }.into_any()
                    } else {
                        loot_list.iter().map(|item| view! {
                            <div class="flex items-center justify-between text-sm">
                                <span class="text-zinc-300">
                                    {if item.quantity > 1 { format!("{}x ", item.quantity) } else { String::new() }}
                                    {item.name.clone()}
                                </span>
                                <span class="text-zinc-500 text-xs">
                                    {item.value.clone().unwrap_or_default()}
                                </span>
                            </div>
                        }).collect_view().into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Main session plan preview component
#[component]
pub fn SessionPlanPreview(
    /// The session plan data
    plan: RwSignal<SessionPlan>,
    /// Whether in edit mode
    #[prop(default = RwSignal::new(false))]
    is_editing: RwSignal<bool>,
) -> impl IntoView {
    let scenes = Signal::derive(move || plan.get().scenes);
    let appearances = Signal::derive(move || plan.get().npc_appearances);
    let loot = Signal::derive(move || plan.get().potential_loot);
    let is_editing_signal = Signal::derive(move || is_editing.get());

    let total_minutes = Signal::derive(move || {
        plan.get()
            .scenes
            .iter()
            .map(|s| s.estimated_minutes)
            .sum::<u32>()
    });

    let on_scene_update = Callback::new(move |(idx, scene): (usize, PlannedScene)| {
        plan.update(|p| {
            if let Some(s) = p.scenes.get_mut(idx) {
                *s = scene;
            }
        });
    });

    view! {
        <div class="space-y-4">
            // Header
            <div class="flex items-center justify-between">
                <div>
                    <h4 class="font-bold text-white">{move || plan.get().title}</h4>
                    <p class="text-sm text-zinc-400">
                        "Session "{move || plan.get().session_number}
                    </p>
                </div>
                <div class="text-right">
                    <p class="text-lg font-mono text-white">
                        {move || {
                            let mins = total_minutes.get();
                            format!("{}:{:02}", mins / 60, mins % 60)
                        }}
                    </p>
                    <p class="text-xs text-zinc-500">"Estimated duration"</p>
                </div>
            </div>

            // Tension curve
            <div>
                <h5 class="text-xs text-zinc-500 mb-1">"Pacing / Tension Curve"</h5>
                <TensionCurve scenes=scenes />
            </div>

            // Two-column layout
            <div class="grid grid-cols-3 gap-4">
                // Scene timeline (2 cols)
                <div class="col-span-2">
                    <h5 class="text-sm font-medium text-zinc-300 mb-3">"Scene Timeline"</h5>
                    <div class="relative">
                        {move || plan.get().scenes.iter().enumerate().map(|(i, scene)| {
                            view! {
                                <SceneCard
                                    scene=scene.clone()
                                    index=i
                                    is_editing=is_editing_signal
                                    on_update=on_scene_update
                                />
                            }
                        }).collect_view()}
                    </div>
                </div>

                // Sidebar (1 col)
                <div class="space-y-4">
                    <NpcAppearances appearances=appearances />
                    <LootSummary loot=loot />

                    // Session notes
                    {move || plan.get().notes.clone().map(|notes| view! {
                        <div class="p-3 bg-zinc-800/50 rounded-lg">
                            <h5 class="text-sm font-medium text-zinc-300 mb-2">"GM Notes"</h5>
                            <p class="text-sm text-zinc-400">{notes}</p>
                        </div>
                    })}
                </div>
            </div>
        </div>
    }
}
