//! Active session workspace component
//!
//! Displays the main workspace for an active game session including combat tracking

use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    end_session, start_combat, end_combat, get_combat,
    add_combatant, remove_combatant, next_turn,
    damage_combatant, heal_combatant, add_condition,
    GameSession, CombatState, Combatant,
};
use crate::components::design_system::{Button, ButtonVariant, Input, Card, CardHeader, CardBody, Badge, BadgeVariant};
use crate::components::session::SessionChatPanel;

/// Active session workspace component
#[component]
pub fn ActiveSessionWorkspace(
    /// The current active session
    session: GameSession,
    /// Callback when session is ended
    on_session_ended: Callback<()>,
) -> impl IntoView {
    let session_id = StoredValue::new(session.id.clone());
    let campaign_id = StoredValue::new(session.campaign_id.clone());
    let session_number = session.session_number;

    // Chat panel state
    let show_chat_panel = RwSignal::new(true);

    // Combat state
    let combat = RwSignal::new(Option::<CombatState>::None);

    // Combatant form state
    let new_combatant_name = RwSignal::new(String::new());
    let new_combatant_init = RwSignal::new("10".to_string());
    let new_combatant_type = RwSignal::new("monster".to_string());

    // Condition modal state
    let condition_modal_open = RwSignal::new(false);
    let condition_combatant_id = RwSignal::new(Option::<String>::None);
    let new_condition = RwSignal::new(String::new());

    // Load combat state on mount
    Effect::new(move |_| {
        let sid = session_id.get_value();
        spawn_local(async move {
            if let Ok(Some(c)) = get_combat(sid).await {
                combat.set(Some(c));
            }
        });
    });

    // Close condition modal handler
    let close_condition_modal = move || {
        condition_modal_open.set(false);
        condition_combatant_id.set(None);
        new_condition.set(String::new());
    };

    // Derive campaign_id signal for chat panel
    let campaign_id_signal = Signal::derive(move || Some(campaign_id.get_value()));

    view! {
        <div class="flex gap-4 h-full">
            // Main content area
            <div class=move || format!(
                "space-y-6 transition-all duration-300 {}",
                if show_chat_panel.get() { "flex-1" } else { "w-full max-w-5xl mx-auto" }
            )>
                // Session Control Bar
                <Card>
                <div class="flex justify-between items-center p-4">
                    <div>
                        <div class="text-xs text-zinc-400 uppercase tracking-widest">"Current Session"</div>
                        <div class="text-2xl font-bold text-white">{format!("Session #{}", session_number)}</div>
                    </div>
                    <Button
                        variant=ButtonVariant::Destructive
                        class="px-4 py-2 bg-red-600/20 text-red-400 border border-red-600/50"
                        on_click=move |_: ev::MouseEvent| {
                            let sid = session_id.get_value();
                            spawn_local(async move {
                                if end_session(sid).await.is_ok() {
                                    on_session_ended.run(());
                                }
                            });
                        }
                    >
                        "End Session"
                    </Button>
                </div>
            </Card>

            // Combat Section
            <Card>
                <CardHeader>
                    <h3 class="font-bold text-zinc-200">"Encounter Tracker"</h3>
                    <Show
                        when=move || combat.get().is_none()
                        fallback=move || view! {
                            <div class="flex gap-2">
                                <Button
                                    variant=ButtonVariant::Secondary
                                    class="px-3 py-1 bg-blue-600/20 text-blue-400 border border-blue-600/50 text-sm"
                                    on_click=move |_: ev::MouseEvent| {
                                        let sid = session_id.get_value();
                                        spawn_local(async move {
                                            if next_turn(sid.clone()).await.is_ok() {
                                                if let Ok(Some(c)) = get_combat(sid).await {
                                                    combat.set(Some(c));
                                                }
                                            }
                                        });
                                    }
                                >
                                    "Next Turn"
                                </Button>
                                <Button
                                    variant=ButtonVariant::Ghost
                                    class="px-3 py-1 bg-zinc-700 text-zinc-300 text-sm"
                                    on_click=move |_: ev::MouseEvent| {
                                        let sid = session_id.get_value();
                                        spawn_local(async move {
                                            if end_combat(sid).await.is_ok() {
                                                combat.set(None);
                                            }
                                        });
                                    }
                                >
                                    "End Encounter"
                                </Button>
                            </div>
                        }
                    >
                        <Button
                            variant=ButtonVariant::Primary
                            class="px-3 py-1 bg-purple-600 text-white text-sm"
                            on_click=move |_: ev::MouseEvent| {
                                let sid = session_id.get_value();
                                spawn_local(async move {
                                    if let Ok(c) = start_combat(sid).await {
                                        combat.set(Some(c));
                                    }
                                });
                            }
                        >
                            "Start Combat"
                        </Button>
                    </Show>
                </CardHeader>

                <Show
                    when=move || combat.get().is_some()
                    fallback=|| view! {
                        <CardBody>
                            <div class="text-center text-zinc-500 py-8">
                                "Peaceful times. Start combat to track initiative."
                            </div>
                        </CardBody>
                    }
                >
                    <div class="p-0">
                        // Turn Order List
                        <div class="divide-y divide-zinc-700">
                            <For
                                each=move || {
                                    combat.get().map(|c| c.combatants).unwrap_or_default()
                                        .into_iter()
                                        .enumerate()
                                        .collect::<Vec<_>>()
                                }
                                key=|(_, combatant)| combatant.id.clone()
                                children=move |(idx, combatant)| {
                                    let current_turn = combat.get().map(|c| c.current_turn).unwrap_or(0);
                                    let combatant_id = combatant.id.clone();
                                    view! {
                                        <CombatantRow
                                            combatant=combatant
                                            is_current_turn=idx == current_turn
                                            session_id=session_id
                                            combat=combat
                                            on_open_condition_modal=Callback::new(move |_| {
                                                condition_combatant_id.set(Some(combatant_id.clone()));
                                                condition_modal_open.set(true);
                                            })
                                        />
                                    }
                                }
                            />
                        </div>

                        // Add Combatant Form
                        <div class="p-4 bg-zinc-900/50 flex gap-2 border-t border-zinc-700">
                            <Input
                                value=new_combatant_name
                                placeholder="Name"
                                class="bg-zinc-800 border-zinc-700 rounded px-3 py-2 text-sm text-white flex-1"
                            />
                            <Input
                                value=new_combatant_init
                                placeholder="Init"
                                r#type="number".to_string()
                                class="bg-zinc-800 border-zinc-700 rounded px-3 py-2 text-sm text-white w-20 text-center"
                            />
                            <select
                                class="bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-white"
                                prop:value=move || new_combatant_type.get()
                                on:change=move |ev| {
                                    new_combatant_type.set(event_target_value(&ev));
                                }
                            >
                                <option value="player">"Player"</option>
                                <option value="monster" selected>"Monster"</option>
                                <option value="npc">"NPC"</option>
                                <option value="ally">"Ally"</option>
                            </select>
                            <Button
                                variant=ButtonVariant::Secondary
                                class="px-4 py-2 bg-zinc-700 text-white text-sm font-medium"
                                on_click=move |_: ev::MouseEvent| {
                                    let sid = session_id.get_value();
                                    let name = new_combatant_name.get();
                                    let init: i32 = new_combatant_init.get().parse().unwrap_or(10);
                                    let ctype = new_combatant_type.get();

                                    if name.is_empty() {
                                        return;
                                    }

                                    spawn_local(async move {
                                        if add_combatant(sid.clone(), name, init, ctype).await.is_ok() {
                                            if let Ok(Some(c)) = get_combat(sid).await {
                                                combat.set(Some(c));
                                            }
                                        }
                                    });

                                    new_combatant_name.set(String::new());
                                    new_combatant_init.set("10".to_string());
                                }
                            >
                                "Add"
                            </Button>
                        </div>
                    </div>
                </Show>
            </Card>

            // Session History placeholder
            <Card>
                <CardHeader>
                    <h3 class="font-bold text-zinc-200">"Session Events"</h3>
                </CardHeader>
                <CardBody>
                    <div class="text-center text-zinc-500 py-4">
                        "Session event log coming soon..."
                    </div>
                </CardBody>
            </Card>

            // Condition Modal
            <Show when=move || condition_modal_open.get()>
                <ConditionModal
                    combatant_id=condition_combatant_id
                    session_id=session_id
                    combat=combat
                    new_condition=new_condition
                    on_close=Callback::new(move |_| close_condition_modal())
                />
            </Show>
            </div>

            // Chat Panel (collapsible sidebar)
            <div class=move || format!(
                "transition-all duration-300 flex flex-col {}",
                if show_chat_panel.get() { "w-96" } else { "w-0 overflow-hidden" }
            )>
                // Chat panel toggle button
                <div class="flex items-center justify-between mb-2">
                    <h3 class="font-bold text-zinc-200 text-sm">"AI Assistant"</h3>
                    <button
                        class="text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
                        on:click=move |_| show_chat_panel.update(|v| *v = !*v)
                    >
                        {move || if show_chat_panel.get() { "Hide" } else { "Show" }}
                    </button>
                </div>

                // Session Chat Panel
                <Show when=move || show_chat_panel.get()>
                    <div class="flex-1 min-h-[400px]">
                        <SessionChatPanel campaign_id=campaign_id_signal />
                    </div>
                </Show>
            </div>
        </div>
    }
}

