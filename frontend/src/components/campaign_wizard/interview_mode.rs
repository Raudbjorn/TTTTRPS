//! Interview Mode for Campaign Wizard
//!
//! Phase 9 of the Campaign Generation Overhaul.
//!
//! Provides a conversational interview flow as an alternative to the
//! standard step-by-step wizard:
//! - One question at a time UI
//! - Suggestion chips (2-4 answers per question)
//! - "I'm stuck" helper with random inspiration
//! - Summarize-and-edit flow at end
//!
//! Design principles:
//! - Conversational, non-intimidating experience
//! - Progressive disclosure of complexity
//! - Easy escape hatches when stuck

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Interview question with suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterviewQuestion {
    pub id: String,
    pub question_text: String,
    pub help_text: Option<String>,
    pub suggestions: Vec<SuggestionChip>,
    pub field_type: FieldType,
    pub is_required: bool,
    pub category: QuestionCategory,
}

/// Suggestion chip for quick answers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionChip {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
    pub icon: Option<String>,
}

/// Field type for the question
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    TextArea,
    Select,
    MultiSelect,
    Number,
    Toggle,
}

/// Question category for grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionCategory {
    Basics,
    Theme,
    Setting,
    Tone,
    Characters,
    Story,
    Advanced,
}

impl QuestionCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            QuestionCategory::Basics => "The Basics",
            QuestionCategory::Theme => "Theme & Genre",
            QuestionCategory::Setting => "World & Setting",
            QuestionCategory::Tone => "Tone & Style",
            QuestionCategory::Characters => "Characters",
            QuestionCategory::Story => "Story Elements",
            QuestionCategory::Advanced => "Advanced Options",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            QuestionCategory::Basics => "text-blue-400",
            QuestionCategory::Theme => "text-purple-400",
            QuestionCategory::Setting => "text-green-400",
            QuestionCategory::Tone => "text-amber-400",
            QuestionCategory::Characters => "text-pink-400",
            QuestionCategory::Story => "text-cyan-400",
            QuestionCategory::Advanced => "text-zinc-400",
        }
    }
}

/// User's answer to a question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterviewAnswer {
    pub question_id: String,
    pub value: String,
    pub suggestion_used: Option<String>,
    pub answered_at: String,
}

/// Interview state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InterviewState {
    pub current_question_index: usize,
    pub answers: Vec<InterviewAnswer>,
    pub is_complete: bool,
    pub started_at: String,
}

/// Inspiration prompt for "I'm stuck" feature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspirationPrompt {
    pub prompt: String,
    pub examples: Vec<String>,
    pub category: String,
}

// ============================================================================
// Default Questions
// ============================================================================

