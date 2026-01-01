use dioxus::prelude::*;
use crate::bindings::ThemeWeights;

// Represents the interpolation targets
#[derive(Clone, Debug)]
pub struct ThemeDefinition {
    // Colors (OKLCH L C H Alpha)
    pub bg_deep: [f32; 4],
    pub bg_surface: [f32; 4],
    pub bg_elevated: [f32; 4],
    pub text_primary: [f32; 4],
    pub text_muted: [f32; 4],
    pub accent: [f32; 4],
    pub accent_hover: [f32; 4],
    pub danger: [f32; 4],
    pub border_subtle: [f32; 4],
    pub border_strong: [f32; 4],

    // Radii (px)
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,

    // Effects
    pub effect_blur: f32, // px
    pub effect_grain: f32, // 0-1
    pub effect_scanline: f32, // 0-1
    pub effect_glow: f32, // 0-1
}

impl Default for ThemeDefinition {
    fn default() -> Self {
        ThemeDefinition {
            // Fantasy Default
            bg_deep: [0.15, 0.02, 280.0, 1.0],
            bg_surface: [0.20, 0.03, 280.0, 0.8],
            bg_elevated: [0.25, 0.04, 280.0, 0.9],
            text_primary: [0.95, 0.01, 60.0, 1.0],
            text_muted: [0.60, 0.02, 280.0, 1.0],
            accent: [0.75, 0.15, 45.0, 1.0],
            accent_hover: [0.80, 0.18, 45.0, 1.0],
            danger: [0.60, 0.20, 25.0, 1.0],
            border_subtle: [0.30, 0.03, 280.0, 0.5],
            border_strong: [0.75, 0.12, 45.0, 0.6],

            radius_sm: 4.0,
            radius_md: 8.0,
            radius_lg: 16.0,

            effect_blur: 12.0,
            effect_grain: 0.0,
            effect_scanline: 0.0,
            effect_glow: 0.4,
        }
    }
}

pub fn get_preset(name: &str) -> ThemeDefinition {
    match name {
        "fantasy" => ThemeDefinition::default(),
        "cosmic" => ThemeDefinition {
            bg_deep: [0.08, 0.02, 160.0, 1.0],
            bg_surface: [0.12, 0.03, 160.0, 0.85],
            bg_elevated: [0.16, 0.04, 150.0, 0.9],
            text_primary: [0.85, 0.02, 100.0, 1.0],
            text_muted: [0.50, 0.04, 160.0, 1.0],
            accent: [0.55, 0.12, 160.0, 1.0],
            accent_hover: [0.60, 0.15, 160.0, 1.0],
            danger: [0.50, 0.15, 320.0, 1.0],
            border_subtle: [0.20, 0.05, 160.0, 0.4],
            border_strong: [0.55, 0.10, 160.0, 0.5],
            radius_sm: 2.0, radius_md: 4.0, radius_lg: 6.0,
            effect_blur: 4.0, effect_grain: 0.15, effect_scanline: 0.0, effect_glow: 0.2
        },
        "terminal" => ThemeDefinition {
            bg_deep: [0.05, 0.0, 0.0, 1.0],
            bg_surface: [0.10, 0.01, 145.0, 1.0],
            bg_elevated: [0.15, 0.02, 145.0, 1.0],
            text_primary: [0.85, 0.15, 145.0, 1.0],
            text_muted: [0.55, 0.08, 145.0, 1.0],
            accent: [0.75, 0.18, 80.0, 1.0],
            accent_hover: [0.80, 0.20, 80.0, 1.0],
            danger: [0.65, 0.20, 25.0, 1.0],
            border_subtle: [0.25, 0.05, 145.0, 0.3],
            border_strong: [0.70, 0.12, 145.0, 0.5],
            radius_sm: 0.0, radius_md: 0.0, radius_lg: 2.0,
            effect_blur: 0.0, effect_grain: 0.05, effect_scanline: 0.3, effect_glow: 0.6
        },
        "noir" => ThemeDefinition {
            bg_deep: [0.20, 0.01, 80.0, 1.0],
            bg_surface: [0.28, 0.02, 75.0, 1.0],
            bg_elevated: [0.35, 0.03, 70.0, 1.0],
            text_primary: [0.90, 0.01, 90.0, 1.0],
            text_muted: [0.55, 0.02, 80.0, 1.0],
            accent: [0.45, 0.08, 25.0, 1.0],
            accent_hover: [0.50, 0.10, 25.0, 1.0],
            danger: [0.55, 0.15, 25.0, 1.0],
            border_subtle: [0.40, 0.02, 80.0, 0.3],
            border_strong: [0.30, 0.03, 80.0, 0.6],
            radius_sm: 0.0, radius_md: 2.0, radius_lg: 4.0,
            effect_blur: 0.0, effect_grain: 0.08, effect_scanline: 0.0, effect_glow: 0.0
        },
        "neon" => ThemeDefinition {
            bg_deep: [0.08, 0.01, 270.0, 1.0],
            bg_surface: [0.12, 0.02, 280.0, 1.0],
            bg_elevated: [0.18, 0.03, 290.0, 1.0],
            text_primary: [0.95, 0.02, 200.0, 1.0],
            text_muted: [0.60, 0.05, 280.0, 1.0],
            accent: [0.70, 0.25, 330.0, 1.0],
            accent_hover: [0.75, 0.28, 330.0, 1.0],
            danger: [0.65, 0.22, 25.0, 1.0],
            border_subtle: [0.25, 0.08, 280.0, 0.3],
            border_strong: [0.70, 0.20, 330.0, 0.5],
            radius_sm: 0.0, radius_md: 4.0, radius_lg: 8.0,
            effect_blur: 8.0, effect_grain: 0.03, effect_scanline: 0.1, effect_glow: 0.8
        },
        _ => ThemeDefinition::default(),
    }
}

