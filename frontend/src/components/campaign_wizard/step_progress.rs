//! Step Progress Rail Component
//!
//! Visual indicator showing all wizard steps with current position
//! and completion status.

use leptos::prelude::*;

use crate::services::wizard_state::{use_wizard_context, WizardStep};

/// Checkmark icon SVG
fn check_icon() -> impl IntoView {
    view! {
        <svg
            class="w-4 h-4"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
        >
            <path
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="3"
                d="M5 13l4 4L19 7"
            />
        </svg>
    }
}

/// Individual step indicator
#[component]
fn StepIndicator(
    step: WizardStep,
    is_current: Signal<bool>,
    is_completed: Signal<bool>,
    is_clickable: Signal<bool>,
    on_click: Callback<WizardStep>,
) -> impl IntoView {
    let step_for_click = step;

    let circle_class = Signal::derive(move || {
        let current = is_current.get();
        let completed = is_completed.get();

        if current {
            "bg-purple-600 text-white ring-2 ring-purple-400 ring-offset-2 ring-offset-zinc-900"
        } else if completed {
            "bg-purple-900 text-purple-300 hover:bg-purple-800"
        } else {
            "bg-zinc-800 text-zinc-500"
        }
    });

    let label_class = Signal::derive(move || {
        if is_current.get() {
            "text-white font-medium"
        } else if is_completed.get() {
            "text-zinc-400"
        } else {
            "text-zinc-500"
        }
    });

    let handle_click = move |_| {
        if is_clickable.get() {
            on_click.run(step_for_click);
        }
    };

    view! {
        <button
            type="button"
            class=move || format!(
                "flex flex-col items-center gap-2 transition-all duration-200 {}",
                if is_clickable.get() { "cursor-pointer" } else { "cursor-default" }
            )
            disabled=move || !is_clickable.get()
            on:click=handle_click
        >
            <div class=move || format!(
                "w-10 h-10 rounded-full flex items-center justify-center text-sm font-medium transition-all duration-200 {}",
                circle_class.get()
            )>
                {move || {
                    if is_completed.get() && !is_current.get() {
                        check_icon().into_any()
                    } else {
                        view! { <span>{step.index() + 1}</span> }.into_any()
                    }
                }}
            </div>
            <div class="flex flex-col items-center">
                <span class=move || format!("text-xs transition-colors {}", label_class.get())>
                    {step.label()}
                </span>
                {move || is_current.get().then(|| view! {
                    <span class="text-[10px] text-zinc-500 mt-0.5">
                        {step.description()}
                    </span>
                })}
            </div>
        </button>
    }
}

/// Connector line between steps
#[component]
fn StepConnector(is_completed: Signal<bool>) -> impl IntoView {
    view! {
        <div class=move || format!(
            "flex-1 h-0.5 mx-2 mt-5 transition-colors duration-200 {}",
            if is_completed.get() { "bg-purple-600" } else { "bg-zinc-700" }
        ) />
    }
}

