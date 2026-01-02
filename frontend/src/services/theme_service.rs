//! Theme Service for Leptos frontend
//!
//! Provides dynamic theme blending using OKLCH color space interpolation.
//! Supports five theme presets (fantasy, cosmic, terminal, noir, neon) that
//! can be blended using weighted interpolation to create custom themes.
//!
//! # CSS Variables Generated
//!
//! Background colors:
//! - `--bg-deep`: Deepest background (app shell)
//! - `--bg-surface`: Surface-level containers
//! - `--bg-elevated`: Elevated elements (modals, dropdowns)
//!
//! Text colors:
//! - `--text-primary`: Main text color
//! - `--text-secondary`: Less prominent text
//! - `--text-muted`: Subdued text (hints, disabled)
//!
//! Accent colors:
//! - `--accent-primary`: Primary accent color
//! - `--accent-secondary`: Secondary accent color
//! - `--accent-hover`: Accent hover state
//!
//! Border & Shadow:
//! - `--border-subtle`: Subtle borders
//! - `--border-strong`: Prominent borders
//! - `--border-color`: Default border color
//! - `--shadow-color`: Shadow color for elevation
//!
//! Semantic colors:
//! - `--success`: Success/positive states
//! - `--warning`: Warning states
//! - `--error`: Error/danger states
//!
//! Effects:
//! - `--effect-blur`: Background blur amount
//! - `--effect-grain`: Film grain intensity
//! - `--effect-scanline`: CRT scanline intensity
//! - `--effect-glow`: Glow effect intensity

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Color Math Utilities
// ============================================================================

/// OKLCH color representation: [Lightness (0-1), Chroma (0-0.4), Hue (0-360), Alpha (0-1)]
pub type OklchColor = [f32; 4];

/// Lerp (linear interpolation) between two values
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Interpolate between two OKLCH colors using weighted blend
/// Uses shortest-path hue interpolation
pub fn blend_oklch(colors: &[(f32, OklchColor)]) -> OklchColor {
    if colors.is_empty() {
        return [0.0, 0.0, 0.0, 1.0];
    }

    let total_weight: f32 = colors.iter().map(|(w, _)| w).sum();
    if total_weight <= 0.0 {
        return colors
            .first()
            .map(|(_, c)| *c)
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    }

    let mut l = 0.0;
    let mut c = 0.0;
    let mut a = 0.0;

    // For hue, we use circular interpolation via sin/cos
    let mut hue_sin = 0.0;
    let mut hue_cos = 0.0;

    for (weight, color) in colors {
        let w = weight / total_weight;
        if w <= 0.0 {
            continue;
        }

        l += color[0] * w;
        c += color[1] * w;

        // Circular hue interpolation
        let hue_rad = color[2].to_radians();
        hue_sin += hue_rad.sin() * w;
        hue_cos += hue_rad.cos() * w;

        a += color[3] * w;
    }

    // Convert back from sin/cos to angle
    let h = hue_sin.atan2(hue_cos).to_degrees();
    let h = if h < 0.0 { h + 360.0 } else { h };

    [l, c, h, a]
}

/// Adjust lightness of an OKLCH color
pub fn adjust_lightness(color: OklchColor, delta: f32) -> OklchColor {
    [
        (color[0] + delta).clamp(0.0, 1.0),
        color[1],
        color[2],
        color[3],
    ]
}

/// Adjust chroma (saturation) of an OKLCH color
pub fn adjust_chroma(color: OklchColor, factor: f32) -> OklchColor {
    [
        color[0],
        (color[1] * factor).clamp(0.0, 0.4),
        color[2],
        color[3],
    ]
}

/// Create a complementary color (180 degree hue shift)
pub fn complementary(color: OklchColor) -> OklchColor {
    [color[0], color[1], (color[2] + 180.0) % 360.0, color[3]]
}

/// Create an analogous color (30 degree hue shift)
pub fn analogous(color: OklchColor, offset: f32) -> OklchColor {
    [
        color[0],
        color[1],
        (color[2] + offset + 360.0) % 360.0,
        color[3],
    ]
}

// ============================================================================
// Theme Weights
// ============================================================================

/// Theme type enum for type-safe theme identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThemeType {
    Fantasy,
    Cosmic,
    Terminal,
    Noir,
    Neon,
}

