//! Slider Component
//!
//! An accessible range slider component for numeric input.
//! Features:
//!   - Customizable min/max/step values
//!   - Optional label and value display
//!   - Theme-aware styling using CSS variables
//!   - Keyboard accessible
//!   - Optional percentage formatting

use leptos::prelude::*;

/// Slider component props
#[component]
pub fn Slider(
    /// Current value (0.0 - 1.0 for normalized, or any range with min/max)
    #[prop(into)]
    value: RwSignal<f32>,
    /// Minimum value (default: 0.0)
    #[prop(default = 0.0)]
    min: f32,
    /// Maximum value (default: 1.0)
    max: f32,
    /// Step increment (default: 0.01)
    #[prop(default = 0.01)]
    step: f32,
    /// Optional label text
    #[prop(optional, into)]
    label: Option<String>,
    /// Show current value as percentage
    #[prop(default = false)]
    show_percentage: bool,
    /// Show current value as raw number
    #[prop(default = false)]
    show_value: bool,
    /// Callback on value change
    #[prop(optional, into)]
    on_change: Option<Callback<f32>>,
    /// Whether the slider is disabled
    #[prop(default = false)]
    disabled: bool,
    /// Additional CSS classes
    #[prop(optional, into)]
    class: Option<String>,
) -> impl IntoView {
    let on_input = move |e: web_sys::Event| {
        if disabled {
            return;
        }
        if let Some(target) = e.target() {
            if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
                if let Ok(val) = input.value().parse::<f32>() {
                    value.set(val);
                    if let Some(ref cb) = on_change {
                        cb.run(val);
                    }
                }
            }
        }
    };

    // Calculate fill percentage for the track
    let fill_percent = Signal::derive(move || {
        let v = value.get();
        ((v - min) / (max - min) * 100.0).clamp(0.0, 100.0)
    });

    let extra_class = class.unwrap_or_default();

    view! {
        <div class=format!("flex flex-col gap-1.5 {}", extra_class)>
            // Label and value row
            {move || {
                let has_label = label.is_some();
                let show_val = show_percentage || show_value;

                if has_label || show_val {
                    Some(view! {
                        <div class="flex items-center justify-between">
                            {label.clone().map(|l| view! {
                                <label class="text-sm font-medium text-[var(--text-muted)]">
                                    {l}
                                </label>
                            })}
                            {if show_percentage {
                                Some(view! {
                                    <span class="text-xs font-mono text-[var(--text-muted)]">
                                        {move || format!("{}%", (value.get() * 100.0) as i32)}
                                    </span>
                                })
                            } else if show_value {
                                Some(view! {
                                    <span class="text-xs font-mono text-[var(--text-muted)]">
                                        {move || format!("{:.2}", value.get())}
                                    </span>
                                })
                            } else {
                                None
                            }}
                        </div>
                    })
                } else {
                    None
                }
            }}

            // Slider track
            <div class="relative group">
                <input
                    type="range"
                    min=min.to_string()
                    max=max.to_string()
                    step=step.to_string()
                    prop:value=move || value.get()
                    disabled=disabled
                    class=move || format!(
                        "w-full h-2 bg-[var(--bg-surface)] rounded-full appearance-none cursor-pointer
                         [&::-webkit-slider-thumb]:appearance-none
                         [&::-webkit-slider-thumb]:w-4
                         [&::-webkit-slider-thumb]:h-4
                         [&::-webkit-slider-thumb]:rounded-full
                         [&::-webkit-slider-thumb]:bg-[var(--accent)]
                         [&::-webkit-slider-thumb]:shadow-md
                         [&::-webkit-slider-thumb]:cursor-pointer
                         [&::-webkit-slider-thumb]:transition-transform
                         [&::-webkit-slider-thumb]:hover:scale-110
                         [&::-moz-range-thumb]:w-4
                         [&::-moz-range-thumb]:h-4
                         [&::-moz-range-thumb]:rounded-full
                         [&::-moz-range-thumb]:bg-[var(--accent)]
                         [&::-moz-range-thumb]:border-0
                         [&::-moz-range-thumb]:cursor-pointer
                         focus:outline-none focus:ring-2 focus:ring-[var(--accent)] focus:ring-offset-2 focus:ring-offset-[var(--bg-deep)]
                         {}",
                        if disabled { "opacity-50 cursor-not-allowed" } else { "" }
                    )
                    aria-label=move || label.clone().unwrap_or_else(|| "Slider".to_string())
                    aria-valuemin=min.to_string()
                    aria-valuemax=max.to_string()
                    aria-valuenow=move || value.get().to_string()
                    on:input=on_input
                />
                // Fill track overlay
                <div
                    class="absolute top-0 left-0 h-2 bg-[var(--accent)] rounded-full pointer-events-none opacity-60"
                    style:width=move || format!("{}%", fill_percent.get())
                ></div>
            </div>
        </div>
    }
}

