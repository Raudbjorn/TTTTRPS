use leptos::prelude::*;
use leptos::ev;
use crate::components::design_system::Select;

/// Mock personality data structure
#[derive(Clone, PartialEq)]
struct MockPersonality {
    id: String,
    name: String,
    voice_provider: String,
    source_doc: Option<String>,
    avatar_color: String,
}

/// Personality Manager component for managing NPC voice and behavior profiles
#[component]
pub fn PersonalityManager() -> impl IntoView {
    // Mock Data
    let personalities = RwSignal::new(vec![
        MockPersonality {
            id: "1".into(),
            name: "Narrator (Dark)".into(),
            voice_provider: "ElevenLabs".into(),
            source_doc: None,
            avatar_color: "bg-purple-900".into(),
        },
        MockPersonality {
            id: "2".into(),
            name: "Shopkeeper".into(),
            voice_provider: "OpenAI".into(),
            source_doc: Some("prices.pdf".into()),
            avatar_color: "bg-yellow-900".into(),
        },
        MockPersonality {
            id: "3".into(),
            name: "Goblin King".into(),
            voice_provider: "ElevenLabs".into(),
            source_doc: Some("goblin_tactics.md".into()),
            avatar_color: "bg-green-900".into(),
        },
        MockPersonality {
            id: "4".into(),
            name: "Gladiator".into(),
            voice_provider: "FishAudio".into(),
            source_doc: Some("arena_stats.txt".into()),
            avatar_color: "bg-red-900".into(),
        },
    ]);

    let selected_id = RwSignal::new(Option::<String>::None);
    let is_editing = RwSignal::new(false);

    view! {
        <div class="flex flex-col h-full bg-zinc-900 text-zinc-100 p-8">
            // Header
            <div class="flex justify-between items-end mb-8">
                <div>
                    <h1 class="text-4xl font-bold bg-clip-text text-transparent bg-gradient-to-r from-purple-400 to-pink-600">
                        "Personalities"
                    </h1>
                    <p class="text-zinc-400 mt-2">"Manage voices and behavior profiles for your NPCs."</p>
                </div>
                <button class="px-6 py-2 bg-zinc-100 text-zinc-900 rounded-full font-bold hover:scale-105 transition-transform">
                    "Create New"
                </button>
            </div>

            // Grid Layout (Spotify Style)
            <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-6">
                <For
                    each=move || personalities.get()
                    key=|p| p.id.clone()
                    children=move |p| {
                        let p_id_edit = p.id.clone();
                        let p_id_play = p.id.clone();
                        let p_name = p.name.clone();
                        let p_name_display = p.name.clone();
                        let p_avatar_color = p.avatar_color.clone();
                        let p_voice_provider = p.voice_provider.clone();
                        let p_source_doc = p.source_doc.clone();
                        let p_initial = p.name.chars().next().unwrap_or('?');

                        view! {
                            <div class="group bg-zinc-800/40 p-4 rounded-lg hover:bg-zinc-800 transition-all relative">
                                // "Album Art" with action buttons
                                <div class=format!("aspect-square w-full {} rounded shadow-lg mb-4 flex items-center justify-center text-4xl font-bold text-white/20 group-hover:shadow-xl transition-shadow relative", p_avatar_color)>
                                    {p_initial.to_string()}
                                    // Action buttons overlay
                                    <div class="absolute bottom-2 right-2 flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
                                        // Play button
                                        <button
                                            class="w-10 h-10 bg-green-500 rounded-full flex items-center justify-center shadow-lg text-black hover:scale-110 transition-transform"
                                            aria-label=format!("Play voice sample for {}", p_name)
                                            on:click=move |evt: ev::MouseEvent| {
                                                evt.stop_propagation();
                                                // TODO: Trigger voice playback
                                            }
                                        >
                                            {"\u{25B6}"}
                                        </button>
                                        // Edit button
                                        <button
                                            class="w-10 h-10 bg-zinc-600 rounded-full flex items-center justify-center shadow-lg text-white hover:scale-110 transition-transform"
                                            aria-label="Edit personality"
                                            on:click={
                                                let id = p_id_edit.clone();
                                                move |_| {
                                                    selected_id.set(Some(id.clone()));
                                                    is_editing.set(true);
                                                }
                                            }
                                        >
                                            {"\u{270E}"}
                                        </button>
                                    </div>
                                </div>

                                // Meta - clickable to edit
                                <button
                                    class="w-full text-left"
                                    on:click={
                                        let id = p_id_play.clone();
                                        move |_| {
                                            selected_id.set(Some(id.clone()));
                                            is_editing.set(true);
                                        }
                                    }
                                >
                                    <div class="font-bold text-white truncate">{p_name_display}</div>
                                    <div class="text-sm text-zinc-500">{p_voice_provider}</div>
                                    {p_source_doc.map(|doc| view! {
                                        <div class="text-xs text-zinc-600 mt-1 flex items-center gap-1">
                                            <span>{"\u{1F4C4}"}</span>
                                            {doc}
                                        </div>
                                    })}
                                </button>
                            </div>
                        }
                    }
                />
            </div>

            // Edit Modal (Overlay)
            <Show
                when=move || is_editing.get()
                fallback=|| ()
            >
                <div class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50">
                    <div class="bg-zinc-900 w-full max-w-2xl rounded-xl border border-zinc-800 shadow-2xl overflow-hidden">
                        // Modal Header
                        <div class="h-32 bg-gradient-to-br from-purple-900 to-zinc-900 p-8 flex items-end">
                            <h2 class="text-3xl font-bold">"Edit Personality"</h2>
                        </div>
                        // Body
                        <div class="p-8 space-y-6">
                            <div>
                                <label class="block text-sm font-bold text-zinc-400 mb-2">"Name"</label>
                                <input
                                    class="w-full bg-zinc-800 border-zinc-700 rounded p-3 focus:ring-2 ring-purple-500 outline-none"
                                    value="Narrator (Dark)"
                                />
                            </div>

                            <div>
                                <label class="block text-sm font-bold text-zinc-400 mb-2">"Voice Provider"</label>
                                <Select value="ElevenLabs".to_string()>
                                    <option value="ElevenLabs">"ElevenLabs"</option>
                                    <option value="OpenAI">"OpenAI"</option>
                                </Select>
                            </div>

                            <div>
                                <label class="block text-sm font-bold text-zinc-400 mb-2">"Source Knowledge (RAG)"</label>
                                <div class="flex gap-2">
                                    <input
                                        class="flex-1 bg-zinc-800 border-zinc-700 rounded p-3 text-zinc-500"
                                        value="No document selected"
                                        disabled=true
                                    />
                                    <button class="px-4 bg-zinc-700 hover:bg-zinc-600 rounded font-medium">
                                        "Browse Library"
                                    </button>
                                </div>
                                <p class="text-xs text-zinc-500 mt-1">
                                    "Link a PDF or Markdown file to ground this personality's responses."
                                </p>
                            </div>
                        </div>
                        // Footer
                        <div class="p-6 bg-zinc-950/50 flex justify-end gap-3">
                            <button
                                class="px-6 py-2 text-zinc-400 hover:text-white font-bold"
                                on:click=move |_| is_editing.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                class="px-6 py-2 bg-white text-black rounded-full font-bold hover:scale-105 transition-transform"
                                on:click=move |_| is_editing.set(false)
                            >
                                "Save Changes"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