impl ThemeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeType::Fantasy => "fantasy",
            ThemeType::Cosmic => "cosmic",
            ThemeType::Terminal => "terminal",
            ThemeType::Noir => "noir",
            ThemeType::Neon => "neon",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "fantasy" => Some(ThemeType::Fantasy),
            "cosmic" => Some(ThemeType::Cosmic),
            "terminal" => Some(ThemeType::Terminal),
            "noir" => Some(ThemeType::Noir),
            "neon" => Some(ThemeType::Neon),
            _ => None,
        }
    }

    pub fn all() -> &'static [ThemeType] {
        &[
            ThemeType::Fantasy,
            ThemeType::Cosmic,
            ThemeType::Terminal,
            ThemeType::Noir,
            ThemeType::Neon,
        ]
    }
}

/// Theme blending weights - determines how much each theme contributes
/// to the final interpolated theme. Values should be 0.0-1.0 and will
/// be normalized when generating CSS.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThemeWeights {
    pub fantasy: f32,
    pub cosmic: f32,
    pub terminal: f32,
    pub noir: f32,
    pub neon: f32,
}

impl Default for ThemeWeights {
    fn default() -> Self {
        Self {
            fantasy: 1.0,
            cosmic: 0.0,
            terminal: 0.0,
            noir: 0.0,
            neon: 0.0,
        }
    }
}

impl ThemeWeights {
    /// Create weights for a single preset
    pub fn preset(name: &str) -> Self {
        let mut weights = Self::zeroed();
        match name {
            "fantasy" => weights.fantasy = 1.0,
            "cosmic" => weights.cosmic = 1.0,
            "terminal" => weights.terminal = 1.0,
            "noir" => weights.noir = 1.0,
            "neon" => weights.neon = 1.0,
            _ => weights.fantasy = 1.0, // fallback to fantasy
        }
        weights
    }

    /// Create weights from ThemeType
    pub fn from_theme_type(theme: ThemeType) -> Self {
        Self::preset(theme.as_str())
    }

    /// Create zeroed weights (useful as a starting point for blending)
    pub fn zeroed() -> Self {
        Self {
            fantasy: 0.0,
            cosmic: 0.0,
            terminal: 0.0,
            noir: 0.0,
            neon: 0.0,
        }
    }

    /// Get weight for a specific theme type
    pub fn get(&self, theme: ThemeType) -> f32 {
        match theme {
            ThemeType::Fantasy => self.fantasy,
            ThemeType::Cosmic => self.cosmic,
            ThemeType::Terminal => self.terminal,
            ThemeType::Noir => self.noir,
            ThemeType::Neon => self.neon,
        }
    }

    /// Set weight for a specific theme type
    pub fn set(&mut self, theme: ThemeType, value: f32) {
        match theme {
            ThemeType::Fantasy => self.fantasy = value,
            ThemeType::Cosmic => self.cosmic = value,
            ThemeType::Terminal => self.terminal = value,
            ThemeType::Noir => self.noir = value,
            ThemeType::Neon => self.neon = value,
        }
    }

    /// Calculate the sum of all weights
    pub fn total(&self) -> f32 {
        self.fantasy + self.cosmic + self.terminal + self.noir + self.neon
    }

    /// Normalize weights to sum to 1.0
    pub fn normalize(&mut self) {
        let total = self.total();
        if total > 0.0 {
            self.fantasy /= total;
            self.cosmic /= total;
            self.terminal /= total;
            self.noir /= total;
            self.neon /= total;
        }
    }

    /// Get normalized weights without mutating
    pub fn normalized(&self) -> Self {
        let mut copy = *self;
        copy.normalize();
        copy
    }

    /// Convert to array of (weight, ThemeType) pairs
    pub fn to_pairs(&self) -> [(f32, ThemeType); 5] {
        [
            (self.fantasy, ThemeType::Fantasy),
            (self.cosmic, ThemeType::Cosmic),
            (self.terminal, ThemeType::Terminal),
            (self.noir, ThemeType::Noir),
            (self.neon, ThemeType::Neon),
        ]
    }

    /// Get the dominant theme (highest weight)
    pub fn dominant(&self) -> ThemeType {
        let pairs = self.to_pairs();
        pairs
            .iter()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(_, t)| *t)
            .unwrap_or(ThemeType::Fantasy)
    }
}

// ============================================================================
// Theme Definition
// ============================================================================

