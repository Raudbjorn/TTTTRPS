//! Design System Components for Leptos
//!
//! A collection of reusable, theme-aware UI components.

mod badge;
mod button;
mod card;
mod effects;
mod input;
mod loading;
mod markdown;
mod modal;
mod select;
mod slider;
mod toast;

#[cfg(test)]
mod tests;

pub use badge::{Badge, BadgeVariant};
pub use button::{Button, ButtonSize, ButtonVariant};
pub use card::{Card, CardBody, CardDescription, CardHeader, CardTitle};
pub use effects::{
    recommended_effects, recommended_glow, BoxGlow, EffectsContainer, FilmGrain, Flicker,
    GlassPanel, GlitchText, GlowIntensity, RedactedReveal, RedactedText, RedactedTextToggle,
    ScanlineVariant, Scanlines, Stamp, StampStyle, TextGlow, Typewriter, Vignette,
    VignetteIntensity,
};
pub use input::Input;
pub use loading::{LoadingSpinner, TypingIndicator};
pub use markdown::Markdown;
pub use modal::Modal;
pub use select::{Select, SelectOption, SelectRw, OPTION_CLASS, SELECT_CLASS};
pub use slider::{DiscreteSlider, Slider};
pub use toast::{Toast, ToastContainer};
