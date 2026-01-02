//! Visual Effects Components for Leptos
//!
//! Provides Leptos wrapper components for the visual effects defined in CSS.
//! These components make it easy to apply theme-appropriate effects in Rust code
//! while respecting accessibility preferences.
//!
//! # Usage
//!
//! ```rust
//! use crate::components::design_system::effects::*;
//!
//! // Wrap content in a film grain effect
//! view! {
//!     <FilmGrain>
//!         <p>"Atmospheric horror content"</p>
//!     </FilmGrain>
//! }
//!
//! // Apply text glow to headers
//! view! {
//!     <TextGlow intensity=GlowIntensity::Intense>
//!         <h1>"SYSTEM ONLINE"</h1>
//!     </TextGlow>
//! }
//!
//! // Create redacted text that reveals on hover
//! view! {
//!     <p>"The agent's name is " <RedactedText>"John Smith"</RedactedText></p>
//! }
//! ```

use leptos::prelude::*;

// =============================================================================
// Effect Intensity Enums
// =============================================================================

/// Intensity level for glow effects
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum GlowIntensity {
    /// Subtle glow, barely visible
    Subtle,
    /// Standard glow effect
    #[default]
    Normal,
    /// Intense, eye-catching glow
    Intense,
    /// Animated pulsing glow
    Pulse,
}

impl GlowIntensity {
    fn text_class(&self) -> &'static str {
        match self {
            GlowIntensity::Subtle => "effect-text-glow-subtle",
            GlowIntensity::Normal => "effect-text-glow",
            GlowIntensity::Intense => "effect-text-glow-intense",
            GlowIntensity::Pulse => "effect-text-glow effect-text-glow-pulse",
        }
    }

    fn box_class(&self) -> &'static str {
        match self {
            GlowIntensity::Subtle => "effect-box-glow",
            GlowIntensity::Normal => "effect-box-glow",
            GlowIntensity::Intense => "effect-box-glow-intense",
            GlowIntensity::Pulse => "effect-box-glow effect-box-glow-pulse",
        }
    }
}

/// Scanline thickness variant
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ScanlineVariant {
    /// Standard thin scanlines
    #[default]
    Normal,
    /// Thicker, more pronounced scanlines
    Thick,
}

impl ScanlineVariant {
    fn class(&self) -> &'static str {
        match self {
            ScanlineVariant::Normal => "effect-scanlines",
            ScanlineVariant::Thick => "effect-scanlines-thick",
        }
    }
}

/// Redacted text reveal behavior
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum RedactedReveal {
    /// Reveals on hover
    #[default]
    Hover,
    /// Requires click to reveal (use with RwSignal)
    Click,
    /// Striped overlay style
    Striped,
}

impl RedactedReveal {
    fn class(&self) -> &'static str {
        match self {
            RedactedReveal::Hover => "effect-redacted",
            RedactedReveal::Click => "effect-redacted-permanent",
            RedactedReveal::Striped => "effect-redacted-striped",
        }
    }
}

/// Vignette intensity
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum VignetteIntensity {
    /// Standard vignette
    #[default]
    Normal,
    /// Stronger, more dramatic vignette
    Strong,
}

impl VignetteIntensity {
    fn class(&self) -> &'static str {
        match self {
            VignetteIntensity::Normal => "effect-vignette",
            VignetteIntensity::Strong => "effect-vignette-strong",
        }
    }
}

/// Stamp style variant
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum StampStyle {
    /// Bold, prominent stamp
    #[default]
    Bold,
    /// Faded, aged stamp
    Faded,
}

impl StampStyle {
    fn class(&self) -> &'static str {
        match self {
            StampStyle::Bold => "effect-stamp",
            StampStyle::Faded => "effect-stamp-faded",
        }
    }
}

// =============================================================================
// Film Grain Effect
// =============================================================================

