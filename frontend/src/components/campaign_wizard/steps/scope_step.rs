//! Scope Step - Campaign duration and pacing
//!
//! Configure session count, duration, and overall pacing preferences.

use leptos::prelude::*;

use crate::services::wizard_state::{use_wizard_context, CampaignPacing, ScopeData, StepData};

/// Duration preset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DurationPreset {
    OneShot,
    ShortArc,
    MediumCampaign,
    LongCampaign,
    Ongoing,
    Custom,
}

impl DurationPreset {
    fn label(&self) -> &'static str {
        match self {
            DurationPreset::OneShot => "One-Shot",
            DurationPreset::ShortArc => "Short Arc",
            DurationPreset::MediumCampaign => "Medium Campaign",
            DurationPreset::LongCampaign => "Long Campaign",
            DurationPreset::Ongoing => "Ongoing",
            DurationPreset::Custom => "Custom",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            DurationPreset::OneShot => "1 session, complete story",
            DurationPreset::ShortArc => "3-6 sessions, focused arc",
            DurationPreset::MediumCampaign => "10-20 sessions, full story",
            DurationPreset::LongCampaign => "30+ sessions, epic journey",
            DurationPreset::Ongoing => "No set end, continuous play",
            DurationPreset::Custom => "Set your own parameters",
        }
    }

    fn session_count(&self) -> Option<u32> {
        match self {
            DurationPreset::OneShot => Some(1),
            DurationPreset::ShortArc => Some(5),
            DurationPreset::MediumCampaign => Some(15),
            DurationPreset::LongCampaign => Some(40),
            DurationPreset::Ongoing => None,
            DurationPreset::Custom => None,
        }
    }

    fn all() -> Vec<Self> {
        vec![
            DurationPreset::OneShot,
            DurationPreset::ShortArc,
            DurationPreset::MediumCampaign,
            DurationPreset::LongCampaign,
            DurationPreset::Ongoing,
            DurationPreset::Custom,
        ]
    }
}

