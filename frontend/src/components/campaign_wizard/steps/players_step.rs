//! Players Step - Player count and experience level
//!
//! Configure the number of players and their general experience level.

use leptos::prelude::*;

use crate::services::wizard_state::{use_wizard_context, ExperienceLevel, PlayersData, StepData};

/// Players step component
#[component]
pub fn PlayersStep(
    form_data: RwSignal<Option<StepData>>,
    form_valid: RwSignal<bool>,
) -> impl IntoView {
    let ctx = use_wizard_context();
    let draft = ctx.draft();

    // Local form state
    let player_count = RwSignal::new(draft.player_count.unwrap_or(4));
    let experience_level = RwSignal::new(draft.experience_level.unwrap_or_default());

    // This step is always valid (player count has default)
    Effect::new(move |_| {
        form_valid.set(true);
    });

    // Update form_data when inputs change
    Effect::new(move |_| {
        form_data.set(Some(StepData::Players(PlayersData {
            player_count: player_count.get(),
            experience_level: Some(experience_level.get()),
        })));
    });

    view! {
        <div class="space-y-8 max-w-2xl mx-auto">
            // Header
            <div class="text-center">
                <h3 class="text-2xl font-bold text-white mb-2">"Your Players"</h3>
                <p class="text-zinc-400">
                    "Tell us about your group to help tailor content suggestions"
                </p>
            </div>

            // Player Count
            <div class="space-y-4">
                <label class="block text-sm font-medium text-zinc-300">
                    "Number of Players"
                </label>

                <div class="flex items-center justify-center gap-4">
                    // Decrease button
                    <button
                        type="button"
                        class="w-12 h-12 rounded-lg bg-zinc-800 border border-zinc-700 text-white text-xl font-bold
                               hover:bg-zinc-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        disabled=move || player_count.get() <= 1
                        on:click=move |_| player_count.update(|c| *c = (*c).saturating_sub(1).max(1))
                    >
                        "-"
                    </button>

                    // Count display
                    <div class="w-24 h-24 rounded-xl bg-zinc-800 border border-zinc-700 flex items-center justify-center">
                        <span class="text-4xl font-bold text-white">
                            {move || player_count.get()}
                        </span>
                    </div>

                    // Increase button
                    <button
                        type="button"
                        class="w-12 h-12 rounded-lg bg-zinc-800 border border-zinc-700 text-white text-xl font-bold
                               hover:bg-zinc-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        disabled=move || player_count.get() >= 10
                        on:click=move |_| player_count.update(|c| *c = (*c + 1).min(10))
                    >
                        "+"
                    </button>
                </div>

                // Quick select buttons
                <div class="flex justify-center gap-2">
                    {[2, 3, 4, 5, 6].into_iter().map(|count| {
                        let is_selected = move || player_count.get() == count;

                        view! {
                            <button
                                type="button"
                                class=move || format!(
                                    "px-4 py-2 rounded-lg text-sm font-medium transition-colors {}",
                                    if is_selected() {
                                        "bg-purple-600 text-white"
                                    } else {
                                        "bg-zinc-800 text-zinc-400 hover:bg-zinc-700"
                                    }
                                )
                                on:click=move |_| player_count.set(count)
                            >
                                {count}
                            </button>
                        }
                    }).collect_view()}
                </div>

                <p class="text-xs text-zinc-500 text-center">
                    "Not counting the GM"
                </p>
            </div>

            // Experience Level
            <div class="space-y-3">
                <label class="block text-sm font-medium text-zinc-300">
                    "Player Experience Level"
                </label>
                <div class="grid grid-cols-2 gap-3">
                    {ExperienceLevel::all().into_iter().map(|level| {
                        let is_selected = move || experience_level.get() == level;
                        let description = match level {
                            ExperienceLevel::Beginner => "New to TTRPGs",
                            ExperienceLevel::Intermediate => "A few campaigns under their belt",
                            ExperienceLevel::Experienced => "Seasoned players",
                            ExperienceLevel::Mixed => "Variety of experience levels",
                        };

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
                                on:click=move |_| experience_level.set(level)
                            >
                                <div class="font-medium text-white">{level.label()}</div>
                                <div class="text-xs text-zinc-400 mt-1">{description}</div>
                            </button>
                        }
                    }).collect_view()}
                </div>
                <p class="text-xs text-zinc-500">
                    "This affects complexity recommendations and tutorial suggestions"
                </p>
            </div>

            // Party size recommendation
            <div class="p-4 bg-zinc-800/50 border border-zinc-700 rounded-lg">
                <h4 class="text-sm font-medium text-zinc-300 mb-2">"Party Size Notes"</h4>
                <div class="text-sm text-zinc-400 space-y-2">
                    {move || {
                        let count = player_count.get();
                        let exp = experience_level.get();

                        let party_note = match count {
                            1 => "Solo campaign - consider sidekick NPCs or gestalt rules",
                            2 => "Small party - may need extra NPC support or adjusted encounters",
                            3 => "Flexible size - works well for most content",
                            4 => "Ideal party size - most content is balanced for 4 players",
                            5 => "Standard party - slight adjustment to encounters recommended",
                            6 => "Large party - encounters may need scaling up",
                            7..=10 => "Very large group - combat may slow down, consider splitting",
                            _ => "",
                        };

                        let exp_note = match exp {
                            ExperienceLevel::Beginner => "Consider starting with straightforward encounters and clear objectives",
                            ExperienceLevel::Intermediate => "Can handle moderate complexity and some tactical depth",
                            ExperienceLevel::Experienced => "Ready for complex encounters and nuanced roleplay",
                            ExperienceLevel::Mixed => "Balance content to engage all experience levels",
                        };

                        view! {
                            <p>{party_note}</p>
                            <p class="text-zinc-500">{exp_note}</p>
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
