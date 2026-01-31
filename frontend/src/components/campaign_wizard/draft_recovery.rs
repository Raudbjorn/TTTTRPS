//! Draft Recovery Component
//!
//! Detects incomplete wizards on app start and provides options to resume or discard.

use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::services::notification_service::{show_error, show_info};
use crate::services::wizard_state::{delete_wizard, list_incomplete_wizards, WizardSummary};

// ============================================================================
// Draft Recovery Modal
// ============================================================================

/// Modal for recovering incomplete wizard drafts
#[component]
pub fn DraftRecoveryModal(
    /// List of incomplete wizard drafts
    drafts: Vec<WizardSummary>,
    /// Whether the modal is open
    is_open: RwSignal<bool>,
    /// Callback when a draft is selected to resume
    on_resume: Callback<String>,
    /// Callback when drafts are discarded
    on_discard: Callback<Vec<String>>,
) -> impl IntoView {
    let selected_drafts: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
    let is_discarding = RwSignal::new(false);

    // Clone drafts for use in closures
    let drafts_for_view = drafts.clone();
    let all_draft_ids: RwSignal<Vec<String>> =
        RwSignal::new(drafts.iter().map(|d| d.id.clone()).collect());

    let toggle_selection = move |draft_id: String| {
        selected_drafts.update(|ids| {
            if ids.contains(&draft_id) {
                ids.retain(|id| id != &draft_id);
            } else {
                ids.push(draft_id);
            }
        });
    };

    let handle_discard = move |_: leptos::ev::MouseEvent| {
        let ids = selected_drafts.get();
        if ids.is_empty() {
            return;
        }

        is_discarding.set(true);
        let on_discard = on_discard;
        let is_open = is_open;

        spawn_local(async move {
            // Delete each selected draft, tracking successes and failures
            let mut successes = Vec::new();
            let mut failures = Vec::new();

            for id in ids.iter() {
                match delete_wizard(id.clone()).await {
                    Ok(_) => successes.push(id.clone()),
                    Err(e) => failures.push((id.clone(), e)),
                }
            }

            // Surface errors to user if any deletions failed
            if !failures.is_empty() {
                show_error(
                    &format!("Failed to delete {} draft(s)", failures.len()),
                    None,
                    None,
                );
            }

            // Only notify parent about successful deletions
            on_discard.run(successes);
            is_discarding.set(false);

            // Only close modal if all deletions succeeded
            if failures.is_empty() {
                is_open.set(false);
            }
        });
    };

    let handle_resume = move |draft_id: String| {
        on_resume.run(draft_id);
        is_open.set(false);
    };

    let handle_close = move |_: leptos::ev::MouseEvent| {
        is_open.set(false);
    };

    view! {
        <Show when=move || is_open.get()>
            <div
                class="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 flex items-center justify-center p-4"
                on:click=handle_close
            >
                <div
                    class="bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl w-full max-w-lg overflow-hidden"
                    on:click=move |ev: leptos::ev::MouseEvent| ev.stop_propagation()
                >
                    // Header
                    <div class="px-6 py-4 border-b border-zinc-800">
                        <div class="flex items-center gap-3">
                            <div class="p-2 bg-amber-900/30 rounded-lg">
                                <svg class="w-6 h-6 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                                </svg>
                            </div>
                            <div>
                                <h2 class="text-lg font-bold text-white">"Incomplete Campaign Drafts"</h2>
                                <p class="text-sm text-zinc-400">
                                    {format!("You have {} incomplete campaign{}", drafts_for_view.len(), if drafts_for_view.len() == 1 { "" } else { "s" })}
                                </p>
                            </div>
                        </div>
                    </div>

                    // Draft list
                    <div class="max-h-80 overflow-y-auto">
                        {drafts_for_view.iter().map(|draft| {
                            let draft_id = draft.id.clone();
                            let draft_id_for_toggle = draft_id.clone();
                            let draft_id_for_resume = draft_id.clone();
                            let is_selected = Signal::derive(move || selected_drafts.get().contains(&draft_id));

                            view! {
                                <div class=move || format!(
                                    "px-6 py-4 border-b border-zinc-800/50 hover:bg-zinc-800/30 transition-colors {}",
                                    if is_selected.get() { "bg-zinc-800/50" } else { "" }
                                )>
                                    <div class="flex items-start gap-4">
                                        // Checkbox
                                        <button
                                            type="button"
                                            class=move || format!(
                                                "mt-1 w-5 h-5 rounded border-2 flex items-center justify-center transition-colors {}",
                                                if is_selected.get() {
                                                    "bg-purple-600 border-purple-600"
                                                } else {
                                                    "border-zinc-600 hover:border-zinc-500"
                                                }
                                            )
                                            on:click={
                                                let id = draft_id_for_toggle.clone();
                                                move |_| toggle_selection(id.clone())
                                            }
                                        >
                                            <Show when=move || is_selected.get()>
                                                <svg class="w-3 h-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
                                                </svg>
                                            </Show>
                                        </button>

                                        // Draft info
                                        <div class="flex-1 min-w-0">
                                            <div class="flex items-center gap-2">
                                                <h3 class="font-medium text-white truncate">
                                                    {draft.campaign_name.clone().unwrap_or_else(|| "Untitled Campaign".to_string())}
                                                </h3>
                                                {draft.ai_assisted.then(|| view! {
                                                    <span class="px-1.5 py-0.5 bg-purple-900/50 text-purple-300 text-xs rounded">
                                                        "AI"
                                                    </span>
                                                })}
                                            </div>
                                            <div class="flex items-center gap-3 mt-1 text-sm text-zinc-400">
                                                <span>
                                                    {format!("Step: {}", draft.current_step.label())}
                                                </span>
                                                <span class="text-zinc-600">"|"</span>
                                                <span>
                                                    {format!("{}% complete", draft.progress_percent)}
                                                </span>
                                            </div>
                                            <div class="mt-1 text-xs text-zinc-500">
                                                {format!("Last updated: {}", format_relative_time(&draft.updated_at))}
                                            </div>
                                        </div>

                                        // Resume button
                                        <button
                                            type="button"
                                            class="px-3 py-1.5 bg-purple-600 hover:bg-purple-500 text-white text-sm rounded-lg transition-colors"
                                            on:click={
                                                let id = draft_id_for_resume.clone();
                                                move |_| handle_resume(id.clone())
                                            }
                                        >
                                            "Resume"
                                        </button>
                                    </div>
                                </div>
                            }
                        }).collect_view()}
                    </div>

                    // Footer
                    <div class="px-6 py-4 bg-zinc-900/50 border-t border-zinc-800">
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <button
                                    type="button"
                                    class="text-sm text-zinc-400 hover:text-white transition-colors"
                                    on:click=move |_| selected_drafts.set(all_draft_ids.get())
                                >
                                    "Select All"
                                </button>
                                <span class="text-zinc-600">"|"</span>
                                <button
                                    type="button"
                                    class="text-sm text-zinc-400 hover:text-white transition-colors"
                                    on:click=move |_| selected_drafts.set(Vec::new())
                                >
                                    "Select None"
                                </button>
                            </div>
                            <div class="flex items-center gap-2">
                                <button
                                    type="button"
                                    class="px-4 py-2 text-zinc-400 hover:text-white transition-colors"
                                    on:click=handle_close
                                >
                                    "Later"
                                </button>
                                <button
                                    type="button"
                                    class="px-4 py-2 bg-red-600 hover:bg-red-500 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                    disabled=move || selected_drafts.get().is_empty() || is_discarding.get()
                                    on:click=handle_discard
                                >
                                    {move || {
                                        let count = selected_drafts.get().len();
                                        if is_discarding.get() {
                                            "Discarding...".to_string()
                                        } else if count == 0 {
                                            "Discard Selected".to_string()
                                        } else {
                                            format!("Discard {}", count)
                                        }
                                    }}
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </Show>
    }
}

