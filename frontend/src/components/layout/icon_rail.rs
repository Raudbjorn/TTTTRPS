//! IconRail Navigation Component
//!
//! A vertical navigation rail with icon buttons, tooltips, and keyboard navigation.
//! Features:
//!   - Fixed 64px width rail on the left side
//!   - Icons with hover tooltips showing labels and keyboard shortcuts
//!   - Active state indicator with accent color
//!   - Keyboard accessible with focus indicators
//!   - Collapsible toggle buttons for sidebar/info panel

use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_location};
use leptos_router::NavigateOptions;
use crate::services::layout_service::{LayoutState, ViewType};

/// Icon definitions with metadata
struct NavIcon {
    path: &'static str,
    view_type: ViewType,
    icon: &'static str,
    label: &'static str,
    shortcut: Option<&'static str>,
}

const NAV_ICONS: &[NavIcon] = &[
    NavIcon {
        path: "/",
        view_type: ViewType::Home,
        icon: "home",
        label: "Home",
        shortcut: Some("1"),
    },
    NavIcon {
        path: "/campaigns",
        view_type: ViewType::Campaigns,
        icon: "folder",
        label: "Campaigns",
        shortcut: Some("2"),
    },
    NavIcon {
        path: "/library",
        view_type: ViewType::Library,
        icon: "book",
        label: "Library",
        shortcut: Some("3"),
    },
    NavIcon {
        path: "/chat",
        view_type: ViewType::Chat,
        icon: "message",
        label: "Chat",
        shortcut: Some("4"),
    },
];

#[component]
pub fn IconRail() -> impl IntoView {
    let layout = expect_context::<LayoutState>();
    let navigate = use_navigate();
    let location = use_location();

    // Helper to check if a path is active
    let is_active = move |path: &str| -> bool {
        let current = location.pathname.get();
        if path == "/" {
            current == "/" || current.is_empty()
        } else {
            current.starts_with(path)
        }
    };

    // Clone navigate for use in multiple closures
    let nav_for_make = navigate.clone();
    let nav_for_logo = navigate.clone();

    // Helper to create navigation callback
    let make_nav = move |path: &'static str, view: ViewType| {
        let nav = nav_for_make.clone();
        Callback::new(move |_: ()| {
            layout.active_view.set(view);
            nav(path, NavigateOptions::default());
        })
    };

    // Toggle handlers for sidebar and info panel
    let toggle_sidebar = move |_: web_sys::MouseEvent| {
        layout.toggle_sidebar();
    };

    let toggle_info = move |_: web_sys::MouseEvent| {
        layout.toggle_infopanel();
    };

    // Derive visibility states for toggle buttons
    let sidebar_visible = layout.sidebar_visible;
    let infopanel_visible = layout.infopanel_visible;

    view! {
        <nav
            class="h-full w-full flex flex-col items-center py-4 gap-1 bg-[var(--bg-deep)] border-r border-[var(--border-subtle)]"
            role="navigation"
            aria-label="Main navigation"
        >
            // Logo / Home
            <button
                class="mb-4 cursor-pointer focus:outline-none focus:ring-2 focus:ring-[var(--accent)] rounded-full"
                aria-label="Go to home"
                on:click={
                    let nav = nav_for_logo.clone();
                    move |_| {
                        layout.active_view.set(ViewType::Home);
                        nav("/", Default::default());
                    }
                }
            >
                <div class="w-10 h-10 rounded-full bg-gradient-to-br from-purple-500 to-blue-600 flex items-center justify-center text-white font-bold shadow-lg hover:shadow-xl transition-shadow">
                    "A"
                </div>
            </button>

            // Main Navigation Icons
            <RailIcon
                active=Signal::derive(move || {
                    let current = location.pathname.get();
                    current == "/" || current.is_empty()
                })
                icon="home"
                label="Home"
                shortcut="Ctrl+1"
                on_click=make_nav("/", ViewType::Home)
            />
            <RailIcon
                active=Signal::derive(move || is_active("/campaigns"))
                icon="folder"
                label="Campaigns"
                shortcut="Ctrl+2"
                on_click=make_nav("/campaigns", ViewType::Campaigns)
            />
            <RailIcon
                active=Signal::derive(move || is_active("/library"))
                icon="book"
                label="Library"
                shortcut="Ctrl+3"
                on_click=make_nav("/library", ViewType::Library)
            />
            <RailIcon
                active=Signal::derive(move || is_active("/chat"))
                icon="message"
                label="Chat"
                shortcut="Ctrl+4"
                on_click=make_nav("/chat", ViewType::Chat)
            />

            // Spacer
            <div class="flex-1" aria-hidden="true"></div>

            // Panel Toggle Section
            <div class="flex flex-col gap-1 mb-2">
                // Sidebar Toggle
                <button
                    class=move || format!(
                        "w-10 h-10 rounded-lg flex items-center justify-center transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--accent)] {}",
                        if sidebar_visible.get() {
                            "text-[var(--accent)] bg-[var(--bg-surface)]"
                        } else {
                            "text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface)]"
                        }
                    )
                    title="Toggle Sidebar (Ctrl+.)"
                    aria-label=move || if sidebar_visible.get() { "Hide sidebar" } else { "Show sidebar" }
                    aria-pressed=move || sidebar_visible.get().to_string()
                    on:click=toggle_sidebar
                >
                    <SidebarIcon />
                </button>

                // Info Panel Toggle
                <button
                    class=move || format!(
                        "w-10 h-10 rounded-lg flex items-center justify-center transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--accent)] {}",
                        if infopanel_visible.get() {
                            "text-[var(--accent)] bg-[var(--bg-surface)]"
                        } else {
                            "text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface)]"
                        }
                    )
                    title="Toggle Info Panel (Ctrl+/)"
                    aria-label=move || if infopanel_visible.get() { "Hide info panel" } else { "Show info panel" }
                    aria-pressed=move || infopanel_visible.get().to_string()
                    on:click=toggle_info
                >
                    <InfoPanelIcon />
                </button>
            </div>

            // Settings (always at bottom)
            <RailIcon
                active=Signal::derive(move || is_active("/settings"))
                icon="settings"
                label="Settings"
                shortcut="Ctrl+,"
                on_click=make_nav("/settings", ViewType::Settings)
            />
        </nav>
    }
}

