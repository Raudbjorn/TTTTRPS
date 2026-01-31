//! Generation Preview Container
//!
//! Container component for previewing generated content with
//! accept/reject/modify controls.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Generation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStatus {
    #[default]
    Pending,
    Generating,
    Ready,
    Accepted,
    Rejected,
    Modified,
}

impl GenerationStatus {
    pub fn label(&self) -> &'static str {
        match self {
            GenerationStatus::Pending => "Pending",
            GenerationStatus::Generating => "Generating",
            GenerationStatus::Ready => "Ready for Review",
            GenerationStatus::Accepted => "Accepted",
            GenerationStatus::Rejected => "Rejected",
            GenerationStatus::Modified => "Modified",
        }
    }

    pub fn color_class(&self) -> &'static str {
        match self {
            GenerationStatus::Pending => "bg-zinc-700 text-zinc-300",
            GenerationStatus::Generating => "bg-purple-700 text-purple-200 animate-pulse",
            GenerationStatus::Ready => "bg-blue-700 text-blue-200",
            GenerationStatus::Accepted => "bg-green-700 text-green-200",
            GenerationStatus::Rejected => "bg-red-700 text-red-200",
            GenerationStatus::Modified => "bg-amber-700 text-amber-200",
        }
    }
}

/// Action callback type
#[derive(Clone)]
pub enum PreviewAction {
    Accept,
    Reject,
    Regenerate,
    Edit,
}

// ============================================================================
// Components
// ============================================================================

/// Status badge component
#[component]
fn StatusBadge(status: Signal<GenerationStatus>) -> impl IntoView {
    view! {
        <span class=move || format!(
            "px-2 py-0.5 text-xs rounded-full font-medium {}",
            status.get().color_class()
        )>
            {move || status.get().label()}
        </span>
    }
}

/// Action bar component
#[component]
fn ActionBar(
    status: Signal<GenerationStatus>,
    on_accept: Callback<()>,
    on_reject: Callback<()>,
    on_regenerate: Callback<()>,
    on_edit: Callback<()>,
    is_processing: Signal<bool>,
) -> impl IntoView {
    let can_act = Signal::derive(move || {
        let s = status.get();
        !is_processing.get() && (s == GenerationStatus::Ready || s == GenerationStatus::Modified)
    });

    view! {
        <div class="flex items-center gap-2 pt-4 border-t border-zinc-700">
            // Accept button
            <button
                type="button"
                class="flex items-center gap-2 px-4 py-2 bg-green-600 hover:bg-green-500 text-white rounded-lg
                       transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                disabled=move || !can_act.get()
                on:click=move |_| on_accept.run(())
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                </svg>
                "Accept"
            </button>

            // Reject button
            <button
                type="button"
                class="flex items-center gap-2 px-4 py-2 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded-lg
                       transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                disabled=move || !can_act.get()
                on:click=move |_| on_reject.run(())
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
                "Reject"
            </button>

            // Edit button
            <button
                type="button"
                class="flex items-center gap-2 px-4 py-2 bg-zinc-700 hover:bg-zinc-600 text-white rounded-lg
                       transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                disabled=move || !can_act.get()
                on:click=move |_| on_edit.run(())
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                </svg>
                "Edit"
            </button>

            <div class="flex-1" />

            // Regenerate button
            <button
                type="button"
                class="flex items-center gap-2 px-4 py-2 bg-purple-600/20 hover:bg-purple-600/30 text-purple-400 rounded-lg
                       transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                disabled=move || is_processing.get()
                on:click=move |_| on_regenerate.run(())
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                </svg>
                "Regenerate"
            </button>
        </div>
    }
}

/// Loading overlay for generation
#[component]
fn GeneratingOverlay() -> impl IntoView {
    view! {
        <div class="absolute inset-0 bg-zinc-900/80 backdrop-blur-sm flex items-center justify-center rounded-lg">
            <div class="flex flex-col items-center gap-3">
                <div class="w-10 h-10 border-4 border-purple-600 border-t-transparent rounded-full animate-spin" />
                <span class="text-zinc-300 text-sm">"Generating content..."</span>
            </div>
        </div>
    }
}

/// Main generation preview container
#[component]
pub fn GenerationPreview(
    /// Title of the generated content
    #[prop(into)]
    title: String,
    /// Current generation status
    status: RwSignal<GenerationStatus>,
    /// Callback when content is accepted
    on_accept: Callback<()>,
    /// Callback when content is rejected
    on_reject: Callback<()>,
    /// Callback to regenerate content
    on_regenerate: Callback<()>,
    /// Callback to edit content
    on_edit: Callback<()>,
    /// Whether an operation is in progress
    #[prop(default = RwSignal::new(false))]
    is_processing: RwSignal<bool>,
    /// The content to preview
    children: ChildrenFn,
) -> impl IntoView {
    let status_signal = Signal::derive(move || status.get());
    let is_processing_signal = Signal::derive(move || is_processing.get());
    let is_generating = Signal::derive(move || status.get() == GenerationStatus::Generating);

    view! {
        <div class="relative p-4 bg-zinc-800/50 border border-zinc-700 rounded-lg">
            // Header
            <div class="flex items-center justify-between mb-4">
                <h4 class="font-medium text-white">{title}</h4>
                <StatusBadge status=status_signal />
            </div>

            // Content area
            <div class="relative">
                {children()}

                // Overlay when generating
                <Show when=move || is_generating.get()>
                    <GeneratingOverlay />
                </Show>
            </div>

            // Action bar
            <ActionBar
                status=status_signal
                on_accept=on_accept
                on_reject=on_reject
                on_regenerate=on_regenerate
                on_edit=on_edit
                is_processing=is_processing_signal
            />
        </div>
    }
}
