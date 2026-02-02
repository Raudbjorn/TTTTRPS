//! Session page component for managing TTRPG game sessions
//!
//! Session management components

mod session_list;
mod npc_list;
mod active_session_workspace;

// TASK-016: Combat Tracker UI components
pub mod combat_tracker;
pub mod combatant_card;
pub mod initiative_list;
pub mod condition_manager;

// TASK-014: Timeline View
pub mod timeline_view;

// TASK-017: Notes Panel
pub mod notes_panel;

// Phase 6: Session Control Panel
pub mod control_panel;

// Phase 8: Session Recaps
pub mod recap_viewer;

// Phase 9: Quick Reference Cards & Cheat Sheets
pub mod entity_card;
pub mod card_tray;
pub mod cheat_sheet_viewer;

// Phase 8: Conversation Thread Tabs & Session Chat
pub mod thread_tabs;
pub mod session_chat_panel;

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
use crate::components::campaign_details::NpcConversation;
use crate::services::chat_context::use_chat_context;

use session_list::SessionList;
use npc_list::NpcList;
use active_session_workspace::ActiveSessionWorkspace;

pub use session_list::SessionList as SessionListComponent;
pub use npc_list::NpcList as NpcListComponent;
pub use active_session_workspace::ActiveSessionWorkspace as ActiveSessionWorkspaceComponent;

// TASK-016: Combat Tracker exports
pub use combat_tracker::{CombatTracker, CombatStatsBar};
pub use combatant_card::{CombatantCard, CombatantRowCompact};
pub use initiative_list::{InitiativeList, InitiativeOrderSummary};
pub use condition_manager::{ConditionModal, ConditionBadge, ActiveConditionsList};

// TASK-014: Timeline exports
pub use timeline_view::{
    TimelineView, TimelineCompact,
    TimelineEvent, TimelineEventType, EventSeverity,
    TimelineEventTypeExt, EventSeverityExt,
};

// TASK-017: Notes exports
pub use notes_panel::{NotesPanel, SessionNote, NoteCategory};

// Phase 6: Control Panel exports
pub use control_panel::{ControlPanel, ReadAloudBox, StoryBeat, BeatType, QuickRule, PinnedTable};

// Phase 8: Recap exports
pub use recap_viewer::{RecapViewer, SessionRecap, RecapStatus, PCFilter};

// Phase 9: Quick Reference Cards & Cheat Sheets exports
pub use entity_card::{
    EntityCard, EntityCardCompact, EntityHoverPreview,
    NpcCard, LocationCard, ItemCard, PlotCard,
    CardEntityType, DisclosureLevel, RenderedCard, HoverPreview, PinnedCard, QuickStat,
};
pub use card_tray::{CardTrayPanel, FloatingCardTray, MiniCardTray, CardTray, MAX_PINNED_CARDS};
pub use cheat_sheet_viewer::{
    CheatSheetViewer, FloatingCheatSheet,
    CheatSheet, CheatSheetSection, CheatSheetItem, SectionType, TruncationWarning,
};

// Phase 8: Thread Tabs & Session Chat exports
pub use thread_tabs::{ThreadTabs, ThreadIndicator};
pub use session_chat_panel::SessionChatPanel;

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

    // Get chat context for campaign-aware AI chat
    let chat_ctx = use_chat_context();

    // State signals
    let campaign = RwSignal::new(Option::<Campaign>::None);
    let sessions = RwSignal::new(Vec::<SessionSummary>::new());
    let active_session = RwSignal::new(Option::<GameSession>::None);
    let selected_session_id = RwSignal::new(Option::<String>::None);
    let is_loading = RwSignal::new(true);

    // NPC Selection State
    let selected_npc_id = RwSignal::new(Option::<String>::None);
    let selected_npc_name = RwSignal::new(Option::<String>::None);

    // Initial data load effect - also loads chat context for AI
    Effect::new(move |_| {
        let cid = campaign_id_memo.get();
        if cid.is_empty() {
            is_loading.set(false);
            return;
        }

        // Load campaign context for AI chat (NPCs, locations, etc.)
        chat_ctx.set_campaign(cid.clone());

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

    // Clear chat context when leaving session workspace
    on_cleanup(move || {
        chat_ctx.clear();
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
                                                        // Session number derived from sessions list instead of campaign stats
                                                        let sess_num = sessions.get().iter().map(|s| s.session_number).max().unwrap_or(0) + 1;
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
