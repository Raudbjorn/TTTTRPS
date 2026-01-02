//! Session list sidebar component (ContextSidebar / Campaign View)
//!
//! Displays the list of sessions for a campaign using a Spotify-style
//! "track list" metaphor where sessions are like tracks in an album.
//!
//! Design metaphor: Spotify
//! - Sessions as "Tracks" in a playlist
//! - Current session as "Now Playing"
//! - Past sessions as played tracks
//! - Planned sessions as queue

use leptos::prelude::*;

use crate::bindings::SessionSummary;

/// Session status for display styling
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Planned,
    Completed,
}

impl SessionStatus {
    pub fn from_str(status: &str) -> Self {
        match status.to_lowercase().as_str() {
            "active" | "in_progress" => SessionStatus::Active,
            "planned" | "scheduled" => SessionStatus::Planned,
            _ => SessionStatus::Completed,
        }
    }
}

/// Format duration in minutes to human-readable string
fn format_duration(minutes: i64) -> String {
    if minutes < 60 {
        format!("{}m", minutes)
    } else {
        let hours = minutes / 60;
        let mins = minutes % 60;
        if mins == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h {}m", hours, mins)
        }
    }
}

/// Format date from ISO timestamp
fn format_session_date(iso: &str) -> String {
    if let Some(date_part) = iso.split('T').next() {
        // Return just the date portion
        return date_part.to_string();
    }
    String::new()
}

