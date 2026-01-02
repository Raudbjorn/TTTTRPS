use leptos::prelude::*;
use crate::services::layout_service::{LayoutState, ViewType};

#[component]
pub fn IconRail() -> impl IntoView {
    let layout = expect_context::<LayoutState>();

    // Derived signal for active view
    let active = Signal::derive(move || layout.active_view.get());

    view! {
        <div class="h-full w-full flex flex-col items-center py-4 gap-4 bg-[var(--bg-deep)] border-r border-[var(--border-subtle)]">
            // Logo / Home
            <div class="mb-4">
                <div class="w-10 h-10 rounded-full bg-gradient-to-br from-purple-500 to-blue-600 flex items-center justify-center text-white font-bold">
                    "A"
                </div>
            </div>

            // Nav Items
            <RailIcon
                active=Signal::derive(move || active.get() == ViewType::Campaigns)
                icon="📚"
                label="Campaigns"
                on_click=Callback::new(move |_| layout.active_view.set(ViewType::Campaigns))
            />
            <RailIcon
                active=Signal::derive(move || active.get() == ViewType::Chat)
                icon="💬"
                label="Chat"
                on_click=Callback::new(move |_| layout.active_view.set(ViewType::Chat))
            />
            <RailIcon
                active=Signal::derive(move || active.get() == ViewType::Library)
                icon="🧠"
                label="Library"
                on_click=Callback::new(move |_| layout.active_view.set(ViewType::Library))
            />
            <RailIcon
                active=Signal::derive(move || active.get() == ViewType::Graph)
                icon="🔮"
                label="Graph"
                on_click=Callback::new(move |_| layout.active_view.set(ViewType::Graph))
            />

            <div class="flex-1"></div> // Spacer

            <RailIcon
                active=Signal::derive(move || active.get() == ViewType::Settings)
                icon="⚙️"
                label="Settings"
                on_click=Callback::new(move |_| layout.active_view.set(ViewType::Settings))
            />
        </div>
    }
}

#[component]
fn RailIcon(
    #[prop(into)] active: Signal<bool>,
    icon: &'static str,
    label: &'static str,
    #[prop(into)] on_click: Callback<()>,
) -> impl IntoView {
    let active_class = Signal::derive(move || {
        if active.get() {
            "text-[var(--accent)] bg-[var(--bg-surface)] border-l-2 border-[var(--accent)]"
        } else {
            "text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface)]"
        }
    });

    view! {
        <div
            class=move || format!(
                "group relative w-full h-12 flex items-center justify-center cursor-pointer transition-colors {}",
                active_class.get()
            )
            on:click=move |_| on_click.run(())
        >
            <span class="text-xl">{icon}</span>

            // Tooltip
            <div class="absolute left-14 top-2 bg-[var(--bg-elevated)] text-[var(--text-primary)] text-xs px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap border border-[var(--border-subtle)] z-50 pointer-events-none">
                {label}
            </div>
        </div>
    }
}