/// Individual combatant row in the initiative order
#[component]
fn CombatantRow(
    combatant: Combatant,
    is_current_turn: bool,
    session_id: StoredValue<String>,
    combat: RwSignal<Option<CombatState>>,
    on_open_condition_modal: Callback<()>,
) -> impl IntoView {
    let combatant_id = StoredValue::new(combatant.id.clone());
    let combatant_name = combatant.name.clone();
    let combatant_type = combatant.combatant_type.clone();
    let initiative = combatant.initiative;
    let hp_current = combatant.hp_current;
    let hp_max = combatant.hp_max;
    let conditions = StoredValue::new(combatant.conditions.clone());
    let has_conditions = !combatant.conditions.is_empty();

    let base_class = if is_current_turn {
        "bg-purple-900/20 flex items-center p-3 border-l-4 border-purple-500"
    } else {
        "flex items-center p-3 hover:bg-zinc-700/50"
    };

    view! {
        <div class=base_class>
            // Initiative
            <div class="w-12 text-center font-mono text-xl text-zinc-500">
                {initiative.to_string()}
            </div>

            // Info
            <div class="flex-1 px-4">
                <div class="font-bold text-zinc-200">{combatant_name.clone()}</div>
                <div class="text-xs text-zinc-500 uppercase">{combatant_type}</div>
                // Conditions
                {if has_conditions {
                    Some(view! {
                        <div class="flex flex-wrap gap-1 mt-1">
                            {conditions.get_value().into_iter().map(|condition| {
                                view! {
                                    <Badge variant=BadgeVariant::Warning>
                                        {condition}
                                    </Badge>
                                }
                            }).collect_view()}
                        </div>
                    })
                } else {
                    None
                }}
            </div>

            // HP & Actions
            <div class="flex items-center gap-3">
                <div class="text-zinc-400 font-mono">
                    {format!("{} / {}", hp_current, hp_max)}
                </div>

                // Quick Actions
                <button
                    class="w-8 h-8 rounded bg-red-900/50 text-red-400 hover:bg-red-600 hover:text-white transition-colors"
                    aria-label=format!("Deal 1 damage to {}", combatant_name)
                    on:click=move |_| {
                        let sid = session_id.get_value();
                        let cid = combatant_id.get_value();
                        spawn_local(async move {
                            if damage_combatant(sid.clone(), cid, 1).await.is_ok() {
                                if let Ok(Some(c)) = get_combat(sid).await {
                                    combat.set(Some(c));
                                }
                            }
                        });
                    }
                >
                    "-"
                </button>
                <button
                    class="w-8 h-8 rounded bg-green-900/50 text-green-400 hover:bg-green-600 hover:text-white transition-colors"
                    aria-label=format!("Heal 1 HP for {}", combatant_name)
                    on:click=move |_| {
                        let sid = session_id.get_value();
                        let cid = combatant_id.get_value();
                        spawn_local(async move {
                            if heal_combatant(sid.clone(), cid, 1).await.is_ok() {
                                if let Ok(Some(c)) = get_combat(sid).await {
                                    combat.set(Some(c));
                                }
                            }
                        });
                    }
                >
                    "+"
                </button>
                <button
                    class="w-8 h-8 rounded bg-yellow-900/50 text-yellow-400 hover:bg-yellow-600 hover:text-white transition-colors"
                    aria-label=format!("Add condition to {}", combatant_name)
                    on:click=move |_| on_open_condition_modal.run(())
                >
                    "!"
                </button>
                <button
                    class="w-8 h-8 rounded bg-zinc-700/50 text-zinc-400 hover:bg-zinc-600 hover:text-white transition-colors ml-2"
                    aria-label=format!("Remove {} from combat", combatant_name)
                    on:click=move |_| {
                        let sid = session_id.get_value();
                        let cid = combatant_id.get_value();
                        spawn_local(async move {
                            if remove_combatant(sid.clone(), cid).await.is_ok() {
                                if let Ok(Some(c)) = get_combat(sid).await {
                                    combat.set(Some(c));
                                }
                            }
                        });
                    }
                >
                    "x"
                </button>
            </div>
        </div>
    }
}