/// Context Sidebar component (Campaign View)
/// Displays sessions as a Spotify-style track list
#[component]
pub fn SessionList(
    /// List of all sessions
    sessions: RwSignal<Vec<SessionSummary>>,
    /// ID of the currently active session (if any)
    active_session_id: Signal<Option<String>>,
    /// Callback when a session is selected
    on_select_session: Callback<String>,
) -> impl IntoView {
    // Selected session for highlighting
    let selected_session_id = RwSignal::new(Option::<String>::None);

    // Derive session groupings
    let session_groups = Memo::new(move |_| {
        let all_sessions = sessions.get();
        let active_id = active_session_id.get();

        let max_sess_num = all_sessions.iter().map(|s| s.session_number).max().unwrap_or(0);

        let mut past_sessions: Vec<SessionSummary> = vec![];
        let mut current_session: Option<SessionSummary> = None;

        for s in all_sessions {
            if Some(&s.id) == active_id.as_ref() {
                current_session = Some(s);
            } else if s.status == "active" || s.status == "in_progress" {
                current_session = Some(s);
            } else {
                past_sessions.push(s);
            }
        }

        // Sort past sessions by session number descending
        past_sessions.sort_by(|a, b| b.session_number.cmp(&a.session_number));

        // Create a planned session placeholder
        let planned_sessions = vec![SessionSummary {
            id: "planned-next".to_string(),
            campaign_id: String::new(),
            session_number: max_sess_num + 1,
            started_at: String::new(),
            ended_at: None,
            duration_minutes: None,
            status: "planned".to_string(),
            note_count: 0,
            had_combat: false,
            order_index: 0,
        }];

        (past_sessions, current_session, planned_sessions)
    });

    // Total play time calculation
    let total_playtime = Memo::new(move |_| {
        let all_sessions = sessions.get();
        let total: i64 = all_sessions
            .iter()
            .filter_map(|s| s.duration_minutes)
            .sum();
        format_duration(total)
    });

    // Session count
    let session_count = Memo::new(move |_| {
        sessions.get().len()
    });

    view! {
        <div class="flex flex-col h-full bg-zinc-900 border-r border-zinc-800 w-64">
            // Header - Album/Campaign Info
            <div class="p-4 border-b border-zinc-800">
                <div class="flex items-center gap-2 mb-3">
                    <PlaylistIcon />
                    <h2 class="text-zinc-300 text-sm font-bold">"Campaign Tracks"</h2>
                </div>

                // Stats row
                <div class="flex items-center gap-4 text-[11px] text-zinc-500">
                    <span>{move || format!("{} sessions", session_count.get())}</span>
                    <span>"*"</span>
                    <span>{move || total_playtime.get()}</span>
                </div>
            </div>

            // Track List
            <div class="flex-1 overflow-y-auto">
                // Now Playing Section
                {move || {
                    let (_, current, _) = session_groups.get();
                    current.map(|curr| {
                        let curr_id = curr.id.clone();
                        let sess_num = curr.session_number;
                        let note_count = curr.note_count;
                        let had_combat = curr.had_combat;

                        view! {
                            <div class="p-3 border-b border-zinc-800/50">
                                <div class="px-2 mb-2 flex items-center gap-2">
                                    <NowPlayingIcon />
                                    <span class="text-green-400 text-[10px] font-bold uppercase tracking-wider">
                                        "Now Playing"
                                    </span>
                                </div>
                                {
                                    let curr_id_clone = curr_id.clone();
                                    view! {
                                <button
                                    class="w-full group"
                                    on:click=move |_| {
                                        selected_session_id.set(Some(curr_id.clone()));
                                        on_select_session.run(curr_id.clone());
                                    }
                                >
                                    <SessionTrackItem
                                        session_number=sess_num
                                        is_active=true
                                        is_selected=Signal::derive(move || selected_session_id.get() == Some(curr_id_clone.clone()))
                                        duration=None
                                        note_count=note_count
                                        had_combat=had_combat
                                        date=String::new()
                                    />
                                </button>
                                    }
                                }
                            </div>
                        }
                    })
                }}

                // Queue Section (Planned)
                <div class="p-3 border-b border-zinc-800/50">
                    <div class="px-2 mb-2 flex items-center gap-2">
                        <QueueIcon />
                        <span class="text-zinc-500 text-[10px] font-bold uppercase tracking-wider">
                            "Up Next"
                        </span>
                    </div>
                    <For
                        each=move || session_groups.get().2
                        key=|s| s.id.clone()
                        children=move |s| {
                            let sess_num = s.session_number;
                            view! {
                                <div class="group flex items-center gap-3 px-3 py-2.5 rounded-lg text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800/50 cursor-default border border-dashed border-zinc-800 hover:border-zinc-700 transition-colors">
                                    <div class="w-5 h-5 flex items-center justify-center">
                                        <span class="text-xs text-zinc-600">{sess_num}</span>
                                    </div>
                                    <div class="flex-1">
                                        <div class="text-sm font-medium">
                                            {format!("Session {}", sess_num)}
                                        </div>
                                        <div class="text-[10px] text-zinc-600 italic">
                                            "Schedule next session..."
                                        </div>
                                    </div>
                                    <AddIcon />
                                </div>
                            }
                        }
                    />
                </div>

                // History Section (Past Sessions)
                <div class="p-3">
                    <div class="px-2 mb-2 flex items-center justify-between">
                        <div class="flex items-center gap-2">
                            <HistoryIcon />
                            <span class="text-zinc-500 text-[10px] font-bold uppercase tracking-wider">
                                "Session History"
                            </span>
                        </div>
                        <span class="text-[10px] text-zinc-600">
                            {move || format!("{} played", session_groups.get().0.len())}
                        </span>
                    </div>

                    <div class="space-y-1">
                        <For
                            each=move || session_groups.get().0
                            key=|s| s.id.clone()
                            children=move |s| {
                                let s_id = s.id.clone();
                                let sess_num = s.session_number;
                                let duration = s.duration_minutes;
                                let note_count = s.note_count;
                                let had_combat = s.had_combat;
                                let date = format_session_date(&s.started_at);


                                let s_id_clone = s_id.clone();

                                view! {
                                    <button
                                        class="w-full"
                                        on:click=move |_| {
                                            selected_session_id.set(Some(s_id.clone()));
                                            on_select_session.run(s_id.clone());
                                        }
                                    >
                                        <SessionTrackItem
                                            session_number=sess_num
                                            is_active=false
                                            is_selected=Signal::derive(move || selected_session_id.get() == Some(s_id_clone.clone()))
                                            duration=duration
                                            note_count=note_count
                                            had_combat=had_combat
                                            date=date
                                        />
                                    </button>
                                }
                            }
                        />
                    </div>
                </div>
            </div>

            // Footer
            <div class="p-3 border-t border-zinc-800 bg-zinc-900/50">
                <div class="flex items-center justify-between text-[10px] text-zinc-600">
                    <span>"Campaign Timeline"</span>
                    <button class="hover:text-zinc-400 transition-colors">
                        <ExpandIcon />
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Individual session track item (Spotify-style list row)
#[component]
fn SessionTrackItem(
    session_number: u32,
    is_active: bool,
    is_selected: Signal<bool>,
    duration: Option<i64>,
    note_count: usize,
    had_combat: bool,
    date: String,
) -> impl IntoView {
    let base_class = move || {
        let mut classes = vec![
            "flex items-center gap-3 px-3 py-2.5 rounded-lg transition-all cursor-pointer group",
        ];

        if is_active {
            classes.push("bg-green-500/10 border border-green-500/20");
        } else if is_selected.get() {
            classes.push("bg-purple-500/10 border border-purple-500/20");
        } else {
            classes.push("hover:bg-zinc-800/50 border border-transparent");
        }

        classes.join(" ")
    };

    view! {
        <div class=base_class>
            // Track Number / Now Playing Animation
            <div class="w-5 h-5 flex items-center justify-center">
                {if is_active {
                    view! {
                        <div class="flex items-end gap-0.5 h-3">
                            <div class="w-0.5 bg-green-400 rounded-full animate-soundbar-1"></div>
                            <div class="w-0.5 bg-green-400 rounded-full animate-soundbar-2"></div>
                            <div class="w-0.5 bg-green-400 rounded-full animate-soundbar-3"></div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <span class=move || {
                            if is_selected.get() {
                                "text-xs text-purple-400 font-medium"
                            } else {
                                "text-xs text-zinc-600 group-hover:text-zinc-400 transition-colors"
                            }
                        }>
                            {session_number}
                        </span>
                    }.into_any()
                }}
            </div>

            // Track Info
            <div class="flex-1 min-w-0">
                <div class=move || {
                    if is_active {
                        "text-sm font-medium text-green-400"
                    } else if is_selected.get() {
                        "text-sm font-medium text-purple-300"
                    } else {
                        "text-sm font-medium text-zinc-300 group-hover:text-white transition-colors"
                    }
                }>
                    {format!("Session {}", session_number)}
                </div>

                // Metadata row
                <div class="flex items-center gap-2 mt-0.5">
                    {if !date.is_empty() {
                        Some(view! {
                            <span class="text-[10px] text-zinc-600">{date}</span>
                        })
                    } else {
                        None
                    }}

                    {if had_combat {
                        Some(view! {
                            <span class="text-[10px] text-red-400/70 flex items-center gap-0.5">
                                <CombatIcon />
                                "Combat"
                            </span>
                        })
                    } else {
                        None
                    }}

                    {if note_count > 0 {
                        Some(view! {
                            <span class="text-[10px] text-zinc-600 flex items-center gap-0.5">
                                <NoteIcon />
                                {note_count}
                            </span>
                        })
                    } else {
                        None
                    }}
                </div>
            </div>

            // Duration
            {duration.map(|d| view! {
                <span class="text-xs text-zinc-600 tabular-nums">
                    {format_duration(d)}
                </span>
            })}

            // Hover actions
            <div class="opacity-0 group-hover:opacity-100 transition-opacity">
                <MoreIcon />
            </div>
        </div>
    }
}

// Icon Components

#[component]
fn PlaylistIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-purple-400">
            <line x1="8" y1="6" x2="21" y2="6"></line>
            <line x1="8" y1="12" x2="21" y2="12"></line>
            <line x1="8" y1="18" x2="21" y2="18"></line>
            <line x1="3" y1="6" x2="3.01" y2="6"></line>
            <line x1="3" y1="12" x2="3.01" y2="12"></line>
            <line x1="3" y1="18" x2="3.01" y2="18"></line>
        </svg>
    }
}

