//! Model Selection Dashboard Component
//!
//! Displays Claude Code smart model selection status including:
//! - Current plan badge
//! - Selected model
//! - Usage meters (5h and 7d windows)
//! - Selection reason

use leptos::prelude::*;
use crate::bindings::get_model_selection;
use crate::components::design_system::{Badge, BadgeVariant, Card};

/// A progress bar for displaying usage utilization
#[component]
fn UsageMeter(
    /// Label for the meter (e.g., "5h" or "7d")
    label: &'static str,
    /// Utilization value (0.0 - 1.0)
    value: f64,
    /// Optional reset time string (empty string treated as None)
    #[prop(optional, into)]
    resets_at: String,
) -> impl IntoView {
    let percentage = (value * 100.0).min(100.0);
    let color_class = if percentage >= 90.0 {
        "bg-red-500"
    } else if percentage >= 70.0 {
        "bg-yellow-500"
    } else {
        "bg-green-500"
    };

    view! {
        <div class="space-y-1">
            <div class="flex justify-between text-xs">
                <span class="text-[var(--text-muted)]">{label}</span>
                <span class="text-[var(--text-secondary)]">{format!("{:.0}%", percentage)}</span>
            </div>
            <div class="h-2 bg-[var(--bg-deep)] rounded-full overflow-hidden">
                <div
                    class=format!("h-full {} transition-all duration-300", color_class)
                    style=format!("width: {}%", percentage)
                />
            </div>
            {(!resets_at.is_empty()).then(|| view! {
                <div class="text-[10px] text-[var(--text-muted)]">
                    "Resets: " {resets_at.clone()}
                </div>
            })}
        </div>
    }
}

/// Get badge variant for subscription plan
fn plan_badge_variant(plan: &str) -> BadgeVariant {
    match plan {
        "max_20" => BadgeVariant::Success,  // Max 20x - premium
        "max" => BadgeVariant::Success,     // Max 5x
        "pro" => BadgeVariant::Info,
        "team" | "enterprise" => BadgeVariant::Info,
        "api" => BadgeVariant::Default,
        "free" => BadgeVariant::Default,
        _ => BadgeVariant::Warning,
    }
}

/// Get display name for subscription plan
fn plan_display(plan: &str) -> &'static str {
    match plan {
        "max_20" => "Max 20x",
        "max" => "Max 5x",
        "pro" => "Pro",
        "team" => "Team",
        "enterprise" => "Enterprise",
        "api" => "API",
        "free" => "Free",
        _ => "Unknown",
    }
}

/// Dashboard showing smart model selection status
#[component]
pub fn ModelSelectionDashboard() -> impl IntoView {
    // Fetch model selection on mount using LocalResource for WASM compatibility
    let selection = LocalResource::new(move || async move {
        get_model_selection().await.ok()
    });

    view! {
        <Card class="p-4">
            <Suspense fallback=move || view! {
                <div class="flex items-center justify-center py-4">
                    <div class="animate-pulse text-[var(--text-muted)]">"Loading model selection..."</div>
                </div>
            }>
                {move || {
                    let data = selection.get();
                    let sel_opt = data.as_deref().and_then(|o| o.as_ref());

                    match sel_opt {
                        Some(sel) => {
                            let plan = sel.plan.clone();
                            let plan_for_badge = sel.plan.clone();
                            let model_short = sel.model_short.clone();
                            let override_active = sel.override_active;
                            let five_hour_util = sel.usage.five_hour_util;
                            let seven_day_util = sel.usage.seven_day_util;
                            let five_hour_resets = sel.usage.five_hour_resets_at.clone();
                            let seven_day_resets = sel.usage.seven_day_resets_at.clone();
                            let reason = sel.selection_reason.clone();

                            view! {
                                <div class="space-y-4">
                                    // Header with plan badge
                                    <div class="flex items-center justify-between">
                                        <h4 class="text-sm font-semibold text-[var(--text-primary)]">
                                            "Smart Model Selection"
                                        </h4>
                                        <Badge variant=plan_badge_variant(&plan_for_badge)>
                                            {plan_display(&plan)}
                                        </Badge>
                                    </div>

                                    // Selected model
                                    <div class="flex items-center gap-2">
                                        <span class="text-xs text-[var(--text-muted)]">"Model:"</span>
                                        <span class="text-sm font-medium text-[var(--text-primary)]">
                                            {model_short}
                                        </span>
                                        {override_active.then(|| view! {
                                            <Badge variant=BadgeVariant::Warning class="text-[10px]">
                                                "Override"
                                            </Badge>
                                        })}
                                    </div>

                                    // Usage meters
                                    <div class="grid grid-cols-2 gap-4">
                                        <UsageMeter
                                            label="5h Window"
                                            value=five_hour_util
                                            resets_at=five_hour_resets.unwrap_or_default()
                                        />
                                        <UsageMeter
                                            label="7d Window"
                                            value=seven_day_util
                                            resets_at=seven_day_resets.unwrap_or_default()
                                        />
                                    </div>

                                    // Selection reason
                                    <div class="pt-2 border-t border-[var(--border-subtle)]">
                                        <p class="text-xs text-[var(--text-muted)] italic">
                                            {reason}
                                        </p>
                                    </div>
                                </div>
                            }.into_any()
                        }
                        None => {
                            view! {
                                <div class="text-center py-4">
                                    <p class="text-sm text-[var(--text-muted)]">
                                        "Model selection unavailable"
                                    </p>
                                    <p class="text-xs text-[var(--text-muted)] mt-1">
                                        "Configure Claude Code provider to enable"
                                    </p>
                                </div>
                            }.into_any()
                        }
                    }
                }}
            </Suspense>
        </Card>
    }
}
