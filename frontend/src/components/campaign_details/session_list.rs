use dioxus::prelude::*;
use crate::bindings::SessionSummary;

#[derive(Props, Clone, PartialEq)]
pub struct SessionListProps {
    pub sessions: Vec<SessionSummary>,
    pub active_session_id: Option<String>,
    pub on_select_session: EventHandler<String>,
}

#[component]
pub fn SessionList(props: SessionListProps) -> Element {
    // Mocking status logic since it's missing from backend
    // In real app, we'd sort these.

    // TODO [BE B5]: Replace this mock grouping logic with backend status field
    // Currently using session_number heuristic - backend should return SessionSummary.status
    // See tasks.md for proper implementation plan

    let mut past_sessions = vec![];
    let mut current_session = None;
    let mut planned_sessions = vec![];

    let max_sess_num = props.sessions.iter().map(|s| s.session_number).max().unwrap_or(0);

    for s in &props.sessions {
        if s.session_number == max_sess_num {
            current_session = Some(s);
        } else {
            past_sessions.push(s);
        }
    }

    // Mock a planned session - TODO: Remove when backend supports Planned status
    let planned_mock = SessionSummary {
        id: "planned-1".to_string(),
        session_number: max_sess_num + 1,
        duration_mins: 0,
        combat_count: 0
    };
    planned_sessions.push(&planned_mock);

    rsx! {
        div {
            class: "flex flex-col h-full bg-zinc-900 border-r border-zinc-800 w-64",

            // Header
            div { class: "p-4 border-b border-zinc-800",
                h2 { class: "text-zinc-400 text-xs font-bold uppercase tracking-wider", "Sessions" }
            }

            // Lists
            div { class: "flex-1 overflow-y-auto p-2 space-y-6",

                // Current
                if let Some(curr) = current_session {
                    let curr_id = curr.id.clone();
                    let sess_num = curr.session_number;
                    rsx! {
                        div {
                            div { class: "px-2 mb-2 flex items-center gap-2",
                                div { class: "w-2 h-2 rounded-full bg-green-500 animate-pulse" }
                                span { class: "text-zinc-500 text-xs font-semibold", "CURRENT" }
                            }
                            button {
                                class: "bg-zinc-800/50 border border-purple-500/30 rounded p-3 cursor-pointer hover:bg-zinc-800 transition-colors w-full text-left",
                                onclick: move |_| props.on_select_session.call(curr_id.clone()),
                                div { class: "text-sm font-bold text-white", "Session {sess_num}" }
                                div { class: "text-xs text-zinc-400 mt-1", "Active Now" }
                            }
                        }
                    }
                }

                // Planned
                div {
                    div { class: "px-2 mb-2 text-zinc-500 text-xs font-semibold", "PLANNED" }
                    for s in planned_sessions {
                        div {
                            class: "group flex items-center gap-3 px-2 py-2 rounded text-zinc-400 hover:text-white hover:bg-zinc-800/50 cursor-pointer border border-transparent hover:border-zinc-700 border-dashed",
                             div { class: "text-sm font-medium", "Session {s.session_number}" }
                        }
                    }
                }

                // Past
                 div {
                    div { class: "px-2 mb-2 text-zinc-500 text-xs font-semibold", "HISTORY" }
                    for s in past_sessions {
                         let s_id = s.id.clone();
                         let sess_num = s.session_number;
                         let duration = s.duration_mins;
                         rsx! {
                             button {
                                class: "group flex items-center justify-between px-2 py-2 rounded text-zinc-400 hover:text-white hover:bg-zinc-800/50 cursor-pointer w-full text-left",
                                onclick: move |_| props.on_select_session.call(s_id.clone()),
                                div { class: "text-sm", "Session {sess_num}" }
                                div { class: "text-xs text-zinc-600", "{duration}m" }
                            }
                        }
                    }
                }
            }
        }
    }
}