/// A horizontal slider with tick marks for discrete values
#[component]
pub fn DiscreteSlider(
    /// Current value
    #[prop(into)]
    value: RwSignal<f32>,
    /// Available tick values
    ticks: Vec<f32>,
    /// Optional labels for each tick
    #[prop(optional)]
    tick_labels: Option<Vec<String>>,
    /// Optional main label
    #[prop(optional, into)]
    label: Option<String>,
    /// Callback on value change
    #[prop(optional, into)]
    on_change: Option<Callback<f32>>,
) -> impl IntoView {
    let min = ticks.first().copied().unwrap_or(0.0);
    let max = ticks.last().copied().unwrap_or(1.0);
    let step = if ticks.len() > 1 {
        (max - min) / (ticks.len() as f32 - 1.0)
    } else {
        1.0
    };

    let ticks_clone = ticks.clone();
    let on_input = move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Some(input) = target.dyn_ref::<web_sys::HtmlInputElement>() {
                if let Ok(val) = input.value().parse::<f32>() {
                    // Snap to nearest tick
                    let closest = ticks_clone
                        .iter()
                        .min_by(|a, b| {
                            ((*a - val).abs())
                                .partial_cmp(&((*b - val).abs()))
                                .unwrap()
                        })
                        .copied()
                        .unwrap_or(val);
                    value.set(closest);
                    if let Some(ref cb) = on_change {
                        cb.run(closest);
                    }
                }
            }
        }
    };

    view! {
        <div class="flex flex-col gap-2">
            {label.map(|l| view! {
                <label class="text-sm font-medium text-[var(--text-muted)]">
                    {l}
                </label>
            })}

            <div class="relative">
                <input
                    type="range"
                    min=min.to_string()
                    max=max.to_string()
                    step=step.to_string()
                    prop:value=move || value.get()
                    class="w-full h-2 bg-[var(--bg-surface)] rounded-full appearance-none cursor-pointer
                           [&::-webkit-slider-thumb]:appearance-none
                           [&::-webkit-slider-thumb]:w-4
                           [&::-webkit-slider-thumb]:h-4
                           [&::-webkit-slider-thumb]:rounded-full
                           [&::-webkit-slider-thumb]:bg-[var(--accent)]
                           [&::-webkit-slider-thumb]:shadow-md
                           [&::-webkit-slider-thumb]:cursor-pointer
                           [&::-moz-range-thumb]:w-4
                           [&::-moz-range-thumb]:h-4
                           [&::-moz-range-thumb]:rounded-full
                           [&::-moz-range-thumb]:bg-[var(--accent)]
                           [&::-moz-range-thumb]:border-0
                           focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                    on:input=on_input
                />

                // Tick marks
                <div class="absolute top-4 left-0 right-0 flex justify-between px-1">
                    {ticks.iter().enumerate().map(|(i, tick)| {
                        let is_active = move || value.get() >= *tick;
                        let label_text = tick_labels.as_ref().and_then(|l| l.get(i).cloned());
                        view! {
                            <div class="flex flex-col items-center">
                                <div class=move || format!(
                                    "w-1 h-1 rounded-full transition-colors {}",
                                    if is_active() { "bg-[var(--accent)]" } else { "bg-[var(--border-subtle)]" }
                                )></div>
                                {label_text.map(|lt| view! {
                                    <span class="text-[10px] text-[var(--text-muted)] mt-1">
                                        {lt}
                                    </span>
                                })}
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}