pub fn generate_css(weights: &ThemeWeights) -> String {
    let mut mixed = ThemeDefinition {
        bg_deep: [0.0; 4], bg_surface: [0.0; 4], bg_elevated: [0.0; 4],
        text_primary: [0.0; 4], text_muted: [0.0; 4],
        accent: [0.0; 4], accent_hover: [0.0; 4], danger: [0.0; 4],
        border_subtle: [0.0; 4], border_strong: [0.0; 4],
        radius_sm: 0.0, radius_md: 0.0, radius_lg: 0.0,
        effect_blur: 0.0, effect_grain: 0.0, effect_scanline: 0.0, effect_glow: 0.0
    };

    let definitions = [
        (weights.fantasy, get_preset("fantasy")),
        (weights.cosmic, get_preset("cosmic")),
        (weights.terminal, get_preset("terminal")),
        (weights.noir, get_preset("noir")),
        (weights.neon, get_preset("neon")),
    ];

    let total_weight: f32 = definitions.iter().map(|(w, _)| w).sum();
    let norm = if total_weight > 0.0 { 1.0 / total_weight } else { 1.0 };

    for (w, def) in definitions.iter() {
        let weight = w * norm;
        if weight <= 0.0 { continue; }

        // Colors
        add_color(&mut mixed.bg_deep, &def.bg_deep, weight);
        add_color(&mut mixed.bg_surface, &def.bg_surface, weight);
        add_color(&mut mixed.bg_elevated, &def.bg_elevated, weight);
        add_color(&mut mixed.text_primary, &def.text_primary, weight);
        add_color(&mut mixed.text_muted, &def.text_muted, weight);
        add_color(&mut mixed.accent, &def.accent, weight);
        add_color(&mut mixed.accent_hover, &def.accent_hover, weight);
        add_color(&mut mixed.danger, &def.danger, weight);
        add_color(&mut mixed.border_subtle, &def.border_subtle, weight);
        add_color(&mut mixed.border_strong, &def.border_strong, weight);

        // Values
        mixed.radius_sm += def.radius_sm * weight;
        mixed.radius_md += def.radius_md * weight;
        mixed.radius_lg += def.radius_lg * weight;
        mixed.effect_blur += def.effect_blur * weight;
        mixed.effect_grain += def.effect_grain * weight;
        mixed.effect_scanline += def.effect_scanline * weight;
        mixed.effect_glow += def.effect_glow * weight;
    }

    // Construct CSS
    format!("
        :root {{
            --bg-deep: {};
            --bg-surface: {};
            --bg-elevated: {};
            --text-primary: {};
            --text-muted: {};
            --accent: {};
            --accent-hover: {};
            --danger: {};
            --border-subtle: {};
            --border-strong: {};

            --radius-sm: {}px;
            --radius-md: {}px;
            --radius-lg: {}px;

            --effect-blur: {}px;
            --effect-grain: {};
            --effect-scanline: {};
            --effect-glow: {};
        }}
    ",
    fmt_oklch(mixed.bg_deep), fmt_oklch(mixed.bg_surface), fmt_oklch(mixed.bg_elevated),
    fmt_oklch(mixed.text_primary), fmt_oklch(mixed.text_muted),
    fmt_oklch(mixed.accent), fmt_oklch(mixed.accent_hover), fmt_oklch(mixed.danger),
    fmt_oklch(mixed.border_subtle), fmt_oklch(mixed.border_strong),
    mixed.radius_sm, mixed.radius_md, mixed.radius_lg,
    mixed.effect_blur, mixed.effect_grain, mixed.effect_scanline, mixed.effect_glow
    )
}

fn add_color(acc: &mut [f32; 4], val: &[f32; 4], w: f32) {
    acc[0] += val[0] * w;
    acc[1] += val[1] * w;

    // Hue interpolation logic (shortest path? for now just linear)
    acc[2] += val[2] * w;

    acc[3] += val[3] * w;
}

fn fmt_oklch(c: [f32; 4]) -> String {
    if c[3] >= 0.99 {
        format!("oklch({:.2}% {:.3} {:.1})", c[0]*100.0, c[1], c[2])
    } else {
        format!("oklch({:.2}% {:.3} {:.1} / {:.2})", c[0]*100.0, c[1], c[2], c[3])
    }
}