/// Represents the complete set of theme values for interpolation.
/// All colors are in OKLCH format: [Lightness, Chroma, Hue, Alpha]
#[derive(Clone, Debug)]
pub struct ThemeDefinition {
    // Background colors (OKLCH L C H Alpha)
    pub bg_deep: OklchColor,
    pub bg_surface: OklchColor,
    pub bg_elevated: OklchColor,

    // Text colors
    pub text_primary: OklchColor,
    pub text_secondary: OklchColor,
    pub text_muted: OklchColor,

    // Accent colors
    pub accent_primary: OklchColor,
    pub accent_secondary: OklchColor,
    pub accent_hover: OklchColor,

    // Border & Shadow
    pub border_subtle: OklchColor,
    pub border_strong: OklchColor,
    pub border_color: OklchColor,
    pub shadow_color: OklchColor,

    // Semantic colors
    pub success: OklchColor,
    pub warning: OklchColor,
    pub error: OklchColor,

    // Radii (px)
    pub radius_sm: f32,
    pub radius_md: f32,
    pub radius_lg: f32,

    // Effects
    pub effect_blur: f32,     // px
    pub effect_grain: f32,    // 0-1
    pub effect_scanline: f32, // 0-1
    pub effect_glow: f32,     // 0-1
}

impl Default for ThemeDefinition {
    fn default() -> Self {
        // Fantasy theme as default
        Self::fantasy()
    }
}

impl ThemeDefinition {
    /// Fantasy theme: Warm browns, golds, parchment tones
    /// Ideal for D&D, Pathfinder, and traditional fantasy RPGs
    pub fn fantasy() -> Self {
        ThemeDefinition {
            // Warm dark browns/purples for backgrounds
            bg_deep: [0.15, 0.02, 280.0, 1.0],
            bg_surface: [0.20, 0.03, 280.0, 0.8],
            bg_elevated: [0.25, 0.04, 280.0, 0.9],

            // Warm parchment-like text
            text_primary: [0.95, 0.01, 60.0, 1.0],
            text_secondary: [0.80, 0.02, 50.0, 1.0],
            text_muted: [0.60, 0.02, 280.0, 1.0],

            // Golden/amber accents
            accent_primary: [0.75, 0.15, 45.0, 1.0],
            accent_secondary: [0.65, 0.12, 30.0, 1.0],
            accent_hover: [0.80, 0.18, 45.0, 1.0],

            // Borders
            border_subtle: [0.30, 0.03, 280.0, 0.5],
            border_strong: [0.75, 0.12, 45.0, 0.6],
            border_color: [0.40, 0.04, 50.0, 0.6],
            shadow_color: [0.10, 0.02, 280.0, 0.5],

            // Semantic colors
            success: [0.65, 0.15, 145.0, 1.0],
            warning: [0.75, 0.18, 65.0, 1.0],
            error: [0.60, 0.20, 25.0, 1.0],

            radius_sm: 4.0,
            radius_md: 8.0,
            radius_lg: 16.0,

            effect_blur: 12.0,
            effect_grain: 0.0,
            effect_scanline: 0.0,
            effect_glow: 0.4,
        }
    }

    /// Cosmic theme: Deep purples, blues, starfield blacks
    /// Perfect for Call of Cthulhu, cosmic horror, and space settings
    pub fn cosmic() -> Self {
        ThemeDefinition {
            // Deep cosmic void backgrounds
            bg_deep: [0.08, 0.02, 260.0, 1.0],
            bg_surface: [0.12, 0.03, 265.0, 0.85],
            bg_elevated: [0.16, 0.04, 270.0, 0.9],

            // Cool, ethereal text
            text_primary: [0.85, 0.02, 200.0, 1.0],
            text_secondary: [0.70, 0.03, 220.0, 1.0],
            text_muted: [0.50, 0.04, 260.0, 1.0],

            // Teal/cyan accents with purple secondary
            accent_primary: [0.55, 0.12, 180.0, 1.0],
            accent_secondary: [0.50, 0.15, 290.0, 1.0],
            accent_hover: [0.60, 0.15, 180.0, 1.0],

            // Borders
            border_subtle: [0.20, 0.05, 260.0, 0.4],
            border_strong: [0.55, 0.10, 180.0, 0.5],
            border_color: [0.30, 0.06, 265.0, 0.5],
            shadow_color: [0.05, 0.03, 280.0, 0.6],

            // Semantic colors (shifted toward cosmic palette)
            success: [0.60, 0.12, 160.0, 1.0],
            warning: [0.65, 0.15, 80.0, 1.0],
            error: [0.50, 0.15, 320.0, 1.0],

            radius_sm: 2.0,
            radius_md: 4.0,
            radius_lg: 6.0,

            effect_blur: 4.0,
            effect_grain: 0.15,
            effect_scanline: 0.0,
            effect_glow: 0.2,
        }
    }

