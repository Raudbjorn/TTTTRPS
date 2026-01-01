#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::bindings::{
    get_campaign, get_active_session, list_sessions, start_session, end_session,
    start_combat, end_combat, get_combat, add_combatant, remove_combatant,
    next_turn, damage_combatant, heal_combatant,
    Campaign, GameSession, CombatState, SessionSummary,
};
use crate::components::campaign_details::session_list::SessionList;
use crate::components::campaign_details::npc_list::NPCList;
use crate::components::campaign_details::npc_conversation::NpcConversation;

#[component]
pub fn Session(campaign_id: String) -> Element {
    let campaign_id = use_signal(|| campaign_id.clone());
    let mut campaign = use_signal(|| Option::<Campaign>::None);
    let mut sessions = use_signal(|| Vec::<SessionSummary>::new());
    let mut active_session = use_signal(|| Option::<GameSession>::None);

    // UI State
    let mut selected_session_id = use_signal(|| Option::<String>::None);
    let mut is_loading = use_signal(|| true);

    // NPC Selection State
    let mut selected_npc_id = use_signal(|| Option::<String>::None);
    let mut selected_npc_name = use_signal(|| Option::<String>::None);

    let campaign_id_sig = use_signal(|| campaign_id.clone());
    let campaign_id_clone = campaign_id.clone();
    // Initial Load
    use_effect(move || {
        let cid = campaign_id.read().clone();
        spawn(async move {
            // Parallel fetch could be better but sequential is fine for now
            if let Ok(Some(c)) = get_campaign(cid.clone()).await {
                campaign.set(Some(c));
            }

            if let Ok(list) = list_sessions(cid.clone()).await {
                sessions.set(list);
            }

            if let Ok(Some(s)) = get_active_session(cid.clone()).await {
                active_session.set(Some(s.clone()));
                // Default select the active session
                selected_session_id.set(Some(s.id));
            } else {
                 // If no active session, maybe select the last one or none
            }

            is_loading.set(false);
        });
    });

    let handle_session_select = move |id: String| {
        selected_session_id.set(Some(id));
        // Clear NPC selection when selecting a session
        selected_npc_id.set(None);
        selected_npc_name.set(None);
    };

    // NPC selection handler - maps mock IDs to names
    let handle_npc_select = move |id: String| {
        // Mock NPC name lookup - in production, would fetch from backend
        let name = match id.as_str() {
            "npc-1" => "Garrosh",
            "npc-2" => "Elara",
            "npc-3" => "Zoltan",
            _ => "Unknown NPC",
        };
        selected_npc_id.set(Some(id));
        selected_npc_name.set(Some(name.to_string()));
    };

    let handle_npc_close = move |_| {
        selected_npc_id.set(None);
        selected_npc_name.set(None);
    };

    // Callback when a new session is started via the Active View (if empty)
    let mut on_session_started = move |s: GameSession| {
        active_session.set(Some(s.clone()));
        selected_session_id.set(Some(s.id.clone()));
        // Refresh list
        let cid = campaign_id.read().clone();
        spawn(async move {
            if let Ok(list) = list_sessions(cid).await {
                sessions.set(list);
            }
        });
    };

    let on_session_ended = move |_| {
         active_session.set(None);
         selected_session_id.set(None); // Or switch to "Summary" view of just ended
         // Refresh list
        let cid = campaign_id.read().clone();
        spawn(async move {
            if let Ok(list) = list_sessions(cid).await {
                sessions.set(list);
            }
        });
    };

    // Theme Logic - Dynamic Class Selection based on Campaign System
    // Supports: fantasy, cosmic, terminal, noir, neon (per design.md)
    //
    // TODO [FE F4]: Implement theme interpolation for blended settings
    // Currently uses single theme detection. Design spec (design.md) calls for
    // weighted theme blending via CSS custom property interpolation, e.g.:
    //   Delta Green = cosmic(0.4) + noir(0.6)
    // See ThemeWeights struct in design.md for full implementation plan.
    let theme_class = use_memo(move || {
        match campaign.read().as_ref() {
            Some(c) => {
                let system = c.system.to_lowercase();
                match system.as_str() {
                    // Noir themes: 90s office paranoia
                    s if s.contains("delta green") => "theme-noir",
                    s if s.contains("night's black agents") || s.contains("nba") => "theme-noir",

                    // Cosmic horror themes
                    s if s.contains("cthulhu") || s.contains("coc") => "theme-cosmic",
                    s if s.contains("kult") || s.contains("vaesen") => "theme-cosmic",

                    // Terminal/Sci-Fi themes
                    s if s.contains("mothership") => "theme-terminal",
                    s if s.contains("alien") && s.contains("rpg") => "theme-terminal",
                    s if s.contains("traveller") => "theme-terminal",
                    s if s.contains("stars without number") || s.contains("swn") => "theme-terminal",

                    // Neon/Cyberpunk themes
                    s if s.contains("cyberpunk") => "theme-neon",
                    s if s.contains("shadowrun") => "theme-neon",
                    s if s.contains("the sprawl") => "theme-neon",

                    // Fantasy (default)
                    s if s.contains("d&d") || s.contains("dnd") || s.contains("5e") => "theme-fantasy",
                    s if s.contains("pathfinder") => "theme-fantasy",
                    s if s.contains("warhammer fantasy") => "theme-fantasy",

                    // Default to fantasy for unknown systems
                    _ => "theme-fantasy"
                }
            },
            None => "theme-fantasy"
        }
    }).read().clone();

    rsx! {
        div {
            class: "flex h-screen w-screen bg-deep text-primary overflow-hidden font-body {theme_class}",

            // Left Sidebar: Session List
            SessionList {
                sessions: sessions.read().clone(),
                active_session_id: active_session.read().as_ref().map(|s| s.id.clone()),
                on_select_session: handle_session_select
            }

            // Center: Main Content
            div { class: "flex-1 flex flex-col min-w-0 bg-zinc-900",
                if is_loading.read().clone() {
                    div { class: "flex items-center justify-center h-full", "Loading Realm..." }
                } else {
                    // Header
                   div { class: "h-14 border-b border-zinc-800 flex items-center justify-between px-6 bg-zinc-900/50 backdrop-blur-sm",
                        div {
                            class: "flex items-center gap-4",
                            Link { to: crate::Route::Campaigns{}, class: "text-zinc-500 hover:text-white transition-colors", "← Back" }
                            h1 { class: "font-bold text-lg text-zinc-100", "{campaign.read().as_ref().map(|c| c.name.clone()).unwrap_or_default()}" }
                        }
                        div { class: "flex items-center gap-4",
                            // Transcription Toggle (Mock)
                            div { class: "flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-800 border border-zinc-700",
                                div { class: "w-2 h-2 rounded-full bg-red-500" } // Active dot logic to be added
                                span { class: "text-xs font-medium text-zinc-400", "Live Listen" }
                                // Toggle Switch Visual
                                div { class: "w-8 h-4 bg-zinc-700 rounded-full relative ml-2",
                                    div { class: "absolute left-0 top-0 w-4 h-4 bg-zinc-400 rounded-full shadow-sm transform scale-90 transition-transform" }
                                }
                            }
                        }
                   }

                   // Workspace
                   div { class: "flex-1 overflow-y-auto relative",
                        // NPC Conversation takes priority when selected
                        if let (Some(npc_id), Some(npc_name)) = (selected_npc_id.read().clone(), selected_npc_name.read().clone()) {
                            NpcConversation {
                                npc_id: npc_id,
                                npc_name: npc_name,
                                on_close: handle_npc_close
                            }
                        } else if let Some(selected_id) = selected_session_id.read().as_ref() {
                            // Check if it is the active session
                            div { class: "p-6",
                                if let Some(active) = active_session.read().as_ref() {
                                    if &active.id == selected_id {
                                        ActiveSessionWorkspace {
                                            session: active.clone(),
                                            on_session_ended: on_session_ended
                                        }
                                    } else {
                                        // Past Session View (Placeholder for now, implementation could be fetching logs)
                                         div { class: "flex flex-col items-center justify-center h-full text-zinc-500",
                                            h3 { class: "text-xl font-bold text-zinc-400 mb-2", "Historical Archive" }
                                            p { "Reviewing past logs for session {selected_id}..." }
                                            // Potential improvement: Fetch session details and show summary
                                        }
                                    }
                                } else {
                                    // Selected ID exists but no active session?
                                    // Means we are viewing history while no session is active.
                                    div { class: "flex flex-col items-center justify-center h-full text-zinc-500",
                                        h3 { class: "text-xl font-bold text-zinc-400 mb-2", "Historical Archive" }
                                        p { "Reviewing past logs for session {selected_id}..." }
                                    }
                                }
                            }
                        } else {
                            // No session selected
                            div { class: "p-6",
                                if active_session.read().is_none() {
                                    // Prompt to start new
                                    div { class: "flex flex-col items-center justify-center h-full",
                                        button {
                                            class: "px-6 py-3 bg-purple-600 hover:bg-purple-500 text-white rounded-lg shadow-lg font-bold transition-all transform hover:scale-105",
                                            onclick: move |_| {
                                               let cid = campaign_id_sig.read().clone();
                                               let s_num = campaign.read().as_ref().map(|c| c.session_count + 1).unwrap_or(1);
                                               spawn(async move {
                                                   if let Ok(s) = start_session(cid.clone(), s_num).await {
                                                       // Inline on_session_started behavior
                                                       active_session.set(Some(s.clone()));
                                                       selected_session_id.set(Some(s.id.clone()));
                                                       // Refresh list
                                                       if let Ok(list) = list_sessions(cid).await {
                                                           sessions.set(list);
                                                       }
                                                   }
                                               });
                                            },
                                            "Start New Session"
                                        }
                                    }
                                } else {
                                    div { class: "text-center text-zinc-500 mt-20", "Select a session from the sidebar" }
                                }
                            }
                        }
                   }
                }
            }

            // Right Sidebar: NPCs
            NPCList {
                campaign_id: campaign_id_sig.read().clone(),
                selected_npc_id: selected_npc_id.read().clone(),
                on_select_npc: handle_npc_select
            }
        }
    }
}

