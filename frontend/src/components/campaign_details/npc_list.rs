use dioxus::prelude::*;
use crate::bindings::{list_npc_summaries, NpcSummary};

#[derive(Props, Clone, PartialEq)]
pub struct NPCListProps {
    pub campaign_id: String,
    #[props(default)]
    pub selected_npc_id: Option<String>,
    #[props(default)]
    pub on_select_npc: EventHandler<String>,
}

#[component]
pub fn NPCList(props: NPCListProps) -> Element {
    let campaign_id = props.campaign_id.clone();

    // Fetch NPCs from backend
    let npcs_resource = use_resource(move || {
        let cid = campaign_id.clone();
        async move {
            list_npc_summaries(cid).await.unwrap_or_default()
        }
    });

    rsx! {
        div {
            class: "flex flex-col h-full bg-zinc-950 border-l border-zinc-900 w-72 flex-shrink-0",

            // Header
            div { class: "p-4 border-b border-zinc-900",
                div { class: "flex justify-between items-center mb-2",
                    h2 { class: "text-zinc-400 text-xs font-bold uppercase tracking-wider", "Direct Messages" }
                    button {
                        class: "w-6 h-6 rounded hover:bg-zinc-800 flex items-center justify-center text-zinc-500 hover:text-white transition-colors",
                        aria_label: "New Message",
                        "+"
                    }
                }

                // Search (Visual only for now)
                div { class: "relative",
                    input {
                        class: "w-full bg-zinc-900 border border-zinc-800 rounded px-3 py-1.5 text-xs text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-700",
                        placeholder: "Find a character..."
                    }
                }
            }

            // List
            div { class: "flex-1 overflow-y-auto p-2 space-y-1",
                match &*npcs_resource.read_unchecked() {
                    Some(list) => {
                        if list.is_empty() {
                            rsx! {
                                div { class: "p-4 text-center text-zinc-600 text-xs italic",
                                    "No NPCs found."
                                }
                            }
                        } else {
                            rsx! {
                                for npc in list {
                                    NpcListItem {
                                        key: "{npc.id}",
                                        npc: npc.clone(),
                                        is_selected: props.selected_npc_id.as_ref() == Some(&npc.id),
                                        on_click: props.on_select_npc
                                    }
                                }
                            }
                        }
                    }
                    None => rsx! {
                        div { class: "p-4 text-center text-zinc-600 text-xs",
                            "Loading..."
                        }
                    }
                }
            }

            // Footer
            div { class: "p-2 border-t border-zinc-900 text-[10px] text-center text-zinc-700",
                "Synced with Neural Link"
            }
        }
    }
}

#[component]
fn NpcListItem(
    npc: NpcSummary,
    is_selected: bool,
    on_click: EventHandler<String>
) -> Element {
    let base_class = if is_selected {
        "flex items-center gap-3 p-2 rounded bg-blue-900/20 w-full text-left relative overflow-hidden"
    } else {
        "flex items-center gap-3 p-2 rounded hover:bg-zinc-900 transition-colors w-full text-left relative group"
    };

    let status_color = match npc.status.as_str() {
        "online" => "bg-green-500",
        "away" => "bg-yellow-500",
        _ => "bg-zinc-500"
    };

    let id_click = npc.id.clone();

    rsx! {
        button {
            class: "{base_class}",
            onclick: move |_| on_click.call(id_click.clone()),

            if is_selected {
                div { class: "absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-6 bg-blue-500 rounded-r" }
            }

            // Avatar with Status Bubble
            div { class: "relative",
                div {
                    class: "w-9 h-9 rounded-md bg-zinc-800 flex items-center justify-center text-sm font-bold text-zinc-400 border border-zinc-700",
                    "{npc.avatar_url}"
                }
                div {
                    class: "absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-zinc-950 {status_color}"
                }
            }

            // Info
            div { class: "flex-1 min-w-0",
                div { class: "flex justify-between items-baseline",
                    div {
                        class: if npc.unread_count > 0 { "text-sm font-bold text-white truncate" } else { "text-sm font-medium text-zinc-300 truncate" },
                        "{npc.name}"
                    }
                    if !npc.last_active.is_empty() {
                         span { class: "text-[10px] text-zinc-500 font-mono ml-2", "{format_time_short(&npc.last_active)}" }
                    }
                }
                div { class: "flex justify-between items-center",
                    p {
                        class: if npc.unread_count > 0 { "text-xs text-zinc-300 truncate font-medium" } else { "text-xs text-zinc-500 truncate" },
                        "{npc.last_message}"
                    }
                    if npc.unread_count > 0 {
                        div { class: "px-1.5 py-0.5 min-w-[1.25rem] bg-indigo-600 rounded-full text-[10px] font-bold text-white text-center ml-2",
                            "{npc.unread_count}"
                        }
                    }
                }
            }
        }
    }
}

fn format_time_short(iso: &str) -> String {
    if let Some(time_part) = iso.split('T').nth(1) {
        if let Some(hm) = time_part.get(0..5) {
            return hm.to_string();
        }
    }
    "" .to_string()
}
