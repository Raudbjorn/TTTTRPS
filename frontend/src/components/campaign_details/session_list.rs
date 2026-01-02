//! Session List / Context Sidebar Component
//!
//! A Campaign View pattern sidebar showing sessions organized by status.
//! Features:
//!   - Collapsible section groups (Current, Planned, History)
//!   - Visual status indicators with animation
//!   - Session cards with metadata preview
//!   - Quick-action buttons for session management
//!   - Keyboard accessible with proper ARIA labels

use leptos::prelude::*;
use crate::bindings::SessionSummary;

/// Session status for grouping
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Current,
    Planned,
    Past,
}

/// Context sidebar for session management (Campaign View pattern)
#[component]
pub fn ContextSidebar(
    /// List of session summaries
    sessions: Vec<SessionSummary>,
    /// Currently active session ID
    #[prop(optional)]
    active_session_id: Option<String>,
    /// Callback when a session is selected
    on_select_session: Callback<String>,
    /// Optional callback to create new session
    #[prop(optional, into)]
    on_create_session: Option<Callback<()>>,
) -> impl IntoView {
    // Group sessions by status
    let max_sess_num = sessions.iter().map(|s| s.session_number).max().unwrap_or(0);

    let current_session = sessions
        .iter()
        .find(|s| s.session_number == max_sess_num && s.status != "planned")
        .cloned();

    let planned_sessions: Vec<_> = sessions
        .iter()
        .filter(|s| s.status == "planned")
        .cloned()
        .collect();

    let past_sessions: Vec<_> = sessions
        .iter()
        .filter(|s| s.session_number != max_sess_num && s.status != "planned")
        .cloned()
        .collect();

    // Section collapse states
    let current_expanded = RwSignal::new(true);
    let planned_expanded = RwSignal::new(true);
    let history_expanded = RwSignal::new(true);

    let active_id_current = active_session_id.clone();
    let active_id_planned = active_session_id.clone();
    let active_id_past = active_session_id;

    view! {
        <aside
            class="flex flex-col h-full bg-[var(--bg-surface)] border-r border-[var(--border-subtle)]"
            role="complementary"
            aria-label="Session navigation"
        >
            // Header
            <header class="p-4 border-b border-[var(--border-subtle)]">
                <div class="flex items-center justify-between">
                    <h2 class="text-xs font-bold uppercase tracking-wider text-[var(--text-muted)]">
                        "Sessions"
                    </h2>
                    {on_create_session.map(|cb| view! {
                        <button
                            class="w-6 h-6 rounded flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-elevated)] transition-colors focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                            aria-label="Create new session"
                            title="New Session"
                            on:click=move |_| cb.run(())
                        >
                            <PlusIcon />
                        </button>
                    })}
                </div>
            </header>

            // Session Lists
            <nav class="flex-1 overflow-y-auto p-2 space-y-4">
                // Current Session Section
                {move || current_session.clone().map(|session| {
                    let session_id = session.id.clone();
                    let sess_num = session.session_number;
                    let on_click = on_select_session.clone();
                    let is_active = active_id_current.as_ref() == Some(&session_id);

                    view! {
                        <SessionSection
                            title="Current"
                            expanded=current_expanded
                            indicator=Some(view! {
                                <div class="w-2 h-2 rounded-full bg-green-500 animate-pulse"></div>
                            })
                        >
                            <SessionCard
                                session=session
                                is_active=is_active
                                status=SessionStatus::Current
                                on_select=on_click
                            />
                        </SessionSection>
                    }
                })}

                // Planned Sessions Section
                <SessionSection
                    title="Planned"
                    expanded=planned_expanded
                    count=planned_sessions.len()
                    indicator={Option::<&str>::None}
                >
                    {if planned_sessions.is_empty() {
                        view! {
                            <div class="px-3 py-2">
                                <button
                                    class="w-full flex items-center gap-2 px-3 py-2 rounded-lg border border-dashed border-[var(--border-subtle)] text-[var(--text-muted)] hover:border-[var(--accent)] hover:text-[var(--accent)] transition-colors group"
                                    on:click=move |_| {
                                        if let Some(ref cb) = on_create_session {
                                            cb.run(());
                                        }
                                    }
                                >
                                    <PlusIcon />
                                    <span class="text-sm">"Plan next session"</span>
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="space-y-1">
                                {planned_sessions.iter().map(|s| {
                                    let session = s.clone();
                                    let is_active = active_id_planned.as_ref() == Some(&s.id);
                                    let on_click = on_select_session.clone();
                                    view! {
                                        <SessionCard
                                            session=session
                                            is_active=is_active
                                            status=SessionStatus::Planned
                                            on_select=on_click
                                        />
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </SessionSection>

                // History Section
                <SessionSection
                    title="History"
                    expanded=history_expanded
                    count=past_sessions.len()
                    indicator={Option::<&str>::None}
                >
                    {if past_sessions.is_empty() {
                        view! {
                            <p class="px-3 py-4 text-center text-xs text-[var(--text-muted)] italic">
                                "No past sessions"
                            </p>
                        }.into_any()
                    } else {
                        view! {
                            <div class="space-y-1">
                                {past_sessions.iter().map(|s| {
                                    let session = s.clone();
                                    let is_active = active_id_past.as_ref() == Some(&s.id);
                                    let on_click = on_select_session.clone();
                                    view! {
                                        <SessionCard
                                            session=session
                                            is_active=is_active
                                            status=SessionStatus::Past
                                            on_select=on_click
                                        />
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }}
                </SessionSection>
            </nav>

            // Footer
            <footer class="p-2 border-t border-[var(--border-subtle)] text-center">
                <span class="text-[10px] text-[var(--text-muted)]">
                    {format!("{} total sessions", sessions.len())}
                </span>
            </footer>
        </aside>
    }
}

/// Collapsible section component
#[component]
fn SessionSection(
    title: &'static str,
    expanded: RwSignal<bool>,
    #[prop(optional)]
    indicator: Option<impl IntoView + 'static>,
    #[prop(optional)]
    count: Option<usize>,
    children: Children,
) -> impl IntoView {
    let toggle = move |_: web_sys::MouseEvent| {
        expanded.update(|v| *v = !*v);
    };

    view! {
        <section class="rounded-lg overflow-hidden">
            // Section Header
            <button
                class="w-full flex items-center gap-2 px-3 py-2 text-left text-xs font-semibold text-[var(--text-muted)] uppercase tracking-wide hover:bg-[var(--bg-elevated)] transition-colors focus:outline-none focus:ring-2 focus:ring-inset focus:ring-[var(--accent)]"
                aria-expanded=move || expanded.get().to_string()
                on:click=toggle
            >
                // Chevron
                <span class=move || format!(
                    "transition-transform {}",
                    if expanded.get() { "rotate-90" } else { "" }
                )>
                    <ChevronIcon />
                </span>

                // Status indicator
                {indicator}

                // Title
                <span class="flex-1">{title}</span>

                // Count badge
                {count.map(|c| view! {
                    <span class="px-1.5 py-0.5 text-[10px] rounded-full bg-[var(--bg-deep)] text-[var(--text-muted)]">
                        {c}
                    </span>
                })}
            </button>

            // Content
            <div class=move || format!(
                "transition-all overflow-hidden {}",
                if expanded.get() { "max-h-[1000px] opacity-100" } else { "max-h-0 opacity-0" }
            )>
                {children()}
            </div>
        </section>
    }
}

/// Individual session card
#[component]
fn SessionCard(
    session: SessionSummary,
    is_active: bool,
    status: SessionStatus,
    on_select: Callback<String>,
) -> impl IntoView {
    let session_id = session.id.clone();
    let sess_num = session.session_number;
    let duration = session.duration_minutes.unwrap_or(0);
    let note_count = session.note_count;
    let had_combat = session.had_combat;

    let status_badge = match status {
        SessionStatus::Current => Some(("Live", "bg-green-500/20 text-green-400 border-green-500/30")),
        SessionStatus::Planned => Some(("Scheduled", "bg-blue-500/20 text-blue-400 border-blue-500/30")),
        SessionStatus::Past => None,
    };

    let card_class = if is_active {
        "bg-[var(--accent)]/10 border-[var(--accent)]/50 ring-1 ring-[var(--accent)]"
    } else {
        "bg-[var(--bg-elevated)] border-transparent hover:border-[var(--border-strong)]"
    };

    view! {
        <button
            class=format!(
                "w-full text-left p-3 rounded-lg border transition-all focus:outline-none focus:ring-2 focus:ring-[var(--accent)] {}",
                card_class
            )
            aria-current=move || if is_active { Some("true") } else { None }
            on:click=move |_| on_select.run(session_id.clone())
        >
            // Header row
            <div class="flex items-center justify-between mb-1">
                <span class="font-semibold text-sm text-[var(--text-primary)]">
                    {format!("Session {}", sess_num)}
                </span>
                {status_badge.map(|(text, class)| view! {
                    <span class=format!("px-2 py-0.5 text-[10px] font-medium rounded-full border {}", class)>
                        {text}
                    </span>
                })}
            </div>

            // Metadata row
            <div class="flex items-center gap-3 text-[10px] text-[var(--text-muted)]">
                {if duration > 0 {
                    Some(view! {
                        <span class="flex items-center gap-1">
                            <ClockIcon />
                            {format!("{}m", duration)}
                        </span>
                    })
                } else {
                    None
                }}
                {if note_count > 0 {
                    Some(view! {
                        <span class="flex items-center gap-1">
                            <NoteIcon />
                            {note_count}
                        </span>
                    })
                } else {
                    None
                }}
                {if had_combat {
                    Some(view! {
                        <span class="flex items-center gap-1 text-red-400">
                            <SwordIcon />
                            "Combat"
                        </span>
                    })
                } else {
                    None
                }}
            </div>
        </button>
    }
}

// Backwards compatibility alias
pub use ContextSidebar as SessionList;

// SVG Icon Components

#[component]
fn PlusIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="5" x2="12" y2="19"></line>
            <line x1="5" y1="12" x2="19" y2="12"></line>
        </svg>
    }
}

#[component]
fn ChevronIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="9 18 15 12 9 6"></polyline>
        </svg>
    }
}

#[component]
fn ClockIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="10"></circle>
            <polyline points="12 6 12 12 16 14"></polyline>
        </svg>
    }
}

#[component]
fn NoteIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
            <polyline points="14 2 14 8 20 8"></polyline>
            <line x1="16" y1="13" x2="8" y2="13"></line>
            <line x1="16" y1="17" x2="8" y2="17"></line>
        </svg>
    }
}

#[component]
fn SwordIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="14.5 17.5 3 6 3 3 6 3 17.5 14.5"></polyline>
            <line x1="13" y1="19" x2="19" y2="13"></line>
            <line x1="16" y1="16" x2="20" y2="20"></line>
            <line x1="19" y1="21" x2="21" y2="19"></line>
        </svg>
    }
}
