use dioxus::prelude::*;
use crate::services::layout_service::{LayoutState, ViewType};

#[component]
pub fn IconRail() -> Element {
    let mut layout = use_context::<LayoutState>();
    let active = *layout.active_view.read();

    rsx! {
        div {
            class: "h-full w-full flex flex-col items-center py-4 gap-4 bg-[var(--bg-deep)] border-r border-[var(--border-subtle)]",

            // Logo / Home
            div { class: "mb-4",
                div { class: "w-10 h-10 rounded-full bg-gradient-to-br from-purple-500 to-blue-600 flex items-center justify-center text-white font-bold",
                    "A"
                }
            }

            // Nav Items
            RailIcon {
                active: active == ViewType::Campaigns,
                icon: "📚",
                label: "Campaigns",
                onclick: move |_| layout.active_view.set(ViewType::Campaigns)
            }
            RailIcon {
                active: active == ViewType::Chat,
                icon: "💬",
                label: "Chat",
                onclick: move |_| layout.active_view.set(ViewType::Chat)
            }
            RailIcon {
                active: active == ViewType::Library,
                icon: "🧠",
                label: "Library",
                onclick: move |_| layout.active_view.set(ViewType::Library)
            }
             RailIcon {
                active: active == ViewType::Graph,
                icon: "🔮",
                label: "Graph",
                onclick: move |_| layout.active_view.set(ViewType::Graph)
            }

            div { class: "flex-1" } // Spacer

            RailIcon {
                active: active == ViewType::Settings,
                icon: "⚙️",
                label: "Settings",
                onclick: move |_| layout.active_view.set(ViewType::Settings)
            }
        }
    }
}

#[component]
fn RailIcon(active: bool, icon: &'static str, label: &'static str, onclick: EventHandler<()>) -> Element {
    let active_class = if active { "text-[var(--accent)] bg-[var(--bg-surface)] border-l-2 border-[var(--accent)]" } else { "text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface)]" };

    rsx! {
        div {
            class: "group relative w-full h-12 flex items-center justify-center cursor-pointer transition-colors {active_class}",
            onclick: move |_| onclick.call(()),

            span { class: "text-xl", "{icon}" }

            // Tooltip
            div {
                class: "absolute left-14 top-2 bg-[var(--bg-elevated)] text-[var(--text-primary)] text-xs px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap border border-[var(--border-subtle)] z-50 pointer-events-none",
                "{label}"
            }
        }
    }
}
