//! Arc Structure Step - Story arc template selection
//!
//! Optional step for selecting narrative structure and story arc templates.

use leptos::prelude::*;

use crate::services::wizard_state::{
    use_wizard_context, ArcPhaseConfig, ArcStructureData, ArcTemplate, NarrativeStyle, StepData,
};

/// Arc template details for display
fn template_phases(template: ArcTemplate) -> Vec<(&'static str, &'static str)> {
    match template {
        ArcTemplate::HerosJourney => vec![
            ("Ordinary World", "Establish the status quo"),
            ("Call to Adventure", "Inciting incident"),
            ("Crossing the Threshold", "Commitment to the quest"),
            ("Tests & Allies", "Rising challenges"),
            ("The Ordeal", "Major crisis"),
            ("Return", "Resolution and transformation"),
        ],
        ArcTemplate::ThreeAct => vec![
            ("Setup", "Introduce characters and conflict"),
            ("Confrontation", "Rising stakes and obstacles"),
            ("Resolution", "Climax and conclusion"),
        ],
        ArcTemplate::FiveAct => vec![
            ("Exposition", "World and character introduction"),
            ("Rising Action", "Complications build"),
            ("Climax", "Point of no return"),
            ("Falling Action", "Consequences unfold"),
            ("Resolution", "New equilibrium"),
        ],
        ArcTemplate::Mystery => vec![
            ("Discovery", "Find the mystery"),
            ("Investigation", "Gather clues"),
            ("Complication", "False leads and dangers"),
            ("Revelation", "Truth uncovered"),
            ("Resolution", "Justice or tragedy"),
        ],
        ArcTemplate::PoliticalIntrigue => vec![
            ("Introduction", "Faction landscape"),
            ("Entanglement", "Party gets involved"),
            ("Escalation", "Stakes rise"),
            ("Betrayal", "Allegiances shift"),
            ("Resolution", "Power settles"),
        ],
        ArcTemplate::DungeonDelve => vec![
            ("Preparation", "Gather resources and intel"),
            ("Descent", "Enter the dungeon"),
            ("Exploration", "Navigate challenges"),
            ("Boss", "Final confrontation"),
            ("Escape/Reward", "Consequences"),
        ],
        ArcTemplate::Sandbox | ArcTemplate::Custom => vec![],
    }
}

