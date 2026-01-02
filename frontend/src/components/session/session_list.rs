//! Session list sidebar component
//!
//! Displays the list of sessions (past, current, planned) for a campaign

use leptos::prelude::*;

use crate::bindings::SessionSummary;

/// Session list sidebar component
#[component]
pub fn SessionList(
    /// List of all sessions
    sessions: RwSignal<Vec<SessionSummary>>,
    /// ID of the currently active session (if any)
    active_session_id: Signal<Option<String>>,
    /// Callback when a session is selected
    on_select_session: Callback<String>,
) -> impl IntoView {
    // Derive session groupings
    let session_groups = Memo::new(move |_| {
        let all_sessions = sessions.get();
        let active_id = active_session_id.get();

        let max_sess_num = all_sessions.iter().map(|s| s.session_number).max().unwrap_or(0);

        let mut past_sessions: Vec<SessionSummary> = vec![];
        let mut current_session: Option<SessionSummary> = None;

        for s in all_sessions {
            if Some(&s.id) == active_id.as_ref() || s.session_number == max_sess_num {
                current_session = Some(s);
            } else {
                past_sessions.push(s);
            }
        }

        // Mock a planned session
        let planned_sessions = vec![SessionSummary {
            id: "planned-1".to_string(),
            campaign_id: String::new(),
            session_number: max_sess_num + 1,
            started_at: String::new(),
            ended_at: None,
            duration_minutes: Some(0),
            status: "planned".to_string(),
            note_count: 0,
            had_combat: false,
            order_index: 0,
        }];

        (past_sessions, current_session, planned_sessions)
    });

    view! {
        <div class="flex flex-col h-full bg-zinc-900 border-r border-zinc-800 w-64">
            // Header
            <div class="p-4 border-b border-zinc-800">
                <h2 class="text-zinc-400 text-xs font-bold uppercase tracking-wider">"Sessions"</h2>
            </div>

            // Lists
            <div class="flex-1 overflow-y-auto p-2 space-y-6">
                // Current Session
                {move || {
                    let (_, current, _) = session_groups.get();
                    if let Some(curr) = current {
                        let curr_id = curr.id.clone();
                        let sess_num = curr.session_number;
                        Some(view! {
                            <div>
                                <div class="px-2 mb-2 flex items-center gap-2">
                                    <div class="w-2 h-2 rounded-full bg-green-500 animate-pulse"></div>
                                    <span class="text-zinc-500 text-xs font-semibold">"CURRENT"</span>
                                </div>
                                <button
                                    class="bg-zinc-800/50 border border-purple-500/30 rounded p-3 cursor-pointer hover:bg-zinc-800 transition-colors w-full text-left"
                                    on:click=move |_| on_select_session.run(curr_id.clone())
                                >
                                    <div class="text-sm font-bold text-white">
                                        {format!("Session {}", sess_num)}
                                    </div>
                                    <div class="text-xs text-zinc-400 mt-1">"Active Now"</div>
                                </button>
                            </div>
                        })
                    } else {
                        None
                    }
                }}

                // Planned Sessions
                <div>
                    <div class="px-2 mb-2 text-zinc-500 text-xs font-semibold">"PLANNED"</div>
                    <For
                        each=move || session_groups.get().2
                        key=|s| s.id.clone()
                        children=move |s| {
                            view! {
                                <div class="group flex items-center gap-3 px-2 py-2 rounded text-zinc-400 hover:text-white hover:bg-zinc-800/50 cursor-pointer border border-transparent hover:border-zinc-700 border-dashed">
                                    <div class="text-sm font-medium">
                                        {format!("Session {}", s.session_number)}
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>

                // Past Sessions (History)
                <div>
                    <div class="px-2 mb-2 text-zinc-500 text-xs font-semibold">"HISTORY"</div>
                    <For
                        each=move || session_groups.get().0
                        key=|s| s.id.clone()
                        children=move |s| {
                            let s_id = s.id.clone();
                            let sess_num = s.session_number;
                            let duration = s.duration_minutes.unwrap_or(0);
                            view! {
                                <button
                                    class="group flex items-center justify-between px-2 py-2 rounded text-zinc-400 hover:text-white hover:bg-zinc-800/50 cursor-pointer w-full text-left"
                                    on:click=move |_| on_select_session.run(s_id.clone())
                                >
                                    <div class="text-sm">{format!("Session {}", sess_num)}</div>
                                    <div class="text-xs text-zinc-600">{format!("{}m", duration)}</div>
                                </button>
                            }
                        }
                    />
                </div>
            </div>
        </div>
    }
}
