use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_location};
use crate::services::layout_service::{LayoutState, ViewType};

#[component]
pub fn IconRail() -> impl IntoView {
    let layout = expect_context::<LayoutState>();
    let navigate = use_navigate();
    let location = use_location();

    // Helper to check active path
    let is_active = move |path: &str| {
        location.pathname.get() == path
    };



    view! {
        <div class="h-full w-full flex flex-col items-center py-4 gap-4 bg-[var(--bg-deep)] border-r border-[var(--border-subtle)]">
            // Logo / Home
            <div class="mb-4 cursor-pointer" on:click={
                let nav = navigate.clone();
                move |_| {
                    layout.active_view.set(ViewType::Chat);
                    nav("/", Default::default());
                }
            }>
                <div class="w-10 h-10 rounded-full bg-gradient-to-br from-purple-500 to-blue-600 flex items-center justify-center text-white font-bold">
                    "A"
                </div>
            </div>

            // Nav Items
            <RailIcon
                active=Signal::derive(move || is_active("/campaigns"))
                icon="📚"
                label="Campaigns"
                on_click=Callback::new({
                    let nav = navigate.clone();
                    move |_| {
                        layout.active_view.set(ViewType::Campaigns);
                        nav("/campaigns", Default::default());
                    }
                })
            />
            <RailIcon
                active=Signal::derive(move || is_active("/"))
                icon="💬"
                label="Chat"
                on_click=Callback::new({
                    let nav = navigate.clone();
                    move |_| {
                        layout.active_view.set(ViewType::Chat);
                        nav("/", Default::default());
                    }
                })
            />
            <RailIcon
                active=Signal::derive(move || is_active("/library"))
                icon="🧠"
                label="Library"
                on_click=Callback::new({
                    let nav = navigate.clone();
                    move |_| {
                        layout.active_view.set(ViewType::Library);
                        nav("/library", Default::default());
                    }
                })
            />
            // Graph - currently disabled/hidden or point to library? Let's hide it if no route exists
            /*
            <RailIcon
                active=Signal::derive(move || is_active("/graph"))
                icon="🔮"
                label="Graph"
                on_click=Callback::new(move |_| handle_nav("/graph", ViewType::Graph))
            />
            */

            <div class="flex-1"></div> // Spacer

            <RailIcon
                active=Signal::derive(move || is_active("/settings"))
                icon="⚙️"
                label="Settings"
                on_click=Callback::new({
                    let nav = navigate.clone();
                    move |_| {
                        layout.active_view.set(ViewType::Settings);
                        nav("/settings", Default::default());
                    }
                })
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
