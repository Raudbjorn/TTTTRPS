//! Theme Editor Component
//!
//! An advanced theme customization UI with weight sliders for blending themes.
//! Features:
//!   - Individual weight sliders for each of 5 themes
//!   - Real-time preview of blended theme
//!   - Preset quick-select buttons
//!   - Theme description tooltips
//!   - Visual representation of current theme weights
//!   - Live preview panel with sample UI elements

use crate::components::design_system::{
    Badge, BadgeVariant, Button, ButtonVariant, Card, CardBody, CardHeader, Slider,
};
use crate::services::theme_service::{preset_description, ThemeState, ThemeWeights};
use leptos::prelude::*;

/// Theme metadata for UI display
#[allow(dead_code)]
struct ThemeInfo {
    name: &'static str,
    label: &'static str,
    description: &'static str,
    color_class: &'static str,
    icon: &'static str,
}

const THEME_INFO: &[ThemeInfo] = &[
    ThemeInfo {
        name: "fantasy",
        label: "Fantasy",
        description: "Warm, magical tones with golden accents - ideal for D&D and Pathfinder",
        color_class: "from-amber-600 to-purple-800",
        icon: "crystal",
    },
    ThemeInfo {
        name: "cosmic",
        label: "Cosmic",
        description: "Deep teal and cyan hues evoking cosmic horror - perfect for Call of Cthulhu",
        color_class: "from-teal-800 to-slate-900",
        icon: "tentacle",
    },
    ThemeInfo {
        name: "terminal",
        label: "Terminal",
        description: "Classic green-on-black hacker aesthetic with scanlines",
        color_class: "from-green-700 to-black",
        icon: "terminal",
    },
    ThemeInfo {
        name: "noir",
        label: "Noir",
        description: "Sepia-toned noir atmosphere - great for Delta Green and spy thrillers",
        color_class: "from-amber-900 to-stone-900",
        icon: "redacted",
    },
    ThemeInfo {
        name: "neon",
        label: "Neon",
        description: "Vibrant cyberpunk palette with pink/purple neon glow",
        color_class: "from-fuchsia-600 to-violet-900",
        icon: "bolt",
    },
];

