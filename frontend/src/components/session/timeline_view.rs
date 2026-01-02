//! Timeline View Component (TASK-014)
//!
//! Displays session timeline events in a chronological view.
//! Uses bindings to fetch data from the Tauri backend.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    get_session_timeline, get_timeline_summary,
    TimelineEventData, TimelineEventType as BindingEventType,
    TimelineEventSeverity, TimelineSummaryData,
};
use crate::components::design_system::{Card, CardHeader, CardBody, Badge, BadgeVariant};

// ============================================================================
// Timeline Types (Frontend display versions with helper methods)
// ============================================================================

// Re-export the binding types for external use
pub use crate::bindings::{
    TimelineEventType, TimelineEventSeverity as EventSeverity,
    TimelineEventData as TimelineEvent,
};

/// Extension trait for TimelineEventType to provide display helpers
pub trait TimelineEventTypeExt {
    fn icon(&self) -> &'static str;
    fn color(&self) -> &'static str;
    fn label(&self) -> String;
}

impl TimelineEventTypeExt for BindingEventType {
    fn icon(&self) -> &'static str {
        match self {
            Self::SessionStart => "play-circle",
            Self::SessionEnd => "stop-circle",
            Self::SessionPause => "pause-circle",
            Self::SessionResume => "play-circle",
            Self::CombatStart => "swords",
            Self::CombatEnd => "shield",
            Self::CombatRoundStart => "rotate-cw",
            Self::CombatTurnStart => "clock",
            Self::CombatDamage => "heart-crack",
            Self::CombatHealing => "heart-pulse",
            Self::CombatDeath => "skull",
            Self::NoteAdded => "file-text",
            Self::NoteEdited => "edit",
            Self::NoteDeleted => "trash",
            Self::NPCInteraction => "message-circle",
            Self::NPCDialogue => "message-square",
            Self::NPCMood => "smile",
            Self::LocationChange => "map-pin",
            Self::SceneChange => "image",
            Self::PlayerAction => "zap",
            Self::PlayerRoll => "dice",
            Self::SkillCheck => "target",
            Self::SavingThrow => "shield",
            Self::ConditionApplied => "alert-triangle",
            Self::ConditionRemoved => "check-circle",
            Self::ConditionExpired => "clock",
            Self::ItemAcquired => "package",
            Self::ItemUsed => "sparkles",
            Self::ItemLost => "package-x",
            Self::Custom(_) => "circle",
        }
    }

    fn color(&self) -> &'static str {
        match self {
            Self::SessionStart | Self::SessionResume => "#22c55e",
            Self::SessionEnd | Self::SessionPause => "#ef4444",
            Self::CombatStart | Self::CombatDamage => "#f97316",
            Self::CombatEnd | Self::SavingThrow => "#3b82f6",
            Self::CombatRoundStart | Self::CombatTurnStart => "#8b5cf6",
            Self::CombatHealing => "#10b981",
            Self::CombatDeath => "#dc2626",
            Self::NoteAdded | Self::NoteEdited | Self::NoteDeleted => "#6b7280",
            Self::NPCInteraction | Self::NPCDialogue | Self::NPCMood => "#ec4899",
            Self::LocationChange | Self::SceneChange => "#14b8a6",
            Self::PlayerAction | Self::PlayerRoll | Self::SkillCheck => "#eab308",
            Self::ConditionApplied | Self::ConditionExpired => "#f59e0b",
            Self::ConditionRemoved => "#22c55e",
            Self::ItemAcquired | Self::ItemUsed => "#a855f7",
            Self::ItemLost => "#f87171",
            Self::Custom(_) => "#71717a",
        }
    }

    fn label(&self) -> String {
        match self {
            Self::SessionStart => "Session Started".to_string(),
            Self::SessionEnd => "Session Ended".to_string(),
            Self::SessionPause => "Session Paused".to_string(),
            Self::SessionResume => "Session Resumed".to_string(),
            Self::CombatStart => "Combat Started".to_string(),
            Self::CombatEnd => "Combat Ended".to_string(),
            Self::CombatRoundStart => "Round".to_string(),
            Self::CombatTurnStart => "Turn".to_string(),
            Self::CombatDamage => "Damage".to_string(),
            Self::CombatHealing => "Healing".to_string(),
            Self::CombatDeath => "Death".to_string(),
            Self::NoteAdded => "Note".to_string(),
            Self::NoteEdited => "Note Edited".to_string(),
            Self::NoteDeleted => "Note Deleted".to_string(),
            Self::NPCInteraction => "NPC".to_string(),
            Self::NPCDialogue => "Dialogue".to_string(),
            Self::NPCMood => "Mood Change".to_string(),
            Self::LocationChange => "Location".to_string(),
            Self::SceneChange => "Scene".to_string(),
            Self::PlayerAction => "Action".to_string(),
            Self::PlayerRoll => "Roll".to_string(),
            Self::SkillCheck => "Skill Check".to_string(),
            Self::SavingThrow => "Saving Throw".to_string(),
            Self::ConditionApplied => "Condition".to_string(),
            Self::ConditionRemoved => "Condition Removed".to_string(),
            Self::ConditionExpired => "Condition Expired".to_string(),
            Self::ItemAcquired => "Item".to_string(),
            Self::ItemUsed => "Item Used".to_string(),
            Self::ItemLost => "Item Lost".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }
}

