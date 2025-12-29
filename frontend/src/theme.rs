//! Theme utilities for dynamic theme selection based on TTRPG system.
//!
//! This module provides functions to determine the appropriate visual theme
//! based on the campaign's game system. Themes are defined in `/public/themes.css`.
//!
//! Supported themes:
//! - `theme-fantasy`: D&D, Pathfinder, Warhammer Fantasy (default)
//! - `theme-cosmic`: Call of Cthulhu, Kult, Vaesen
//! - `theme-terminal`: Mothership, Alien RPG, Traveller, Stars Without Number
//! - `theme-noir`: Delta Green, Night's Black Agents
//! - `theme-neon`: Cyberpunk RED, Shadowrun, The Sprawl

/// Returns the CSS theme class name for a given TTRPG system string.
///
/// The function performs case-insensitive substring matching to determine
/// the appropriate theme. Falls back to "theme-fantasy" for unknown systems.
///
/// # Arguments
/// * `system` - The TTRPG system name from the campaign (e.g., "D&D 5e", "Call of Cthulhu")
///
/// # Returns
/// A static string slice containing the CSS class name (e.g., "theme-cosmic")
///
/// # Example
/// ```
/// let theme = get_theme_class("Call of Cthulhu 7th Edition");
/// assert_eq!(theme, "theme-cosmic");
/// ```
pub fn get_theme_class(system: &str) -> &'static str {
    let system_lower = system.to_lowercase();

    // Noir themes: 90s office paranoia
    if system_lower.contains("delta green")
        || system_lower.contains("night's black agents")
        || system_lower.contains("nba")
    {
        return "theme-noir";
    }

    // Cosmic horror themes
    if system_lower.contains("cthulhu")
        || system_lower.contains("coc")
        || system_lower.contains("kult")
        || system_lower.contains("vaesen")
    {
        return "theme-cosmic";
    }

    // Terminal/Sci-Fi themes
    if system_lower.contains("mothership")
        || (system_lower.contains("alien") && system_lower.contains("rpg"))
        || system_lower.contains("traveller")
        || system_lower.contains("stars without number")
        || system_lower.contains("swn")
    {
        return "theme-terminal";
    }

    // Neon/Cyberpunk themes
    if system_lower.contains("cyberpunk")
        || system_lower.contains("shadowrun")
        || system_lower.contains("the sprawl")
    {
        return "theme-neon";
    }

    // Fantasy (explicit matches)
    if system_lower.contains("d&d")
        || system_lower.contains("dnd")
        || system_lower.contains("5e")
        || system_lower.contains("pathfinder")
        || system_lower.contains("warhammer fantasy")
    {
        return "theme-fantasy";
    }

    // Default to fantasy for unknown systems
    "theme-fantasy"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fantasy_themes() {
        assert_eq!(get_theme_class("D&D 5e"), "theme-fantasy");
        assert_eq!(get_theme_class("DnD"), "theme-fantasy");
        assert_eq!(get_theme_class("Pathfinder 2e"), "theme-fantasy");
        assert_eq!(get_theme_class("Warhammer Fantasy"), "theme-fantasy");
    }

    #[test]
    fn test_cosmic_themes() {
        assert_eq!(get_theme_class("Call of Cthulhu"), "theme-cosmic");
        assert_eq!(get_theme_class("CoC 7th Edition"), "theme-cosmic");
        assert_eq!(get_theme_class("Kult: Divinity Lost"), "theme-cosmic");
        assert_eq!(get_theme_class("Vaesen"), "theme-cosmic");
    }

    #[test]
    fn test_terminal_themes() {
        assert_eq!(get_theme_class("Mothership"), "theme-terminal");
        assert_eq!(get_theme_class("Alien RPG"), "theme-terminal");
        assert_eq!(get_theme_class("Traveller"), "theme-terminal");
        assert_eq!(get_theme_class("Stars Without Number"), "theme-terminal");
        assert_eq!(get_theme_class("SWN"), "theme-terminal");
    }

    #[test]
    fn test_noir_themes() {
        assert_eq!(get_theme_class("Delta Green"), "theme-noir");
        assert_eq!(get_theme_class("Night's Black Agents"), "theme-noir");
    }

    #[test]
    fn test_neon_themes() {
        assert_eq!(get_theme_class("Cyberpunk RED"), "theme-neon");
        assert_eq!(get_theme_class("Shadowrun 6e"), "theme-neon");
        assert_eq!(get_theme_class("The Sprawl"), "theme-neon");
    }

    #[test]
    fn test_unknown_defaults_to_fantasy() {
        assert_eq!(get_theme_class("Unknown System"), "theme-fantasy");
        assert_eq!(get_theme_class(""), "theme-fantasy");
    }
}