/// Condition management modal
#[component]
fn ConditionModal(
    combatant_id: RwSignal<Option<String>>,
    session_id: StoredValue<String>,
    combat: RwSignal<Option<CombatState>>,
    new_condition: RwSignal<String>,
    on_close: Callback<()>,
) -> impl IntoView {
    // Common conditions for quick selection
    let common_conditions: &'static [&'static str] = &[
        "Blinded", "Charmed", "Deafened", "Frightened", "Grappled",
        "Incapacitated", "Invisible", "Paralyzed", "Petrified", "Poisoned",
        "Prone", "Restrained", "Stunned", "Unconscious", "Exhaustion",
    ];

    view! {
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
            <Card class="w-96">
                <CardHeader>
                    <h3 class="font-bold text-zinc-200">"Add Condition"</h3>
                    <button
                        class="text-zinc-400 hover:text-white"
                        on:click=move |_| on_close.run(())
                    >
                        "x"
                    </button>
                </CardHeader>
                <CardBody>
                    // Quick select conditions
                    <div class="mb-4">
                        <div class="text-xs text-zinc-500 mb-2">"Common Conditions"</div>
                        <div class="flex flex-wrap gap-1">
                            {common_conditions.iter().map(|&condition| {
                                let condition_str = condition.to_string();
                                view! {
                                    <button
                                        class="px-2 py-1 text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded transition-colors"
                                        on:click=move |_| new_condition.set(condition_str.clone())
                                    >
                                        {condition}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    </div>

                    // Custom condition input
                    <div class="flex gap-2">
                        <Input
                            value=new_condition
                            placeholder="Custom condition..."
                            class="flex-1"
                        />
                        <Button
                            variant=ButtonVariant::Primary
                            on_click=move |_: ev::MouseEvent| {
                                let condition = new_condition.get();
                                if condition.is_empty() {
                                    return;
                                }

                                let sid = session_id.get_value();
                                let cid = combatant_id.get().unwrap_or_default();

                                spawn_local(async move {
                                    if add_condition(sid.clone(), cid, condition).await.is_ok() {
                                        if let Ok(Some(c)) = get_combat(sid).await {
                                            combat.set(Some(c));
                                        }
                                    }
                                });
                                on_close.run(());
                            }
                        >
                            "Add"
                        </Button>
                    </div>
                </CardBody>
            </Card>
        </div>
    }
}