/// Scope step component
#[component]
pub fn ScopeStep(
    form_data: RwSignal<Option<StepData>>,
    form_valid: RwSignal<bool>,
) -> impl IntoView {
    let ctx = use_wizard_context();
    let draft = ctx.draft();
    let scope = draft.session_scope.unwrap_or_default();

    // Local form state
    let duration_preset = RwSignal::new(DurationPreset::MediumCampaign);
    let session_count = RwSignal::new(
        scope
            .session_count
            .map(|c| c.to_string())
            .unwrap_or_else(|| "15".to_string()),
    );
    let session_duration = RwSignal::new(
        scope
            .session_duration_hours
            .map(|h| h.to_string())
            .unwrap_or_else(|| "3".to_string()),
    );
    let pacing = RwSignal::new(scope.pacing.unwrap_or_default());

    // This step is required - must have valid numbers
    let is_valid = Signal::derive(move || {
        let preset = duration_preset.get();
        if preset == DurationPreset::Ongoing {
            return true;
        }

        let count_valid = session_count.get().parse::<u32>().is_ok();
        let duration_valid = session_duration.get().parse::<f32>().is_ok();

        count_valid && duration_valid
    });

    Effect::new(move |_| {
        form_valid.set(is_valid.get());
    });

    // Update form_data when inputs change
    Effect::new(move |_| {
        let preset = duration_preset.get();
        let count: Option<u32> = if preset == DurationPreset::Ongoing {
            None
        } else {
            session_count.get().parse().ok()
        };

        form_data.set(Some(StepData::Scope(ScopeData {
            session_count: count,
            session_duration_hours: session_duration.get().parse().ok(),
            pacing: Some(pacing.get()),
            duration_months: None, // Could calculate from session count and frequency
        })));
    });

    // Handle preset selection
    let select_preset = move |preset: DurationPreset| {
        duration_preset.set(preset);
        if let Some(count) = preset.session_count() {
            session_count.set(count.to_string());
        }
    };

    view! {
        <div class="space-y-8 max-w-2xl mx-auto">
            // Header
            <div class="text-center">
                <h3 class="text-2xl font-bold text-white mb-2">"Campaign Scope"</h3>
                <p class="text-zinc-400">
                    "Define the length and pacing of your campaign"
                </p>
            </div>

            // Duration Presets
            <div class="space-y-3">
                <label class="block text-sm font-medium text-zinc-300">
                    "Campaign Duration"
                </label>
                <div class="grid grid-cols-2 md:grid-cols-3 gap-3">
                    {DurationPreset::all().into_iter().map(|preset| {
                        let is_selected = move || duration_preset.get() == preset;

                        view! {
                            <button
                                type="button"
                                class=move || format!(
                                    "p-4 rounded-lg border text-left transition-all duration-200 {}",
                                    if is_selected() {
                                        "bg-purple-900/30 border-purple-500 ring-1 ring-purple-500/50"
                                    } else {
                                        "bg-zinc-800/50 border-zinc-700 hover:border-zinc-600 hover:bg-zinc-800"
                                    }
                                )
                                on:click=move |_| select_preset(preset)
                            >
                                <div class="font-medium text-white">{preset.label()}</div>
                                <div class="text-xs text-zinc-400 mt-1">{preset.description()}</div>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>

            // Custom session count (when not ongoing)
            <Show when=move || duration_preset.get() != DurationPreset::Ongoing>
                <div class="grid grid-cols-2 gap-6">
                    // Session count
                    <div class="space-y-2">
                        <label class="block text-sm font-medium text-zinc-300">
                            "Number of Sessions"
                        </label>
                        <input
                            type="number"
                            min="1"
                            max="200"
                            class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white
                                   placeholder-zinc-500 focus:border-purple-500 focus:ring-1 focus:ring-purple-500 focus:outline-none"
                            prop:value=move || session_count.get()
                            on:input=move |ev| {
                                session_count.set(event_target_value(&ev));
                                duration_preset.set(DurationPreset::Custom);
                            }
                        />
                        <p class="text-xs text-zinc-500">"Approximate number of sessions"</p>
                    </div>

                    // Session duration
                    <div class="space-y-2">
                        <label class="block text-sm font-medium text-zinc-300">
                            "Session Length (hours)"
                        </label>
                        <input
                            type="number"
                            min="1"
                            max="12"
                            step="0.5"
                            class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white
                                   placeholder-zinc-500 focus:border-purple-500 focus:ring-1 focus:ring-purple-500 focus:outline-none"
                            prop:value=move || session_duration.get()
                            on:input=move |ev| session_duration.set(event_target_value(&ev))
                        />
                        <p class="text-xs text-zinc-500">"Typical session length"</p>
                    </div>
                </div>
            </Show>

            // Pacing
            <div class="space-y-3">
                <label class="block text-sm font-medium text-zinc-300">
                    "Pacing Style"
                </label>
                <div class="grid grid-cols-2 gap-3">
                    {CampaignPacing::all().into_iter().map(|p| {
                        let is_selected = move || pacing.get() == p;

                        view! {
                            <button
                                type="button"
                                class=move || format!(
                                    "p-4 rounded-lg border text-left transition-all duration-200 {}",
                                    if is_selected() {
                                        "bg-purple-900/30 border-purple-500 ring-1 ring-purple-500/50"
                                    } else {
                                        "bg-zinc-800/50 border-zinc-700 hover:border-zinc-600 hover:bg-zinc-800"
                                    }
                                )
                                on:click=move |_| pacing.set(p)
                            >
                                <div class="font-medium text-white">{p.label()}</div>
                                <div class="text-xs text-zinc-400 mt-1">{p.description()}</div>
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>

            // Summary box
            <div class="p-4 bg-zinc-800/50 border border-zinc-700 rounded-lg">
                <h4 class="text-sm font-medium text-zinc-300 mb-2">"Campaign Summary"</h4>
                <div class="text-sm text-zinc-400 space-y-1">
                    {move || {
                        let preset = duration_preset.get();
                        let count = session_count.get().parse::<u32>().ok();
                        let duration = session_duration.get().parse::<f32>().ok();
                        let pace = pacing.get();

                        let count_text = if preset == DurationPreset::Ongoing {
                            "Ongoing (no set end)".to_string()
                        } else {
                            count.map(|c| format!("{} sessions", c)).unwrap_or_else(|| "? sessions".to_string())
                        };

                        let duration_text = duration
                            .map(|d| format!("{:.1} hours each", d))
                            .unwrap_or_else(|| "? hours each".to_string());

                        let total_hours = count.zip(duration).map(|(c, d)| c as f32 * d);
                        let total_text = total_hours
                            .map(|h| format!("~{:.0} hours total", h))
                            .unwrap_or_default();

                        view! {
                            <p>{count_text}" at "{duration_text}</p>
                            <p class="text-zinc-500">{total_text}</p>
                            <p>"Pacing: "{pace.label()}</p>
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
