//! Review Step - Summary and validation before creation
//!
//! Final step showing all collected data with validation.

use leptos::prelude::*;

use crate::services::wizard_state::{use_wizard_context, WizardStep};

/// Section display component
#[component]
fn ReviewSection(title: &'static str, step: WizardStep, children: Children) -> impl IntoView {
    let ctx = use_wizard_context();
    let is_completed = Signal::derive(move || ctx.is_step_completed(step));

    view! {
        <div class="p-4 bg-zinc-800/50 border border-zinc-700 rounded-lg">
            <div class="flex items-center justify-between mb-3">
                <h4 class="font-medium text-white">{title}</h4>
                {move || if is_completed.get() {
                    view! {
                        <span class="px-2 py-0.5 bg-green-900/50 text-green-400 text-xs rounded-full">
                            "Completed"
                        </span>
                    }.into_any()
                } else {
                    view! {
                        <span class="px-2 py-0.5 bg-zinc-700 text-zinc-400 text-xs rounded-full">
                            "Skipped"
                        </span>
                    }.into_any()
                }}
            </div>
            <div class="text-sm text-zinc-400">
                {children()}
            </div>
        </div>
    }
}

/// Data row display
#[component]
fn DataRow(label: &'static str, value: Signal<String>) -> impl IntoView {
    view! {
        <div class="flex justify-between py-1">
            <span class="text-zinc-500">{label}</span>
            <span class="text-zinc-300">{move || value.get()}</span>
        </div>
    }
}

/// Tag list display
#[component]
fn TagList(tags: Signal<Vec<String>>) -> impl IntoView {
    view! {
        <div class="flex flex-wrap gap-1">
            {move || {
                let t = tags.get();
                if t.is_empty() {
                    view! { <span class="text-zinc-500 text-sm">"None selected"</span> }.into_any()
                } else {
                    t.iter().map(|tag| view! {
                        <span class="px-2 py-0.5 bg-purple-900/30 text-purple-300 text-xs rounded">
                            {tag.clone()}
                        </span>
                    }).collect_view().into_any()
                }
            }}
        </div>
    }
}

