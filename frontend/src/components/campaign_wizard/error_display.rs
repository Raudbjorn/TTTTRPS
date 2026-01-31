//! Error Handling and User Feedback Components
//!
//! Consistent error display, retry buttons, and loading states.

use leptos::prelude::*;

// ============================================================================
// Error Types
// ============================================================================

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorSeverity {
    /// Informational message
    Info,
    /// Warning that doesn't block progress
    Warning,
    /// Error that may be recoverable
    #[default]
    Error,
    /// Critical error that blocks all progress
    Critical,
}

impl ErrorSeverity {
    fn bg_class(&self) -> &'static str {
        match self {
            ErrorSeverity::Info => "bg-blue-900/30 border-blue-700/50",
            ErrorSeverity::Warning => "bg-amber-900/30 border-amber-700/50",
            ErrorSeverity::Error => "bg-red-900/30 border-red-700/50",
            ErrorSeverity::Critical => "bg-red-900/50 border-red-600",
        }
    }

    fn text_class(&self) -> &'static str {
        match self {
            ErrorSeverity::Info => "text-blue-400",
            ErrorSeverity::Warning => "text-amber-400",
            ErrorSeverity::Error => "text-red-400",
            ErrorSeverity::Critical => "text-red-300",
        }
    }

    fn icon_class(&self) -> &'static str {
        match self {
            ErrorSeverity::Info => "text-blue-400",
            ErrorSeverity::Warning => "text-amber-400",
            ErrorSeverity::Error => "text-red-400",
            ErrorSeverity::Critical => "text-red-300",
        }
    }
}

// ============================================================================
// Error Banner Component
// ============================================================================

/// Error banner with optional retry and dismiss actions
#[component]
pub fn ErrorBanner(
    /// Error message to display
    message: Signal<Option<String>>,
    /// Severity of the error
    #[prop(default = ErrorSeverity::Error)]
    severity: ErrorSeverity,
    /// Callback when user dismisses the error
    #[prop(optional)]
    on_dismiss: Option<Callback<()>>,
    /// Callback for retry action
    #[prop(optional)]
    on_retry: Option<Callback<()>>,
    /// Whether retry is in progress
    #[prop(optional)]
    is_retrying: Option<Signal<bool>>,
    /// Custom title (defaults to "Error")
    #[prop(optional)]
    title: Option<&'static str>,
) -> impl IntoView {
    let default_title = match severity {
        ErrorSeverity::Info => "Notice",
        ErrorSeverity::Warning => "Warning",
        ErrorSeverity::Error => "Error",
        ErrorSeverity::Critical => "Critical Error",
    };

    let display_title = title.unwrap_or(default_title);

    view! {
        <Show when=move || message.get().is_some()>
            <div class=format!(
                "p-4 border rounded-lg animate-in fade-in slide-in-from-top-2 duration-200 {}",
                severity.bg_class()
            )>
                <div class="flex items-start gap-3">
                    // Icon
                    <div class=format!("shrink-0 mt-0.5 {}", severity.icon_class())>
                        {match severity {
                            ErrorSeverity::Info => view! {
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                                </svg>
                            }.into_any(),
                            ErrorSeverity::Warning => view! {
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                                </svg>
                            }.into_any(),
                            ErrorSeverity::Error | ErrorSeverity::Critical => view! {
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                        d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                                </svg>
                            }.into_any(),
                        }}
                    </div>

                    // Content
                    <div class="flex-1 min-w-0">
                        <h4 class=format!("font-medium {}", severity.text_class())>
                            {display_title}
                        </h4>
                        <p class="text-sm text-zinc-300 mt-1">
                            {move || message.get().unwrap_or_default()}
                        </p>

                        // Actions
                        {(on_retry.is_some()).then(|| {
                            let on_retry = on_retry.unwrap();
                            let is_retrying = is_retrying.unwrap_or(Signal::derive(|| false));

                            view! {
                                <div class="flex items-center gap-3 mt-3">
                                    <button
                                        type="button"
                                        class=format!(
                                            "px-3 py-1.5 text-sm rounded transition-colors disabled:opacity-50 {}",
                                            match severity {
                                                ErrorSeverity::Info => "bg-blue-700 hover:bg-blue-600 text-white",
                                                ErrorSeverity::Warning => "bg-amber-700 hover:bg-amber-600 text-white",
                                                _ => "bg-red-700 hover:bg-red-600 text-white",
                                            }
                                        )
                                        disabled=move || is_retrying.get()
                                        on:click=move |_| on_retry.run(())
                                    >
                                        {move || if is_retrying.get() { "Retrying..." } else { "Try Again" }}
                                    </button>
                                </div>
                            }
                        })}
                    </div>

                    // Dismiss button
                    {on_dismiss.map(|dismiss| view! {
                        <button
                            type="button"
                            class=format!("shrink-0 hover:opacity-80 transition-opacity {}", severity.text_class())
                            on:click=move |_| dismiss.run(())
                        >
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                            </svg>
                        </button>
                    })}
                </div>
            </div>
        </Show>
    }
}

