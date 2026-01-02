use leptos::prelude::*;
use crate::bindings::SessionSummary;

/// Session list component showing current, planned, and past sessions
#[component]
pub fn SessionList(
    /// List of session summaries
    sessions: Vec<SessionSummary>,
    /// Currently active session ID
    #[prop(optional)]
    _active_session_id: Option<String>,
    /// Callback when a session is selected
    on_select_session: Callback<String>,
) -> impl IntoView {
    // Mocking status logic since it's missing from backend
    // TODO [BE B5]: Replace this mock grouping logic with backend status field
    // Currently using session_number heuristic - backend should return SessionSummary.status

    let max_sess_num = sessions.iter().map(|s| s.session_number).max().unwrap_or(0);

    let current_session = sessions
        .iter()
        .find(|s| s.session_number == max_sess_num)
        .cloned();

    let past_sessions: Vec<_> = sessions
        .iter()
        .filter(|s| s.session_number != max_sess_num)
        .cloned()
        .collect();

    // Mock a planned session - TODO: Remove when backend supports Planned status
    let planned_session = SessionSummary {
        id: "planned-1".to_string(),
        session_number: max_sess_num + 1,
        duration_mins: 0,
        combat_count: 0,
    };

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
                    current_session.clone().map(|curr| {
                        let curr_id = curr.id.clone();
                        let sess_num = curr.session_number;
                        let on_click = on_select_session.clone();

                        view! {
                            <div>
                                <div class="px-2 mb-2 flex items-center gap-2">
                                    <div class="w-2 h-2 rounded-full bg-green-500 animate-pulse"></div>
                                    <span class="text-zinc-500 text-xs font-semibold">"CURRENT"</span>
                                </div>
                                <button
                                    class="bg-zinc-800/50 border border-purple-500/30 rounded p-3 cursor-pointer hover:bg-zinc-800 transition-colors w-full text-left"
                                    on:click=move |_| on_click.run(curr_id.clone())
                                >
                                    <div class="text-sm font-bold text-white">
                                        {format!("Session {}", sess_num)}
                                    </div>
                                    <div class="text-xs text-zinc-400 mt-1">"Active Now"</div>
                                </button>
                            </div>
                        }
                    })
                }}

                // Planned Sessions
                <div>
                    <div class="px-2 mb-2 text-zinc-500 text-xs font-semibold">"PLANNED"</div>
                    <div class="group flex items-center gap-3 px-2 py-2 rounded text-zinc-400 hover:text-white hover:bg-zinc-800/50 cursor-pointer border border-transparent hover:border-zinc-700 border-dashed">
                        <div class="text-sm font-medium">
                            {format!("Session {}", planned_session.session_number)}
                        </div>
                    </div>
                </div>

                // Past Sessions (History)
                <div>
                    <div class="px-2 mb-2 text-zinc-500 text-xs font-semibold">"HISTORY"</div>
                    <For
                        each=move || past_sessions.clone()
                        key=|s| s.id.clone()
                        children=move |s| {
                            let s_id = s.id.clone();
                            let sess_num = s.session_number;
                            let duration = s.duration_mins;
                            let on_click = on_select_session.clone();

                            view! {
                                <button
                                    class="group flex items-center justify-between px-2 py-2 rounded text-zinc-400 hover:text-white hover:bg-zinc-800/50 cursor-pointer w-full text-left"
                                    on:click=move |_| on_click.run(s_id.clone())
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