// ============================================================================
// Draft Recovery Hook
// ============================================================================

/// Hook to check for incomplete drafts and show recovery UI
///
/// Returns a tuple of (show_modal signal, drafts list signal).
/// The modal handles resume/discard internally via the DraftRecoveryModal component.
pub fn use_draft_recovery() -> (RwSignal<bool>, RwSignal<Vec<WizardSummary>>) {
    let show_modal = RwSignal::new(false);
    let drafts: RwSignal<Vec<WizardSummary>> = RwSignal::new(Vec::new());

    // Check for incomplete drafts on mount
    Effect::new(move |_| {
        spawn_local(async move {
            match list_incomplete_wizards().await {
                Ok(incomplete) if !incomplete.is_empty() => {
                    drafts.set(incomplete.clone());

                    // Show notification for quick access
                    if incomplete.len() == 1 {
                        let draft = &incomplete[0];
                        let name = draft.campaign_name.clone().unwrap_or_else(|| "Untitled".to_string());
                        show_info(
                            &format!("Resume \"{}\"?", name),
                            Some("You have an incomplete campaign draft."),
                        );
                    } else {
                        show_info(
                            &format!("{} incomplete drafts", incomplete.len()),
                            Some("Click to review your campaign drafts."),
                        );
                    }

                    show_modal.set(true);
                }
                Ok(_) => {
                    // No drafts, nothing to do
                }
                Err(_) => {
                    // Silent failure - don't disrupt user
                }
            }
        });
    });

    (show_modal, drafts)
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Format a timestamp as relative time (e.g., "2 hours ago")
fn format_relative_time(timestamp: &str) -> String {
    // Parse the timestamp
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(timestamp) {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(parsed.with_timezone(&chrono::Utc));

        // Handle future timestamps (negative duration) as "just now"
        let seconds = duration.num_seconds();
        if seconds < 0 {
            return "just now".to_string();
        }
        if seconds < 60 {
            return "just now".to_string();
        }

        let minutes = duration.num_minutes();
        if minutes < 60 {
            return format!("{} minute{} ago", minutes, if minutes == 1 { "" } else { "s" });
        }

        let hours = duration.num_hours();
        if hours < 24 {
            return format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" });
        }

        let days = duration.num_days();
        if days < 7 {
            return format!("{} day{} ago", days, if days == 1 { "" } else { "s" });
        }

        let weeks = days / 7;
        if weeks < 4 {
            return format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" });
        }

        // Fall back to date
        return parsed.format("%b %d, %Y").to_string();
    }

    // Fallback
    timestamp.to_string()
}

// ============================================================================
// Compact Draft Badge (for header/nav)
// ============================================================================

/// Small badge indicator for drafts available
#[component]
pub fn DraftBadge(
    count: Signal<usize>,
    on_click: Callback<()>,
) -> impl IntoView {
    view! {
        <Show when=move || { count.get() > 0 }>
            <button
                type="button"
                class="relative p-2 text-zinc-400 hover:text-white transition-colors"
                title="Incomplete campaign drafts"
                on:click=move |_| on_click.run(())
            >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
                <span class="absolute -top-1 -right-1 w-5 h-5 bg-amber-500 text-white text-xs font-bold rounded-full flex items-center justify-center">
                    {move || count.get()}
                </span>
            </button>
        </Show>
    }
}
