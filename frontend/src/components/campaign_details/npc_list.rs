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
    avatar_url: String, // could be a color entry or icon char
}

#[component]
pub fn NPCList(props: NPCListProps) -> Element {
    let _campaign_id = &props.campaign_id; // Will be used when backend integration is complete
    // Mock Data for now
    let npcs = use_signal(|| vec![
        MockNPC { id: "npc-1".into(), name: "Garrosh".into(), role: "Blacksmith".into(), avatar_url: "G".into() },
        MockNPC { id: "npc-2".into(), name: "Elara".into(), role: "Quest Giver".into(), avatar_url: "E".into() },
        MockNPC { id: "npc-3".into(), name: "Zoltan".into(), role: "Villain".into(), avatar_url: "Z".into() },
    ]);

    rsx! {
        div {
            class: "flex flex-col h-full bg-zinc-900 border-l border-zinc-800 w-64",

            // Header
            div { class: "p-4 border-b border-zinc-800 flex justify-between items-center",
                h2 { class: "text-zinc-400 text-xs font-bold uppercase tracking-wider", "Dramatis Personae" }
                button {
                    class: "text-zinc-500 hover:text-white transition-colors",
                    aria_label: "Add NPC",
                    "+"
                }
            }

            // List
            div { class: "flex-1 overflow-y-auto p-2 space-y-2",
               for npc in npcs.read().clone() {{
                   let npc_id = npc.id.clone();
                   let npc_id_click = npc.id.clone();
                   let npc_name = npc.name.clone();
                   let npc_role = npc.role.clone();
                   let npc_avatar = npc.avatar_url.clone();
                   let is_selected = props.selected_npc_id.as_ref() == Some(&npc_id);

                   let base_class = if is_selected {
                       "flex items-center gap-3 p-2 rounded bg-[var(--accent)]/20 border border-[var(--accent)]/40 cursor-pointer group w-full text-left"
                   } else {
                       "flex items-center gap-3 p-2 rounded hover:bg-zinc-800 transition-colors cursor-pointer group w-full text-left border border-transparent"
                   };

                   rsx! {
                       button {
                           key: "{npc_id}",
                           class: "{base_class}",
                           onclick: move |_| props.on_select_npc.call(npc_id_click.clone()),
                           // Avatar Mock
                           div {
                               class: if is_selected {
                                   "w-8 h-8 rounded bg-[var(--accent)]/30 flex items-center justify-center text-xs font-bold text-white border border-[var(--accent)]"
                               } else {
                                   "w-8 h-8 rounded bg-zinc-700 flex items-center justify-center text-xs font-bold text-zinc-300 border border-zinc-600 group-hover:border-zinc-500"
                               },
                               "{npc_avatar}"
                           }
                           // Info
                           div {
                               div {
                                   class: if is_selected { "text-sm font-medium text-white" } else { "text-sm font-medium text-zinc-300 group-hover:text-white" },
                                   "{npc_name}"
                               }
                               div { class: "text-xs text-zinc-500", "{npc_role}" }
                           }
                       }
                   }
               }}
            }
        }
    }
}
