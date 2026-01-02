//! Session page component for managing TTRPG game sessions
//!
//! Migrated from Dioxus session.rs to Leptos 0.7

mod session_list;
mod npc_list;
mod active_session_workspace;

use leptos::prelude::*;
use leptos::ev;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    get_campaign, get_active_session, list_sessions, start_session,
    Campaign, GameSession, SessionSummary,
};
use crate::components::design_system::{Button, ButtonVariant};

use session_list::SessionList;
use npc_list::NpcList;
use active_session_workspace::ActiveSessionWorkspace;

pub use session_list::SessionList as SessionListComponent;
pub use npc_list::NpcList as NpcListComponent;
pub use active_session_workspace::ActiveSessionWorkspace as ActiveSessionWorkspaceComponent;

/// Route params for session page
#[derive(Params, PartialEq, Clone, Default)]
pub struct SessionParams {
    pub campaign_id: Option<String>,
}

/// Main Session page component
/// Displays session management, combat tracking, and NPC interactions
#[component]
pub fn Session() -> impl IntoView {
    // Get campaign_id from route params
    let params = use_params::<SessionParams>();
    let campaign_id_memo = Memo::new(move |_| {
        params.get()
            .ok()
            .and_then(|p| p.campaign_id)
            .unwrap_or_default()
    });

    // State signals
    let campaign = RwSignal::new(Option::<Campaign>::None);
    let sessions = RwSignal::new(Vec::<SessionSummary>::new());
    let active_session = RwSignal::new(Option::<GameSession>::None);
    let selected_session_id = RwSignal::new(Option::<String>::None);
    let is_loading = RwSignal::new(true);

    // NPC Selection State
    let selected_npc_id = RwSignal::new(Option::<String>::None);
    let selected_npc_name = RwSignal::new(Option::<String>::None);

    // Initial data load effect
    Effect::new(move |_| {
        let cid = campaign_id_memo.get();
        if cid.is_empty() {
            is_loading.set(false);
            return;
        }

        spawn_local(async move {
            // Load campaign data
            if let Ok(Some(c)) = get_campaign(cid.clone()).await {
                campaign.set(Some(c));
            }

            // Load session list
            if let Ok(list) = list_sessions(cid.clone()).await {
                sessions.set(list);
            }

            // Check for active session
            if let Ok(Some(s)) = get_active_session(cid.clone()).await {
                selected_session_id.set(Some(s.id.clone()));
                active_session.set(Some(s));
            }

            is_loading.set(false);
        });
    });

    // Session ended callback
    let on_session_ended = Callback::new(move |_: ()| {
        active_session.set(None);
        selected_session_id.set(None);
        // Refresh list
        let cid = campaign_id_memo.get();
        spawn_local(async move {
            if let Ok(list) = list_sessions(cid).await {
                sessions.set(list);
            }
        });
    });

    // Theme logic - dynamic class selection based on campaign system
    let theme_class = Memo::new(move |_| {
        match campaign.get().as_ref() {
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
        }.to_string()
    });

    view! {
        <div class=move || format!(
            "flex h-screen w-screen bg-deep text-primary overflow-hidden font-body {}",
            theme_class.get()
        )>
            // Left Sidebar: Session List
            <SessionList
                sessions=sessions
                active_session_id=Signal::derive(move || active_session.get().map(|s| s.id))
                on_select_session=Callback::new(move |id: String| {
                    selected_session_id.set(Some(id));
                    // Clear NPC selection when selecting a session
                    selected_npc_id.set(None);
                    selected_npc_name.set(None);
                })
            />

            // Center: Main Content
            <div class="flex-1 flex flex-col min-w-0 bg-zinc-900">
                <Show
                    when=move || !is_loading.get()
                    fallback=|| view! {
                        <div class="flex items-center justify-center h-full">
                            "Loading Realm..."
                        </div>
                    }
                >
                    // Header
                    <div class="h-14 border-b border-zinc-800 flex items-center justify-between px-6 bg-zinc-900/50 backdrop-blur-sm">
                        <div class="flex items-center gap-4">
                            <a
                                href="/campaigns"
                                class="text-zinc-500 hover:text-white transition-colors"
                            >
                                "< Back"
                            </a>
                            <h1 class="font-bold text-lg text-zinc-100">
                                {move || campaign.get().map(|c| c.name).unwrap_or_default()}
                            </h1>
                        </div>
                        <div class="flex items-center gap-4">
                            // Transcription Toggle (Mock)
                            <div class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-800 border border-zinc-700">
                                <div class="w-2 h-2 rounded-full bg-red-500"></div>
                                <span class="text-xs font-medium text-zinc-400">"Live Listen"</span>
                                // Toggle Switch Visual
                                <div class="w-8 h-4 bg-zinc-700 rounded-full relative ml-2">
                                    <div class="absolute left-0 top-0 w-4 h-4 bg-zinc-400 rounded-full shadow-sm transform scale-90 transition-transform"></div>
                                </div>
                            </div>
                        </div>
                    </div>

                    // Workspace
                    <div class="flex-1 overflow-y-auto relative">
                        {move || {
                            let npc_id = selected_npc_id.get();
                            let npc_name = selected_npc_name.get();
                            let sel_id = selected_session_id.get();
                            let active = active_session.get();

                            if npc_id.is_some() && npc_name.is_some() {
                                // NPC Conversation view
                                view! {
                                    <NpcConversation
                                        npc_id=npc_id.unwrap()
                                        npc_name=npc_name.unwrap()
                                        on_close=Callback::new(move |_| {
                                            selected_npc_id.set(None);
                                            selected_npc_name.set(None);
                                        })
                                    />
                                }.into_any()
                            } else if let Some(selected_id) = sel_id {
                                if let Some(ref active_sess) = active {
                                    if active_sess.id == selected_id {
                                        view! {
                                            <div class="p-6">
                                                <ActiveSessionWorkspace
                                                    session=active_sess.clone()
                                                    on_session_ended=on_session_ended
                                                />
                                            </div>
                                        }.into_any()
                                    } else {
                                        // Past Session View
                                        view! {
                                            <div class="flex flex-col items-center justify-center h-full text-zinc-500">
                                                <h3 class="text-xl font-bold text-zinc-400 mb-2">"Historical Archive"</h3>
                                                <p>{format!("Reviewing past logs for session {}...", selected_id)}</p>
                                            </div>
                                        }.into_any()
                                    }
                                } else {
                                    // Selected ID exists but no active session
                                    view! {
                                        <div class="flex flex-col items-center justify-center h-full text-zinc-500">
                                            <h3 class="text-xl font-bold text-zinc-400 mb-2">"Historical Archive"</h3>
                                            <p>{format!("Reviewing past logs for session {}...", selected_id)}</p>
                                        </div>
                                    }.into_any()
                                }
                            } else {
                                // No session selected
                                if active.is_none() {
                                    view! {
                                        <div class="p-6">
                                            <div class="flex flex-col items-center justify-center h-full">
                                                <Button
                                                    variant=ButtonVariant::Primary
                                                    class="px-6 py-3 bg-purple-600 hover:bg-purple-500 text-white rounded-lg shadow-lg font-bold transition-all transform hover:scale-105"
                                                    on_click=move |_: ev::MouseEvent| {
                                                        let cid = campaign_id_memo.get();
                                                        let sess_num = campaign.get().map(|c| c.session_count + 1).unwrap_or(1);
                                                        spawn_local(async move {
                                                            if let Ok(s) = start_session(cid.clone(), sess_num).await {
                                                                active_session.set(Some(s.clone()));
                                                                selected_session_id.set(Some(s.id.clone()));
                                                                // Refresh list
                                                                if let Ok(list) = list_sessions(cid).await {
                                                                    sessions.set(list);
                                                                }
                                                            }
                                                        });
                                                    }
                                                >
                                                    "Start New Session"
                                                </Button>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="text-center text-zinc-500 mt-20">
                                            "Select a session from the sidebar"
                                        </div>
                                    }.into_any()
                                }
                            }
                        }}
                    </div>
                </Show>
            </div>

            // Right Sidebar: NPCs
            <NpcList
                campaign_id=campaign_id_memo.into()
                selected_npc_id=selected_npc_id.into()
                on_select_npc=Callback::new(move |id: String| {
                    // Mock NPC name lookup - in production, would fetch from backend
                    let name = match id.as_str() {
                        "npc-1" => "Garrosh",
                        "npc-2" => "Elara",
                        "npc-3" => "Zoltan",
                        _ => "Unknown NPC",
                    };
                    selected_npc_id.set(Some(id));
                    selected_npc_name.set(Some(name.to_string()));
                })
            />
        </div>
    }
}

/// NPC Conversation placeholder component
#[component]
fn NpcConversation(
    npc_id: String,
    npc_name: String,
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="p-6 h-full flex flex-col">
            <div class="flex items-center justify-between mb-4">
                <h2 class="text-xl font-bold text-white">{npc_name}</h2>
                <button
                    class="px-3 py-1 text-zinc-400 hover:text-white transition-colors"
                    on:click=move |_| on_close.run(())
                >
                    "Close"
                </button>
            </div>
            <div class="flex-1 bg-zinc-800/50 rounded-lg p-4 text-zinc-400">
                <p class="text-center mt-20">"NPC conversation interface coming soon..."</p>
                <p class="text-center text-sm text-zinc-600 mt-2">{format!("NPC ID: {}", npc_id)}</p>
            </div>
        </div>
    }
}
