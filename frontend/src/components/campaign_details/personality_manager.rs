use dioxus::prelude::*;
use crate::components::design_system::Select;

#[derive(Clone, PartialEq)]
struct MockPersonality {
    id: String,
    name: String,
    voice_provider: String,
    source_doc: Option<String>,
    avatar_color: String,
}

#[component]
pub fn PersonalityManager() -> Element {
    // Mock Data
    let personalities = use_signal(|| vec![
        MockPersonality {
            id: "1".into(), name: "Narrator (Dark)".into(),
            voice_provider: "ElevenLabs".into(), source_doc: None,
            avatar_color: "bg-purple-900".into()
        },
        MockPersonality {
            id: "2".into(), name: "Shopkeeper".into(),
            voice_provider: "OpenAI".into(), source_doc: Some("prices.pdf".into()),
            avatar_color: "bg-yellow-900".into()
        },
        MockPersonality {
            id: "3".into(), name: "Goblin King".into(),
            voice_provider: "ElevenLabs".into(), source_doc: Some("goblin_tactics.md".into()),
            avatar_color: "bg-green-900".into()
        },
        MockPersonality {
            id: "4".into(), name: "Gladiator".into(),
            voice_provider: "FishAudio".into(), source_doc: Some("arena_stats.txt".into()),
            avatar_color: "bg-red-900".into()
        },
    ]);

    let mut selected_id = use_signal(|| Option::<String>::None);
    let mut is_editing = use_signal(|| false);

    rsx! {
        div { class: "flex flex-col h-full bg-zinc-900 text-zinc-100 p-8",
            // Header
            div { class: "flex justify-between items-end mb-8",
                div {
                    h1 { class: "text-4xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-purple-400 to-pink-600", "Personalities" }
                    p { class: "text-zinc-400 mt-2", "Manage voices and behavior profiles for your NPCs." }
                }
                button {
                    class: "px-6 py-2 bg-zinc-100 text-zinc-900 rounded-full font-bold hover:scale-105 transition-transform",
                    "Create New"
                }
            }

            // Grid Layout (Spotify Style)
            div { class: "grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6",
                {personalities.read().iter().map(|p| {
                    let p_id_edit = p.id.clone();
                    let p_id_play = p.id.clone();
                    let p_name = p.name.clone();
                    let p_name_display = p.name.clone();
                    let p_avatar_color = p.avatar_color.clone();
                    let p_voice_provider = p.voice_provider.clone();
                    let p_source_doc = p.source_doc.clone();
                    let p_initial = p.name.chars().next().unwrap_or('?');
                    rsx! {
                        div {
                            key: "{p_id_edit}",
                            class: "group bg-zinc-800/40 p-4 rounded-lg hover:bg-zinc-800 transition-all relative",

                            // "Album Art" with action buttons
                            div { class: "aspect-square w-full {p_avatar_color} rounded shadow-lg mb-4 flex items-center justify-center text-4xl font-bold text-white/20 group-hover:shadow-xl transition-shadow relative",
                                "{p_initial}"
                                // Action buttons overlay - separate from card click
                                div { class: "absolute bottom-2 right-2 flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity",
                                    // Play button
                                    button {
                                        class: "w-10 h-10 bg-green-500 rounded-full flex items-center justify-center shadow-lg text-black hover:scale-110 transition-transform",
                                        aria_label: "Play voice sample for {p_name}",
                                        onclick: move |evt| {
                                            evt.stop_propagation();
                                            // TODO: Trigger voice playback
                                        },
                                        "▶"
                                    }
                                    // Edit button
                                    button {
                                        class: "w-10 h-10 bg-zinc-600 rounded-full flex items-center justify-center shadow-lg text-white hover:scale-110 transition-transform",
                                        aria_label: "Edit personality",
                                        onclick: move |_| {
                                            selected_id.set(Some(p_id_edit.clone()));
                                            is_editing.set(true);
                                        },
                                        "✎"
                                    }
                                }
                            }

                            // Meta - clickable to edit
                            button {
                                class: "w-full text-left",
                                onclick: move |_| { selected_id.set(Some(p_id_play.clone())); is_editing.set(true); },
                                div { class: "font-bold text-white truncate", "{p_name_display}" }
                                div { class: "text-sm text-zinc-500", "{p_voice_provider}" }
                                if let Some(doc) = &p_source_doc {
                                     div { class: "text-xs text-zinc-600 mt-1 flex items-center gap-1",
                                        span { "📄" }
                                        "{doc}"
                                     }
                                }
                            }
                        }
                    }
                })}
            }
            }

            // Edit Modal (Overlay)
            if is_editing.read().clone() {
                div { class: "fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50",
                    div { class: "bg-zinc-900 w-full max-w-2xl rounded-xl border border-zinc-800 shadow-2xl overflow-hidden",
                        // Modal Header
                        div { class: "h-32 bg-gradient-to-br from-purple-900 to-zinc-900 p-8 flex items-end",
                             h2 { class: "text-3xl font-bold", "Edit Personality" }
                        }
                        // Body
                        div { class: "p-8 space-y-6",
                             div {
                                 label { class: "block text-sm font-bold text-zinc-400 mb-2", "Name" }
                                 input { class: "w-full bg-zinc-800 border-zinc-700 rounded p-3 focus:ring-2 ring-purple-500 outline-none", value: "Narrator (Dark)" } // Mock binding
                             }

                             div {
                                 label { class: "block text-sm font-bold text-zinc-400 mb-2", "Voice Provider" }
                                 Select {
                                     value: "ElevenLabs",
                                     option { value: "ElevenLabs", "ElevenLabs" }
                                     option { value: "OpenAI", "OpenAI" }
                                 }
                             }

                             div {
                                 label { class: "block text-sm font-bold text-zinc-400 mb-2", "Source Knowledge (RAG)" }
                                 div { class: "flex gap-2",
                                     input { class: "flex-1 bg-zinc-800 border-zinc-700 rounded p-3 text-zinc-500", value: "No document selected", disabled: true }
                                     button { class: "px-4 bg-zinc-700 hover:bg-zinc-600 rounded font-medium", "Browse Library" }
                                 }
                                 p { class: "text-xs text-zinc-500 mt-1", "Link a PDF or Markdown file to ground this personality's responses." }
                             }
                        }
                        // Footer
                        div { class: "p-6 bg-zinc-950/50 flex justify-end gap-3",
                            button {
                                class: "px-6 py-2 text-zinc-400 hover:text-white font-bold",
                                onclick: move |_| is_editing.set(false),
                                "Cancel"
                            }
                            button {
                                class: "px-6 py-2 bg-white text-black rounded-full font-bold hover:scale-105 transition-transform",
                                onclick: move |_| is_editing.set(false),
                                "Save Changes"
                            }
                        }
                    }
                }
            }
        }
    }

