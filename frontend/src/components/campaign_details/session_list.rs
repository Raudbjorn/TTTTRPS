use dioxus::prelude::*;
use crate::bindings::SessionSummary;

#[derive(Props, Clone, PartialEq)]
pub struct SessionListProps {
    pub sessions: Vec<SessionSummary>,
    pub active_session_id: Option<String>,
    pub on_select_session: EventHandler<String>,
    pub on_refresh: EventHandler<()>,
}

#[component]
pub fn SessionList(props: SessionListProps) -> Element {
    use crate::bindings::reorder_session;

    // Partition sessions
    let mut active_session = None;
    let mut planned_sessions = vec![];
    let mut past_sessions = vec![];

    for s in &props.sessions {
        if s.status == "active" {
            active_session = Some(s);
        } else if s.status == "planned" {
            planned_sessions.push(s);
        } else {
             past_sessions.push(s);
        }
    }

    // Sort planned by order_index (should be sorted by backend, but ensure constraint)
    // Actually we iterate over props order. Assuming props sorted.

    let handle_swap = move |s1_id: String, s1_order: i32, s2_id: String, s2_order: i32| {
        spawn(async move {
            // Swap
            let _ = reorder_session(s1_id, s2_order).await;
            let _ = reorder_session(s2_id, s1_order).await;
            props.on_refresh.call(());
        });
    };

    rsx! {
        div {
            class: "flex flex-col h-full bg-zinc-900 border-r border-zinc-800 w-64",

            // Header
            div { class: "p-4 border-b border-zinc-800",
                h2 { class: "text-zinc-400 text-xs font-bold uppercase tracking-wider", "Sessions" }
            }

            // Lists
            div { class: "flex-1 overflow-y-auto p-2 space-y-6",

                // Active
                {if let Some(curr) = active_session {
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
                } else { rsx!({}) }}

                // Planned
                if !planned_sessions.is_empty() {
                    div {
                        div { class: "px-2 mb-2 text-zinc-500 text-xs font-semibold", "PLANNED" }
                        div { class: "space-y-2",
                            for (i, s) in planned_sessions.iter().enumerate() {
                                {
                                    let s_order = s.order_index;
                                    let prev = if i > 0 { Some(planned_sessions[i-1]) } else { None };
                                    let next = if i < planned_sessions.len() - 1 { Some(planned_sessions[i+1]) } else { None };

                                    let prev_info = prev.map(|p| (p.id.clone(), p.order_index));
                                    let next_info = next.map(|n| (n.id.clone(), n.order_index));

                                    let this_id_up = s.id.clone();
                                    let this_id_down = s.id.clone();

                                    rsx! {
                                        div {
                                            class: "group flex items-center justify-between px-2 py-2 rounded text-zinc-400 hover:text-white hover:bg-zinc-800/50 border border-transparent hover:border-zinc-700 border-dashed",
                                            div { class: "flex items-center gap-3",
                                                span { class: "text-sm font-medium", "Session {s.session_number}" }
                                            }
                                            // Controls
                                            div { class: "flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity",
                                                if let Some((p_id, p_order)) = prev_info {
                                                    button {
                                                        class: "p-1 hover:text-purple-400",
                                                        title: "Move Up",
                                                        onclick: move |_| handle_swap(this_id_up.clone(), s_order, p_id.clone(), p_order),
                                                        "↑"
                                                    }
                                                }
                                                if let Some((n_id, n_order)) = next_info {
                                                     button {
                                                        class: "p-1 hover:text-purple-400",
                                                        title: "Move Down",
                                                        onclick: move |_| handle_swap(this_id_down.clone(), s_order, n_id.clone(), n_order),
                                                        "↓"
                                                    }
                                                }
                                                button {
                                                    class: "p-1 hover:text-green-400",
                                                    title: "Start Session",
                                                    "▶"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Past
                 div {
                    div { class: "px-2 mb-2 text-zinc-500 text-xs font-semibold", "HISTORY" }
                    for s in past_sessions {
                         {
                             let s_id = s.id.clone();
                             let sess_num = s.session_number;
                             let duration = s.duration_minutes.unwrap_or(0);
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
}