/// Applies an animated film grain overlay to create a vintage/horror atmosphere.
///
/// Best used on the main application container or specific sections that
/// should have an aged, film-like quality.
///
/// # Props
/// - `class`: Additional CSS classes to apply
/// - `children`: Content to render with the grain effect
///
/// # Accessibility
/// - Effect is disabled when user prefers reduced motion
/// - Grain is purely decorative and does not affect content readability
#[component]
pub fn FilmGrain(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content to wrap with grain effect
    children: Children,
) -> impl IntoView {
    let full_class = format!("effect-film-grain relative {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

// =============================================================================
// CRT Scanlines Effect
// =============================================================================

/// Applies CRT scanline overlay for a retro terminal/monitor aesthetic.
///
/// Ideal for Terminal and Neon themes to create that classic CRT look.
///
/// # Props
/// - `variant`: Thickness of the scanlines (Normal or Thick)
/// - `class`: Additional CSS classes
/// - `children`: Content to display with scanlines
///
/// # Accessibility
/// - Effect is disabled when user prefers reduced motion
/// - Scanlines are purely decorative
#[component]
pub fn Scanlines(
    /// Scanline thickness variant
    #[prop(default = ScanlineVariant::Normal)]
    variant: ScanlineVariant,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content to wrap with scanlines
    children: Children,
) -> impl IntoView {
    let effect_class = variant.class();
    let full_class = format!("{effect_class} relative {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

// =============================================================================
// Text Glow Effect
// =============================================================================

/// Applies a glowing text effect using the theme's accent color.
///
/// Perfect for Terminal (green phosphor) and Neon (cyberpunk) themes.
///
/// # Props
/// - `intensity`: How strong the glow should be
/// - `class`: Additional CSS classes
/// - `children`: Text content to glow
///
/// # Accessibility
/// - Pulsing animation is disabled when user prefers reduced motion
/// - Glow is purely decorative and does not affect text readability
#[component]
pub fn TextGlow(
    /// Glow intensity level
    #[prop(default = GlowIntensity::Normal)]
    intensity: GlowIntensity,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content to apply glow to
    children: Children,
) -> impl IntoView {
    let effect_class = intensity.text_class();
    let full_class = format!("{effect_class} {class}");

    view! {
        <span class=full_class>
            {children()}
        </span>
    }
}

// =============================================================================
// Box Glow Effect
// =============================================================================

/// Applies a glowing box shadow effect using the theme's glow color.
///
/// Use on cards, panels, or buttons for emphasis.
///
/// # Props
/// - `intensity`: How strong the glow should be
/// - `class`: Additional CSS classes
/// - `children`: Content to wrap with glowing border
#[component]
pub fn BoxGlow(
    /// Glow intensity level
    #[prop(default = GlowIntensity::Normal)]
    intensity: GlowIntensity,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content to wrap
    children: Children,
) -> impl IntoView {
    let effect_class = intensity.box_class();
    let full_class = format!("{effect_class} {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

// =============================================================================
// Redacted Text Effect
// =============================================================================

/// Displays text as redacted/censored, revealing on interaction.
///
/// Perfect for Noir theme when displaying classified information.
///
/// # Props
/// - `reveal`: How the text should be revealed (Hover, Click, or Striped)
/// - `revealed`: For Click variant, whether text is currently revealed
/// - `on_reveal`: Callback when text is clicked (for Click variant)
/// - `class`: Additional CSS classes
/// - `children`: Text to redact
///
/// # Accessibility
/// - Uses cursor: pointer to indicate interactivity
/// - Supports keyboard focus for reveal
#[component]
pub fn RedactedText(
    /// How the text reveals itself
    #[prop(default = RedactedReveal::Hover)]
    reveal: RedactedReveal,
    /// For Click variant: whether currently revealed
    #[prop(default = false)]
    revealed: bool,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Sensitive text content to redact
    children: Children,
) -> impl IntoView {
    let effect_class = reveal.class();
    let revealed_class = if revealed && matches!(reveal, RedactedReveal::Click) {
        "revealed"
    } else {
        ""
    };
    let full_class = format!("{effect_class} {revealed_class} {class}");

    view! {
        <span
            class=full_class
            tabindex="0"
            role="button"
            aria-label="Redacted text - hover or click to reveal"
        >
            {children()}
        </span>
    }
}

/// Interactive redacted text with click-to-reveal functionality.
///
/// # Props
/// - `class`: Additional CSS classes
/// - `children`: Text to redact
#[component]
pub fn RedactedTextToggle(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Sensitive text content to redact
    children: Children,
) -> impl IntoView {
    let revealed = RwSignal::new(false);

    let toggle = move |_| {
        revealed.update(|r| *r = !*r);
    };

    let full_class = move || {
        let base = "effect-redacted-permanent";
        let state = if revealed.get() { "revealed" } else { "" };
        format!("{base} {state} {class}")
    };

    view! {
        <span
            class=full_class
            tabindex="0"
            role="button"
            aria-pressed=move || revealed.get().to_string()
            aria-label="Redacted text - click to toggle reveal"
            on:click=toggle
            on:keydown=move |evt: leptos::ev::KeyboardEvent| {
                if evt.key() == "Enter" || evt.key() == " " {
                    evt.prevent_default();
                    revealed.update(|r| *r = !*r);
                }
            }
        >
            {children()}
        </span>
    }
}

// =============================================================================
// Vignette Effect
// =============================================================================

/// Applies a vignette (darkened edges) effect for cinematic atmosphere.
///
/// Best applied to the main container for full-viewport effect.
///
/// # Props
/// - `intensity`: How strong the vignette should be
/// - `class`: Additional CSS classes
/// - `children`: Content to display with vignette
#[component]
pub fn Vignette(
    /// Vignette intensity
    #[prop(default = VignetteIntensity::Normal)]
    intensity: VignetteIntensity,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content to wrap
    children: Children,
) -> impl IntoView {
    let effect_class = intensity.class();
    let full_class = format!("{effect_class} relative {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

// =============================================================================
// Glitch Effect
// =============================================================================

/// Applies a chromatic aberration glitch effect to text.
///
/// Ideal for Neon/cyberpunk themes when displaying corrupted or digital text.
///
/// # Props
/// - `text`: The text to display (required for the data-text attribute)
/// - `class`: Additional CSS classes
///
/// # Note
/// This component requires the text as a prop because it needs to duplicate
/// it in the data-text attribute for the CSS effect to work.
#[component]
pub fn GlitchText(
    /// The text to display with glitch effect
    #[prop(into)]
    text: String,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
) -> impl IntoView {
    let full_class = format!("effect-glitch {class}");
    let text_clone = text.clone();

    view! {
        <span class=full_class data-text=text_clone>
            {text}
        </span>
    }
}

// =============================================================================
// Terminal Flicker Effect
// =============================================================================

/// Applies a subtle screen flicker effect simulating old CRT monitors.
///
/// Use sparingly on Terminal theme for atmosphere.
///
/// # Props
/// - `class`: Additional CSS classes
/// - `children`: Content to apply flicker to
///
/// # Accessibility
/// - Animation is disabled when user prefers reduced motion
#[component]
pub fn Flicker(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content to apply flicker to
    children: Children,
) -> impl IntoView {
    let full_class = format!("effect-flicker {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

// =============================================================================
// Typewriter Effect
// =============================================================================

/// Applies a typewriter text reveal animation.
///
/// Best for Noir theme when displaying text that should appear letter-by-letter.
///
/// # Props
/// - `class`: Additional CSS classes
/// - `children`: Text content to animate
///
/// # Accessibility
/// - Animation is disabled when user prefers reduced motion
/// - Text is immediately visible with reduced motion preference
#[component]
pub fn Typewriter(
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content to animate
    children: Children,
) -> impl IntoView {
    let full_class = format!("effect-typewriter font-mono {class}");

    view! {
        <span class=full_class>
            {children()}
        </span>
    }
}

// =============================================================================
// Glass Panel Effect
// =============================================================================

/// Applies a glassmorphism effect (frosted glass with blur).
///
/// Primary panel style for Fantasy theme.
///
/// # Props
/// - `strong`: Whether to use the stronger glass variant
/// - `class`: Additional CSS classes
/// - `children`: Content to display in the glass panel
#[component]
pub fn GlassPanel(
    /// Use stronger glass effect
    #[prop(default = false)]
    strong: bool,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content inside the glass panel
    children: Children,
) -> impl IntoView {
    let effect_class = if strong {
        "effect-glass-strong"
    } else {
        "effect-glass"
    };
    let full_class = format!("{effect_class} {class}");

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

// =============================================================================
// Classified Stamp Effect
// =============================================================================

/// Displays text as a "CLASSIFIED" or "TOP SECRET" style stamp.
///
/// Perfect for Noir theme document styling.
///
/// # Props
/// - `style`: Whether the stamp is Bold or Faded
/// - `class`: Additional CSS classes
/// - `children`: Stamp text (e.g., "CLASSIFIED", "TOP SECRET")
#[component]
pub fn Stamp(
    /// Stamp visual style
    #[prop(default = StampStyle::Bold)]
    style: StampStyle,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Stamp text
    children: Children,
) -> impl IntoView {
    let effect_class = style.class();
    let full_class = format!("{effect_class} {class}");

    view! {
        <span class=full_class aria-label="Document classification stamp">
            {children()}
        </span>
    }
}

// =============================================================================
// Effect Container - Combines Multiple Effects
// =============================================================================

/// Container that can apply multiple atmospheric effects at once.
///
/// Useful for applying consistent effects across a section.
///
/// # Props
/// - `grain`: Whether to apply film grain
/// - `scanlines`: Whether to apply CRT scanlines
/// - `vignette`: Whether to apply vignette
/// - `class`: Additional CSS classes
/// - `children`: Content to wrap with effects
#[component]
pub fn EffectsContainer(
    /// Apply film grain effect
    #[prop(default = false)]
    grain: bool,
    /// Apply scanlines effect
    #[prop(default = false)]
    scanlines: bool,
    /// Apply vignette effect
    #[prop(default = false)]
    vignette: bool,
    /// Additional CSS classes
    #[prop(into, optional)]
    class: String,
    /// Content to wrap
    children: Children,
) -> impl IntoView {
    let mut classes = vec!["relative"];

    if grain {
        classes.push("effect-film-grain");
    }
    if scanlines {
        classes.push("effect-scanlines");
    }
    if vignette {
        classes.push("effect-vignette");
    }

    let full_class = format!("{} {class}", classes.join(" "));

    view! {
        <div class=full_class>
            {children()}
        </div>
    }
}

// =============================================================================
// Theme-Aware Effect Selector
// =============================================================================

/// Returns recommended effects for a given theme preset.
///
/// # Arguments
/// - `theme`: Theme name ("fantasy", "cosmic", "terminal", "noir", "neon")
///
/// # Returns
/// Tuple of (grain, scanlines, vignette) booleans
pub fn recommended_effects(theme: &str) -> (bool, bool, bool) {
    match theme {
        "fantasy" => (false, false, false), // Fantasy relies on glassmorphism, not overlays
        "cosmic" => (true, false, true),    // Grain + vignette for horror atmosphere
        "terminal" => (true, true, false),  // Grain + scanlines for CRT look
        "noir" => (true, false, true),      // Grain + vignette for noir film look
        "neon" => (false, true, false),     // Just scanlines for cyberpunk
        _ => (false, false, false),
    }
}

/// Returns the recommended glow intensity for a theme.
pub fn recommended_glow(theme: &str) -> Option<GlowIntensity> {
    match theme {
        "fantasy" => Some(GlowIntensity::Subtle),
        "cosmic" => Some(GlowIntensity::Subtle),
        "terminal" => Some(GlowIntensity::Normal),
        "noir" => None, // Noir has no glow
        "neon" => Some(GlowIntensity::Intense),
        _ => Some(GlowIntensity::Subtle),
    }
}