/// Extension trait for EventSeverity to provide display helpers
pub trait EventSeverityExt {
    fn badge_variant(&self) -> BadgeVariant;
}

impl EventSeverityExt for TimelineEventSeverity {
    fn badge_variant(&self) -> BadgeVariant {
        match self {
            Self::Trace => BadgeVariant::Default,
            Self::Info => BadgeVariant::Default,
            Self::Notable => BadgeVariant::Info,
            Self::Important => BadgeVariant::Warning,
            Self::Critical => BadgeVariant::Danger,
        }
    }
}

// ============================================================================
// Timeline View Component
// ============================================================================

/// Main timeline view component with auto-fetching from backend
#[component]
pub fn TimelineView(
    /// Session ID to fetch timeline for
    session_id: Signal<String>,
    /// Optional: Provide events directly instead of fetching
    #[prop(optional)]
    events: Option<Signal<Vec<TimelineEventData>>>,
    /// Minimum severity to show
    #[prop(optional)]
    min_severity: Option<Signal<TimelineEventSeverity>>,
    /// Auto-refresh interval in seconds (0 = no refresh)
    #[prop(default = 30)]
    refresh_interval: u32,
) -> impl IntoView {
    // Local state for fetched events
    let fetched_events = RwSignal::new(Vec::<TimelineEventData>::new());
    let is_loading = RwSignal::new(false);

    // Fetch timeline events from backend
    let fetch_timeline = move || {
        let sid = session_id.get();
        if sid.is_empty() {
            return;
        }
        is_loading.set(true);
        spawn_local(async move {
            if let Ok(timeline) = get_session_timeline(sid).await {
                fetched_events.set(timeline);
            }
            is_loading.set(false);
        });
    };

    // Initial fetch
    Effect::new(move |_| {
        fetch_timeline();
    });

    // Use provided events or fetched events
    let all_events = Memo::new(move |_| {
        events.map(|e| e.get()).unwrap_or_else(|| fetched_events.get())
    });

    // Filter state
    let show_all = RwSignal::new(true);
    let show_combat = RwSignal::new(true);
    let show_notes = RwSignal::new(true);
    let show_npc = RwSignal::new(true);

    // Filtered events
    let filtered_events = Memo::new(move |_| {
        let events_list = all_events.get();
        let min_sev = min_severity.map(|s| s.get()).unwrap_or(TimelineEventSeverity::Trace);

        events_list
            .into_iter()
            .filter(|e| e.severity >= min_sev)
            .filter(|e| {
                if show_all.get() {
                    return true;
                }
                match &e.event_type {
                    TimelineEventType::CombatStart |
                    TimelineEventType::CombatEnd |
                    TimelineEventType::CombatRoundStart |
                    TimelineEventType::CombatTurnStart |
                    TimelineEventType::CombatDamage |
                    TimelineEventType::CombatHealing |
                    TimelineEventType::CombatDeath => show_combat.get(),
                    TimelineEventType::NoteAdded |
                    TimelineEventType::NoteEdited |
                    TimelineEventType::NoteDeleted => show_notes.get(),
                    TimelineEventType::NPCInteraction |
                    TimelineEventType::NPCDialogue |
                    TimelineEventType::NPCMood => show_npc.get(),
                    _ => true,
                }
            })
            .collect::<Vec<_>>()
    });

    view! {
        <Card class="timeline-view">
            <CardHeader class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                    <svg class="w-5 h-5 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    <h3 class="font-bold text-zinc-200">"Session Timeline"</h3>
                </div>

                // Filter toggles
                <div class="flex items-center gap-2">
                    <FilterToggle
                        label="Combat"
                        active=show_combat
                        color="#f97316"
                    />
                    <FilterToggle
                        label="Notes"
                        active=show_notes
                        color="#6b7280"
                    />
                    <FilterToggle
                        label="NPCs"
                        active=show_npc
                        color="#ec4899"
                    />
                </div>
            </CardHeader>

            <CardBody class="p-0">
                <Show
                    when=move || !filtered_events.get().is_empty()
                    fallback=|| view! {
                        <div class="py-12 text-center">
                            <svg class="w-12 h-12 mx-auto text-zinc-600 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            <p class="text-zinc-500">"No events yet"</p>
                            <p class="text-sm text-zinc-600">"Events will appear here as the session progresses"</p>
                        </div>
                    }
                >
                    <div class="relative pl-8 pr-4 py-4">
                        // Timeline line
                        <div class="absolute left-6 top-0 bottom-0 w-0.5 bg-zinc-700"/>

                        // Events
                        <div class="space-y-4">
                            <For
                                each=move || filtered_events.get()
                                key=|event| event.id.clone()
                                children=|event| view! {
                                    <TimelineEventItem event=event/>
                                }
                            />
                        </div>
                    </div>
                </Show>
            </CardBody>
        </Card>
    }
}

