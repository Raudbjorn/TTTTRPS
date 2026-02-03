//! Combatant Card Component (TASK-016)
//!
//! Individual combatant display with HP bar, AC, conditions, and quick actions.

use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    damage_combatant, heal_combatant, remove_combatant, remove_condition, Combatant,
};

/// Combatant card component
#[component]
pub fn CombatantCard(
    /// The combatant to display
    combatant: Combatant,
    /// Whether this is the current turn
    is_current_turn: bool,
    /// Session ID for API calls
    session_id: Signal<String>,
    /// Callback when combatant is updated
    on_update: Callback<()>,
    /// Callback to open condition modal
    on_add_condition: Callback<String>,
) -> impl IntoView {
    let combatant_id = StoredValue::new(combatant.id.clone());
    let combatant_name = combatant.name.clone();
    let combatant_type = combatant.combatant_type.clone();
    let initiative = combatant.initiative;
    let hp_current = RwSignal::new(combatant.hp_current);
    let hp_max = combatant.hp_max;
    let hp_temp = combatant.hp_temp.unwrap_or(0);
    let ac = combatant.ac;
    let conditions = RwSignal::new(combatant.conditions.clone());
    let is_active = combatant.is_active;

    // Calculate HP percentage for bar
    let hp_percentage = move || {
        let current = hp_current.get();
        if hp_max > 0 {
            ((current as f32 / hp_max as f32) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        }
    };

    // HP bar color based on percentage
    let hp_bar_class = move || {
        let pct = hp_percentage();
        if pct <= 25.0 {
            "bg-red-500"
        } else if pct <= 50.0 {
            "bg-orange-500"
        } else if pct <= 75.0 {
            "bg-yellow-500"
        } else {
            "bg-green-500"
        }
    };

    // Combatant type icon and color
    let type_info = move || match combatant_type.as_str() {
        "player" => ("P", "bg-blue-600", "text-blue-100"),
        "monster" => ("M", "bg-red-600", "text-red-100"),
        "npc" => ("N", "bg-purple-600", "text-purple-100"),
        "ally" => ("A", "bg-green-600", "text-green-100"),
        _ => ("?", "bg-zinc-600", "text-zinc-100"),
    };

    // Quick damage/heal handlers
    let damage_amount = RwSignal::new(1_i32);
    let heal_amount = RwSignal::new(1_i32);

    let handle_damage = move |_: ev::MouseEvent| {
        let sid = session_id.get();
        let cid = combatant_id.get_value();
        let amount = damage_amount.get();

        spawn_local(async move {
            if let Ok(new_hp) = damage_combatant(sid, cid, amount).await {
                hp_current.set(new_hp);
                on_update.run(());
            }
        });
    };

    let handle_heal = move |_: ev::MouseEvent| {
        let sid = session_id.get();
        let cid = combatant_id.get_value();
        let amount = heal_amount.get();

        spawn_local(async move {
            if let Ok(new_hp) = heal_combatant(sid, cid, amount).await {
                hp_current.set(new_hp);
                on_update.run(());
            }
        });
    };

    let handle_remove = move |_: ev::MouseEvent| {
        let sid = session_id.get();
        let cid = combatant_id.get_value();

        spawn_local(async move {
            if remove_combatant(sid, cid).await.is_ok() {
                on_update.run(());
            }
        });
    };

    let handle_open_conditions = {
        let cid = combatant_id.get_value();
        move |_: ev::MouseEvent| {
            on_add_condition.run(cid.clone());
        }
    };

    // Card styling based on state
    let card_class = move || {
        let mut classes = vec![
            "flex",
            "items-stretch",
            "gap-3",
            "p-3",
            "transition-all",
            "duration-200",
        ];

        if is_current_turn {
            classes.extend([
                "bg-gradient-to-r",
                "from-purple-900/40",
                "to-transparent",
                "border-l-4",
                "border-purple-500",
                "shadow-lg",
                "shadow-purple-900/20",
            ]);
        } else if !is_active {
            classes.extend(["opacity-50", "bg-zinc-900/50"]);
        } else {
            classes.extend(["hover:bg-zinc-700/30"]);
        }

        classes.join(" ")
    };

    view! {
        <div class=card_class>
            // Initiative badge
            <div class="flex flex-col items-center justify-center w-14 shrink-0">
                <div class="text-2xl font-bold font-mono text-zinc-400">
                    {initiative.to_string()}
                </div>
                <div class={move || format!(
                    "w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold {} {}",
                    type_info().1,
                    type_info().2
                )}>
                    {type_info().0}
                </div>
            </div>

            // Main content
            <div class="flex-1 min-w-0">
                // Name row with AC badge
                <div class="flex items-center gap-2 mb-2">
                    <h4 class="font-bold text-zinc-100 truncate">
                        {combatant_name.clone()}
                    </h4>
                    // AC badge
                    {ac.map(|armor_class| view! {
                        <span class="px-2 py-0.5 text-xs font-medium bg-blue-900/50 text-blue-300 rounded border border-blue-700/50"
                              title="Armor Class">
                            <span class="text-blue-500">"AC "</span>
                            {armor_class.to_string()}
                        </span>
                    })}
                    // Temp HP badge
                    {if hp_temp > 0 {
                        Some(view! {
                            <span class="px-2 py-0.5 text-xs font-medium bg-cyan-900/50 text-cyan-300 rounded border border-cyan-700/50"
                                  title="Temporary Hit Points">
                                <span class="text-cyan-500">"+"</span>
                                {hp_temp.to_string()}
                            </span>
                        })
                    } else {
                        None
                    }}
                    {if is_current_turn {
                        Some(view! {
                            <span class="px-2 py-0.5 text-xs font-medium bg-purple-600/50 text-purple-200 rounded-full animate-pulse">
                                "TURN"
                            </span>
                        })
                    } else {
                        None
                    }}
                </div>

                // HP Bar
                <div class="mb-2">
                    <div class="flex items-center gap-2 mb-1">
                        <div class="flex-1 h-3 bg-zinc-800 rounded-full overflow-hidden relative">
                            <div
                                class=move || format!(
                                    "absolute inset-y-0 left-0 {} transition-all duration-300 ease-out rounded-full",
                                    hp_bar_class()
                                )
                                style:width=move || format!("{}%", hp_percentage())
                            />
                            // HP text overlay
                            <div class="absolute inset-0 flex items-center justify-center">
                                <span class="text-xs font-bold text-white drop-shadow-md">
                                    {move || format!("{} / {}", hp_current.get(), hp_max)}
                                </span>
                            </div>
                        </div>
                    </div>
                </div>

                // Conditions with removal capability
                <div class="flex flex-wrap gap-1 mb-2">
                    <For
                        each=move || conditions.get()
                        key=|condition| condition.clone()
                        children=move |condition| {
                            let condition_name = condition.clone();
                            let condition_for_remove = condition.clone();
                            let display_name = condition.clone();

                            let handle_remove_condition = move |_: ev::MouseEvent| {
                                let sid = session_id.get();
                                let cid = combatant_id.get_value();
                                let cond = condition_for_remove.clone();

                                spawn_local(async move {
                                    if remove_condition(sid, cid, cond).await.is_ok() {
                                        on_update.run(());
                                    }
                                });
                            };

                            view! {
                                <span class="group inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium bg-yellow-900/50 text-yellow-400 border border-yellow-500/30 rounded-full">
                                    {display_name}
                                    <button
                                        class="w-3 h-3 rounded-full bg-yellow-800/50 text-yellow-300 hover:bg-red-600 hover:text-white transition-colors flex items-center justify-center opacity-0 group-hover:opacity-100"
                                        on:click=handle_remove_condition
                                        aria-label=format!("Remove {}", condition_name)
                                    >
                                        <svg class="w-2 h-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M6 18L18 6M6 6l12 12"/>
                                        </svg>
                                    </button>
                                </span>
                            }
                        }
                    />
                </div>
            </div>

            // Quick actions panel
            <div class="flex flex-col gap-2 shrink-0">
                // Damage controls
                <div class="flex items-center gap-1">
                    <input
                        type="number"
                        class="w-12 px-2 py-1 text-center text-sm bg-zinc-800 border border-zinc-700 rounded text-white"
                        prop:value=move || damage_amount.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                damage_amount.set(v.max(1));
                            }
                        }
                        min="1"
                    />
                    <button
                        class="w-8 h-8 rounded bg-red-900/60 text-red-400 hover:bg-red-600 hover:text-white transition-colors flex items-center justify-center"
                        aria-label=format!("Deal damage to {}", combatant_name)
                        on:click=handle_damage
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4"/>
                        </svg>
                    </button>
                </div>

                // Heal controls
                <div class="flex items-center gap-1">
                    <input
                        type="number"
                        class="w-12 px-2 py-1 text-center text-sm bg-zinc-800 border border-zinc-700 rounded text-white"
                        prop:value=move || heal_amount.get().to_string()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                heal_amount.set(v.max(1));
                            }
                        }
                        min="1"
                    />
                    <button
                        class="w-8 h-8 rounded bg-green-900/60 text-green-400 hover:bg-green-600 hover:text-white transition-colors flex items-center justify-center"
                        aria-label=format!("Heal {}", combatant_name)
                        on:click=handle_heal
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                        </svg>
                    </button>
                </div>

                // Condition and remove buttons
                <div class="flex gap-1">
                    <button
                        class="flex-1 h-8 rounded bg-yellow-900/60 text-yellow-400 hover:bg-yellow-600 hover:text-white transition-colors flex items-center justify-center"
                        aria-label=format!("Add condition to {}", combatant_name)
                        on:click=handle_open_conditions
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                        </svg>
                    </button>
                    <button
                        class="flex-1 h-8 rounded bg-zinc-700/60 text-zinc-400 hover:bg-zinc-600 hover:text-white transition-colors flex items-center justify-center"
                        aria-label=format!("Remove {} from combat", combatant_name)
                        on:click=handle_remove
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Compact Combatant Row (for minimal UI)
// ============================================================================

