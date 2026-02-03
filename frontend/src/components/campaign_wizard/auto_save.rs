//! Auto-save System
//!
//! Provides debounced auto-save functionality for the campaign wizard.
//! Auto-saves occur at 30-second intervals when there are pending changes.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::services::wizard_state::{auto_save_wizard, use_wizard_context, PartialCampaign};

// ============================================================================
// Auto-save Constants
// ============================================================================

/// Auto-save interval in milliseconds
const AUTO_SAVE_INTERVAL_MS: u64 = 30_000;

// DEBOUNCE_DELAY_MS removed (unused)

// ============================================================================
// Auto-save State
// ============================================================================

/// Auto-save status indicator
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AutoSaveStatus {
    /// No changes pending
    Idle,
    /// Changes detected, waiting for debounce
    Pending,
    /// Currently saving
    Saving,
    /// Last save succeeded
    Saved,
    /// Last save failed
    Failed,
}

impl AutoSaveStatus {
    pub fn is_busy(&self) -> bool {
        matches!(self, AutoSaveStatus::Saving)
    }

    pub fn has_pending(&self) -> bool {
        matches!(self, AutoSaveStatus::Pending)
    }
}

/// Auto-save state container
#[derive(Clone, Copy)]
pub struct AutoSaveState {
    /// Current status
    pub status: RwSignal<AutoSaveStatus>,
    /// Last successful save timestamp
    pub last_save: RwSignal<Option<String>>,
    /// Error message from last failed save
    pub last_error: RwSignal<Option<String>>,
    /// Whether auto-save is enabled
    pub enabled: RwSignal<bool>,
    /// Signal to trigger a manual retry
    pub trigger_retry: RwSignal<bool>,
}

impl AutoSaveState {
    pub fn new() -> Self {
        Self {
            status: RwSignal::new(AutoSaveStatus::Idle),
            last_save: RwSignal::new(None),
            last_error: RwSignal::new(None),
            enabled: RwSignal::new(true),
            trigger_retry: RwSignal::new(false),
        }
    }
}

impl Default for AutoSaveState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Auto-save Hook
// ============================================================================

/// Hook to manage auto-saving for the wizard
///
/// Returns the auto-save state and a function to mark content as dirty
pub fn use_auto_save() -> (AutoSaveState, Callback<Option<PartialCampaign>>) {
    let ctx = use_wizard_context();
    let state = AutoSaveState::new();

    // Pending data to save
    let pending_data: RwSignal<Option<PartialCampaign>> = RwSignal::new(None);
    let has_pending = RwSignal::new(false);

    // Flag to control interval execution (set to false on cleanup)
    // Note: gloo_timers::Interval is not Send+Sync in WASM, so we use .forget() to
    // keep the interval alive and control execution via this flag. The interval
    // callback early-returns when the flag is false.
    let interval_active = RwSignal::new(false);

    // Setup auto-save interval
    Effect::new(move |_| {
        if !state.enabled.get() {
            interval_active.set(false);
            return;
        }

        // Only create interval once (check prevents multiple intervals on re-runs)
        if interval_active.get_untracked() {
            return;
        }

        // Mark interval as active before creating
        interval_active.set(true);

        // Check for pending saves periodically
        gloo_timers::callback::Interval::new(AUTO_SAVE_INTERVAL_MS as u32, move || {
            // Check if interval should still be active
            if !interval_active.get_untracked() {
                return;
            }

            if !has_pending.get() || state.status.get() == AutoSaveStatus::Saving {
                return;
            }

            if let Some(wizard_id) = ctx.wizard_id() {
                let data = pending_data.get();
                state.status.set(AutoSaveStatus::Saving);

                spawn_local(async move {
                    match auto_save_wizard(wizard_id, data).await {
                        Ok(()) => {
                            state.status.set(AutoSaveStatus::Saved);
                            state.last_save.set(Some(chrono::Utc::now().to_rfc3339()));
                            state.last_error.set(None);
                            has_pending.set(false);
                            pending_data.set(None);

                            // Update context
                            ctx.auto_save_pending.set(false);
                            ctx.last_auto_save
                                .set(Some(chrono::Utc::now().to_rfc3339()));
                        }
                        Err(e) => {
                            state.status.set(AutoSaveStatus::Failed);
                            state.last_error.set(Some(e));
                            // Keep has_pending true so it retries
                        }
                    }
                });
            }
        })
        .forget(); // Keep interval alive; execution controlled by interval_active flag

        // Effect-level cleanup when effect re-runs
        on_cleanup(move || {
            interval_active.set(false);
        });
    });

    // Listen for manual retry triggers
    Effect::new(move |_| {
        if state.trigger_retry.get() {
            state.trigger_retry.set(false);

            // Perform immediate save on retry
            if let Some(wizard_id) = ctx.wizard_id() {
                if state.status.get() == AutoSaveStatus::Saving {
                    return;
                }
                let data = pending_data.get();
                state.status.set(AutoSaveStatus::Saving);

                spawn_local(async move {
                    match auto_save_wizard(wizard_id, data).await {
                        Ok(()) => {
                            state.status.set(AutoSaveStatus::Saved);
                            state.last_save.set(Some(chrono::Utc::now().to_rfc3339()));
                            state.last_error.set(None);
                            has_pending.set(false);
                            pending_data.set(None);
                            ctx.auto_save_pending.set(false);
                            ctx.last_auto_save
                                .set(Some(chrono::Utc::now().to_rfc3339()));
                        }
                        Err(e) => {
                            state.status.set(AutoSaveStatus::Failed);
                            state.last_error.set(Some(e));
                        }
                    }
                });
            }
        }
    });

    // Cleanup: disable interval execution on component unmount
    on_cleanup(move || {
        interval_active.set(false);
    });

    // Mark content as dirty callback
    let mark_dirty = Callback::new(move |data: Option<PartialCampaign>| {
        pending_data.set(data);
        has_pending.set(true);
        state.status.set(AutoSaveStatus::Pending);
        ctx.auto_save_pending.set(true);
    });

    (state, mark_dirty)
}