/// Review step component
#[component]
pub fn ReviewStep(form_valid: RwSignal<bool>) -> impl IntoView {
    let ctx = use_wizard_context();
    let draft = Signal::derive(move || ctx.draft());

    // Validation checks
    let validation_errors = Signal::derive(move || {
        let d = draft.get();
        let mut errors = Vec::new();

        if d.name.as_ref().map(|n| n.trim().is_empty()).unwrap_or(true) {
            errors.push("Campaign name is required");
        }
        if d.system.is_none() {
            errors.push("Game system is required");
        }
        if d.player_count.is_none() {
            errors.push("Player count is required");
        }

        errors
    });

    let is_valid = Signal::derive(move || validation_errors.get().is_empty());

    Effect::new(move |_| {
        form_valid.set(is_valid.get());
    });

    // Derived display values
    let campaign_name = Signal::derive(move || {
        draft
            .get()
            .name
            .unwrap_or_else(|| "Untitled Campaign".to_string())
    });

    let game_system = Signal::derive(move || {
        draft
            .get()
            .system
            .unwrap_or_else(|| "Not selected".to_string())
    });

    let description = Signal::derive(move || {
        draft
            .get()
            .description
            .unwrap_or_else(|| "No description".to_string())
    });

    let player_count = Signal::derive(move || {
        draft
            .get()
            .player_count
            .map(|c| c.to_string())
            .unwrap_or_else(|| "Not set".to_string())
    });

    let experience_level = Signal::derive(move || {
        draft
            .get()
            .experience_level
            .map(|e| e.label().to_string())
            .unwrap_or_else(|| "Not set".to_string())
    });

    let session_count = Signal::derive(move || {
        draft
            .get()
            .session_scope
            .and_then(|s| s.session_count)
            .map(|c| format!("{} sessions", c))
            .unwrap_or_else(|| "Ongoing".to_string())
    });

    let pacing = Signal::derive(move || {
        draft
            .get()
            .session_scope
            .and_then(|s| s.pacing)
            .map(|p| p.label().to_string())
            .unwrap_or_else(|| "Not set".to_string())
    });

    let themes = Signal::derive(move || draft.get().intent.map(|i| i.themes).unwrap_or_default());

    let tones = Signal::derive(move || {
        draft
            .get()
            .intent
            .map(|i| i.tone_keywords)
            .unwrap_or_default()
    });

    let arc_template = Signal::derive(move || {
        draft
            .get()
            .arc_structure
            .and_then(|a| a.template)
            .map(|t| t.label().to_string())
            .unwrap_or_else(|| "Not selected".to_string())
    });

    let party_size = Signal::derive(move || {
        draft
            .get()
            .party_composition
            .and_then(|p| p.party_size)
            .map(|s| format!("{} characters defined", s))
            .unwrap_or_else(|| "Not defined".to_string())
    });

    let level_range = Signal::derive(move || {
        draft
            .get()
            .party_composition
            .and_then(|p| p.level_range)
            .map(|r| format!("Level {} to {}", r.start_level, r.end_level))
            .unwrap_or_else(|| "Not set".to_string())
    });

    let location_count = Signal::derive(move || {
        draft
            .get()
            .initial_content
            .map(|c| c.locations.len())
            .unwrap_or(0)
    });

    let npc_count = Signal::derive(move || {
        draft
            .get()
            .initial_content
            .map(|c| c.npcs.len())
            .unwrap_or(0)
    });

    let hook_count = Signal::derive(move || {
        draft
            .get()
            .initial_content
            .map(|c| c.plot_hooks.len())
            .unwrap_or(0)
    });

    view! {
        <div class="space-y-6 max-w-3xl mx-auto">
            // Header
            <div class="text-center">
                <h3 class="text-2xl font-bold text-white mb-2">"Review Your Campaign"</h3>
                <p class="text-zinc-400">
                    "Confirm your choices before creating the campaign"
                </p>
            </div>

            // Validation errors
            <Show when=move || !is_valid.get()>
                <div class="p-4 bg-red-900/20 border border-red-800 rounded-lg">
                    <div class="flex items-start gap-3">
                        <svg class="w-5 h-5 text-red-400 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                        </svg>
                        <div>
                            <h4 class="font-medium text-red-400 mb-1">"Missing Required Information"</h4>
                            <ul class="text-sm text-red-300 list-disc list-inside">
                                {move || validation_errors.get().iter().map(|e| view! {
                                    <li>{*e}</li>
                                }).collect_view()}
                            </ul>
                        </div>
                    </div>
                </div>
            </Show>

            // Campaign header preview
            <div class="p-6 bg-gradient-to-br from-purple-900/20 to-zinc-900 border border-purple-500/30 rounded-xl">
                <h2 class="text-2xl font-bold text-white mb-2">{campaign_name}</h2>
                <p class="text-zinc-400 text-sm mb-4">{description}</p>
                <div class="flex items-center gap-4 text-sm">
                    <span class="px-3 py-1 bg-purple-900/50 text-purple-300 rounded-full">{game_system}</span>
                    <span class="text-zinc-400">{move || format!("{} players", player_count.get())}</span>
                    <span class="text-zinc-400">{session_count}</span>
                </div>
            </div>

            // Basics
            <ReviewSection title="Campaign Basics" step=WizardStep::Basics>
                <DataRow label="Name" value=campaign_name />
                <DataRow label="Game System" value=game_system />
            </ReviewSection>

            // Creative Vision
            <ReviewSection title="Creative Vision" step=WizardStep::Intent>
                <div class="space-y-3">
                    <div>
                        <span class="text-zinc-500">"Themes: "</span>
                        <TagList tags=themes />
                    </div>
                    <div>
                        <span class="text-zinc-500">"Tone: "</span>
                        <TagList tags=tones />
                    </div>
                </div>
            </ReviewSection>

            // Scope
            <ReviewSection title="Campaign Scope" step=WizardStep::Scope>
                <DataRow label="Duration" value=session_count />
                <DataRow label="Pacing" value=pacing />
            </ReviewSection>

            // Players
            <ReviewSection title="Players" step=WizardStep::Players>
                <DataRow label="Player Count" value=player_count />
                <DataRow label="Experience Level" value=experience_level />
            </ReviewSection>

            // Party Composition
            <ReviewSection title="Party Composition" step=WizardStep::PartyComposition>
                <DataRow label="Party" value=party_size />
                <DataRow label="Level Range" value=level_range />
            </ReviewSection>

            // Arc Structure
            <ReviewSection title="Story Arc" step=WizardStep::ArcStructure>
                <DataRow label="Template" value=arc_template />
            </ReviewSection>

            // Initial Content
            <ReviewSection title="Initial Content" step=WizardStep::InitialContent>
                <div class="flex gap-4">
                    <span>{move || format!("{} locations", location_count.get())}</span>
                    <span>{move || format!("{} NPCs", npc_count.get())}</span>
                    <span>{move || format!("{} plot hooks", hook_count.get())}</span>
                </div>
            </ReviewSection>

            // Ready message
            <Show when=move || is_valid.get()>
                <div class="p-4 bg-green-900/20 border border-green-800 rounded-lg">
                    <div class="flex items-center gap-3">
                        <svg class="w-5 h-5 text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                        </svg>
                        <div>
                            <h4 class="font-medium text-green-400">"Ready to Create"</h4>
                            <p class="text-sm text-green-300/70">
                                "Your campaign configuration is complete. Click \"Create Campaign\" to begin your adventure."
                            </p>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
