//! Tooltips and Help Text Components
//!
//! Provides tooltip explanations for TTRPG terminology and contextual help.

use leptos::prelude::*;

// ============================================================================
// Tooltip Component
// ============================================================================

/// Position for tooltip display
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TooltipPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

impl TooltipPosition {
    fn classes(&self) -> &'static str {
        match self {
            TooltipPosition::Top => "bottom-full left-1/2 -translate-x-1/2 mb-2",
            TooltipPosition::Bottom => "top-full left-1/2 -translate-x-1/2 mt-2",
            TooltipPosition::Left => "right-full top-1/2 -translate-y-1/2 mr-2",
            TooltipPosition::Right => "left-full top-1/2 -translate-y-1/2 ml-2",
        }
    }

    fn arrow_classes(&self) -> &'static str {
        match self {
            TooltipPosition::Top => "top-full left-1/2 -translate-x-1/2 border-t-zinc-700 border-x-transparent border-b-transparent",
            TooltipPosition::Bottom => "bottom-full left-1/2 -translate-x-1/2 border-b-zinc-700 border-x-transparent border-t-transparent",
            TooltipPosition::Left => "left-full top-1/2 -translate-y-1/2 border-l-zinc-700 border-y-transparent border-r-transparent",
            TooltipPosition::Right => "right-full top-1/2 -translate-y-1/2 border-r-zinc-700 border-y-transparent border-l-transparent",
        }
    }
}

/// Basic tooltip wrapper
#[component]
pub fn Tooltip(
    /// Tooltip content
    text: &'static str,
    /// Position of the tooltip
    #[prop(default = TooltipPosition::Top)]
    position: TooltipPosition,
    /// Child content that triggers tooltip
    children: Children,
) -> impl IntoView {
    let is_visible = RwSignal::new(false);

    view! {
        <div
            class="relative inline-flex"
            on:mouseenter=move |_| is_visible.set(true)
            on:mouseleave=move |_| is_visible.set(false)
            on:focus=move |_| is_visible.set(true)
            on:blur=move |_| is_visible.set(false)
        >
            {children()}

            <Show when=move || is_visible.get()>
                <div
                    class=format!(
                        "absolute z-50 px-3 py-2 text-sm text-white bg-zinc-800 border border-zinc-700 rounded-lg shadow-lg whitespace-nowrap animate-in fade-in zoom-in-95 duration-150 {}",
                        position.classes()
                    )
                    role="tooltip"
                >
                    {text}
                    // Arrow
                    <div class=format!(
                        "absolute w-0 h-0 border-4 {}",
                        position.arrow_classes()
                    ) />
                </div>
            </Show>
        </div>
    }
}

/// Rich tooltip with title and description
#[component]
pub fn RichTooltip(
    /// Title text
    title: &'static str,
    /// Description text
    description: &'static str,
    /// Position of the tooltip
    #[prop(default = TooltipPosition::Top)]
    position: TooltipPosition,
    /// Maximum width class
    #[prop(default = "max-w-xs")]
    max_width: &'static str,
    /// Child content that triggers tooltip
    children: Children,
) -> impl IntoView {
    let is_visible = RwSignal::new(false);

    view! {
        <div
            class="relative inline-flex"
            tabindex="0"
            on:mouseenter=move |_| is_visible.set(true)
            on:mouseleave=move |_| is_visible.set(false)
            on:focus=move |_| is_visible.set(true)
            on:blur=move |_| is_visible.set(false)
        >
            {children()}

            <Show when=move || is_visible.get()>
                <div
                    class=format!(
                        "absolute z-50 p-3 bg-zinc-800 border border-zinc-700 rounded-lg shadow-lg animate-in fade-in zoom-in-95 duration-150 {} {}",
                        max_width,
                        position.classes()
                    )
                    role="tooltip"
                >
                    <div class="font-medium text-white text-sm mb-1">{title}</div>
                    <div class="text-xs text-zinc-400 leading-relaxed">{description}</div>
                </div>
            </Show>
        </div>
    }
}

// ============================================================================
// Help Icon Component
// ============================================================================

/// Help icon that shows a tooltip on hover
#[component]
pub fn HelpIcon(
    /// Tooltip text
    text: &'static str,
    /// Position of the tooltip
    #[prop(default = TooltipPosition::Top)]
    position: TooltipPosition,
    /// Size class for the icon
    #[prop(default = "w-4 h-4")]
    size: &'static str,
) -> impl IntoView {
    view! {
        <Tooltip text=text position=position>
            <button
                type="button"
                class=format!(
                    "inline-flex items-center justify-center text-zinc-500 hover:text-zinc-400 transition-colors cursor-help {}",
                    size
                )
            >
                <svg class="w-full h-full" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
            </button>
        </Tooltip>
    }
}

