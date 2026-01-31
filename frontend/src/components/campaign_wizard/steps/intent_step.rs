//! Intent Step - Creative vision and campaign themes
//!
//! Captures the GM's creative vision: themes, tone, player experiences.

use leptos::prelude::*;

use crate::services::wizard_state::{use_wizard_context, IntentData, StepData};

/// Theme tag presets
const THEME_PRESETS: &[(&str, &str)] = &[
    ("epic_adventure", "Epic Adventure"),
    ("dark_fantasy", "Dark Fantasy"),
    ("mystery", "Mystery & Intrigue"),
    ("horror", "Horror"),
    ("comedy", "Comedy"),
    ("political", "Political Drama"),
    ("exploration", "Exploration"),
    ("survival", "Survival"),
    ("romance", "Romance"),
    ("redemption", "Redemption"),
];

/// Tone keywords
const TONE_KEYWORDS: &[(&str, &str)] = &[
    ("gritty", "Gritty"),
    ("heroic", "Heroic"),
    ("whimsical", "Whimsical"),
    ("tense", "Tense"),
    ("melancholic", "Melancholic"),
    ("hopeful", "Hopeful"),
    ("mysterious", "Mysterious"),
    ("action_packed", "Action-Packed"),
];

/// Tag chip component
#[component]
fn TagChip(
    label: &'static str,
    is_selected: Signal<bool>,
    on_toggle: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=move || format!(
                "px-3 py-1.5 rounded-full text-sm font-medium transition-all duration-200 {}",
                if is_selected.get() {
                    "bg-purple-600 text-white"
                } else {
                    "bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-zinc-300"
                }
            )
            on:click=move |_| on_toggle.run(())
        >
            {label}
        </button>
    }
}

