//! Command Palette component for Leptos
//! Global keyboard shortcut (Ctrl+K) activated command palette

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn CommandPalette() -> impl IntoView {
    let is_open = RwSignal::new(false);
    let search_query = RwSignal::new(String::new());

    // Toggle on Ctrl+K or Cmd+K
    Effect::new(move |_| {
        let handle_keydown = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            if (e.ctrl_key() || e.meta_key()) && e.key() == "k" {
                e.prevent_default();
                let current = is_open.get();
                is_open.set(!current);
            }
            if e.key() == "Escape" {
                is_open.set(false);
            }
        }) as Box<dyn FnMut(_)>);

        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback(
                "keydown",
                handle_keydown.as_ref().unchecked_ref(),
            );
        }

        // Keep the closure alive
        handle_keydown.forget();
    });

    view! {
        <Show when=move || is_open.get()>
            <div
                class="fixed inset-0 z-[100] flex items-start justify-center pt-[20vh] bg-black/50 backdrop-blur-sm"
                on:click=move |_| is_open.set(false)
            >
                <div
                    class="w-full max-w-2xl bg-zinc-900 border border-zinc-700 rounded-xl shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 duration-100"
                    on:click=move |e| e.stop_propagation()
                >
                    // Input
                    <div class="flex items-center px-4 py-3 border-b border-zinc-800">
                        <svg
                            class="w-5 h-5 text-zinc-500 mr-3"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                        >
                            <path d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"></path>
                        </svg>
                        <input
                            class="w-full bg-transparent border-none text-lg text-white placeholder-zinc-500 focus:outline-none focus:ring-0"
                            placeholder="Type a command or search..."
                            prop:value=move || search_query.get()
                            on:input=move |e| search_query.set(event_target_value(&e))
                            autofocus=true
                        />
                    </div>

                    // Results (Static for now)
                    <div class="max-h-[60vh] overflow-y-auto p-2">
                        <div class="px-2 py-1 text-xs font-semibold text-zinc-500">"SUGGESTIONS"</div>

                        <button class="w-full text-left px-3 py-2 rounded-md hover:bg-zinc-800 text-zinc-300 flex items-center gap-3">
                            <span class="p-1 bg-zinc-800 rounded bg-blue-500/20 text-blue-400 font-mono text-xs">"NPC"</span>
                            "Generate new NPC"
                        </button>

                        <button class="w-full text-left px-3 py-2 rounded-md hover:bg-zinc-800 text-zinc-300 flex items-center gap-3">
                            <span class="p-1 bg-zinc-800 rounded bg-green-500/20 text-green-400 font-mono text-xs">"SESSION"</span>
                            "Start new session"
                        </button>

                        <button class="w-full text-left px-3 py-2 rounded-md hover:bg-zinc-800 text-zinc-300 flex items-center gap-3">
                            <span class="p-1 bg-zinc-800 rounded bg-purple-500/20 text-purple-400 font-mono text-xs">"THEME"</span>
                            "Change Theme: Cyberpunk"
                        </button>
                    </div>

                    <div class="border-t border-zinc-800 px-4 py-2 flex justify-between items-center text-xs text-zinc-500">
                        <div class="flex gap-4">
                            <span class="flex items-center gap-1">
                                <kbd class="px-1 bg-zinc-800 rounded text-zinc-400">"↑↓"</kbd>
                                " navigate"
                            </span>
                            <span class="flex items-center gap-1">
                                <kbd class="px-1 bg-zinc-800 rounded text-zinc-400">"↵"</kbd>
                                " select"
                            </span>
                        </div>
                        <span class="flex items-center gap-1">
                            <kbd class="px-1 bg-zinc-800 rounded text-zinc-400">"esc"</kbd>
                            " close"
                        </span>
                    </div>
                </div>
            </div>
        </Show>
    }
}
