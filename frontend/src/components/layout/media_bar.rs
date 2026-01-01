use dioxus::prelude::*;

#[component]
pub fn MediaBar() -> Element {
    // Media Bar skeleton
    // In future phases, this will hook into an AudioState service
    rsx! {
        div {
            class: "h-full w-full flex items-center justify-between px-4 text-[var(--text-muted)]",

            // Left: Player Controls
            div { class: "flex items-center gap-4",
                button { class: "hover:text-[var(--text-primary)]", "⏮" }
                button { class: "w-8 h-8 rounded-full bg-[var(--text-primary)] text-[var(--bg-deep)] flex items-center justify-center hover:scale-105 transition-transform", "▶" }
                button { class: "hover:text-[var(--text-primary)]", "⏭" }
                span { class: "text-xs font-mono", "0:00 / 0:00" }
            }

            // Center: Scrubber (Visual only)
            div { class: "flex-1 mx-8 h-1 bg-[var(--bg-surface)] rounded-full overflow-hidden",
                div { class: "w-1/3 h-full bg-[var(--accent)]" }
            }

            // Right: Character Speaking / Volume
            div { class: "flex items-center gap-4",
                div { class: "flex items-center gap-2 px-3 py-1 bg-[var(--bg-surface)] rounded-full border border-[var(--border-subtle)]",
                     div { class: "w-2 h-2 rounded-full bg-green-500 animate-pulse" }
                     span { class: "text-xs font-bold text-[var(--text-primary)]", "SYSTEM" }
                }
                button { "🔊" }
            }
        }
    }
}
