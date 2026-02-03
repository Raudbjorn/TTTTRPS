//! Combat Tracker Component (TASK-016)
//!
//! Main combat tracking component with initiative order, HP tracking,
//! conditions, and round management.

use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use super::initiative_list::InitiativeList;
use crate::bindings::{end_combat, get_combat, next_turn, start_combat, CombatState};
use crate::components::design_system::{Button, ButtonVariant, Card, CardBody, CardHeader};

/// Combat tracker component
#[component]
pub fn CombatTracker(
    /// Session ID to track combat for
    session_id: Signal<String>,
    /// Optional callback when combat state changes
    #[prop(optional)]
    on_combat_change: Option<Callback<Option<CombatState>>>,
) -> impl IntoView {
    // Combat state
    let combat = RwSignal::new(Option::<CombatState>::None);
    let is_loading = RwSignal::new(false);

    // Load combat state on mount and when session changes
    Effect::new(move |_| {
        let sid = session_id.get();
        if sid.is_empty() {
            return;
        }

        is_loading.set(true);
        spawn_local(async move {
            if let Ok(Some(c)) = get_combat(sid).await {
                combat.set(Some(c.clone()));
                if let Some(callback) = on_combat_change {
                    callback.run(Some(c));
                }
            }
            is_loading.set(false);
        });
    });

    // Refresh combat state helper
    let refresh_combat = move || {
        let sid = session_id.get();
        spawn_local(async move {
            if let Ok(result) = get_combat(sid).await {
                combat.set(result.clone());
                if let Some(callback) = on_combat_change {
                    callback.run(result);
                }
            }
        });
    };

    // Start combat handler
    let handle_start_combat = move |_: ev::MouseEvent| {
        let sid = session_id.get();
        spawn_local(async move {
            if let Ok(c) = start_combat(sid).await {
                combat.set(Some(c.clone()));
                if let Some(callback) = on_combat_change {
                    callback.run(Some(c));
                }
            }
        });
    };

    // End combat handler
    let handle_end_combat = move |_: ev::MouseEvent| {
        let sid = session_id.get();
        spawn_local(async move {
            if end_combat(sid).await.is_ok() {
                combat.set(None);
                if let Some(callback) = on_combat_change {
                    callback.run(None);
                }
            }
        });
    };

    // Next turn handler
    let handle_next_turn = move |_: ev::MouseEvent| {
        let sid = session_id.get();
        spawn_local(async move {
            if next_turn(sid.clone()).await.is_ok() {
                if let Ok(Some(c)) = get_combat(sid).await {
                    combat.set(Some(c.clone()));
                    if let Some(callback) = on_combat_change {
                        callback.run(Some(c));
                    }
                }
            }
        });
    };

    view! {
        <Card class="combat-tracker">
            <CardHeader class="flex flex-row justify-between items-center space-y-0">
                <div class="flex items-center gap-3">
                    <div class="w-3 h-3 rounded-full animate-pulse"
                        class:bg-red-500=move || combat.get().is_some()
                        class:bg-zinc-600=move || combat.get().is_none()
                    />
                    <h3 class="font-bold text-zinc-200 text-lg">"Encounter Tracker"</h3>
                </div>

                <div class="flex items-center gap-2">
                    <Show when=move || combat.get().is_some()>
                        // Round counter
                        <div class="flex items-center gap-2 px-3 py-1.5 bg-zinc-800 rounded-lg border border-zinc-700">
                            <span class="text-xs text-zinc-500 uppercase tracking-wider">"Round"</span>
                            <span class="text-lg font-bold text-white font-mono">
                                {move || combat.get().map(|c| c.round.to_string()).unwrap_or_default()}
                            </span>
                        </div>

                        // Turn indicator
                        <div class="flex items-center gap-2 px-3 py-1.5 bg-purple-900/30 rounded-lg border border-purple-700/50">
                            <span class="text-xs text-purple-400 uppercase tracking-wider">"Turn"</span>
                            <span class="text-lg font-bold text-purple-300 font-mono">
                                {move || combat.get().map(|c| (c.current_turn + 1).to_string()).unwrap_or_default()}
                            </span>
                        </div>
                    </Show>
                </div>

                // Combat control buttons
                <div class="flex gap-2">
                    <Show
                        when=move || combat.get().is_none()
                        fallback=move || view! {
                            <Button
                                variant=ButtonVariant::Secondary
                                class="px-3 py-1.5 bg-blue-600/20 text-blue-400 border border-blue-600/50 text-sm font-medium hover:bg-blue-600/30 transition-colors"
                                on_click=handle_next_turn
                            >
                                <span class="flex items-center gap-1.5">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7"/>
                                    </svg>
                                    "Next Turn"
                                </span>
                            </Button>
                            <Button
                                variant=ButtonVariant::Ghost
                                class="px-3 py-1.5 bg-zinc-700/50 text-zinc-300 text-sm font-medium hover:bg-zinc-600/50 transition-colors"
                                on_click=handle_end_combat
                            >
                                <span class="flex items-center gap-1.5">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z"/>
                                    </svg>
                                    "End Combat"
                                </span>
                            </Button>
                        }
                    >
                        <Button
                            variant=ButtonVariant::Primary
                            class="px-4 py-2 bg-gradient-to-r from-red-600 to-orange-600 text-white text-sm font-bold hover:from-red-500 hover:to-orange-500 transition-all shadow-lg shadow-red-900/30"
                            on_click=handle_start_combat
                        >
                            <span class="flex items-center gap-2">
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                                </svg>
                                "Start Combat"
                            </span>
                        </Button>
                    </Show>
                </div>
            </CardHeader>

            <Show
                when=move || combat.get().is_some()
                fallback=|| view! {
                    <CardBody class="flex flex-col items-center justify-center py-12 text-center">
                        <div class="w-16 h-16 rounded-full bg-zinc-800 flex items-center justify-center mb-4">
                            <svg class="w-8 h-8 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707"/>
                            </svg>
                        </div>
                        <h4 class="text-lg font-medium text-zinc-400 mb-2">"No Active Combat"</h4>
                        <p class="text-sm text-zinc-500 max-w-xs">
                            "Start combat to track initiative order, HP, conditions, and more."
                        </p>
                    </CardBody>
                }
            >
                <div class="p-0">
                    // Initiative list with all combatants
                    <InitiativeList
                        session_id=session_id
                        combat=combat.into()
                        on_combat_update=Callback::new(move |_| refresh_combat())
                    />
                </div>
            </Show>
        </Card>
    }
}

