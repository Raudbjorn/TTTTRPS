//! Centralized Teal & Coral color theme for the TTTTRPS TUI.
//!
//! All color constants are RGB truecolor. Views import from here
//! instead of using inline `Color::*` literals.

use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders};

// ── Primary palette ─────────────────────────────────────────────────────────

/// Teal — primary accent, active items, focused borders.
pub const PRIMARY: Color = Color::Rgb(0x00, 0x80, 0x80);
/// Light teal — highlights, hints, secondary focus.
pub const PRIMARY_LIGHT: Color = Color::Rgb(0x00, 0x96, 0x88);
/// Dark teal — subtle backgrounds, pressed states.
pub const PRIMARY_DARK: Color = Color::Rgb(0x00, 0x4D, 0x40);

// ── Accent ──────────────────────────────────────────────────────────────────

/// Coral — accent, calls to action, important items.
pub const ACCENT: Color = Color::Rgb(0xFF, 0x7F, 0x50);
/// Soft coral — hover states, secondary emphasis.
pub const ACCENT_SOFT: Color = Color::Rgb(0xFF, 0x8A, 0x65);

// ── Backgrounds ─────────────────────────────────────────────────────────────

/// Charcoal — base background.
pub const BG_BASE: Color = Color::Rgb(0x0A, 0x19, 0x19);
/// Surface — elevated panels, sidebar.
pub const BG_SURFACE: Color = Color::Rgb(0x12, 0x26, 0x26);

// ── Text ────────────────────────────────────────────────────────────────────

/// Primary text.
pub const TEXT: Color = Color::Rgb(0xE0, 0xE0, 0xE0);
/// Muted text — secondary labels, borders.
pub const TEXT_MUTED: Color = Color::Rgb(0x80, 0x80, 0x80);
/// Dim text — disabled items, faint hints.
pub const TEXT_DIM: Color = Color::Rgb(0x50, 0x50, 0x50);

// ── Semantic ────────────────────────────────────────────────────────────────

/// Error — destructive actions, failures.
pub const ERROR: Color = Color::Rgb(0xEF, 0x53, 0x50);
/// Success — confirmations, healthy status.
pub const SUCCESS: Color = Color::Rgb(0x66, 0xBB, 0x6A);
/// Warning — alerts, degraded status.
pub const WARNING: Color = Color::Rgb(0xFF, 0xA7, 0x26);
/// Info — informational highlights.
pub const INFO: Color = Color::Rgb(0x42, 0xA5, 0xF5);

// ── Domain ──────────────────────────────────────────────────────────────────

/// NPC dialogue — lavender.
pub const NPC: Color = Color::Rgb(0xCE, 0x93, 0xD8);

// ── Style helpers ───────────────────────────────────────────────────────────

/// Primary-colored bold text (titles, active items).
pub fn title() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Section header style.
pub fn heading() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

/// Focused border style.
pub fn border_focused() -> Style {
    Style::default().fg(PRIMARY)
}

/// Unfocused border style.
pub fn border_default() -> Style {
    Style::default().fg(TEXT_DIM)
}

/// Highlighted/selected item.
pub fn highlight() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Muted label text.
pub fn muted() -> Style {
    Style::default().fg(TEXT_MUTED)
}

/// Dim text for disabled/faint items.
pub fn dim() -> Style {
    Style::default().fg(TEXT_DIM)
}

/// Key hint style (e.g., "[q]:quit").
pub fn key_hint() -> Style {
    Style::default().fg(TEXT_DIM)
}

/// Status bar brand badge.
pub fn brand_badge() -> Style {
    Style::default()
        .fg(BG_BASE)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

/// Insert mode badge.
pub fn insert_badge() -> Style {
    Style::default()
        .fg(BG_BASE)
        .bg(PRIMARY_LIGHT)
        .add_modifier(Modifier::BOLD)
}

// ── Block builders ──────────────────────────────────────────────────────────

/// A bordered block with focused styling.
pub fn block_focused(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(border_focused())
}

/// A bordered block with default (unfocused) styling.
pub fn block_default(title: &str) -> Block<'_> {
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_style(border_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_is_teal() {
        assert_eq!(PRIMARY, Color::Rgb(0x00, 0x80, 0x80));
    }

    #[test]
    fn test_accent_is_coral() {
        assert_eq!(ACCENT, Color::Rgb(0xFF, 0x7F, 0x50));
    }

    #[test]
    fn test_style_helpers_return_non_default() {
        assert_ne!(title(), Style::default());
        assert_ne!(heading(), Style::default());
        assert_ne!(highlight(), Style::default());
        assert_ne!(muted(), Style::default());
    }
}