// Sub-component for the Active Session Logic (Combat, etc)
#[component]
fn ActiveSessionWorkspace(session: GameSession, on_session_ended: EventHandler<()>) -> Element {
    let mut combat = use_signal(|| Option::<CombatState>::None);
    let _status_message = use_signal(|| String::new());

    // Combatant Form
    let mut new_combatant_name = use_signal(|| String::new());
    let mut new_combatant_init = use_signal(|| "10".to_string());
    let mut new_combatant_type = use_signal(|| "monster".to_string());

    let session_id = use_signal(|| session.id.clone());
    let session_number = session.session_number;

    use_effect(move || {
        let sid = session_id.read().clone();
        spawn(async move {
             if let Ok(Some(c)) = get_combat(sid).await {
                 combat.set(Some(c));
             }
        });
    });

    // Handlers (Similar to original session.rs but using signals local to this component)
    let _handle_end_session = move |_| {
        let sid = session_id.read().clone();
        let cb = on_session_ended;
        spawn(async move {
            if end_session(sid).await.is_ok() {
                on_session_ended.call(());
            }
        });
    };

    // ... (For brevity, I will implement the core combat logic handlers here again)
    // NOTE: In a real refactor, I would extract `CombatTracker` to a separate file, but to keep existing functionality without creating too many files right now, I'll inline.

    rsx! {
        div { class: "space-y-6 max-w-5xl mx-auto",

            // Session Control Bar
            div { class: "flex justify-between items-center bg-zinc-800/50 p-4 rounded-lg border border-zinc-700",
                div {
                    div { class: "text-xs text-zinc-400 uppercase tracking-widest", "Current Session" }
                    div { class: "text-2xl font-bold text-white", "Session #{session_number}" }
                }
                button {
                    class: "px-4 py-2 bg-red-600/20 text-red-400 border border-red-600/50 rounded hover:bg-red-600 hover:text-white transition-colors",
                    onclick: _handle_end_session,
                    "End Session"
                }
            }

            // Combat Section
            div { class: "bg-zinc-800 rounded-lg shadow-xl overflow-hidden border border-zinc-700",
                div { class: "p-4 bg-zinc-900 border-b border-zinc-700 flex justify-between items-center",
                    h3 { class: "font-bold text-zinc-200", "Encounter Tracker" }
                    if combat.read().is_none() {
                         button {
                            class: "px-3 py-1 bg-purple-600 text-white text-sm rounded hover:bg-purple-500",
                            onclick: move |_| {
                                let sid = session_id.read().clone();
                                spawn(async move {
                                    if let Ok(c) = start_combat(sid).await {
                                        combat.set(Some(c));
                                    }
                                });
                            },
                            "Start Combat"
                        }
                    } else {
                        // Combat Controls
                        div { class: "flex gap-2",
                            button {
                                class: "px-3 py-1 bg-blue-600/20 text-blue-400 border border-blue-600/50 rounded text-sm hover:bg-blue-600 hover:text-white",
                                onclick: move |_| {
                                     let sid = session_id.read().clone();
                                     spawn(async move {
                                         if next_turn(sid.clone()).await.is_ok() {
                                              if let Ok(Some(c)) = get_combat(sid).await { combat.set(Some(c)); }
                                         }
                                     });
                                },
                                "Next Turn"
                            }
                             button {
                                class: "px-3 py-1 bg-zinc-700 text-zinc-300 rounded text-sm hover:bg-zinc-600",
                                onclick: move |_| {
                                     let sid = session_id.read().clone();
                                     spawn(async move {
                                         if end_combat(sid).await.is_ok() {
                                              combat.set(None);
                                         }
                                     });
                                },
                                "End Encounter"
                            }
                        }
                    }
                }

                if let Some(c) = combat.read().as_ref() {
                    div { class: "p-0",
                         // Turn Order List
                         div { class: "divide-y divide-zinc-700",
                             for (idx, combatant) in c.combatants.iter().enumerate() {
                                 {
                                     let cid_dmg = combatant.id.clone();
                                     let cid_heal = combatant.id.clone();
                                     let cid_remove = combatant.id.clone();
                                     let combatant_name = combatant.name.clone();
                                     rsx! {
                                         div {
                                     class: if idx == c.current_turn { "bg-purple-900/20 flex items-center p-3 border-l-4 border-purple-500" } else { "flex items-center p-3 hover:bg-zinc-700/50" },
                                     // Init
                                     div { class: "w-12 text-center font-mono text-xl text-zinc-500", "{combatant.initiative}" }
                                     // Info
                                     div { class: "flex-1 px-4",
                                        div { class: "font-bold text-zinc-200", "{combatant.name}" }
                                        div { class: "text-xs text-zinc-500 uppercase", "{combatant.combatant_type}" }
                                     }
                                     // HP & Actions
                                     div { class: "flex items-center gap-3",
                                        div { class: "text-zinc-400 font-mono", "{combatant.hp_current} / {combatant.hp_max}" }
                                        // Quick Actions
                                        button {
                                            class: "w-8 h-8 rounded bg-red-900/50 text-red-400 hover:bg-red-600 hover:text-white",
                                            aria_label: "Deal 1 damage to {combatant_name}",
                                            onclick: move |_| {
                                                let sid = session_id.read().clone();
                                                let cid = cid_dmg.clone();
                                                spawn(async move {
                                                    if damage_combatant(sid.clone(), cid, 1).await.is_ok() {
                                                         if let Ok(Some(c)) = get_combat(sid).await { combat.set(Some(c)); }
                                                    }
                                                });
                                            },
                                            "-"
                                        }
                                        button {
                                            class: "w-8 h-8 rounded bg-green-900/50 text-green-400 hover:bg-green-600 hover:text-white",
                                            aria_label: "Heal 1 HP for {combatant_name}",
                                            onclick: move |_| {
                                                let sid = session_id.read().clone();
                                                let cid = cid_heal.clone();
                                                spawn(async move {
                                                    if heal_combatant(sid.clone(), cid, 1).await.is_ok() {
                                                         if let Ok(Some(c)) = get_combat(sid).await { combat.set(Some(c)); }
                                                    }
                                                });
                                            },
                                            "+"
                                        }
                                        button {
                                            class: "w-8 h-8 rounded bg-zinc-700/50 text-zinc-400 hover:bg-zinc-600 hover:text-white ml-2",
                                            aria_label: "Remove {combatant_name} from combat",
                                            onclick: move |_| {
                                                let sid = session_id.read().clone();
                                                let cid = cid_remove.clone();
                                                spawn(async move {
                                                    if remove_combatant(sid.clone(), cid).await.is_ok() {
                                                         if let Ok(Some(c)) = get_combat(sid).await { combat.set(Some(c)); }
                                                    }
                                                });
                                            },
                                            "×"
                                        }
                                     }
                                 }
                             }
                         }
                         }
                         }
                         // Add Combatant
                         div { class: "p-4 bg-zinc-900/50 flex gap-2 border-t border-zinc-700",
                            input {
                                class: "bg-zinc-800 border-zinc-700 rounded px-3 py-2 text-sm text-white flex-1",
                                placeholder: "Name",
                                value: "{new_combatant_name}",
                                oninput: move |e| new_combatant_name.set(e.value())
                            }
                            input {
                                class: "bg-zinc-800 border-zinc-700 rounded px-3 py-2 text-sm text-white w-20 text-center",
                                placeholder: "Init",
                                r#type: "number",
                                value: "{new_combatant_init}",
                                oninput: move |e| new_combatant_init.set(e.value())
                            }
                            select {
                                class: "bg-zinc-800 border border-zinc-700 rounded px-3 py-2 text-sm text-white",
                                value: "{new_combatant_type}",
                                onchange: move |e| new_combatant_type.set(e.value()),
                                option { value: "player", "Player" }
                                option { value: "monster", selected: true, "Monster" }
                                option { value: "npc", "NPC" }
                                option { value: "ally", "Ally" }
                            }
                            button {
                                class: "px-4 py-2 bg-zinc-700 hover:bg-zinc-600 text-white rounded text-sm font-medium",
                                onclick: move |_| {
                                    let sid = session_id.read().clone();
                                    let name = new_combatant_name.read().clone();
                                    let init = new_combatant_init.read().parse().unwrap_or(10);
                                    let ctype = new_combatant_type.read().clone();

                                    spawn(async move {
                                         if add_combatant(sid.clone(), name, init, ctype).await.is_ok() {
                                             if let Ok(Some(c)) = get_combat(sid).await {
                                                 combat.set(Some(c));
                                             }
                                         }
                                    });
                                },
                                "Add"
                            }
                         }
                    }
                } else {
                    div { class: "p-8 text-center text-zinc-500",
                        "Peaceful times. Start combat to track initiative."
                    }
                }
            }
        }
    }
}