// ============================================================================
// Combat Stats Bar (for header display)
// ============================================================================

/// Displays quick combat stats
#[component]
pub fn CombatStatsBar(
    /// Current combat state
    combat: Signal<Option<CombatState>>,
) -> impl IntoView {
    let stats = Memo::new(move |_| {
        combat.get().map(|c| {
            let total_combatants = c.combatants.len();
            let active_combatants = c.combatants.iter().filter(|cb| cb.is_active).count();
            let total_hp: i32 = c.combatants.iter().map(|cb| cb.hp_current).sum();
            let max_hp: i32 = c.combatants.iter().map(|cb| cb.hp_max).sum();

            (total_combatants, active_combatants, total_hp, max_hp)
        })
    });

    view! {
        <Show when=move || combat.get().is_some()>
            <div class="flex items-center gap-4 px-4 py-2 bg-zinc-800/50 rounded-lg border border-zinc-700/50">
                // Combatant count
                <div class="flex items-center gap-1.5">
                    <svg class="w-4 h-4 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z"/>
                    </svg>
                    <span class="text-sm text-zinc-400">
                        {move || stats.get().map(|(_, active, _, _)| format!("{} active", active)).unwrap_or_default()}
                    </span>
                </div>

                // HP total
                <div class="flex items-center gap-1.5">
                    <svg class="w-4 h-4 text-red-500" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/>
                    </svg>
                    <span class="text-sm text-zinc-400">
                        {move || stats.get().map(|(_, _, total, max)| format!("{}/{} HP", total, max)).unwrap_or_default()}
                    </span>
                </div>
            </div>
        </Show>
    }
}