// ============================================================================
// Inline Error Component
// ============================================================================

/// Small inline error for form fields
#[component]
pub fn InlineError(
    /// Error message
    message: Signal<Option<String>>,
) -> impl IntoView {
    view! {
        <Show when=move || message.get().is_some()>
            <div class="flex items-center gap-1.5 mt-1 text-sm text-red-400 animate-in fade-in slide-in-from-top-1 duration-150">
                <svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span>{move || message.get().unwrap_or_default()}</span>
            </div>
        </Show>
    }
}

// ============================================================================
// Loading States
// ============================================================================

/// Loading overlay for sections
#[component]
pub fn LoadingOverlay(
    /// Whether loading is in progress
    is_loading: Signal<bool>,
    /// Loading message
    #[prop(default = "Loading...")]
    message: &'static str,
    /// Whether to blur the background
    #[prop(default = true)]
    blur_background: bool,
) -> impl IntoView {
    view! {
        <Show when=move || is_loading.get()>
            <div class=format!(
                "absolute inset-0 flex items-center justify-center z-40 {}",
                if blur_background { "bg-zinc-900/80 backdrop-blur-sm" } else { "bg-zinc-900/60" }
            )>
                <div class="flex flex-col items-center gap-4">
                    <div class="w-10 h-10 border-4 border-purple-600 border-t-transparent rounded-full animate-spin" />
                    <span class="text-sm text-zinc-400">{message}</span>
                </div>
            </div>
        </Show>
    }
}

/// Inline loading indicator
#[component]
pub fn InlineLoading(
    /// Loading message
    #[prop(default = "Loading...")]
    message: &'static str,
    /// Size of the spinner
    #[prop(default = "w-4 h-4")]
    size: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2 text-zinc-400">
            <svg class=format!("{} animate-spin", size) fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
            </svg>
            <span class="text-sm">{message}</span>
        </div>
    }
}

/// Button with loading state
#[component]
pub fn LoadingButton(
    /// Whether loading is in progress
    is_loading: Signal<bool>,
    /// Whether button is disabled (in addition to loading)
    #[prop(optional)]
    disabled: Option<Signal<bool>>,
    /// Button text when not loading
    text: &'static str,
    /// Button text when loading
    #[prop(default = "Loading...")]
    loading_text: &'static str,
    /// Click handler
    on_click: Callback<()>,
    /// Additional CSS classes
    #[prop(default = "")]
    class: &'static str,
    /// Button variant
    #[prop(default = ButtonLoadingVariant::Primary)]
    variant: ButtonLoadingVariant,
) -> impl IntoView {
    let is_disabled = Signal::derive(move || {
        is_loading.get() || disabled.map(|d| d.get()).unwrap_or(false)
    });

    let variant_class = match variant {
        ButtonLoadingVariant::Primary => "bg-purple-600 hover:bg-purple-500 text-white",
        ButtonLoadingVariant::Secondary => "bg-zinc-700 hover:bg-zinc-600 text-white",
        ButtonLoadingVariant::Danger => "bg-red-600 hover:bg-red-500 text-white",
        ButtonLoadingVariant::Success => "bg-green-600 hover:bg-green-500 text-white",
    };

    view! {
        <button
            type="button"
            class=format!(
                "px-4 py-2 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2 {} {}",
                variant_class,
                class
            )
            disabled=move || is_disabled.get()
            on:click=move |_| {
                if !is_disabled.get() {
                    on_click.run(());
                }
            }
        >
            {move || if is_loading.get() {
                view! {
                    <>
                        <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
                        </svg>
                        <span>{loading_text}</span>
                    </>
                }.into_any()
            } else {
                view! { <span>{text}</span> }.into_any()
            }}
        </button>
    }
}