/// Info icon with expandable help text
#[component]
pub fn HelpExpander(
    /// Title for the help section
    title: &'static str,
    /// Help content
    content: &'static str,
    /// Whether expanded by default
    #[prop(default = false)]
    default_expanded: bool,
) -> impl IntoView {
    let is_expanded = RwSignal::new(default_expanded);

    view! {
        <div class="border border-zinc-700/50 rounded-lg overflow-hidden">
            <button
                type="button"
                class="w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-zinc-800/50 transition-colors"
                on:click=move |_| is_expanded.update(|e| *e = !*e)
            >
                <svg
                    class=move || format!(
                        "w-4 h-4 text-zinc-500 transition-transform duration-200 {}",
                        if is_expanded.get() { "rotate-90" } else { "" }
                    )
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                </svg>
                <svg class="w-4 h-4 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span class="text-sm font-medium text-zinc-300">{title}</span>
            </button>
            <Show when=move || is_expanded.get()>
                <div class="px-3 pb-3 pt-1 text-sm text-zinc-400 leading-relaxed border-t border-zinc-700/50 bg-zinc-800/30">
                    {content}
                </div>
            </Show>
        </div>
    }
}

// ============================================================================
// TTRPG Terminology Definitions
// ============================================================================

/// Common TTRPG terms and their explanations
pub mod terminology {
    use super::*;

    /// Get the definition for a TTRPG term
    pub fn get_definition(term: &str) -> Option<(&'static str, &'static str)> {
        match term.to_lowercase().as_str() {
            "session" => Some((
                "Session",
                "A single play session, typically lasting 3-4 hours where players gather to play."
            )),
            "campaign" => Some((
                "Campaign",
                "A series of connected sessions telling an ongoing story with the same characters."
            )),
            "arc" => Some((
                "Story Arc",
                "A narrative segment with its own beginning, middle, and end within a larger campaign."
            )),
            "gm" | "dm" | "game master" | "dungeon master" => Some((
                "Game Master (GM)",
                "The player who runs the game, controlling NPCs and narrating the story."
            )),
            "npc" | "non-player character" => Some((
                "NPC",
                "Non-Player Character - any character controlled by the GM rather than a player."
            )),
            "pc" | "player character" => Some((
                "Player Character",
                "A character controlled by one of the players rather than the GM."
            )),
            "encounter" => Some((
                "Encounter",
                "A scene where characters face a challenge - combat, social, exploration, or puzzle."
            )),
            "initiative" => Some((
                "Initiative",
                "The turn order in combat, typically determined by dice rolls at the start of a fight."
            )),
            "sandbox" => Some((
                "Sandbox",
                "An open-world style where players drive the story through their choices."
            )),
            "railroad" | "railroading" => Some((
                "Railroading",
                "When a GM forces players down a predetermined path, limiting meaningful choices."
            )),
            "homebrew" => Some((
                "Homebrew",
                "Custom rules, settings, or content created by the GM rather than published material."
            )),
            "session zero" => Some((
                "Session Zero",
                "A planning session before the campaign starts to discuss expectations and create characters."
            )),
            "oneshot" | "one-shot" => Some((
                "One-Shot",
                "A complete adventure designed to be played in a single session."
            )),
            "tpk" | "total party kill" => Some((
                "TPK",
                "Total Party Kill - when all player characters die in a single encounter."
            )),
            "metagaming" => Some((
                "Metagaming",
                "Using out-of-character knowledge that your character wouldn't have."
            )),
            "fudging" => Some((
                "Fudging",
                "When a GM secretly alters dice results for dramatic or balance purposes."
            )),
            "cr" | "challenge rating" => Some((
                "Challenge Rating (CR)",
                "A measure of how difficult an encounter is for a party of a given level."
            )),
            _ => None,
        }
    }

    /// Tooltip for a TTRPG term
    #[component]
    pub fn TermTooltip(
        /// The term to explain
        term: &'static str,
        /// Child content (the term text)
        children: Children,
    ) -> impl IntoView {
        if let Some((title, description)) = get_definition(term) {
            view! {
                <RichTooltip title=title description=description>
                    <span class="border-b border-dotted border-zinc-500 cursor-help">
                        {children()}
                    </span>
                </RichTooltip>
            }.into_any()
        } else {
            view! {
                {children()}
            }.into_any()
        }
    }
}

// ============================================================================
// Contextual Help Panel
// ============================================================================