    /// Terminal theme: Green on black, phosphor glow
    /// Classic hacker aesthetic, great for Mothership and sci-fi horror
    pub fn terminal() -> Self {
        ThemeDefinition {
            // Pure black backgrounds
            bg_deep: [0.05, 0.0, 0.0, 1.0],
            bg_surface: [0.10, 0.01, 145.0, 1.0],
            bg_elevated: [0.15, 0.02, 145.0, 1.0],

            // Phosphor green text
            text_primary: [0.85, 0.15, 145.0, 1.0],
            text_secondary: [0.70, 0.12, 145.0, 1.0],
            text_muted: [0.55, 0.08, 145.0, 1.0],

            // Amber/green accents
            accent_primary: [0.75, 0.18, 145.0, 1.0],
            accent_secondary: [0.70, 0.15, 80.0, 1.0],
            accent_hover: [0.80, 0.20, 145.0, 1.0],

            // Borders
            border_subtle: [0.25, 0.05, 145.0, 0.3],
            border_strong: [0.70, 0.12, 145.0, 0.5],
            border_color: [0.35, 0.08, 145.0, 0.4],
            shadow_color: [0.15, 0.10, 145.0, 0.3],

            // Semantic colors (terminal palette)
            success: [0.75, 0.15, 145.0, 1.0],
            warning: [0.75, 0.18, 80.0, 1.0],
            error: [0.65, 0.20, 25.0, 1.0],

            radius_sm: 0.0,
            radius_md: 0.0,
            radius_lg: 2.0,

            effect_blur: 0.0,
            effect_grain: 0.05,
            effect_scanline: 0.3,
            effect_glow: 0.6,
        }
    }

    /// Noir theme: High contrast B&W, red accents
    /// Great for Delta Green, spy thrillers, and noir mysteries
    pub fn noir() -> Self {
        ThemeDefinition {
            // Sepia-toned grays
            bg_deep: [0.20, 0.01, 80.0, 1.0],
            bg_surface: [0.28, 0.02, 75.0, 1.0],
            bg_elevated: [0.35, 0.03, 70.0, 1.0],

            // High contrast text
            text_primary: [0.90, 0.01, 90.0, 1.0],
            text_secondary: [0.75, 0.02, 85.0, 1.0],
            text_muted: [0.55, 0.02, 80.0, 1.0],

            // Red/burgundy accents
            accent_primary: [0.45, 0.12, 25.0, 1.0],
            accent_secondary: [0.40, 0.08, 45.0, 1.0],
            accent_hover: [0.50, 0.15, 25.0, 1.0],

            // Borders
            border_subtle: [0.40, 0.02, 80.0, 0.3],
            border_strong: [0.30, 0.03, 80.0, 0.6],
            border_color: [0.45, 0.02, 75.0, 0.4],
            shadow_color: [0.10, 0.01, 80.0, 0.5],

            // Semantic colors (muted)
            success: [0.55, 0.10, 145.0, 1.0],
            warning: [0.60, 0.12, 65.0, 1.0],
            error: [0.55, 0.15, 25.0, 1.0],

            radius_sm: 0.0,
            radius_md: 2.0,
            radius_lg: 4.0,

            effect_blur: 0.0,
            effect_grain: 0.08,
            effect_scanline: 0.0,
            effect_glow: 0.0,
        }
    }