// ============================================================================
// Auto-save Indicator Component
// ============================================================================

/// Visual indicator showing auto-save status
#[component]
pub fn AutoSaveIndicator(
    /// Auto-save state to display
    #[prop(optional)]
    _state: Option<AutoSaveState>,
) -> impl IntoView {
    // Use context if no state provided
    let ctx = use_wizard_context();

    view! {
        <div class="flex items-center gap-2 text-xs">
            {move || {
                // Check context signals if no explicit state
                let is_pending = ctx.auto_save_pending.get();
                let last_save = ctx.last_auto_save.get();

                if is_pending {
                    view! {
                        <div class="flex items-center gap-1.5 text-zinc-400">
                            <div class="w-2 h-2 bg-amber-400 rounded-full animate-pulse" />
                            <span>"Unsaved changes"</span>
                        </div>
                    }.into_any()
                } else if let Some(timestamp) = last_save {
                    let display_time = format_save_time(&timestamp);
                    view! {
                        <div class="flex items-center gap-1.5 text-zinc-500">
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                            </svg>
                            <span>{format!("Saved {}", display_time)}</span>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <span class="text-zinc-600">"Auto-save enabled"</span>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Detailed auto-save status component
#[component]
pub fn AutoSaveStatus(state: AutoSaveState) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2 text-xs">
            {move || {
                match state.status.get() {
                    AutoSaveStatus::Idle => view! {
                        <span class="text-zinc-600">"All changes saved"</span>
                    }.into_any(),
                    AutoSaveStatus::Pending => view! {
                        <div class="flex items-center gap-1.5 text-amber-400">
                            <div class="w-2 h-2 bg-amber-400 rounded-full" />
                            <span>"Changes pending..."</span>
                        </div>
                    }.into_any(),
                    AutoSaveStatus::Saving => view! {
                        <div class="flex items-center gap-1.5 text-purple-400">
                            <svg class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
                                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                            </svg>
                            <span>"Saving..."</span>
                        </div>
                    }.into_any(),
                    AutoSaveStatus::Saved => view! {
                        <div class="flex items-center gap-1.5 text-green-400">
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                            </svg>
                            <span>
                                {move || state.last_save.get().map(|t| format!("Saved {}", format_save_time(&t))).unwrap_or_else(|| "Saved".to_string())}
                            </span>
                        </div>
                    }.into_any(),
                    AutoSaveStatus::Failed => view! {
                        <div class="flex items-center gap-1.5 text-red-400">
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                    d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                            </svg>
                            <span>"Save failed"</span>
                            <button
                                type="button"
                                class="text-red-300 hover:text-red-200 underline"
                                title={move || state.last_error.get().unwrap_or_default()}
                                on:click=move |_| {
                                    state.trigger_retry.set(true);
                                }
                            >
                                "Retry"
                            </button>
                        </div>
                    }.into_any(),
                }
            }}
        </div>
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Format a save timestamp for display
fn format_save_time(timestamp: &str) -> String {
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(parsed.with_timezone(&chrono::Utc));

        let seconds = duration.num_seconds();
        if seconds < 5 {
            return "just now".to_string();
        }
        if seconds < 60 {
            return format!("{}s ago", seconds);
        }

        let minutes = duration.num_minutes();
        if minutes < 60 {
            return format!("{}m ago", minutes);
        }

        return parsed.format("%H:%M").to_string();
    }

    "recently".to_string()
}

// ============================================================================
// Debounced Save Trigger
// ============================================================================

/// Creates a debounced trigger for marking content as dirty
///
/// Returns a callback that can be called on every change, but will only
/// trigger the actual mark_dirty after the debounce delay.
///
/// Note: This uses a simple approach without cancellation. For the wizard
/// auto-save use case, this is acceptable as we want the last change to
/// trigger the save regardless.
pub fn use_debounced_dirty_tracker(
    mark_dirty: Callback<Option<PartialCampaign>>,
    delay_ms: u64,
) -> Callback<PartialCampaign> {
    let pending_data: RwSignal<Option<PartialCampaign>> = RwSignal::new(None);
    let debounce_active = RwSignal::new(false);

    Callback::new(move |data: PartialCampaign| {
        // Store the latest data
        pending_data.set(Some(data));

        // If debounce is already active, the timer will pick up the new data
        if debounce_active.get() {
            return;
        }

        debounce_active.set(true);

        // Set a timeout to trigger the save
        gloo_timers::callback::Timeout::new(delay_ms as u32, move || {
            let data = pending_data.get();
            mark_dirty.run(data);
            pending_data.set(None);
            debounce_active.set(false);
        })
        .forget(); // Prevent drop from cancelling the timer
    })
}
