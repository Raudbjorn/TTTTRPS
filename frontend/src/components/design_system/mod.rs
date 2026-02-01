//! Design System Components for Leptos
//!
//! A collection of reusable, theme-aware UI components.

mod button;
mod input;
mod card;
mod badge;
mod select;
mod modal;
mod loading;
mod markdown;
mod slider;
mod effects;
mod toast;

pub use button::{Button, ButtonVariant, ButtonSize};
pub use input::Input;
pub use card::{Card, CardHeader, CardBody, CardTitle, CardDescription};
pub use badge::{Badge, BadgeVariant};
pub use select::{Select, SelectRw, SelectOption, SELECT_CLASS, OPTION_CLASS};
pub use modal::Modal;
pub use loading::{LoadingSpinner, TypingIndicator};
pub use markdown::Markdown;
pub use slider::{Slider, DiscreteSlider};
pub use effects::{
    BoxGlow, EffectsContainer, Flicker, FilmGrain, GlassPanel, GlitchText,
    GlowIntensity, RedactedReveal, RedactedText, RedactedTextToggle, Scanlines,
    ScanlineVariant, Stamp, StampStyle, TextGlow, Typewriter, Vignette,
    VignetteIntensity, recommended_effects, recommended_glow,
};
pub use toast::{Toast, ToastContainer};