    /// Neon theme: Cyberpunk pinks, cyans, dark backgrounds
    /// Perfect for Cyberpunk, Shadowrun, and high-tech settings
    pub fn neon() -> Self {
        ThemeDefinition {
            // Deep dark purple/black backgrounds
            bg_deep: [0.08, 0.01, 270.0, 1.0],
            bg_surface: [0.12, 0.02, 280.0, 1.0],
            bg_elevated: [0.18, 0.03, 290.0, 1.0],

            // Bright cyan text
            text_primary: [0.95, 0.02, 200.0, 1.0],
            text_secondary: [0.80, 0.04, 190.0, 1.0],
            text_muted: [0.60, 0.05, 280.0, 1.0],

            // Hot pink/magenta primary, cyan secondary
            accent_primary: [0.70, 0.25, 330.0, 1.0],
            accent_secondary: [0.65, 0.20, 195.0, 1.0],
            accent_hover: [0.75, 0.28, 330.0, 1.0],

            // Borders
            border_subtle: [0.25, 0.08, 280.0, 0.3],
            border_strong: [0.70, 0.20, 330.0, 0.5],
            border_color: [0.35, 0.12, 300.0, 0.4],
            shadow_color: [0.20, 0.15, 330.0, 0.4],

            // Semantic colors (neon palette)
            success: [0.70, 0.18, 160.0, 1.0],
            warning: [0.75, 0.20, 55.0, 1.0],
            error: [0.65, 0.22, 25.0, 1.0],

            radius_sm: 0.0,
            radius_md: 4.0,
            radius_lg: 8.0,

            effect_blur: 8.0,
            effect_grain: 0.03,
            effect_scanline: 0.1,
            effect_glow: 0.8,
        }
    }
}

// ============================================================================
// Theme Presets
// ============================================================================

/// Get a theme preset definition by name
pub fn get_preset(name: &str) -> ThemeDefinition {
    match name {
        "fantasy" => ThemeDefinition::fantasy(),
        "cosmic" => ThemeDefinition::cosmic(),
        "terminal" => ThemeDefinition::terminal(),
        "noir" => ThemeDefinition::noir(),
        "neon" => ThemeDefinition::neon(),
        _ => ThemeDefinition::default(),
    }
}

/// Get a theme preset definition by ThemeType
pub fn get_preset_by_type(theme: ThemeType) -> ThemeDefinition {
    match theme {
        ThemeType::Fantasy => ThemeDefinition::fantasy(),
        ThemeType::Cosmic => ThemeDefinition::cosmic(),
        ThemeType::Terminal => ThemeDefinition::terminal(),
        ThemeType::Noir => ThemeDefinition::noir(),
        ThemeType::Neon => ThemeDefinition::neon(),
    }
}

// ============================================================================
// CSS Generation
// ============================================================================

/// Blended theme definition with zeroed values for accumulation
fn zeroed_theme() -> ThemeDefinition {
    ThemeDefinition {
        bg_deep: [0.0; 4],
        bg_surface: [0.0; 4],
        bg_elevated: [0.0; 4],
        text_primary: [0.0; 4],
        text_secondary: [0.0; 4],
        text_muted: [0.0; 4],
        accent_primary: [0.0; 4],
        accent_secondary: [0.0; 4],
        accent_hover: [0.0; 4],
        border_subtle: [0.0; 4],
        border_strong: [0.0; 4],
        border_color: [0.0; 4],
        shadow_color: [0.0; 4],
        success: [0.0; 4],
        warning: [0.0; 4],
        error: [0.0; 4],
        radius_sm: 0.0,
        radius_md: 0.0,
        radius_lg: 0.0,
        effect_blur: 0.0,
        effect_grain: 0.0,
        effect_scanline: 0.0,
        effect_glow: 0.0,
    }
}