/// Help entry for a wizard step
#[derive(Debug, Clone)]
pub struct StepHelpEntry {
    pub icon: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

/// Contextual help panel for wizard steps
#[component]
pub fn ContextualHelp(
    /// Current step identifier
    step: &'static str,
) -> impl IntoView {
    let help_entries = get_step_help(step);

    view! {
        <div class="bg-zinc-800/30 border border-zinc-700/50 rounded-lg p-4">
            <div class="flex items-center gap-2 mb-3">
                <svg class="w-5 h-5 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" />
                </svg>
                <h4 class="font-medium text-white">"Tips for this step"</h4>
            </div>
            <ul class="space-y-3">
                {help_entries.iter().map(|entry| view! {
                    <li class="flex items-start gap-3">
                        <span class="text-lg shrink-0">{entry.icon}</span>
                        <div>
                            <div class="text-sm font-medium text-zinc-300">{entry.title}</div>
                            <div class="text-xs text-zinc-500 mt-0.5">{entry.description}</div>
                        </div>
                    </li>
                }).collect_view()}
            </ul>
        </div>
    }
}

/// Get help entries for a wizard step
fn get_step_help(step: &str) -> Vec<StepHelpEntry> {
    match step {
        "basics" => vec![
            StepHelpEntry {
                icon: "🎯",
                title: "Choose a memorable name",
                description: "Pick something evocative that captures your campaign's essence.",
            },
            StepHelpEntry {
                icon: "📚",
                title: "Select your game system",
                description: "This helps tailor suggestions to your specific ruleset.",
            },
        ],
        "intent" => vec![
            StepHelpEntry {
                icon: "💭",
                title: "Think about the fantasy",
                description: "What experience do you want players to have?",
            },
            StepHelpEntry {
                icon: "🎭",
                title: "Set the tone",
                description: "Dark and serious? Light and comedic? Epic and heroic?",
            },
            StepHelpEntry {
                icon: "🚫",
                title: "Note things to avoid",
                description: "Lines and veils help everyone have a good time.",
            },
        ],
        "scope" => vec![
            StepHelpEntry {
                icon: "📅",
                title: "Plan realistically",
                description: "Consider how often your group can actually meet.",
            },
            StepHelpEntry {
                icon: "⏱️",
                title: "Session length matters",
                description: "Shorter sessions need tighter pacing.",
            },
        ],
        "players" => vec![
            StepHelpEntry {
                icon: "👥",
                title: "Party size affects balance",
                description: "Smaller parties may need adjusted challenges.",
            },
            StepHelpEntry {
                icon: "🎓",
                title: "Experience level helps",
                description: "New players benefit from simpler mechanics.",
            },
        ],
        "party_composition" => vec![
            StepHelpEntry {
                icon: "⚔️",
                title: "Balance is optional",
                description: "Many great campaigns work with 'unbalanced' parties.",
            },
            StepHelpEntry {
                icon: "🤝",
                title: "Consider NPC allies",
                description: "You can fill gaps with recurring NPCs.",
            },
        ],
        "arc_structure" => vec![
            StepHelpEntry {
                icon: "📖",
                title: "Story structure guides, not restricts",
                description: "Templates are starting points, not rigid rules.",
            },
            StepHelpEntry {
                icon: "🔄",
                title: "Flexible is fine",
                description: "Sandbox campaigns can be just as satisfying.",
            },
        ],
        "initial_content" => vec![
            StepHelpEntry {
                icon: "🏠",
                title: "Start small",
                description: "You don't need to detail the whole world yet.",
            },
            StepHelpEntry {
                icon: "🎣",
                title: "Plot hooks matter",
                description: "Give players multiple threads to pull.",
            },
        ],
        "review" => vec![
            StepHelpEntry {
                icon: "✅",
                title: "Review your choices",
                description: "Make sure everything captures your vision.",
            },
            StepHelpEntry {
                icon: "🔄",
                title: "You can always change later",
                description: "Campaigns evolve - this is just the starting point.",
            },
        ],
        _ => vec![
            StepHelpEntry {
                icon: "💡",
                title: "Need help?",
                description: "Use the AI assistant for suggestions.",
            },
        ],
    }
}

// ============================================================================
// Form Field Label with Help
// ============================================================================

/// Label with integrated help tooltip
#[component]
pub fn LabelWithHelp(
    /// Label text
    label: &'static str,
    /// Help text for tooltip
    help: &'static str,
    /// Whether the field is required
    #[prop(default = false)]
    required: bool,
    /// Whether the field is optional (shows badge)
    #[prop(default = false)]
    optional: bool,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2">
            <label class="text-sm font-medium text-zinc-300">
                {label}
                {required.then(|| view! {
                    <span class="text-red-400 ml-1">"*"</span>
                })}
            </label>
            {optional.then(|| view! {
                <span class="text-xs text-zinc-500 font-normal">"(optional)"</span>
            })}
            <HelpIcon text=help position=TooltipPosition::Right size="w-3.5 h-3.5" />
        </div>
    }
}