/// Filter toggle button
#[component]
pub fn FilterToggle(
    label: &'static str,
    active: RwSignal<bool>,
    color: &'static str,
) -> impl IntoView {
    view! {
        <button
            class=move || format!(
                "px-2 py-1 text-xs rounded transition-all {}",
                if active.get() {
                    "bg-opacity-20 border border-opacity-50"
                } else {
                    "bg-zinc-800 border-zinc-700 opacity-50"
                }
            )
            style:background-color=move || if active.get() { format!("{}20", color) } else { String::new() }
            style:border-color=move || if active.get() { format!("{}50", color) } else { String::new() }
            style:color=move || if active.get() { color.to_string() } else { "#71717a".to_string() }
            on:click=move |_| active.update(|v| *v = !*v)
        >
            {label}
        </button>
    }
}

/// Individual timeline event item
#[component]
fn TimelineEventItem(event: TimelineEventData) -> impl IntoView {
    let event_color = event.event_type.color();
    let is_expanded = RwSignal::new(false);

    view! {
        <div class="relative">
            // Event dot
            <div
                class="absolute -left-8 w-4 h-4 rounded-full border-2 border-zinc-900 z-10"
                style:background-color=event_color
            />

            // Event content
            <div
                class="p-3 bg-zinc-800/50 rounded-lg border border-zinc-700/50 hover:border-zinc-600/50 transition-colors cursor-pointer"
                on:click=move |_| is_expanded.update(|v| *v = !*v)
            >
                // Header
                <div class="flex items-start justify-between gap-2 mb-1">
                    <div class="flex items-center gap-2">
                        <span
                            class="text-xs font-medium px-2 py-0.5 rounded"
                            style:background-color=format!("{}20", event_color)
                            style:color=event_color
                        >
                            {event.event_type.label()}
                        </span>
                        <Badge variant=event.severity.badge_variant()>
                            {format!("{:?}", event.severity)}
                        </Badge>
                    </div>
                    <span class="text-xs text-zinc-500">
                        {event.timestamp.clone()}
                    </span>
                </div>

                // Title
                <h4 class="font-medium text-zinc-200 text-sm">
                    {event.title.clone()}
                </h4>

                // Description (collapsed by default)
                <Show when=move || is_expanded.get()>
                    <div class="mt-2 pt-2 border-t border-zinc-700/50">
                        <p class="text-sm text-zinc-400">
                            {event.description.clone()}
                        </p>

                        // Entity links
                        {if !event.entity_refs.is_empty() {
                            Some(view! {
                                <div class="flex flex-wrap gap-1 mt-2">
                                    {event.entity_refs.iter().map(|entity| {
                                        view! {
                                            <span class="text-xs px-2 py-0.5 bg-zinc-700 text-zinc-300 rounded">
                                                {format!("{}: {}", entity.entity_type, entity.name)}
                                            </span>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        } else {
                            None
                        }}
                    </div>
                </Show>

                // Expand indicator
                <div class="flex justify-center mt-2">
                    <svg
                        class=move || format!(
                            "w-4 h-4 text-zinc-500 transition-transform {}",
                            if is_expanded.get() { "rotate-180" } else { "" }
                        )
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                    >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                    </svg>
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Compact Timeline (for sidebar)
// ============================================================================

/// Compact timeline for sidebar display
#[component]
pub fn TimelineCompact(
    /// Session ID to fetch timeline for
    session_id: Signal<String>,
    /// Optional: Provide events directly instead of fetching
    #[prop(optional)]
    events: Option<Signal<Vec<TimelineEventData>>>,
    /// Max events to show
    #[prop(default = 5)]
    max_events: usize,
) -> impl IntoView {
    // Local state for fetched events
    let fetched_events = RwSignal::new(Vec::<TimelineEventData>::new());

    // Fetch timeline events from backend
    Effect::new(move |_| {
        let sid = session_id.get();
        if sid.is_empty() {
            return;
        }
        spawn_local(async move {
            if let Ok(timeline) = get_session_timeline(sid).await {
                fetched_events.set(timeline);
            }
        });
    });

    // Use provided events or fetched events
    let all_events = Memo::new(move |_| {
        events.map(|e| e.get()).unwrap_or_else(|| fetched_events.get())
    });

    let recent_events = Memo::new(move |_| {
        let mut evts = all_events.get();
        evts.reverse();
        evts.into_iter().take(max_events).collect::<Vec<_>>()
    });

    view! {
        <div class="space-y-2">
            <For
                each=move || recent_events.get()
                key=|event| event.id.clone()
                children=|event| {
                    let color = event.event_type.color();
                    view! {
                        <div class="flex items-center gap-2 px-2 py-1.5 rounded bg-zinc-800/30 hover:bg-zinc-800/50 transition-colors">
                            <div
                                class="w-2 h-2 rounded-full shrink-0"
                                style:background-color=color
                            />
                            <span class="flex-1 text-xs text-zinc-300 truncate">
                                {event.title.clone()}
                            </span>
                            <span class="text-xs text-zinc-500 shrink-0">
                                {event.timestamp.clone()}
                            </span>
                        </div>
                    }
                }
            />
        </div>
    }
}