/// Step progress rail showing all wizard steps
#[component]
pub fn StepProgress(
    /// Optional callback when user clicks a completed step to navigate
    #[prop(optional)]
    on_step_click: Option<Callback<WizardStep>>,
) -> impl IntoView {
    let ctx = use_wizard_context();
    let steps = WizardStep::all();

    view! {
        <div class="w-full px-4 py-6">
            // Horizontal layout for wider screens
            <div class="hidden md:flex items-start justify-center gap-1">
                {steps.iter().enumerate().map(|(i, step)| {
                    let step = *step;
                    let is_current = Signal::derive(move || ctx.current_step.get() == step);
                    let is_completed = Signal::derive(move || ctx.is_step_completed(step));
                    let is_clickable = Signal::derive(move || {
                        on_step_click.is_some() && ctx.is_step_completed(step) && !is_current.get()
                    });

                    let on_click_cb = Callback::new(move |clicked_step: WizardStep| {
                        if let Some(cb) = on_step_click {
                            cb.run(clicked_step);
                        }
                    });

                    view! {
                        <>
                            {(i > 0).then(|| {
                                let prev_step = steps[i - 1];
                                let connector_completed = Signal::derive(move || ctx.is_step_completed(prev_step));
                                view! { <StepConnector is_completed=connector_completed /> }
                            })}
                            <StepIndicator
                                step=step
                                is_current=is_current
                                is_completed=is_completed
                                is_clickable=is_clickable
                                on_click=on_click_cb
                            />
                        </>
                    }
                }).collect_view()}
            </div>

            // Compact layout for mobile - just show current step info
            <div class="md:hidden flex flex-col items-center gap-2">
                <div class="flex items-center gap-2">
                    {move || {
                        let current = ctx.current_step.get();
                        let completed = ctx.wizard_state.get()
                            .map(|s| s.completed_steps.len())
                            .unwrap_or(0);
                        view! {
                            <span class="text-sm text-zinc-400">
                                {format!("Step {} of {}", current.index() + 1, WizardStep::all().len())}
                            </span>
                            <span class="text-xs text-zinc-500">
                                {format!("({} completed)", completed)}
                            </span>
                        }
                    }}
                </div>
                <div class="text-center">
                    <h3 class="text-lg font-semibold text-white">
                        {move || ctx.current_step.get().label()}
                    </h3>
                    <p class="text-sm text-zinc-400">
                        {move || ctx.current_step.get().description()}
                    </p>
                </div>
                // Mobile progress bar
                <div class="w-full max-w-xs bg-zinc-800 rounded-full h-2 mt-2">
                    <div
                        class="bg-purple-600 h-2 rounded-full transition-all duration-300"
                        style=move || format!("width: {}%", ctx.progress_percent())
                    />
                </div>
            </div>
        </div>
    }
}

/// Vertical step progress for sidebar layout
#[component]
pub fn StepProgressVertical(
    /// Optional callback when user clicks a completed step
    #[prop(optional)]
    on_step_click: Option<Callback<WizardStep>>,
) -> impl IntoView {
    let ctx = use_wizard_context();
    let steps = WizardStep::all();

    view! {
        <div class="flex flex-col gap-1">
            {steps.iter().map(|step| {
                let step = *step;
                let is_current = Signal::derive(move || ctx.current_step.get() == step);
                let is_completed = Signal::derive(move || ctx.is_step_completed(step));
                let is_clickable = Signal::derive(move || {
                    on_step_click.is_some() && ctx.is_step_completed(step) && !is_current.get()
                });

                let handle_click = move |_| {
                    if is_clickable.get() {
                        if let Some(cb) = on_step_click {
                            cb.run(step);
                        }
                    }
                };

                view! {
                    <button
                        type="button"
                        class=move || format!(
                            "flex items-center gap-3 px-3 py-2 rounded-lg text-left transition-colors {}",
                            if is_current.get() {
                                "bg-purple-900/30 border border-purple-500/50"
                            } else if is_completed.get() {
                                "hover:bg-zinc-800/50 cursor-pointer"
                            } else {
                                "opacity-50 cursor-default"
                            }
                        )
                        disabled=move || !is_clickable.get()
                        on:click=handle_click
                    >
                        // Step number/check
                        <div class=move || format!(
                            "w-7 h-7 rounded-full flex items-center justify-center text-xs font-medium shrink-0 {}",
                            if is_current.get() {
                                "bg-purple-600 text-white"
                            } else if is_completed.get() {
                                "bg-purple-900 text-purple-300"
                            } else {
                                "bg-zinc-800 text-zinc-500"
                            }
                        )>
                            {move || {
                                if is_completed.get() && !is_current.get() {
                                    check_icon().into_any()
                                } else {
                                    view! { <span>{step.index() + 1}</span> }.into_any()
                                }
                            }}
                        </div>

                        // Step label
                        <div class="flex-1 min-w-0">
                            <div class=move || format!(
                                "text-sm truncate {}",
                                if is_current.get() {
                                    "text-white font-medium"
                                } else if is_completed.get() {
                                    "text-zinc-300"
                                } else {
                                    "text-zinc-500"
                                }
                            )>
                                {step.label()}
                            </div>
                            {move || is_current.get().then(|| view! {
                                <div class="text-xs text-zinc-500 truncate">
                                    {step.description()}
                                </div>
                            })}
                        </div>

                        // Skip indicator for skippable steps
                        {step.is_skippable().then(|| view! {
                            <span class="text-[10px] text-zinc-600 uppercase tracking-wider">
                                "Optional"
                            </span>
                        })}
                    </button>
                }
            }).collect_view()}
        </div>
    }
}