/// Get the default interview questions
pub fn get_default_questions() -> Vec<InterviewQuestion> {
    vec![
        InterviewQuestion {
            id: "campaign_name".to_string(),
            question_text: "What would you like to call your campaign?".to_string(),
            help_text: Some("This can be anything - a cryptic title, a location name, or something evocative of the adventure ahead.".to_string()),
            suggestions: vec![
                SuggestionChip {
                    label: "Shadows of...".to_string(),
                    value: "Shadows of ".to_string(),
                    description: Some("Dark, mysterious tone".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "The [Location]".to_string(),
                    value: "The ".to_string(),
                    description: Some("Classic location-based name".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "Rise of...".to_string(),
                    value: "Rise of ".to_string(),
                    description: Some("Epic, escalating conflict".to_string()),
                    icon: None,
                },
            ],
            field_type: FieldType::Text,
            is_required: true,
            category: QuestionCategory::Basics,
        },
        InterviewQuestion {
            id: "game_system".to_string(),
            question_text: "What game system will you be using?".to_string(),
            help_text: Some("The rules system your campaign will use. This affects character options, combat mechanics, and more.".to_string()),
            suggestions: vec![
                SuggestionChip {
                    label: "D&D 5E".to_string(),
                    value: "D&D 5th Edition".to_string(),
                    description: Some("The most popular fantasy TTRPG".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "Pathfinder 2E".to_string(),
                    value: "Pathfinder 2nd Edition".to_string(),
                    description: Some("Tactical fantasy with deep customization".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "Call of Cthulhu".to_string(),
                    value: "Call of Cthulhu 7th Edition".to_string(),
                    description: Some("Investigative cosmic horror".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "Custom/Other".to_string(),
                    value: "".to_string(),
                    description: Some("I'll specify".to_string()),
                    icon: None,
                },
            ],
            field_type: FieldType::Select,
            is_required: true,
            category: QuestionCategory::Basics,
        },
        InterviewQuestion {
            id: "genre".to_string(),
            question_text: "What genre best describes your campaign?".to_string(),
            help_text: Some("The primary genre influences the mood, themes, and types of stories you'll tell.".to_string()),
            suggestions: vec![
                SuggestionChip {
                    label: "High Fantasy".to_string(),
                    value: "High Fantasy".to_string(),
                    description: Some("Magic, heroes, epic quests".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "Dark Fantasy".to_string(),
                    value: "Dark Fantasy".to_string(),
                    description: Some("Grim, morally complex, dangerous".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "Horror".to_string(),
                    value: "Horror".to_string(),
                    description: Some("Fear, mystery, the unknown".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "Sci-Fi".to_string(),
                    value: "Science Fiction".to_string(),
                    description: Some("Space, technology, futures".to_string()),
                    icon: None,
                },
            ],
            field_type: FieldType::Select,
            is_required: true,
            category: QuestionCategory::Theme,
        },
        InterviewQuestion {
            id: "setting_brief".to_string(),
            question_text: "Describe your setting in a few words or sentences.".to_string(),
            help_text: Some("Where does your story take place? A specific world, era, or location?".to_string()),
            suggestions: vec![
                SuggestionChip {
                    label: "A dying empire...".to_string(),
                    value: "A dying empire on the edge of collapse".to_string(),
                    description: None,
                    icon: None,
                },
                SuggestionChip {
                    label: "An isolated village...".to_string(),
                    value: "An isolated village with dark secrets".to_string(),
                    description: None,
                    icon: None,
                },
                SuggestionChip {
                    label: "A bustling city...".to_string(),
                    value: "A bustling city of intrigue and factions".to_string(),
                    description: None,
                    icon: None,
                },
            ],
            field_type: FieldType::TextArea,
            is_required: false,
            category: QuestionCategory::Setting,
        },
        InterviewQuestion {
            id: "tone".to_string(),
            question_text: "What tone are you aiming for?".to_string(),
            help_text: Some("This affects how NPCs behave, how serious threats are, and the overall mood.".to_string()),
            suggestions: vec![
                SuggestionChip {
                    label: "Heroic".to_string(),
                    value: "Heroic".to_string(),
                    description: Some("The heroes can win, good triumphs".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "Gritty".to_string(),
                    value: "Gritty".to_string(),
                    description: Some("Hard choices, real consequences".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "Light-Hearted".to_string(),
                    value: "Light-Hearted".to_string(),
                    description: Some("Fun, humorous, adventure".to_string()),
                    icon: None,
                },
                SuggestionChip {
                    label: "Grimdark".to_string(),
                    value: "Grimdark".to_string(),
                    description: Some("Bleak, morally gray, brutal".to_string()),
                    icon: None,
                },
            ],
            field_type: FieldType::Select,
            is_required: false,
            category: QuestionCategory::Tone,
        },
        InterviewQuestion {
            id: "initial_hook".to_string(),
            question_text: "What brings the party together?".to_string(),
            help_text: Some("The initial premise or 'hook' that starts the adventure.".to_string()),
            suggestions: vec![
                SuggestionChip {
                    label: "A mysterious letter".to_string(),
                    value: "A mysterious letter summons them to a remote location".to_string(),
                    description: None,
                    icon: None,
                },
                SuggestionChip {
                    label: "They're hired for a job".to_string(),
                    value: "They're hired by a patron for a dangerous job".to_string(),
                    description: None,
                    icon: None,
                },
                SuggestionChip {
                    label: "A disaster strikes".to_string(),
                    value: "A catastrophe forces them to work together to survive".to_string(),
                    description: None,
                    icon: None,
                },
            ],
            field_type: FieldType::TextArea,
            is_required: false,
            category: QuestionCategory::Story,
        },
    ]
}

/// Get random inspiration for a question
pub fn get_inspiration(question_id: &str) -> InspirationPrompt {
    match question_id {
        "campaign_name" => InspirationPrompt {
            prompt: "Try combining a mood word with a place or object".to_string(),
            examples: vec![
                "Whispers of the Deep".to_string(),
                "The Amber Throne".to_string(),
                "Shattered Horizons".to_string(),
            ],
            category: "naming".to_string(),
        },
        "setting_brief" => InspirationPrompt {
            prompt: "Start with a location, add a twist or secret".to_string(),
            examples: vec![
                "A floating city hiding a terrible truth".to_string(),
                "The last forest in a world of ash".to_string(),
                "A kingdom where dreams become real".to_string(),
            ],
            category: "setting".to_string(),
        },
        _ => InspirationPrompt {
            prompt: "Think about what excites you as a storyteller".to_string(),
            examples: vec![
                "What moments do you want to create?".to_string(),
                "What would make this memorable?".to_string(),
            ],
            category: "general".to_string(),
        },
    }
}

// ============================================================================
// Components
// ============================================================================

/// Suggestion chip button
#[component]
fn SuggestionChipButton(
    chip: SuggestionChip,
    on_select: Callback<String>,
    is_selected: bool,
) -> impl IntoView {
    let value = chip.value.clone();

    view! {
        <button
            type="button"
            class=format!(
                "px-4 py-2 rounded-full text-sm transition-all {}",
                if is_selected {
                    "bg-purple-600 text-white ring-2 ring-purple-400"
                } else {
                    "bg-zinc-800 text-zinc-300 hover:bg-zinc-700 hover:text-white"
                }
            )
            on:click=move |_| on_select.run(value.clone())
        >
            <span class="font-medium">{chip.label}</span>
            {chip.description.map(|d| view! {
                <span class="text-xs text-zinc-400 ml-1">"- "{d}</span>
            })}
        </button>
    }
}

/// "I'm stuck" helper panel
#[component]
fn StuckHelper(
    question_id: String,
    on_use_example: Callback<String>,
    on_close: Callback<()>,
) -> impl IntoView {
    let inspiration = get_inspiration(&question_id);

    view! {
        <div class="mt-4 p-4 bg-gradient-to-br from-purple-900/30 to-zinc-900 border border-purple-700/30 rounded-lg">
            <div class="flex items-start justify-between mb-3">
                <div class="flex items-center gap-2">
                    <svg class="w-5 h-5 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                    </svg>
                    <h4 class="font-semibold text-purple-300">"Need some inspiration?"</h4>
                </div>
                <button
                    type="button"
                    class="p-1 text-zinc-500 hover:text-white"
                    on:click=move |_| on_close.run(())
                >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                            d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>

            <p class="text-sm text-zinc-300 mb-3">{inspiration.prompt}</p>

            <div class="space-y-2">
                <p class="text-xs text-zinc-500 uppercase tracking-wider">"Try one of these:"</p>
                {inspiration.examples.iter().map(|example| {
                    let ex = example.clone();
                    view! {
                        <button
                            type="button"
                            class="block w-full text-left px-3 py-2 bg-zinc-800/50 hover:bg-zinc-700/50
                                   rounded text-sm text-zinc-300 hover:text-white transition-colors"
                            on:click=move |_| on_use_example.run(ex.clone())
                        >
                            "\""{ example.clone() }"\""
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Progress indicator for interview
#[component]
fn InterviewProgress(
    current: usize,
    total: usize,
    category: QuestionCategory,
) -> impl IntoView {
    // Guard against division by zero when total is 0
    let percentage = if total == 0 {
        0
    } else {
        ((current as f32 / total as f32) * 100.0) as i32
    };

    view! {
        <div class="mb-6">
            <div class="flex items-center justify-between mb-2">
                <span class=format!("text-xs font-medium {}", category.color())>
                    {category.display_name()}
                </span>
                <span class="text-xs text-zinc-500">
                    {current + 1}" of "{total}
                </span>
            </div>
            <div class="h-1 bg-zinc-800 rounded-full overflow-hidden">
                <div
                    class="h-full bg-purple-600 transition-all duration-300"
                    style=format!("width: {}%", percentage)
                />
            </div>
        </div>
    }
}

/// Single question display
#[component]
fn InterviewQuestionDisplay(
    question: InterviewQuestion,
    current_answer: RwSignal<String>,
    on_submit: Callback<String>,
    on_skip: Callback<()>,
    on_back: Option<Callback<()>>,
) -> impl IntoView {
    let show_stuck_helper = RwSignal::new(false);
    let selected_suggestion = RwSignal::new(Option::<String>::None);
    let question_id = question.id.clone();

    // Handle suggestion selection
    let handle_suggestion = Callback::new(move |value: String| {
        current_answer.set(value.clone());
        selected_suggestion.set(Some(value));
    });

    // Handle inspiration example use
    let handle_use_example = Callback::new(move |example: String| {
        current_answer.set(example);
        show_stuck_helper.set(false);
    });

    view! {
        <div class="space-y-6">
            // Question text
            <div class="space-y-2">
                <h2 class="text-2xl font-bold text-white">
                    {question.question_text.clone()}
                </h2>
                {question.help_text.map(|h| view! {
                    <p class="text-zinc-400">{h}</p>
                })}
            </div>

            // Suggestion chips
            {
                let has_suggestions = !question.suggestions.is_empty();
                let suggestions = question.suggestions.clone();
                view! {
                    <Show when=move || has_suggestions>
                        <div class="flex flex-wrap gap-2">
                            {suggestions.iter().map(|chip| {
                                let chip_value = chip.value.clone();
                                let is_selected = Signal::derive(move || {
                                    selected_suggestion.get().as_ref() == Some(&chip_value)
                                });
                                view! {
                                    <SuggestionChipButton
                                        chip=chip.clone()
                                        on_select=handle_suggestion
                                        is_selected=is_selected.get()
                                    />
                                }
                            }).collect_view()}
                        </div>
                    </Show>
                }
            }

            // Input field
            {match question.field_type {
                FieldType::Text => view! {
                    <input
                        type="text"
                        class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg
                               text-white text-lg placeholder-zinc-500
                               focus:border-purple-500 focus:outline-none focus:ring-1 focus:ring-purple-500"
                        placeholder="Type your answer..."
                        prop:value=move || current_answer.get()
                        on:input=move |ev| current_answer.set(event_target_value(&ev))
                        on:keypress=move |ev| {
                            if ev.key() == "Enter" && !current_answer.get().is_empty() {
                                on_submit.run(current_answer.get());
                            }
                        }
                    />
                }.into_any(),
                FieldType::TextArea => view! {
                    <textarea
                        class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg
                               text-white text-lg placeholder-zinc-500 resize-none
                               focus:border-purple-500 focus:outline-none focus:ring-1 focus:ring-purple-500"
                        rows=4
                        placeholder="Type your answer..."
                        prop:value=move || current_answer.get()
                        on:input=move |ev| current_answer.set(event_target_value(&ev))
                    />
                }.into_any(),
                FieldType::Number => view! {
                    <input
                        type="number"
                        class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg
                               text-white text-lg placeholder-zinc-500
                               focus:border-purple-500 focus:outline-none focus:ring-1 focus:ring-purple-500"
                        placeholder="Enter a number..."
                        prop:value=move || current_answer.get()
                        on:input=move |ev| current_answer.set(event_target_value(&ev))
                        on:keypress=move |ev| {
                            if ev.key() == "Enter" && !current_answer.get().is_empty() {
                                on_submit.run(current_answer.get());
                            }
                        }
                    />
                }.into_any(),
                FieldType::Toggle => {
                    let is_toggled = Signal::derive(move || {
                        current_answer.get().to_lowercase() == "true" || current_answer.get() == "1"
                    });
                    view! {
                        <button
                            type="button"
                            class=move || format!(
                                "relative inline-flex h-10 w-20 shrink-0 cursor-pointer rounded-full border-2 \
                                 border-transparent transition-colors duration-200 ease-in-out \
                                 focus:outline-none focus-visible:ring-2 focus-visible:ring-purple-500 {}",
                                if is_toggled.get() { "bg-purple-600" } else { "bg-zinc-700" }
                            )
                            on:click=move |_| {
                                let new_value = if is_toggled.get() { "false" } else { "true" };
                                current_answer.set(new_value.to_string());
                            }
                        >
                            <span
                                class=move || format!(
                                    "pointer-events-none inline-block h-9 w-9 transform rounded-full \
                                     bg-white shadow-lg ring-0 transition duration-200 ease-in-out {}",
                                    if is_toggled.get() { "translate-x-10" } else { "translate-x-0" }
                                )
                            />
                        </button>
                    }.into_any()
                },
                FieldType::Select => {
                    // For Select, use suggestion chips as options (rendered above)
                    // Fall back to text input if no suggestions available
                    view! {
                        <input
                            type="text"
                            class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg
                                   text-white text-lg placeholder-zinc-500
                                   focus:border-purple-500 focus:outline-none focus:ring-1 focus:ring-purple-500"
                            placeholder="Select from options above or type your answer..."
                            prop:value=move || current_answer.get()
                            on:input=move |ev| current_answer.set(event_target_value(&ev))
                        />
                    }.into_any()
                },
                FieldType::MultiSelect => {
                    // For MultiSelect, suggestion chips handle selection
                    // Text input for comma-separated values
                    view! {
                        <input
                            type="text"
                            class="w-full px-4 py-3 bg-zinc-800 border border-zinc-700 rounded-lg
                                   text-white text-lg placeholder-zinc-500
                                   focus:border-purple-500 focus:outline-none focus:ring-1 focus:ring-purple-500"
                            placeholder="Select multiple options above or type comma-separated values..."
                            prop:value=move || current_answer.get()
                            on:input=move |ev| current_answer.set(event_target_value(&ev))
                        />
                    }.into_any()
                },
            }}

            // "I'm stuck" helper
            <Show when=move || show_stuck_helper.get()>
                <StuckHelper
                    question_id=question_id.clone()
                    on_use_example=handle_use_example
                    on_close=Callback::new(move |_| show_stuck_helper.set(false))
                />
            </Show>

            // Actions
            <div class="flex items-center justify-between pt-4">
                <div class="flex items-center gap-3">
                    // Back button
                    {on_back.map(|cb| view! {
                        <button
                            type="button"
                            class="px-4 py-2 text-zinc-400 hover:text-white transition-colors"
                            on:click=move |_| cb.run(())
                        >
                            "Back"
                        </button>
                    })}

                    // "I'm stuck" button
                    <button
                        type="button"
                        class="flex items-center gap-2 px-4 py-2 text-purple-400 hover:text-purple-300 transition-colors"
                        on:click=move |_| show_stuck_helper.update(|s| *s = !*s)
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                        </svg>
                        "I'm stuck"
                    </button>
                </div>

                <div class="flex items-center gap-3">
                    // Skip button (if not required)
                    <Show when=move || !question.is_required>
                        <button
                            type="button"
                            class="px-4 py-2 text-zinc-500 hover:text-white transition-colors"
                            on:click=move |_| on_skip.run(())
                        >
                            "Skip"
                        </button>
                    </Show>

                    // Continue button
                    <button
                        type="button"
                        class="px-6 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg
                               font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || question.is_required && current_answer.get().is_empty()
                        on:click=move |_| on_submit.run(current_answer.get())
                    >
                        "Continue"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Summary view for review before completion
#[component]
fn InterviewSummary(
    questions: Vec<InterviewQuestion>,
    answers: Vec<InterviewAnswer>,
    on_edit: Callback<usize>,
    on_complete: Callback<()>,
    on_back: Callback<()>,
) -> impl IntoView {
    view! {
        <div class="space-y-6">
            <div class="text-center mb-8">
                <h2 class="text-2xl font-bold text-white mb-2">"Review Your Campaign"</h2>
                <p class="text-zinc-400">"Make sure everything looks good before we create your campaign."</p>
            </div>

            // Answers summary
            <div class="space-y-4">
                {questions.iter().enumerate().map(|(i, q)| {
                    let answer = answers.iter()
                        .find(|a| a.question_id == q.id)
                        .map(|a| a.value.clone())
                        .unwrap_or_else(|| "(not answered)".to_string());
                    let index = i;

                    view! {
                        <div class="p-4 bg-zinc-800 rounded-lg">
                            <div class="flex items-start justify-between">
                                <div class="flex-1">
                                    <p class=format!("text-xs font-medium {} mb-1", q.category.color())>
                                        {q.category.display_name()}
                                    </p>
                                    <h4 class="text-sm text-zinc-400 mb-1">
                                        {q.question_text.clone()}
                                    </h4>
                                    <p class="text-white font-medium">
                                        {answer}
                                    </p>
                                </div>
                                <button
                                    type="button"
                                    class="p-2 text-zinc-500 hover:text-white transition-colors"
                                    title="Edit"
                                    on:click=move |_| on_edit.run(index)
                                >
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                                            d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                                    </svg>
                                </button>
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>

            // Actions
            <div class="flex items-center justify-between pt-4 border-t border-zinc-800">
                <button
                    type="button"
                    class="px-4 py-2 text-zinc-400 hover:text-white transition-colors"
                    on:click=move |_| on_back.run(())
                >
                    "Back"
                </button>

                <button
                    type="button"
                    class="px-6 py-3 bg-purple-600 hover:bg-purple-500 text-white rounded-lg
                           font-bold text-lg transition-colors"
                    on:click=move |_| on_complete.run(())
                >
                    "Create Campaign"
                </button>
            </div>
        </div>
    }
}

// ============================================================================
// Main Interview Mode Component
// ============================================================================

/// Main interview mode component
#[component]
pub fn InterviewMode(
    /// Callback when interview is complete with answers
    on_complete: Callback<Vec<InterviewAnswer>>,
    /// Callback to switch to standard wizard
    #[prop(optional)]
    on_switch_to_wizard: Option<Callback<()>>,
    /// Initial answers (for resume)
    #[prop(default = vec![])]
    initial_answers: Vec<InterviewAnswer>,
) -> impl IntoView {
    let questions = get_default_questions();
    let total_questions = questions.len();

    let current_index = RwSignal::new(0usize);
    let answers = RwSignal::new(initial_answers);
    let current_answer = RwSignal::new(String::new());
    let show_summary = RwSignal::new(false);

    // Clone questions for each closure that needs it
    let questions_for_submit = questions.clone();
    let questions_for_back = questions.clone();
    let questions_for_edit = questions.clone();
    let questions_for_render = questions.clone();
    let questions_for_summary = questions;

    // Handle answer submission
    let handle_submit = Callback::new(move |value: String| {
        let index = current_index.get();
        let q = &questions_for_submit[index];

        // Save answer
        answers.update(|ans| {
            // Remove existing answer for this question
            ans.retain(|a| a.question_id != q.id);
            // Add new answer
            ans.push(InterviewAnswer {
                question_id: q.id.clone(),
                value,
                suggestion_used: None,
                answered_at: chrono::Utc::now().to_rfc3339(),
            });
        });

        // Move to next question or summary
        if index + 1 >= total_questions {
            show_summary.set(true);
        } else {
            current_index.set(index + 1);
            current_answer.set(String::new());
        }
    });

    // Handle skip
    let handle_skip = Callback::new(move |_: ()| {
        let index = current_index.get();
        if index + 1 >= total_questions {
            show_summary.set(true);
        } else {
            current_index.set(index + 1);
            current_answer.set(String::new());
        }
    });

    // Handle back
    let handle_back = Callback::new(move |_: ()| {
        let index = current_index.get();
        if index > 0 {
            current_index.set(index - 1);
            // Restore previous answer
            let q_id = &questions_for_back[index - 1].id;
            let prev_answer = answers.get()
                .iter()
                .find(|a| &a.question_id == q_id)
                .map(|a| a.value.clone())
                .unwrap_or_default();
            current_answer.set(prev_answer);
        }
    });

    // Handle edit from summary
    let handle_edit = Callback::new(move |index: usize| {
        current_index.set(index);
        show_summary.set(false);
        // Restore answer
        let q_id = &questions_for_edit[index].id;
        let prev_answer = answers.get()
            .iter()
            .find(|a| &a.question_id == q_id)
            .map(|a| a.value.clone())
            .unwrap_or_default();
        current_answer.set(prev_answer);
    });

    // Handle complete
    let handle_complete = Callback::new(move |_: ()| {
        on_complete.run(answers.get());
    });

    // Handle back from summary
    let handle_summary_back = Callback::new(move |_: ()| {
        show_summary.set(false);
        current_index.set(total_questions - 1);
    });

    view! {
        <div class="max-w-2xl mx-auto p-6">
            // Switch to wizard link
            {on_switch_to_wizard.map(|cb| view! {
                <div class="mb-6 text-right">
                    <button
                        type="button"
                        class="text-sm text-zinc-500 hover:text-purple-400 transition-colors"
                        on:click=move |_| cb.run(())
                    >
                        "Prefer the standard wizard? Switch"
                    </button>
                </div>
            })}

            <Show
                when=move || show_summary.get()
                fallback=move || {
                    let index = current_index.get();
                    let question = questions_for_render[index].clone();

                    view! {
                        <InterviewProgress
                            current=index
                            total=total_questions
                            category=question.category
                        />

                        <InterviewQuestionDisplay
                            question=question
                            current_answer=current_answer
                            on_submit=handle_submit
                            on_skip=handle_skip
                            on_back={(index > 0).then_some(handle_back)}
                        />
                    }
                }
            >
                <InterviewSummary
                    questions=questions_for_summary.clone()
                    answers=answers.get()
                    on_edit=handle_edit
                    on_complete=handle_complete
                    on_back=handle_summary_back
                />
            </Show>
        </div>
    }
}