/// Phase editor component
#[component]
fn PhaseEditor(
    phases: RwSignal<Vec<ArcPhaseConfig>>,
) -> impl IntoView {
    let add_phase = move |_| {
        phases.update(|p| {
            p.push(ArcPhaseConfig {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("Phase {}", p.len() + 1),
                description: None,
                estimated_sessions: Some(2),
            });
        });
    };

    let remove_phase = move |phase_id: String| {
        phases.update(|p| {
            if p.len() > 1 {
                p.retain(|phase| phase.id != phase_id);
            }
        });
    };

    let update_phase = move |phase_id: String, name: String, desc: String, sessions: String| {
        phases.update(|p| {
            if let Some(phase) = p.iter_mut().find(|ph| ph.id == phase_id) {
                phase.name = name;
                phase.description = if desc.is_empty() { None } else { Some(desc) };
                phase.estimated_sessions = sessions.parse().ok();
            }
        });
    };

    view! {
        <div class="space-y-3">
            {move || {
                phases.get().iter().enumerate().map(|(i, phase)| {
                    // Capture the ID for stable updates and removal
                    let phase_id = phase.id.clone();
                    let phase_id_for_name = phase_id.clone();
                    let phase_id_for_desc = phase_id.clone();
                    let phase_id_for_sessions = phase_id.clone();
                    let phase_id_for_remove = phase_id.clone();
                    let phase_name = phase.name.clone();
                    let phase_desc = phase.description.clone().unwrap_or_default();
                    let phase_sessions = phase.estimated_sessions.map(|s| s.to_string()).unwrap_or_default();

                    view! {
                        <div class="flex items-start gap-3 p-3 bg-zinc-800/50 rounded-lg">
                            <div class="w-8 h-8 rounded-full bg-purple-900/50 flex items-center justify-center text-sm text-purple-300 shrink-0">
                                {i + 1}
                            </div>

                            <div class="flex-1 space-y-2">
                                <input
                                    type="text"
                                    class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm
                                           placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                                    placeholder="Phase name"
                                    prop:value=phase_name.clone()
                                    on:input={
                                        let id = phase_id_for_name.clone();
                                        let pd = phase_desc.clone();
                                        let ps = phase_sessions.clone();
                                        move |ev| update_phase(id.clone(), event_target_value(&ev), pd.clone(), ps.clone())
                                    }
                                />
                                <input
                                    type="text"
                                    class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm
                                           placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                                    placeholder="Description (optional)"
                                    prop:value=phase_desc.clone()
                                    on:input={
                                        let id = phase_id_for_desc.clone();
                                        let pn = phase_name.clone();
                                        let ps = phase_sessions.clone();
                                        move |ev| update_phase(id.clone(), pn.clone(), event_target_value(&ev), ps.clone())
                                    }
                                />
                            </div>

                            <div class="w-24">
                                <input
                                    type="number"
                                    min="1"
                                    max="50"
                                    class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm
                                           placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                                    placeholder="Sessions"
                                    prop:value=phase_sessions.clone()
                                    on:input={
                                        let id = phase_id_for_sessions.clone();
                                        let pn = phase_name.clone();
                                        let pd = phase_desc.clone();
                                        move |ev| update_phase(id.clone(), pn.clone(), pd.clone(), event_target_value(&ev))
                                    }
                                />
                                <div class="text-[10px] text-zinc-500 text-center mt-1">"Sessions"</div>
                            </div>

                            <button
                                type="button"
                                class="p-2 text-zinc-500 hover:text-red-400 transition-colors"
                                on:click={
                                    let id = phase_id_for_remove.clone();
                                    move |_| remove_phase(id.clone())
                                }
                            >
                                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                </svg>
                            </button>
                        </div>
                    }
                }).collect_view()
            }}

            <button
                type="button"
                class="w-full py-2 border border-dashed border-zinc-700 rounded-lg text-zinc-400 text-sm hover:border-zinc-600 hover:text-zinc-300 transition-colors"
                on:click=add_phase
            >
                "+ Add Phase"
            </button>
        </div>
    }
}

