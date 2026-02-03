//! Wizard Shell Component
//!
//! Main container for the campaign creation wizard.
//! Manages step navigation, auto-save, and layout.

use leptos::ev;
use leptos::prelude::*;

use crate::bindings::Campaign;
use crate::services::wizard_state::{
    advance_step_action, cancel_wizard_action, complete_wizard_action, go_back_action,
    provide_wizard_context, skip_step_action, start_wizard_action, use_wizard_context, StepData,
    WizardStep,
};

use super::conversation_panel::ConversationPanel;
use super::step_progress::StepProgress;
use super::steps::{
    ArcStructureStep, BasicsStep, InitialContentStep, IntentStep, PartyCompositionStep,
    PlayersStep, ReviewStep, ScopeStep,
};

/// Navigation footer with back/next/skip buttons
#[component]
fn WizardNavigation(
    /// Callback for step data submission (reserved for future direct use)
    #[allow(unused)]
    _on_submit: Callback<StepData>,
    /// Callback for going back
    on_back: Callback<()>,
    /// Callback for skipping
    on_skip: Callback<()>,
    /// Callback for cancellation
    on_cancel: Callback<bool>,
    /// Callback for completion
    on_complete: Callback<()>,
    /// Whether submit is enabled
    can_submit: Signal<bool>,
) -> impl IntoView {
    let ctx = use_wizard_context();

    let is_first_step = Signal::derive(move || !ctx.can_go_back());
    let is_last_step = Signal::derive(move || ctx.current_step.get() == WizardStep::Review);
    let can_skip = Signal::derive(move || ctx.can_skip());

    let handle_back = move |_: ev::MouseEvent| {
        on_back.run(());
    };

    let handle_skip = move |_: ev::MouseEvent| {
        on_skip.run(());
    };

    let handle_cancel = move |_: ev::MouseEvent| {
        on_cancel.run(true); // Save draft by default
    };

    let handle_complete = move |_: ev::MouseEvent| {
        on_complete.run(());
    };

    view! {
        <div class="flex items-center justify-between px-6 py-4 border-t border-zinc-800 bg-zinc-900/50">
            // Left: Cancel + Back
            <div class="flex items-center gap-2">
                <button
                    type="button"
                    class="px-4 py-2 text-zinc-400 hover:text-white transition-colors"
                    on:click=handle_cancel
                >
                    "Cancel"
                </button>

                {move || (!is_first_step.get()).then(|| view! {
                    <button
                        type="button"
                        class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors disabled:opacity-50"
                        disabled=move || ctx.is_saving.get()
                        on:click=handle_back
                    >
                        "Back"
                    </button>
                })}
            </div>

            // Center: Auto-save indicator
            <div class="text-xs text-zinc-500">
                {move || {
                    if ctx.auto_save_pending.get() {
                        view! { <span class="animate-pulse">"Saving..."</span> }.into_any()
                    } else if ctx.last_auto_save.get().is_some() {
                        view! { <span>"Saved"</span> }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}
            </div>

            // Right: Skip + Next/Complete
            <div class="flex items-center gap-2">
                {move || (can_skip.get() && !is_last_step.get()).then(|| view! {
                    <button
                        type="button"
                        class="px-4 py-2 text-zinc-400 hover:text-white transition-colors"
                        disabled=move || ctx.is_saving.get()
                        on:click=handle_skip
                    >
                        "Skip"
                    </button>
                })}

                {move || {
                    if is_last_step.get() {
                        view! {
                            <button
                                type="button"
                                class="px-6 py-2 bg-green-600 hover:bg-green-500 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                disabled=move || ctx.is_saving.get() || !can_submit.get()
                                on:click=handle_complete
                            >
                                {move || if ctx.is_saving.get() { "Creating..." } else { "Create Campaign" }}
                            </button>
                        }.into_any()
                    } else {
                        view! {
                            <button
                                type="submit"
                                class="px-6 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                disabled=move || ctx.is_saving.get() || !can_submit.get()
                            >
                                {move || if ctx.is_saving.get() { "Saving..." } else { "Next" }}
                            </button>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

/// Error display component
#[component]
fn ErrorBanner(message: Signal<Option<String>>, on_dismiss: Callback<()>) -> impl IntoView {
    view! {
        <Show when=move || message.get().is_some()>
            <div class="mx-6 mb-4 p-4 bg-red-900/30 border border-red-800 rounded-lg flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <svg class="w-5 h-5 text-red-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                    </svg>
                    <span class="text-red-400 text-sm">
                        {move || message.get().unwrap_or_default()}
                    </span>
                </div>
                <button
                    type="button"
                    class="text-red-400 hover:text-red-300 transition-colors"
                    on:click=move |_| on_dismiss.run(())
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>
        </Show>
    }
}

/// Loading overlay
#[component]
fn LoadingOverlay() -> impl IntoView {
    let ctx = use_wizard_context();

    view! {
        <Show when=move || ctx.is_loading.get()>
            <div class="absolute inset-0 bg-zinc-900/80 backdrop-blur-sm flex items-center justify-center z-50">
                <div class="flex flex-col items-center gap-4">
                    <div class="w-12 h-12 border-4 border-purple-600 border-t-transparent rounded-full animate-spin" />
                    <span class="text-zinc-400">"Loading wizard..."</span>
                </div>
            </div>
        </Show>
    }
}

/// Step content renderer
#[component]
fn StepContent(
    /// Signal containing the current form data ref
    form_data: RwSignal<Option<StepData>>,
    /// Signal for form validity
    form_valid: RwSignal<bool>,
) -> impl IntoView {
    let ctx = use_wizard_context();

    view! {
        {move || {
            let step = ctx.current_step.get();
            match step {
                WizardStep::Basics => view! {
                    <BasicsStep
                        form_data=form_data
                        form_valid=form_valid
                    />
                }.into_any(),
                WizardStep::Intent => view! {
                    <IntentStep
                        form_data=form_data
                        form_valid=form_valid
                    />
                }.into_any(),
                WizardStep::Scope => view! {
                    <ScopeStep
                        form_data=form_data
                        form_valid=form_valid
                    />
                }.into_any(),
                WizardStep::Players => view! {
                    <PlayersStep
                        form_data=form_data
                        form_valid=form_valid
                    />
                }.into_any(),
                WizardStep::PartyComposition => view! {
                    <PartyCompositionStep
                        form_data=form_data
                        form_valid=form_valid
                    />
                }.into_any(),
                WizardStep::ArcStructure => view! {
                    <ArcStructureStep
                        form_data=form_data
                        form_valid=form_valid
                    />
                }.into_any(),
                WizardStep::InitialContent => view! {
                    <InitialContentStep
                        form_data=form_data
                        form_valid=form_valid
                    />
                }.into_any(),
                WizardStep::Review => view! {
                    <ReviewStep
                        form_valid=form_valid
                    />
                }.into_any(),
            }
        }}
    }
}

/// Main wizard shell component
#[component]
pub fn WizardShell(
    /// Whether the wizard modal is open
    is_open: RwSignal<bool>,
    /// Callback when campaign is created
    on_create: Callback<Campaign>,
    /// Whether to start with AI assistance
    #[prop(default = true)]
    ai_assisted: bool,
    /// Optional existing wizard ID to resume (future use)
    #[prop(optional)]
    _resume_wizard_id: Option<String>,
) -> impl IntoView {
    // Provide context for child components
    provide_wizard_context();
    let ctx = use_wizard_context();

    // Form state
    let form_data: RwSignal<Option<StepData>> = RwSignal::new(None);
    let form_valid: RwSignal<bool> = RwSignal::new(false);

    // Show AI panel
    let show_ai_panel = RwSignal::new(false);

    // Initialize wizard on mount
    Effect::new(move |_| {
        if is_open.get() {
            let start_action = start_wizard_action(ctx);
            start_action(ai_assisted);
        }
    });

    // Action handlers
    let handle_submit = {
        let advance = advance_step_action(ctx);
        Callback::new(move |data: StepData| {
            advance(data);
        })
    };

    let handle_back = {
        let go_back = go_back_action(ctx);
        Callback::new(move |_: ()| {
            go_back();
        })
    };

    let handle_skip = {
        let skip = skip_step_action(ctx);
        Callback::new(move |_: ()| {
            skip();
        })
    };

    let handle_cancel = {
        let cancel = cancel_wizard_action(ctx);
        Callback::new(move |save_draft: bool| {
            let cb = Callback::new(move |_: ()| {
                is_open.set(false);
            });
            cancel(save_draft, Some(cb));
        })
    };

    let handle_complete = {
        let complete = complete_wizard_action(ctx);
        let on_create = on_create;
        Callback::new(move |_: ()| {
            let cb = Callback::new(move |campaign: Campaign| {
                is_open.set(false);
                on_create.run(campaign);
            });
            complete(cb);
        })
    };

    // Form submission handler
    let on_form_submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        if let Some(data) = form_data.get() {
            handle_submit.run(data);
        }
    };

    // Dismiss error
    let dismiss_error = Callback::new(move |_: ()| {
        ctx.clear_error();
    });

    // Toggle AI panel
    let toggle_ai_panel = move |_: ev::MouseEvent| {
        show_ai_panel.update(|v| *v = !*v);
    };

    // Can submit based on form validity
    let can_submit = Signal::derive(move || form_valid.get());

    view! {
        <Show when=move || is_open.get()>
            <div
                class="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 flex items-center justify-center p-4"
                on:click=move |_| handle_cancel.run(true)
            >
                <div
                    class="bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl w-full max-w-5xl h-[90vh] overflow-hidden flex flex-col relative"
                    on:click=move |ev: ev::MouseEvent| ev.stop_propagation()
                >
                    // Loading overlay
                    <LoadingOverlay />

                    // Header
                    <div class="px-6 py-4 border-b border-zinc-800 flex items-center justify-between shrink-0">
                        <div class="flex items-center gap-3">
                            <h2 class="text-xl font-bold text-white">"Create Campaign"</h2>
                            {move || ctx.ai_assisted.get().then(|| view! {
                                <span class="px-2 py-0.5 bg-purple-900/50 text-purple-300 text-xs rounded-full">
                                    "AI Assisted"
                                </span>
                            })}
                        </div>
                        <div class="flex items-center gap-2">
                            // AI Panel toggle
                            {move || ctx.ai_assisted.get().then(|| view! {
                                <button
                                    type="button"
                                    class=move || format!(
                                        "p-2 rounded-lg transition-colors {}",
                                        if show_ai_panel.get() {
                                            "bg-purple-600 text-white"
                                        } else {
                                            "bg-zinc-800 text-zinc-400 hover:text-white hover:bg-zinc-700"
                                        }
                                    )
                                    title="Toggle AI Assistant"
                                    on:click=toggle_ai_panel
                                >
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                            d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" />
                                    </svg>
                                </button>
                            })}
                            // Close button
                            <button
                                type="button"
                                class="p-2 text-zinc-400 hover:text-white transition-colors"
                                on:click=move |_| handle_cancel.run(true)
                            >
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                </svg>
                            </button>
                        </div>
                    </div>

                    // Progress indicator
                    <div class="shrink-0 border-b border-zinc-800">
                        <StepProgress />
                    </div>

                    // Error banner
                    <ErrorBanner
                        message=Signal::derive(move || ctx.error.get())
                        on_dismiss=dismiss_error
                    />

                    // Main content area with optional AI panel
                    <div class="flex-1 flex overflow-hidden">
                        // Step content form
                        <form
                            class=move || format!(
                                "flex-1 overflow-y-auto transition-all duration-300 {}",
                                if show_ai_panel.get() { "w-3/5" } else { "w-full" }
                            )
                            on:submit=on_form_submit
                        >
                            <div class="p-6">
                                <StepContent
                                    form_data=form_data
                                    form_valid=form_valid
                                />
                            </div>

                            // Navigation footer inside form for submit
                            <WizardNavigation
                                _on_submit=handle_submit
                                on_back=handle_back
                                on_skip=handle_skip
                                on_cancel=handle_cancel
                                on_complete=handle_complete
                                can_submit=can_submit
                            />
                        </form>

                        // AI Conversation panel (collapsible)
                        <Show when=move || show_ai_panel.get() && ctx.ai_assisted.get()>
                            <div class="w-2/5 border-l border-zinc-800 flex flex-col bg-zinc-900/50">
                                <ConversationPanel />
                            </div>
                        </Show>
                    </div>
                </div>
            </div>
        </Show>
    }
}
