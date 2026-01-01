use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct NPCListProps {
    pub campaign_id: String,
    #[props(default)]
    pub selected_npc_id: Option<String>,
    #[props(default)]
    pub on_select_npc: EventHandler<String>,
}

#[derive(Clone, PartialEq)]
struct MockNPC {
    id: String,
    name: String,
    role: String,
    avatar_url: String,
    status: String, // "online", "away", "offline"
    last_message: String,
    unread_count: usize,
    last_active: String,
}

#[component]
pub fn NPCList(props: NPCListProps) -> Element {
    let _campaign_id = &props.campaign_id;

    // Mock Data - Enhanced for Slack-style DM list
    let npcs = use_signal(|| vec![
        MockNPC {
            id: "npc-1".into(),
            name: "Garrosh".into(),
            role: "Blacksmith".into(),
            avatar_url: "G".into(),
            status: "online".into(),
            last_message: "The armor will be ready by dawn.".into(),
            unread_count: 2,
            last_active: "now".into()
        },
        MockNPC {
            id: "npc-2".into(),
            name: "Elara".into(),
            role: "Quest Giver".into(),
            avatar_url: "E".into(),
            status: "away".into(),
            last_message: "Have you found the artifact?".into(),
            unread_count: 0,
            last_active: "5m ago".into()
        },
        MockNPC {
            id: "npc-3".into(),
            name: "Zoltan".into(),
            role: "Villain".into(),
            avatar_url: "Z".into(),
            status: "offline".into(),
            last_message: "You will never stop me.".into(),
            unread_count: 1,
            last_active: "1d ago".into()
        },
        MockNPC {
            id: "npc-4".into(),
            name: "Mayor Toben".into(),
            role: "Official".into(),
            avatar_url: "M".into(),
            status: "online".into(),
            last_message: "Taxes are due.".into(),
            unread_count: 0,
            last_active: "10m ago".into()
        }
    ]);

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

                // Search Mock
                div { class: "relative",
                    input {
                        class: "w-full bg-zinc-900 border border-zinc-800 rounded px-3 py-1.5 text-xs text-white placeholder-zinc-600 focus:outline-none focus:border-zinc-700",
                        placeholder: "Find a character..."
                    }
                }
            }

            // List
            div { class: "flex-1 overflow-y-auto p-2 space-y-1",
               for npc in npcs.read().clone() {{
                   let npc_id = npc.id.clone();
                   let npc_id_click = npc.id.clone();
                   let npc_name = npc.name.clone();
                   // let npc_role = npc.role.clone();
                   let npc_avatar = npc.avatar_url.clone();
                   let status = npc.status.clone();
                   let last_msg = npc.last_message.clone();
                   let unread = npc.unread_count;
                   let _time = npc.last_active.clone(); // Could show timestamp

                   let is_selected = props.selected_npc_id.as_ref() == Some(&npc_id);

                   let base_class = if is_selected {
                       "flex items-center gap-3 p-2 rounded bg-blue-900/20 w-full text-left relative overflow-hidden"
                   } else {
                       "flex items-center gap-3 p-2 rounded hover:bg-zinc-900 transition-colors w-full text-left relative group"
                   };

                   let status_color = match status.as_str() {
                       "online" => "bg-green-500",
                       "away" => "bg-yellow-500",
                       _ => "bg-zinc-500"
                   };

                   rsx! {
                       button {
                           key: "{npc_id}",
                           class: "{base_class}",
                           onclick: move |_| props.on_select_npc.call(npc_id_click.clone()),

                           if is_selected {
                               div { class: "absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-6 bg-blue-500 rounded-r" }
                           }

                           // Avatar with Status Bubble
                           div { class: "relative",
                               div {
                                   class: "w-9 h-9 rounded-md bg-zinc-800 flex items-center justify-center text-sm font-bold text-zinc-400 border border-zinc-700",
                                   "{npc_avatar}"
                               }
                               div {
                                   class: "absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full border-2 border-zinc-950 {status_color}"
                               }
                           }

                           // Info
                           div { class: "flex-1 min-w-0",
                               div { class: "flex justify-between items-baseline",
                                   div {
                                       class: if unread > 0 { "text-sm font-bold text-white truncate" } else { "text-sm font-medium text-zinc-300 truncate" },
                                       "{npc_name}"
                                   }
                                   if unread > 0 {
                                       span { class: "text-[10px] text-zinc-500 font-mono ml-2", "9:41 AM" }
                                   }
                               }
                               div { class: "flex justify-between items-center",
                                   p {
                                       class: if unread > 0 { "text-xs text-zinc-300 truncate font-medium" } else { "text-xs text-zinc-500 truncate" },
                                       "{last_msg}"
                                   }
                                   if unread > 0 {
                                       div { class: "px-1.5 py-0.5 min-w-[1.25rem] bg-indigo-600 rounded-full text-[10px] font-bold text-white text-center ml-2",
                                           "{unread}"
                                       }
                                   }
                               }
                           }
                       }
                   }
               }}
            }

            // Footer (optional, maybe sync status)
            div { class: "p-2 border-t border-zinc-900 text-[10px] text-center text-zinc-700",
                "Synced with Neural Link"
            }
        }
    }
}