/// Button variant for loading buttons
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonLoadingVariant {
    #[default]
    Primary,
    Secondary,
    Danger,
    Success,
}

// ============================================================================
// Skeleton Loading
// ============================================================================

/// Skeleton placeholder for loading content
#[component]
pub fn Skeleton(
    /// Width class
    #[prop(default = "w-full")]
    width: &'static str,
    /// Height class
    #[prop(default = "h-4")]
    height: &'static str,
    /// Whether to animate
    #[prop(default = true)]
    animate: bool,
) -> impl IntoView {
    view! {
        <div class=format!(
            "bg-zinc-700 rounded {} {} {}",
            width,
            height,
            if animate { "animate-pulse" } else { "" }
        ) />
    }
}

/// Skeleton for a card
#[component]
pub fn SkeletonCard() -> impl IntoView {
    view! {
        <div class="bg-zinc-800 border border-zinc-700 rounded-lg p-4 space-y-3">
            <Skeleton width="w-3/4" height="h-5" />
            <Skeleton width="w-full" height="h-3" />
            <Skeleton width="w-full" height="h-3" />
            <Skeleton width="w-1/2" height="h-3" />
        </div>
    }
}

// ============================================================================
// Graceful Degradation Messages
// ============================================================================

/// Message shown when a feature is unavailable
#[component]
pub fn FeatureUnavailable(
    /// Feature name
    feature: &'static str,
    /// Reason it's unavailable
    reason: &'static str,
    /// Alternative action text
    #[prop(optional)]
    alternative: Option<&'static str>,
    /// Action callback
    #[prop(optional)]
    on_action: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <div class="p-4 bg-zinc-800/50 border border-zinc-700 rounded-lg">
            <div class="flex items-start gap-3">
                <div class="p-2 bg-zinc-700/50 rounded-lg shrink-0">
                    <svg class="w-5 h-5 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M18.364 18.364A9 9 0 005.636 5.636m12.728 12.728A9 9 0 015.636 5.636m12.728 12.728L5.636 5.636" />
                    </svg>
                </div>
                <div class="flex-1">
                    <h4 class="font-medium text-zinc-300">{feature} " Unavailable"</h4>
                    <p class="text-sm text-zinc-500 mt-1">{reason}</p>
                    {alternative.map(|alt| {
                        let on_action = on_action;
                        view! {
                            <button
                                type="button"
                                class="mt-2 text-sm text-purple-400 hover:text-purple-300 underline"
                                on:click=move |_| {
                                    if let Some(cb) = on_action {
                                        cb.run(());
                                    }
                                }
                            >
                                {alt}
                            </button>
                        }
                    })}
                </div>
            </div>
        </div>
    }
}

// ============================================================================
// Empty State
// ============================================================================

/// Empty state placeholder
#[component]
pub fn EmptyState(
    /// Icon (as SVG path)
    #[prop(optional)]
    icon: Option<&'static str>,
    /// Title
    title: &'static str,
    /// Description
    description: &'static str,
    /// Action button text
    #[prop(optional)]
    action_text: Option<&'static str>,
    /// Action callback
    #[prop(optional)]
    on_action: Option<Callback<()>>,
) -> impl IntoView {
    view! {
        <div class="flex flex-col items-center justify-center py-12 px-4 text-center">
            {icon.map(|path| view! {
                <div class="w-16 h-16 rounded-full bg-zinc-800 flex items-center justify-center mb-4">
                    <svg class="w-8 h-8 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={path} />
                    </svg>
                </div>
            })}
            <h3 class="text-lg font-medium text-white">{title}</h3>
            <p class="text-sm text-zinc-400 mt-1 max-w-sm">{description}</p>
            {action_text.map(|text| {
                let on_action = on_action;
                view! {
                    <button
                        type="button"
                        class="mt-4 px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors"
                        on:click=move |_| {
                            if let Some(cb) = on_action {
                                cb.run(());
                            }
                        }
                    >
                        {text}
                    </button>
                }
            })}
        </div>
    }
}