/// Compact combatant row for tight spaces
#[component]
pub fn CombatantRowCompact(
    /// The combatant to display
    combatant: Combatant,
    /// Whether this is the current turn
    is_current_turn: bool,
) -> impl IntoView {
    let hp_percentage = {
        let current = combatant.hp_current;
        let max = combatant.hp_max;
        if max > 0 {
            ((current as f32 / max as f32) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        }
    };

    let hp_bar_color = if hp_percentage <= 25.0 {
        "bg-red-500"
    } else if hp_percentage <= 50.0 {
        "bg-orange-500"
    } else if hp_percentage <= 75.0 {
        "bg-yellow-500"
    } else {
        "bg-green-500"
    };

    let row_class = if is_current_turn {
        "flex items-center gap-2 px-2 py-1.5 bg-purple-900/30 border-l-2 border-purple-500"
    } else {
        "flex items-center gap-2 px-2 py-1.5 hover:bg-zinc-800/50"
    };

    view! {
        <div class=row_class>
            // Initiative
            <span class="w-8 text-center font-mono text-sm text-zinc-500">
                {combatant.initiative.to_string()}
            </span>

            // Name
            <span class="flex-1 text-sm text-zinc-200 truncate">
                {combatant.name.clone()}
            </span>

            // AC (if present)
            {combatant.ac.map(|ac| view! {
                <span class="w-8 text-center text-xs font-mono text-blue-400" title="AC">
                    {ac.to_string()}
                </span>
            })}

            // HP mini bar
            <div class="w-16 h-2 bg-zinc-800 rounded-full overflow-hidden">
                <div
                    class=format!("h-full {} rounded-full", hp_bar_color)
                    style:width=format!("{}%", hp_percentage)
                />
            </div>

            // HP text
            <span class="w-12 text-right text-xs font-mono text-zinc-400">
                {format!("{}/{}", combatant.hp_current, combatant.hp_max)}
            </span>

            // Condition count
            {if !combatant.conditions.is_empty() {
                Some(view! {
                    <span class="px-1.5 py-0.5 text-xs bg-yellow-900/50 text-yellow-400 rounded" title={combatant.conditions.join(", ")}>
                        {combatant.conditions.len().to_string()}
                    </span>
                })
            } else {
                None
            }}
        </div>
    }
}