#[component]
pub fn ThemeEditor() -> impl IntoView {
    let theme_state = expect_context::<ThemeState>();

    // Local signals for each theme weight (for smooth UI updates)
    let fantasy_weight = RwSignal::new(1.0_f32);
    let cosmic_weight = RwSignal::new(0.0_f32);
    let terminal_weight = RwSignal::new(0.0_f32);
    let noir_weight = RwSignal::new(0.0_f32);
    let neon_weight = RwSignal::new(0.0_f32);

    // Initialize from current theme state
    Effect::new(move |_| {
        let weights = theme_state.weights.get();
        fantasy_weight.set(weights.fantasy);
        cosmic_weight.set(weights.cosmic);
        terminal_weight.set(weights.terminal);
        noir_weight.set(weights.noir);
        neon_weight.set(weights.neon);
    });

    // Update theme when weights change
    let update_theme = move || {
        let weights = ThemeWeights {
            fantasy: fantasy_weight.get(),
            cosmic: cosmic_weight.get(),
            terminal: terminal_weight.get(),
            noir: noir_weight.get(),
            neon: neon_weight.get(),
        };
        theme_state.set_weights(weights);
    };

    // Preset selection handler
    let select_preset = move |name: &'static str| {
        // Reset all weights
        fantasy_weight.set(0.0);
        cosmic_weight.set(0.0);
        terminal_weight.set(0.0);
        noir_weight.set(0.0);
        neon_weight.set(0.0);

        // Set the selected preset
        match name {
            "fantasy" => fantasy_weight.set(1.0),
            "cosmic" => cosmic_weight.set(1.0),
            "terminal" => terminal_weight.set(1.0),
            "noir" => noir_weight.set(1.0),
            "neon" => neon_weight.set(1.0),
            _ => fantasy_weight.set(1.0),
        }

        theme_state.set_preset(name);
    };

    // Derived signal for current preset (if using a single theme)
    let current_preset = theme_state.current_preset;

    // Callback wrappers that update theme after slider change
    let on_fantasy_change = Callback::new(move |_: f32| update_theme());
    let on_cosmic_change = Callback::new(move |_: f32| update_theme());
    let on_terminal_change = Callback::new(move |_: f32| update_theme());
    let on_noir_change = Callback::new(move |_: f32| update_theme());
    let on_neon_change = Callback::new(move |_: f32| update_theme());

    // Show preview panel signal
    let show_preview = RwSignal::new(false);

    view! {
        <div class="space-y-6">
            // Preset Quick-Select
            <div>
                <h3 class="text-sm font-bold text-[var(--text-muted)] uppercase tracking-wider mb-3">
                    "Quick Presets"
                </h3>
                <div class="flex flex-wrap gap-2">
                    {THEME_INFO.iter().map(|info| {
                        let name = info.name;
                        let label = info.label;
                        let color_class = info.color_class;
                        let is_active = move || current_preset.get().as_deref() == Some(name);

                        view! {
                            <button
                                class=move || format!(
                                    "px-4 py-2 rounded-lg text-sm font-medium transition-all border {}",
                                    if is_active() {
                                        format!("bg-gradient-to-br {} text-white border-[var(--accent)] ring-2 ring-[var(--accent)] ring-offset-2 ring-offset-[var(--bg-deep)]", color_class)
                                    } else {
                                        "bg-[var(--bg-surface)] text-[var(--text-muted)] border-[var(--border-subtle)] hover:border-[var(--border-strong)] hover:text-[var(--text-primary)]".to_string()
                                    }
                                )
                                title=preset_description(name)
                                on:click=move |_| select_preset(name)
                            >
                                {label}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </div>

            // Theme Weight Sliders
            <Card>
                <CardHeader>
                    <div class="flex items-center justify-between w-full">
                        <h3 class="text-sm font-bold text-[var(--text-muted)] uppercase tracking-wider">
                            "Theme Blending"
                        </h3>
                        <span class="text-xs text-[var(--text-muted)]">
                            "Adjust weights to blend themes"
                        </span>
                    </div>
                </CardHeader>
                <CardBody class="space-y-4">
                    // Fantasy
                    <ThemeWeightSlider
                        info=&THEME_INFO[0]
                        weight=fantasy_weight
                        on_change=on_fantasy_change
                    />

                    // Cosmic
                    <ThemeWeightSlider
                        info=&THEME_INFO[1]
                        weight=cosmic_weight
                        on_change=on_cosmic_change
                    />

                    // Terminal
                    <ThemeWeightSlider
                        info=&THEME_INFO[2]
                        weight=terminal_weight
                        on_change=on_terminal_change
                    />

                    // Noir
                    <ThemeWeightSlider
                        info=&THEME_INFO[3]
                        weight=noir_weight
                        on_change=on_noir_change
                    />

                    // Neon
                    <ThemeWeightSlider
                        info=&THEME_INFO[4]
                        weight=neon_weight
                        on_change=on_neon_change
                    />
                </CardBody>
            </Card>

            // Visual Weight Preview Bar
            <div>
                <div class="flex items-center justify-between mb-2">
                    <span class="text-xs text-[var(--text-muted)] uppercase tracking-wider">
                        "Weight Distribution"
                    </span>
                    <button
                        class="text-xs text-[var(--accent)] hover:underline"
                        on:click=move |_| show_preview.update(|v| *v = !*v)
                    >
                        {move || if show_preview.get() { "Hide Preview" } else { "Show Preview" }}
                    </button>
                </div>
                <div class="h-4 rounded-full overflow-hidden flex shadow-inner">
                    <div
                        class="bg-gradient-to-r from-amber-600 to-purple-800 transition-all"
                        style:flex-grow=move || fantasy_weight.get().to_string()
                    ></div>
                    <div
                        class="bg-gradient-to-r from-teal-800 to-slate-900 transition-all"
                        style:flex-grow=move || cosmic_weight.get().to_string()
                    ></div>
                    <div
                        class="bg-gradient-to-r from-green-700 to-black transition-all"
                        style:flex-grow=move || terminal_weight.get().to_string()
                    ></div>
                    <div
                        class="bg-gradient-to-r from-amber-900 to-stone-900 transition-all"
                        style:flex-grow=move || noir_weight.get().to_string()
                    ></div>
                    <div
                        class="bg-gradient-to-r from-fuchsia-600 to-violet-900 transition-all"
                        style:flex-grow=move || neon_weight.get().to_string()
                    ></div>
                </div>
            </div>

            // Live Preview Panel
            <Show when=move || show_preview.get()>
                <LivePreview />
            </Show>

            // Effect Controls
            <Card>
                <CardHeader>
                    <h3 class="text-sm font-bold text-[var(--text-muted)] uppercase tracking-wider">
                        "Visual Effects"
                    </h3>
                </CardHeader>
                <CardBody class="space-y-4">
                    <EffectToggle
                        label="Film Grain"
                        description="Adds subtle texture overlay"
                        effect_class="effect-grain"
                    />
                    <EffectToggle
                        label="CRT Scanlines"
                        description="Classic monitor effect"
                        effect_class="effect-scanlines"
                    />
                    <EffectToggle
                        label="Text Glow"
                        description="Neon glow on accent text"
                        effect_class="glow-text"
                    />
                </CardBody>
            </Card>
        </div>
    }
}

/// Individual theme weight slider with label and color indicator
#[component]
fn ThemeWeightSlider(
    info: &'static ThemeInfo,
    weight: RwSignal<f32>,
    on_change: Callback<f32>,
) -> impl IntoView {
    view! {
        <div class="flex items-center gap-4">
            // Color indicator
            <div class=format!(
                "w-3 h-3 rounded-full bg-gradient-to-br {} flex-shrink-0",
                info.color_class
            )></div>

            // Label
            <span class="w-20 text-sm font-medium text-[var(--text-primary)] flex-shrink-0">
                {info.label}
            </span>

            // Slider
            <div class="flex-1">
                <Slider
                    value=weight
                    min=0.0
                    max=1.0
                    step=0.05
                    show_percentage=true
                    on_change=on_change
                />
            </div>
        </div>
    }
}

/// Toggle for visual effects
#[component]
fn EffectToggle(
    label: &'static str,
    description: &'static str,
    effect_class: &'static str,
) -> impl IntoView {
    let is_enabled = RwSignal::new(false);

    // Toggle effect on body element
    let toggle = move |_: web_sys::MouseEvent| {
        is_enabled.update(|v| *v = !*v);

        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(body) = document.body() {
                    if is_enabled.get() {
                        let _ = body.class_list().add_1(effect_class);
                    } else {
                        let _ = body.class_list().remove_1(effect_class);
                    }
                }
            }
        }
    };

    view! {
        <div class="flex items-center justify-between">
            <div>
                <span class="text-sm font-medium text-[var(--text-primary)]">{label}</span>
                <p class="text-xs text-[var(--text-muted)]">{description}</p>
            </div>
            <button
                class=move || format!(
                    "relative w-12 h-6 rounded-full transition-colors {}",
                    if is_enabled.get() {
                        "bg-[var(--accent)]"
                    } else {
                        "bg-[var(--bg-surface)]"
                    }
                )
                role="switch"
                aria-checked=move || is_enabled.get().to_string()
                on:click=toggle
            >
                <div class=move || format!(
                    "absolute top-1 w-4 h-4 rounded-full bg-white shadow-md transition-transform {}",
                    if is_enabled.get() { "left-7" } else { "left-1" }
                )></div>
            </button>
        </div>
    }
}

/// Live preview panel showing sample UI elements with current theme
#[component]
fn LivePreview() -> impl IntoView {
    view! {
        <Card>
            <CardHeader>
                <div class="flex items-center justify-between w-full">
                    <h3 class="text-sm font-bold text-[var(--text-muted)] uppercase tracking-wider">
                        "Live Preview"
                    </h3>
                    <Badge variant=BadgeVariant::Default>"Sample UI"</Badge>
                </div>
            </CardHeader>
            <CardBody class="space-y-6">
                // Color Swatches
                <div>
                    <h4 class="text-xs font-medium text-[var(--text-muted)] uppercase mb-3">
                        "Color Palette"
                    </h4>
                    <div class="grid grid-cols-4 gap-2">
                        <ColorSwatch name="bg-deep" label="BG Deep" />
                        <ColorSwatch name="bg-surface" label="BG Surface" />
                        <ColorSwatch name="bg-elevated" label="BG Elevated" />
                        <ColorSwatch name="accent-primary" label="Accent" />
                        <ColorSwatch name="text-primary" label="Text Primary" />
                        <ColorSwatch name="text-secondary" label="Text Secondary" />
                        <ColorSwatch name="text-muted" label="Text Muted" />
                        <ColorSwatch name="accent-secondary" label="Accent 2" />
                        <ColorSwatch name="success" label="Success" />
                        <ColorSwatch name="warning" label="Warning" />
                        <ColorSwatch name="error" label="Error" />
                        <ColorSwatch name="border-color" label="Border" />
                    </div>
                </div>

                // Typography Preview
                <div>
                    <h4 class="text-xs font-medium text-[var(--text-muted)] uppercase mb-3">
                        "Typography"
                    </h4>
                    <div class="space-y-2">
                        <p class="text-lg font-bold text-[var(--text-primary)]">
                            "Primary Heading Text"
                        </p>
                        <p class="text-base text-[var(--text-secondary)]">
                            "Secondary body text with medium emphasis."
                        </p>
                        <p class="text-sm text-[var(--text-muted)]">
                            "Muted helper text for less important information."
                        </p>
                        <p class="text-sm text-[var(--accent-primary)]">
                            "Accent colored link or highlight"
                        </p>
                    </div>
                </div>

                // Button Preview
                <div>
                    <h4 class="text-xs font-medium text-[var(--text-muted)] uppercase mb-3">
                        "Buttons"
                    </h4>
                    <div class="flex flex-wrap gap-2">
                        <Button variant=ButtonVariant::Primary on_click=move |_| {}>"Primary"</Button>
                        <Button variant=ButtonVariant::Secondary on_click=move |_| {}>"Secondary"</Button>
                        <Button variant=ButtonVariant::Ghost on_click=move |_| {}>"Ghost"</Button>
                        <Button variant=ButtonVariant::Destructive on_click=move |_| {}>"Danger"</Button>
                    </div>
                </div>

                // Badge Preview
                <div>
                    <h4 class="text-xs font-medium text-[var(--text-muted)] uppercase mb-3">
                        "Badges"
                    </h4>
                    <div class="flex flex-wrap gap-2">
                        <Badge variant=BadgeVariant::Default>"Default"</Badge>
                        <Badge variant=BadgeVariant::Success>"Success"</Badge>
                        <Badge variant=BadgeVariant::Warning>"Warning"</Badge>
                        <Badge variant=BadgeVariant::Danger>"Danger"</Badge>
                        <Badge variant=BadgeVariant::Info>"Info"</Badge>
                    </div>
                </div>

                // Card/Surface Preview
                <div>
                    <h4 class="text-xs font-medium text-[var(--text-muted)] uppercase mb-3">
                        "Surfaces & Borders"
                    </h4>
                    <div class="grid grid-cols-2 gap-3">
                        <div class="p-4 rounded-[var(--radius-md)] bg-[var(--bg-surface)] border border-[var(--border-subtle)]">
                            <span class="text-sm text-[var(--text-primary)]">"Subtle border"</span>
                        </div>
                        <div class="p-4 rounded-[var(--radius-md)] bg-[var(--bg-elevated)] border border-[var(--border-strong)]">
                            <span class="text-sm text-[var(--text-primary)]">"Strong border"</span>
                        </div>
                    </div>
                </div>

                // Sample Chat Message
                <div>
                    <h4 class="text-xs font-medium text-[var(--text-muted)] uppercase mb-3">
                        "Sample Chat"
                    </h4>
                    <div class="space-y-3">
                        <div class="flex gap-3">
                            <div class="w-8 h-8 rounded-full bg-[var(--accent-primary)] flex items-center justify-center text-white text-xs font-bold flex-shrink-0">
                                "GM"
                            </div>
                            <div class="flex-1 p-3 rounded-[var(--radius-md)] bg-[var(--bg-elevated)] border border-[var(--border-subtle)]">
                                <p class="text-sm text-[var(--text-primary)]">
                                    "The ancient temple looms before you, its weathered stones covered in strange glyphs that seem to shift in the torchlight."
                                </p>
                            </div>
                        </div>
                        <div class="flex gap-3 justify-end">
                            <div class="flex-1 max-w-[80%] p-3 rounded-[var(--radius-md)] bg-[var(--accent-primary)]/20 border border-[var(--accent-primary)]/30">
                                <p class="text-sm text-[var(--text-primary)]">
                                    "I approach cautiously, examining the glyphs for any sign of magical traps."
                                </p>
                            </div>
                            <div class="w-8 h-8 rounded-full bg-[var(--accent-secondary)] flex items-center justify-center text-white text-xs font-bold flex-shrink-0">
                                "P1"
                            </div>
                        </div>
                    </div>
                </div>
            </CardBody>
        </Card>
    }
}

/// Color swatch component for displaying a theme color
#[component]
fn ColorSwatch(name: &'static str, label: &'static str) -> impl IntoView {
    let var_name = format!("var(--{})", name);

    view! {
        <div class="flex flex-col items-center gap-1">
            <div
                class="w-full h-8 rounded border border-[var(--border-subtle)]"
                style:background-color=var_name.clone()
            ></div>
            <span class="text-[10px] text-[var(--text-muted)] text-center truncate w-full">
                {label}
            </span>
        </div>
    }
}