#[component]
fn NowPlayingIcon() -> impl IntoView {
    view! {
        <div class="w-3 h-3 rounded-full bg-green-500 animate-pulse"></div>
    }
}

#[component]
fn QueueIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-zinc-600">
            <line x1="17" y1="10" x2="3" y2="10"></line>
            <line x1="21" y1="6" x2="3" y2="6"></line>
            <line x1="21" y1="14" x2="3" y2="14"></line>
            <line x1="17" y1="18" x2="3" y2="18"></line>
        </svg>
    }
}

#[component]
fn HistoryIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-zinc-600">
            <circle cx="12" cy="12" r="10"></circle>
            <polyline points="12,6 12,12 16,14"></polyline>
        </svg>
    }
}

#[component]
fn AddIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="opacity-0 group-hover:opacity-100 transition-opacity">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="16"></line>
            <line x1="8" y1="12" x2="16" y2="12"></line>
        </svg>
    }
}

#[component]
fn ExpandIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15,3 21,3 21,9"></polyline>
            <polyline points="9,21 3,21 3,15"></polyline>
            <line x1="21" y1="3" x2="14" y2="10"></line>
            <line x1="3" y1="21" x2="10" y2="14"></line>
        </svg>
    }
}

#[component]
fn CombatIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
            <path d="M14.5 3L12 1l-2.5 2L7 2 5.5 4 3 3v3l2 2-1 2.5L6 12l-2 2.5L5.5 17l2-1 2.5 2L12 16l2.5 2 2.5-2 2 1 1.5-2.5-2-2.5 2-2.5-2-2 2-2V3l-2.5 1L17 2l-2.5 1z"/>
        </svg>
    }
}

#[component]
fn NoteIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
            <polyline points="14,2 14,8 20,8"></polyline>
            <line x1="16" y1="13" x2="8" y2="13"></line>
            <line x1="16" y1="17" x2="8" y2="17"></line>
        </svg>
    }
}

#[component]
fn MoreIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-zinc-600 hover:text-zinc-400">
            <circle cx="12" cy="12" r="1"></circle>
            <circle cx="19" cy="12" r="1"></circle>
            <circle cx="5" cy="12" r="1"></circle>
        </svg>
    }
}

/// Also export as ContextSidebar for the new naming convention
pub use SessionList as ContextSidebar;