/// Generate CSS custom properties from blended theme weights.
/// This performs weighted interpolation in OKLCH color space.
pub fn generate_css(weights: &ThemeWeights) -> String {
    let mixed = blend_themes(weights);

    // Construct CSS with all theme variables
    format!(
        r#"
        :root {{
            /* Background colors */
            --bg-deep: {bg_deep};
            --bg-surface: {bg_surface};
            --bg-elevated: {bg_elevated};

            /* Text colors */
            --text-primary: {text_primary};
            --text-secondary: {text_secondary};
            --text-muted: {text_muted};

            /* Accent colors */
            --accent: {accent_primary};
            --accent-primary: {accent_primary};
            --accent-secondary: {accent_secondary};
            --accent-hover: {accent_hover};

            /* Border & Shadow */
            --border-subtle: {border_subtle};
            --border-strong: {border_strong};
            --border-color: {border_color};
            --shadow-color: {shadow_color};

            /* Semantic colors */
            --success: {success};
            --warning: {warning};
            --error: {error};
            --danger: {error};

            /* Border radii */
            --radius-sm: {radius_sm}px;
            --radius-md: {radius_md}px;
            --radius-lg: {radius_lg}px;

            /* Effect values */
            --effect-blur: {effect_blur}px;
            --effect-grain: {effect_grain};
            --effect-scanline: {effect_scanline};
            --effect-glow: {effect_glow};
        }}
    "#,
        bg_deep = fmt_oklch(mixed.bg_deep),
        bg_surface = fmt_oklch(mixed.bg_surface),
        bg_elevated = fmt_oklch(mixed.bg_elevated),
        text_primary = fmt_oklch(mixed.text_primary),
        text_secondary = fmt_oklch(mixed.text_secondary),
        text_muted = fmt_oklch(mixed.text_muted),
        accent_primary = fmt_oklch(mixed.accent_primary),
        accent_secondary = fmt_oklch(mixed.accent_secondary),
        accent_hover = fmt_oklch(mixed.accent_hover),
        border_subtle = fmt_oklch(mixed.border_subtle),
        border_strong = fmt_oklch(mixed.border_strong),
        border_color = fmt_oklch(mixed.border_color),
        shadow_color = fmt_oklch(mixed.shadow_color),
        success = fmt_oklch(mixed.success),
        warning = fmt_oklch(mixed.warning),
        error = fmt_oklch(mixed.error),
        radius_sm = mixed.radius_sm,
        radius_md = mixed.radius_md,
        radius_lg = mixed.radius_lg,
        effect_blur = mixed.effect_blur,
        effect_grain = mixed.effect_grain,
        effect_scanline = mixed.effect_scanline,
        effect_glow = mixed.effect_glow
    )
}

/// Blend multiple themes according to weights
pub fn blend_themes(weights: &ThemeWeights) -> ThemeDefinition {
    let mut mixed = zeroed_theme();

    let definitions = [
        (weights.fantasy, get_preset("fantasy")),
        (weights.cosmic, get_preset("cosmic")),
        (weights.terminal, get_preset("terminal")),
        (weights.noir, get_preset("noir")),
        (weights.neon, get_preset("neon")),
    ];

    let total_weight: f32 = definitions.iter().map(|(w, _)| w).sum();
    let norm = if total_weight > 0.0 {
        1.0 / total_weight
    } else {
        1.0
    };

    // Collect colors for proper hue interpolation
    let mut bg_deep_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut bg_surface_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut bg_elevated_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut text_primary_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut text_secondary_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut text_muted_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut accent_primary_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut accent_secondary_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut accent_hover_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut border_subtle_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut border_strong_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut border_color_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut shadow_color_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut success_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut warning_colors: Vec<(f32, OklchColor)> = Vec::new();
    let mut error_colors: Vec<(f32, OklchColor)> = Vec::new();

    for (w, def) in definitions.iter() {
        let weight = w * norm;
        if weight <= 0.0 {
            continue;
        }

        // Collect colors for blending
        bg_deep_colors.push((weight, def.bg_deep));
        bg_surface_colors.push((weight, def.bg_surface));
        bg_elevated_colors.push((weight, def.bg_elevated));
        text_primary_colors.push((weight, def.text_primary));
        text_secondary_colors.push((weight, def.text_secondary));
        text_muted_colors.push((weight, def.text_muted));
        accent_primary_colors.push((weight, def.accent_primary));
        accent_secondary_colors.push((weight, def.accent_secondary));
        accent_hover_colors.push((weight, def.accent_hover));
        border_subtle_colors.push((weight, def.border_subtle));
        border_strong_colors.push((weight, def.border_strong));
        border_color_colors.push((weight, def.border_color));
        shadow_color_colors.push((weight, def.shadow_color));
        success_colors.push((weight, def.success));
        warning_colors.push((weight, def.warning));
        error_colors.push((weight, def.error));

        // Numeric values (simple weighted average)
        mixed.radius_sm += def.radius_sm * weight;
        mixed.radius_md += def.radius_md * weight;
        mixed.radius_lg += def.radius_lg * weight;
        mixed.effect_blur += def.effect_blur * weight;
        mixed.effect_grain += def.effect_grain * weight;
        mixed.effect_scanline += def.effect_scanline * weight;
        mixed.effect_glow += def.effect_glow * weight;
    }

    // Blend colors using proper hue interpolation
    mixed.bg_deep = blend_oklch(&bg_deep_colors);
    mixed.bg_surface = blend_oklch(&bg_surface_colors);
    mixed.bg_elevated = blend_oklch(&bg_elevated_colors);
    mixed.text_primary = blend_oklch(&text_primary_colors);
    mixed.text_secondary = blend_oklch(&text_secondary_colors);
    mixed.text_muted = blend_oklch(&text_muted_colors);
    mixed.accent_primary = blend_oklch(&accent_primary_colors);
    mixed.accent_secondary = blend_oklch(&accent_secondary_colors);
    mixed.accent_hover = blend_oklch(&accent_hover_colors);
    mixed.border_subtle = blend_oklch(&border_subtle_colors);
    mixed.border_strong = blend_oklch(&border_strong_colors);
    mixed.border_color = blend_oklch(&border_color_colors);
    mixed.shadow_color = blend_oklch(&shadow_color_colors);
    mixed.success = blend_oklch(&success_colors);
    mixed.warning = blend_oklch(&warning_colors);
    mixed.error = blend_oklch(&error_colors);

    mixed
}