/// Intent step component
#[component]
pub fn IntentStep(
    form_data: RwSignal<Option<StepData>>,
    form_valid: RwSignal<bool>,
) -> impl IntoView {
    let ctx = use_wizard_context();
    let draft = ctx.draft();
    let intent = draft.intent.unwrap_or_default();

    // Local form state
    let fantasy = RwSignal::new(intent.fantasy);
    let selected_themes = RwSignal::new(intent.themes);
    let selected_tones = RwSignal::new(intent.tone_keywords);
    let player_experiences = RwSignal::new(intent.player_experiences.join("\n"));
    let avoid = RwSignal::new(intent.avoid.join("\n"));

    // This step is always valid (optional)
    Effect::new(move |_| {
        form_valid.set(true);
    });

    // Update form_data when inputs change
    Effect::new(move |_| {
        let exp_lines: Vec<String> = player_experiences
            .get()
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let avoid_lines: Vec<String> = avoid
            .get()
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        form_data.set(Some(StepData::Intent(IntentData {
            fantasy: fantasy.get(),
            themes: selected_themes.get(),
            tone_keywords: selected_tones.get(),
            player_experiences: exp_lines,
            avoid: avoid_lines,
            constraints: vec![], // Not collected in this step
        })));
    });

    // Theme toggle handler
    let toggle_theme = move |theme: String| {
        selected_themes.update(|themes| {
            if themes.contains(&theme) {
                themes.retain(|t| t != &theme);
            } else {
                themes.push(theme);
            }
        });
    };

    // Tone toggle handler
    let toggle_tone = move |tone: String| {
        selected_tones.update(|tones| {
            if tones.contains(&tone) {
                tones.retain(|t| t != &tone);
            } else {
                tones.push(tone);
            }
        });
    };

    view! {
        <div class="space-y-8 max-w-2xl mx-auto">
            // Header
            <div class="text-center">
                <h3 class="text-2xl font-bold text-white mb-2">"Creative Vision"</h3>
                <p class="text-zinc-400">
                    "Define the themes, tone, and experiences you want to create"
                </p>
                <p class="text-xs text-purple-400 mt-2">
                    "This step is optional but helps the AI generate better content"
                </p>
            </div>

            // Fantasy / Vision
            <div class="space-y-2">
                <label class="block text-sm font-medium text-zinc-300">
                    "Your Vision"
                </label>
                <textarea
                    class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white
                           placeholder-zinc-500 focus:border-purple-500 focus:ring-1 focus:ring-purple-500 focus:outline-none resize-none"
                    rows="3"
                    placeholder="Describe the kind of campaign you want to run. What's the core fantasy? e.g., 'A gritty noir mystery in a city of eternal rain, where nothing is as it seems...'"
                    prop:value=move || fantasy.get()
                    on:input=move |ev| fantasy.set(event_target_value(&ev))
                />
            </div>

            // Themes
            <div class="space-y-3">
                <label class="block text-sm font-medium text-zinc-300">
                    "Themes"
                </label>
                <div class="flex flex-wrap gap-2">
                    {THEME_PRESETS.iter().map(|(id, label)| {
                        let id_str = id.to_string();
                        let id_for_check = id_str.clone();
                        let id_for_click = id_str.clone();
                        let is_selected = Signal::derive(move || selected_themes.get().contains(&id_for_check));

                        view! {
                            <TagChip
                                label=*label
                                is_selected=is_selected
                                on_toggle=Callback::new(move |_| toggle_theme(id_for_click.clone()))
                            />
                        }
                    }).collect_view()}
                </div>
                <p class="text-xs text-zinc-500">
                    "Select the main themes that will drive your narrative"
                </p>
            </div>

            // Tone
            <div class="space-y-3">
                <label class="block text-sm font-medium text-zinc-300">
                    "Tone & Atmosphere"
                </label>
                <div class="flex flex-wrap gap-2">
                    {TONE_KEYWORDS.iter().map(|(id, label)| {
                        let id_str = id.to_string();
                        let id_for_check = id_str.clone();
                        let id_for_click = id_str.clone();
                        let is_selected = Signal::derive(move || selected_tones.get().contains(&id_for_check));

                        view! {
                            <TagChip
                                label=*label
                                is_selected=is_selected
                                on_toggle=Callback::new(move |_| toggle_tone(id_for_click.clone()))
                            />
                        }
                    }).collect_view()}
                </div>
            </div>

            // Player Experiences
            <div class="space-y-2">
                <label class="block text-sm font-medium text-zinc-300">
                    "Player Experiences"
                </label>
                <textarea
                    class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white
                           placeholder-zinc-500 focus:border-purple-500 focus:ring-1 focus:ring-purple-500 focus:outline-none resize-none"
                    rows="3"
                    placeholder="What do you want your players to feel or experience? (one per line)
e.g., Feel like cunning rogues pulling off heists
      Experience genuine dread in horror moments"
                    prop:value=move || player_experiences.get()
                    on:input=move |ev| player_experiences.set(event_target_value(&ev))
                />
            </div>

            // Things to Avoid
            <div class="space-y-2">
                <label class="block text-sm font-medium text-zinc-300">
                    "Content to Avoid"
                </label>
                <textarea
                    class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg text-white
                           placeholder-zinc-500 focus:border-purple-500 focus:ring-1 focus:ring-purple-500 focus:outline-none resize-none"
                    rows="2"
                    placeholder="Topics, themes, or content types you want to avoid (one per line)
e.g., Gore, harm to children, spiders"
                    prop:value=move || avoid.get()
                    on:input=move |ev| avoid.set(event_target_value(&ev))
                />
                <p class="text-xs text-zinc-500">
                    "This helps ensure generated content respects your boundaries"
                </p>
            </div>

            // AI Suggestion hint
            {move || ctx.ai_assisted.get().then(|| view! {
                <div class="p-4 bg-purple-900/20 border border-purple-700/50 rounded-lg">
                    <div class="flex items-start gap-3">
                        <svg class="w-5 h-5 text-purple-400 shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                        </svg>
                        <div>
                            <p class="text-sm text-purple-300">
                                "Use the AI panel to brainstorm themes, get suggestions for tone combinations, or refine your vision."
                            </p>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}