/// Arc structure step component
#[component]
pub fn ArcStructureStep(
    form_data: RwSignal<Option<StepData>>,
    form_valid: RwSignal<bool>,
) -> impl IntoView {
    let ctx = use_wizard_context();
    let draft = ctx.draft();
    let arc = draft.arc_structure.unwrap_or_default();

    // Local form state
    let template = RwSignal::new(arc.template.unwrap_or_default());
    let narrative_style = RwSignal::new(arc.narrative_style.unwrap_or_default());
    let phases = RwSignal::new(if arc.phases.is_empty() {
        template_phases(template.get())
            .iter()
            .map(|(name, desc)| ArcPhaseConfig {
                id: uuid::Uuid::new_v4().to_string(),
                name: name.to_string(),
                description: Some(desc.to_string()),
                estimated_sessions: Some(3),
            })
            .collect()
    } else {
        arc.phases
    });

    // This step is always valid (optional)
    Effect::new(move |_| {
        form_valid.set(true);
    });

    // Update form_data when inputs change
    Effect::new(move |_| {
        form_data.set(Some(StepData::ArcStructure(ArcStructureData {
            template: Some(template.get()),
            phases: phases.get(),
            narrative_style: Some(narrative_style.get()),
        })));
    });

    // Update phases when template changes
    // NOTE: Intentionally replaces phases when switching templates.
    // Users wanting custom phases should use the Custom template.
    let select_template = move |t: ArcTemplate| {
        template.set(t);
        if t != ArcTemplate::Custom && t != ArcTemplate::Sandbox {
            let new_phases = template_phases(t)
                .iter()
                .map(|(name, desc)| ArcPhaseConfig {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: name.to_string(),
                    description: Some(desc.to_string()),
                    estimated_sessions: Some(3),
                })
                .collect();
            phases.set(new_phases);
        }
    };

    view! {
        <div class="space-y-6 max-w-3xl mx-auto">
            // Header
            <div class="text-center">
                <h3 class="text-2xl font-bold text-white mb-2">"Story Arc Structure"</h3>
                <p class="text-zinc-400">
                    "Choose a narrative template for your campaign's overall story"
                </p>
                <p class="text-xs text-purple-400 mt-2">
                    "This step is optional - templates provide guidance, not constraints"
                </p>
            </div>

            // Arc Template Selection
            <div class="space-y-3">
                <label class="block text-sm font-medium text-zinc-300">
                    "Arc Template"
                </label>
                <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
                    {ArcTemplate::all().into_iter().map(|t| {
                        let is_selected = move || template.get() == t;

                        view! {
                            <button
                                type="button"
                                class=move || format!(
                                    "p-3 rounded-lg border text-center transition-all duration-200 {}",
                                    if is_selected() {
                                        "bg-purple-900/30 border-purple-500 ring-1 ring-purple-500/50"
                                    } else {
                                        "bg-zinc-800/50 border-zinc-700 hover:border-zinc-600 hover:bg-zinc-800"
                                    }
                                )
                                on:click=move |_| select_template(t)
                            >
                                <div class="font-medium text-white text-sm">{t.label()}</div>
                            </button>
                        }
                    }).collect_view()}
                </div>

                // Template description
                <div class="p-3 bg-zinc-800/50 rounded-lg">
                    <p class="text-sm text-zinc-400">
                        {move || template.get().description()}
                    </p>
                </div>
            </div>

            // Narrative Style
            <div class="space-y-3">
                <label class="block text-sm font-medium text-zinc-300">
                    "Narrative Style"
                </label>
                <div class="grid grid-cols-4 gap-3">
                    {NarrativeStyle::all().into_iter().map(|style| {
                        let is_selected = move || narrative_style.get() == style;
                        let desc = match style {
                            NarrativeStyle::Linear => "Single path",
                            NarrativeStyle::Branching => "Player choices matter",
                            NarrativeStyle::Sandbox => "Open world",
                            NarrativeStyle::Episodic => "Self-contained sessions",
                        };

                        view! {
                            <button
                                type="button"
                                class=move || format!(
                                    "p-3 rounded-lg border text-center transition-all duration-200 {}",
                                    if is_selected() {
                                        "bg-purple-900/30 border-purple-500"
                                    } else {
                                        "bg-zinc-800/50 border-zinc-700 hover:border-zinc-600"
                                    }
                                )
                                on:click=move |_| narrative_style.set(style)
                            >
                                <div class="font-medium text-white text-sm">{style.label()}</div>
                                <div class="text-xs text-zinc-500 mt-1">{desc}</div>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>

            // Phase Editor (for non-sandbox templates)
            <Show when=move || template.get() != ArcTemplate::Sandbox>
                <div class="space-y-3">
                    <div class="flex items-center justify-between">
                        <label class="block text-sm font-medium text-zinc-300">
                            "Story Phases"
                        </label>
                        <span class="text-xs text-zinc-500">
                            {move || {
                                let total: u32 = phases.get()
                                    .iter()
                                    .filter_map(|p| p.estimated_sessions)
                                    .sum();
                                format!("~{} sessions total", total)
                            }}
                        </span>
                    </div>

                    <PhaseEditor phases=phases />
                </div>
            </Show>

            // Sandbox note
            <Show when=move || template.get() == ArcTemplate::Sandbox>
                <div class="p-4 bg-zinc-800/50 border border-zinc-700 rounded-lg">
                    <h4 class="font-medium text-white mb-2">"Sandbox Campaign"</h4>
                    <p class="text-sm text-zinc-400">
                        "In a sandbox campaign, the story emerges from player choices and world events.
                        Instead of predefined phases, focus on creating interesting locations, factions,
                        and NPCs that the players can interact with organically."
                    </p>
                </div>
            </Show>

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
                                "Ask the AI for help adapting the template to your themes, or for story beat suggestions for each phase."
                            </p>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}