/// Format OKLCH color as CSS string
pub fn fmt_oklch(c: OklchColor) -> String {
    if c[3] >= 0.99 {
        format!("oklch({:.2}% {:.3} {:.1})", c[0] * 100.0, c[1], c[2])
    } else {
        format!(
            "oklch({:.2}% {:.3} {:.1} / {:.2})",
            c[0] * 100.0,
            c[1],
            c[2],
            c[3]
        )
    }
}

// ============================================================================
// Theme State (Leptos Context)
// ============================================================================

/// Reactive theme state container for use with Leptos context
#[derive(Clone, Copy)]
pub struct ThemeState {
    /// Current theme weights for blending
    pub weights: RwSignal<ThemeWeights>,
    /// The name of the current preset (if using a single preset)
    pub current_preset: RwSignal<Option<String>>,
}

impl ThemeState {
    /// Create new theme state with default (fantasy) theme
    pub fn new() -> Self {
        Self {
            weights: RwSignal::new(ThemeWeights::default()),
            current_preset: RwSignal::new(Some("fantasy".to_string())),
        }
    }

    /// Set theme to a single preset
    pub fn set_preset(&self, name: &str) {
        self.weights.set(ThemeWeights::preset(name));
        self.current_preset.set(Some(name.to_string()));
    }

    /// Set custom theme weights (clears preset name)
    pub fn set_weights(&self, weights: ThemeWeights) {
        self.weights.set(weights);
        self.current_preset.set(None);
    }

    /// Get the current CSS for the theme
    pub fn get_css(&self) -> String {
        generate_css(&self.weights.get())
    }
}

impl Default for ThemeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Provide theme state to the component tree via context
pub fn provide_theme_state() {
    provide_context(ThemeState::new());
}

/// Retrieve the ThemeState from context
pub fn use_theme_state() -> ThemeState {
    expect_context::<ThemeState>()
}

/// Try to retrieve the ThemeState from context
pub fn try_use_theme_state() -> Option<ThemeState> {
    use_context::<ThemeState>()
}

// ============================================================================
// Preset Names
// ============================================================================

/// List of all available theme preset names
pub const PRESET_NAMES: &[&str] = &["fantasy", "cosmic", "terminal", "noir", "neon"];

/// Get the dominant theme class from a HashMap of weights (for campaign settings compatibility)
pub fn get_dominant_theme(weights: &std::collections::HashMap<String, f32>) -> String {
    let mut max = 0.0_f32;
    let mut theme = "theme-fantasy";

    for (name, weight) in weights.iter() {
        if *weight > max {
            max = *weight;
            theme = match name.as_str() {
                "fantasy" => "theme-fantasy",
                "cosmic" => "theme-cosmic",
                "terminal" => "theme-terminal",
                "noir" => "theme-noir",
                "neon" => "theme-neon",
                _ => continue,
            };
        }
    }

    theme.to_string()
}

/// Get a human-readable description for a theme preset
pub fn preset_description(name: &str) -> &'static str {
    match name {
        "fantasy" => "Warm, magical tones with golden accents - ideal for D&D and Pathfinder",
        "cosmic" => "Deep teal and cyan hues evoking cosmic horror - perfect for Call of Cthulhu",
        "terminal" => "Classic green-on-black hacker aesthetic with scanlines",
        "noir" => "Sepia-toned noir atmosphere - great for Delta Green and spy thrillers",
        "neon" => "Vibrant cyberpunk palette with pink/purple neon glow",
        _ => "Unknown theme preset",
    }
}
