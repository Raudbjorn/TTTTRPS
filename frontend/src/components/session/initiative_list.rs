//! Initiative List Component (TASK-016)
//!
//! Displays the initiative order with combatant management.

use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use super::combatant_card::CombatantCard;
use super::condition_manager::ConditionModal;
use crate::bindings::{add_combatant, add_combatant_full, CombatState};
use crate::components::design_system::{Button, ButtonVariant};

/// Initiative list component
#[component]
pub fn InitiativeList(
    /// Session ID
    session_id: Signal<String>,
    /// Current combat state
    combat: Signal<Option<CombatState>>,
    /// Callback when combat is updated
    on_combat_update: Callback<()>,
) -> impl IntoView {
    // New combatant form state
    let new_name = RwSignal::new(String::new());
    let new_initiative = RwSignal::new("10".to_string());
    let new_hp = RwSignal::new("20".to_string());
    let new_max_hp = RwSignal::new("20".to_string());
    let new_ac = RwSignal::new("10".to_string());
    let new_type = RwSignal::new("monster".to_string());
    let show_add_form = RwSignal::new(false);

    // Condition modal state
    let condition_modal_open = RwSignal::new(false);
    let condition_target_id = RwSignal::new(Option::<String>::None);

    // Add combatant handler
    let handle_add_combatant = move |_: ev::MouseEvent| {
        let sid = session_id.get();
        let name = new_name.get();
        let init: i32 = new_initiative.get().parse().unwrap_or(10);
        let ctype = new_type.get();

        if name.is_empty() {
            return;
        }

        spawn_local(async move {
            if add_combatant(sid.clone(), name, init, ctype).await.is_ok() {
                on_combat_update.run(());
            }
        });

        // Reset form
        new_name.set(String::new());
        new_initiative.set("10".to_string());
        new_type.set("monster".to_string());
        show_add_form.set(false);
    };

    // Quick add handler (minimal info)
    let handle_quick_add = move |ev: ev::KeyboardEvent| {
        if ev.key() != "Enter" {
            return;
        }

        let sid = session_id.get();
        let name = new_name.get();
        let init: i32 = new_initiative.get().parse().unwrap_or(10);
        let ctype = new_type.get();

        if name.is_empty() {
            return;
        }

        spawn_local(async move {
            if add_combatant(sid.clone(), name, init, ctype).await.is_ok() {
                on_combat_update.run(());
            }
        });

        new_name.set(String::new());
        new_initiative.set("10".to_string());
    };

    // Close condition modal
    let close_condition_modal = move || {
        condition_modal_open.set(false);
        condition_target_id.set(None);
    };

    view! {
        <div class="initiative-list">
            // Combatant list
            <div class="divide-y divide-zinc-700/50">
                <For
                    each=move || {
                        combat.get()
                            .map(|c| {
                                c.combatants.into_iter()
                                    .enumerate()
                                    .map(|(idx, cb)| (idx, cb, c.current_turn))
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default()
                    }
                    key=|(_, combatant, _)| combatant.id.clone()
                    children=move |(idx, combatant, current_turn)| {
                        let is_current = idx == current_turn;
                        let _cid = combatant.id.clone();

                        view! {
                            <CombatantCard
                                combatant=combatant
                                is_current_turn=is_current
                                session_id=session_id
                                on_update=Callback::new(move |_| on_combat_update.run(()))
                                on_add_condition=Callback::new(move |id: String| {
                                    condition_target_id.set(Some(id));
                                    condition_modal_open.set(true);
                                })
                            />
                        }
                    }
                />
            </div>

            // Empty state
            <Show when=move || combat.get().map(|c| c.combatants.is_empty()).unwrap_or(true)>
                <div class="py-8 text-center text-zinc-500">
                    <p class="mb-2">"No combatants in this encounter"</p>
                    <p class="text-sm">"Add combatants using the form below"</p>
                </div>
            </Show>

            // Add combatant section
            <div class="p-4 bg-zinc-900/50 border-t border-zinc-700">
                <Show
                    when=move || show_add_form.get()
                    fallback=move || view! {
                        // Quick add row
                        <div class="flex gap-2">
                            <input
                                type="text"
                                placeholder="Name"
                                class="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-white text-sm placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                                prop:value=move || new_name.get()
                                on:input=move |ev| new_name.set(event_target_value(&ev))
                                on:keydown=handle_quick_add
                            />
                            <input
                                type="number"
                                placeholder="Init"
                                class="w-20 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-white text-sm text-center focus:border-purple-500 focus:outline-none"
                                prop:value=move || new_initiative.get()
                                on:input=move |ev| new_initiative.set(event_target_value(&ev))
                                on:keydown=handle_quick_add
                            />
                            <select
                                class="w-28 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-white text-sm focus:border-purple-500 focus:outline-none"
                                prop:value=move || new_type.get()
                                on:change=move |ev| new_type.set(event_target_value(&ev))
                            >
                                <option value="player">"Player"</option>
                                <option value="monster" selected>"Monster"</option>
                                <option value="npc">"NPC"</option>
                                <option value="ally">"Ally"</option>
                            </select>
                            <Button
                                variant=ButtonVariant::Primary
                                class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white text-sm font-medium"
                                on_click=handle_add_combatant
                            >
                                "Add"
                            </Button>
                            <button
                                class="px-2 py-2 text-zinc-400 hover:text-white transition-colors"
                                on:click=move |_| show_add_form.set(true)
                                aria-label="Show expanded add form"
                            >
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                                </svg>
                            </button>
                        </div>
                    }
                >
                    // Expanded add form
                    <div class="space-y-4 p-4 bg-zinc-800/50 rounded-lg border border-zinc-700">
                        <div class="flex items-center justify-between">
                            <h4 class="font-medium text-zinc-200">"Add Combatant"</h4>
                            <button
                                class="text-zinc-400 hover:text-white transition-colors"
                                on:click=move |_| show_add_form.set(false)
                            >
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                </svg>
                            </button>
                        </div>

                        <div class="grid grid-cols-2 gap-4">
                            // Name
                            <div class="col-span-2">
                                <label class="block text-xs text-zinc-500 mb-1">"Name"</label>
                                <input
                                    type="text"
                                    class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-white text-sm focus:border-purple-500 focus:outline-none"
                                    prop:value=move || new_name.get()
                                    on:input=move |ev| new_name.set(event_target_value(&ev))
                                />
                            </div>

                            // Initiative
                            <div>
                                <label class="block text-xs text-zinc-500 mb-1">"Initiative"</label>
                                <input
                                    type="number"
                                    class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-white text-sm text-center focus:border-purple-500 focus:outline-none"
                                    prop:value=move || new_initiative.get()
                                    on:input=move |ev| new_initiative.set(event_target_value(&ev))
                                />
                            </div>

                            // Type
                            <div>
                                <label class="block text-xs text-zinc-500 mb-1">"Type"</label>
                                <select
                                    class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-white text-sm focus:border-purple-500 focus:outline-none"
                                    prop:value=move || new_type.get()
                                    on:change=move |ev| new_type.set(event_target_value(&ev))
                                >
                                    <option value="player">"Player Character"</option>
                                    <option value="monster">"Monster/Enemy"</option>
                                    <option value="npc">"NPC"</option>
                                    <option value="ally">"Ally"</option>
                                </select>
                            </div>

                            // AC
                            <div>
                                <label class="block text-xs text-zinc-500 mb-1">"Armor Class"</label>
                                <input
                                    type="number"
                                    class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-white text-sm text-center focus:border-purple-500 focus:outline-none"
                                    prop:value=move || new_ac.get()
                                    on:input=move |ev| new_ac.set(event_target_value(&ev))
                                />
                            </div>

                            // Current HP
                            <div>
                                <label class="block text-xs text-zinc-500 mb-1">"Current HP"</label>
                                <input
                                    type="number"
                                    class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-white text-sm text-center focus:border-purple-500 focus:outline-none"
                                    prop:value=move || new_hp.get()
                                    on:input=move |ev| new_hp.set(event_target_value(&ev))
                                />
                            </div>

                            // Max HP
                            <div>
                                <label class="block text-xs text-zinc-500 mb-1">"Max HP"</label>
                                <input
                                    type="number"
                                    class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded text-white text-sm text-center focus:border-purple-500 focus:outline-none"
                                    prop:value=move || new_max_hp.get()
                                    on:input=move |ev| new_max_hp.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        <div class="flex justify-end gap-2 pt-2">
                            <Button
                                variant=ButtonVariant::Ghost
                                class="px-4 py-2 bg-zinc-700 text-zinc-300 text-sm"
                                on_click=move |_: ev::MouseEvent| show_add_form.set(false)
                            >
                                "Cancel"
                            </Button>
                            <Button
                                variant=ButtonVariant::Primary
                                class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white text-sm font-medium"
                                on_click=move |_: ev::MouseEvent| {
                                    let sid = session_id.get();
                                    let name = new_name.get();
                                    let init: i32 = new_initiative.get().parse().unwrap_or(10);
                                    let ctype = new_type.get();
                                    let hp: Option<i32> = new_hp.get().parse().ok();
                                    let max_hp: Option<i32> = new_max_hp.get().parse().ok();
                                    let ac: Option<i32> = new_ac.get().parse().ok();

                                    if name.is_empty() {
                                        return;
                                    }

                                    spawn_local(async move {
                                        if add_combatant_full(sid.clone(), name, init, ctype, hp, max_hp, ac).await.is_ok() {
                                            on_combat_update.run(());
                                        }
                                    });

                                    // Reset form
                                    new_name.set(String::new());
                                    new_initiative.set("10".to_string());
                                    new_hp.set("20".to_string());
                                    new_max_hp.set("20".to_string());
                                    new_ac.set("10".to_string());
                                    new_type.set("monster".to_string());
                                    show_add_form.set(false);
                                }
                            >
                                "Add Combatant"
                            </Button>
                        </div>
                    </div>
                </Show>
            </div>

            // Condition modal
            <Show when=move || condition_modal_open.get()>
                <ConditionModal
                    combatant_id=condition_target_id
                    session_id=session_id
                    on_close=Callback::new(move |_| close_condition_modal())
                    on_condition_added=Callback::new(move |_| {
                        on_combat_update.run(());
                        close_condition_modal();
                    })
                />
            </Show>
        </div>
    }
}

// ============================================================================
// Initiative Order Summary (for sidebar/compact view)
// ============================================================================

/// Compact initiative order display
#[component]
pub fn InitiativeOrderSummary(
    /// Current combat state
    combat: Signal<Option<CombatState>>,
) -> impl IntoView {
    view! {
        <Show when=move || combat.get().is_some()>
            <div class="p-3 bg-zinc-800/50 rounded-lg border border-zinc-700/50">
                <h4 class="text-xs font-medium text-zinc-500 uppercase tracking-wider mb-2">
                    "Initiative Order"
                </h4>
                <div class="space-y-1">
                    <For
                        each=move || {
                            combat.get()
                                .map(|c| {
                                    c.combatants.into_iter()
                                        .enumerate()
                                        .map(|(idx, cb)| (idx, cb, c.current_turn))
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default()
                        }
                        key=|(_, combatant, _)| combatant.id.clone()
                        children=move |(idx, combatant, current_turn)| {
                            let is_current = idx == current_turn;
                            let row_class = if is_current {
                                "flex items-center gap-2 px-2 py-1 bg-purple-900/30 rounded text-purple-200"
                            } else {
                                "flex items-center gap-2 px-2 py-1 text-zinc-400"
                            };

                            view! {
                                <div class=row_class>
                                    <span class="w-6 text-center text-xs font-mono">
                                        {combatant.initiative.to_string()}
                                    </span>
                                    <span class="flex-1 text-sm truncate">
                                        {combatant.name.clone()}
                                    </span>
                                    {if is_current {
                                        Some(view! {
                                            <span class="w-2 h-2 rounded-full bg-purple-500 animate-pulse"/>
                                        })
                                    } else {
                                        None
                                    }}
                                </div>
                            }
                        }
                    />
                </div>
            </div>
        </Show>
    }
}