/// Individual rail icon button with tooltip
#[component]
fn RailIcon(
    #[prop(into)] active: Signal<bool>,
    icon: &'static str,
    label: &'static str,
    #[prop(optional)] shortcut: Option<&'static str>,
    #[prop(into)] on_click: Callback<()>,
) -> impl IntoView {
    let active_class = Signal::derive(move || {
        if active.get() {
            "text-[var(--accent)] bg-[var(--bg-surface)] border-l-2 border-[var(--accent)]"
        } else {
            "text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface)] border-l-2 border-transparent"
        }
    });

    let tooltip_text = if let Some(sc) = shortcut {
        format!("{} ({})", label, sc)
    } else {
        label.to_string()
    };

    view! {
        <button
            class=move || format!(
                "group relative w-full h-12 flex items-center justify-center cursor-pointer transition-colors focus:outline-none focus:ring-2 focus:ring-inset focus:ring-[var(--accent)] {}",
                active_class.get()
            )
            aria-label=label
            aria-current=move || if active.get() { Some("page") } else { None }
            on:click=move |_| on_click.run(())
        >
            <span class="text-xl" aria-hidden="true">
                {match icon {
                    "home" => view! { <HomeIcon /> }.into_any(),
                    "folder" => view! { <FolderIcon /> }.into_any(),
                    "message" => view! { <MessageIcon /> }.into_any(),
                    "book" => view! { <BookIcon /> }.into_any(),
                    "settings" => view! { <SettingsIcon /> }.into_any(),
                    _ => view! { <span>"?"</span> }.into_any(),
                }}
            </span>

            // Tooltip with proper positioning and accessibility
            // Tooltip with proper positioning and accessibility
            <div
                class="absolute left-16 top-1/2 -translate-y-1/2 bg-gray-900 text-white text-xs px-3 py-1.5 rounded-md opacity-0 group-hover:opacity-100 group-hover:translate-x-0 -translate-x-2 group-focus:opacity-100 transition-all duration-200 whitespace-nowrap border border-white/10 shadow-xl z-[100] pointer-events-none"
                role="tooltip"
            >
                {tooltip_text}
                // Tooltip arrow
                <div class="absolute left-0 top-1/2 -translate-x-1 -translate-y-1/2 w-2 h-2 bg-gray-900 rotate-45"></div>
            </div>
        </button>
    }
}

// SVG Icon Components

#[component]
fn HomeIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path>
            <polyline points="9 22 9 12 15 12 15 22"></polyline>
        </svg>
    }
}

#[component]
fn FolderIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
        </svg>
    }
}

#[component]
fn MessageIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path>
        </svg>
    }
}

#[component]
fn BookIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path>
            <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path>
        </svg>
    }
}

#[component]
fn SettingsIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="3"></circle>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
        </svg>
    }
}

#[component]
fn SidebarIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
            <line x1="9" y1="3" x2="9" y2="21"></line>
        </svg>
    }
}

#[component]
fn InfoPanelIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
            <line x1="15" y1="3" x2="15" y2="21"></line>
        </svg>
    }
}
