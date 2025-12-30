use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct NPCListProps {
    pub campaign_id: String,
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
    // Mock Data for now
    let npcs = use_signal(|| vec![
        MockNPC { id: "1".into(), name: "Garrosh".into(), role: "Blacksmith".into(), avatar_url: "G".into() },
        MockNPC { id: "2".into(), name: "Elara".into(), role: "Quest Giver".into(), avatar_url: "E".into() },
        MockNPC { id: "3".into(), name: "Zoltan".into(), role: "Villain".into(), avatar_url: "Z".into() },
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
               for npc in npcs.read().clone() {
                   button {
                       class: "flex items-center gap-3 p-2 rounded hover:bg-zinc-800 transition-colors cursor-pointer group w-full text-left",
                       // Avatar Mock
                       div {
                           class: "w-8 h-8 rounded bg-zinc-700 flex items-center justify-center text-xs font-bold text-zinc-300 border border-zinc-600 group-hover:border-zinc-500",
                           "{npc.avatar_url}"
                       }
                       // Info
                       div {
                           div { class: "text-sm font-medium text-zinc-300 group-hover:text-white", "{npc.name}" }
                           div { class: "text-xs text-zinc-500", "{npc.role}" }
                       }
                   }
               }
            }
        }
    }
}
