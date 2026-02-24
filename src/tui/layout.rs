//! Root layout computation for sidebar + main content + status bar.

use ratatui::layout::{Constraint, Layout, Rect};

/// Width of the expanded sidebar (group headers + labeled items).
pub const SIDEBAR_EXPANDED_WIDTH: u16 = 20;
/// Width of the collapsed sidebar (single-char icons).
pub const SIDEBAR_COLLAPSED_WIDTH: u16 = 3;
/// Auto-collapse sidebar below this terminal width.
pub const AUTO_COLLAPSE_THRESHOLD: u16 = 60;
/// Hide sidebar entirely below this terminal width.
pub const HIDE_SIDEBAR_THRESHOLD: u16 = 20;

/// Computed layout regions for a single frame.
pub struct AppLayout {
    /// Sidebar area (None if hidden).
    pub sidebar: Option<Rect>,
    /// Main content area.
    pub main: Rect,
    /// Status bar (bottom row).
    pub status: Rect,
}

/// Sidebar visibility state derived from terminal width and user preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarVisibility {
    Expanded,
    Collapsed,
    Hidden,
}

impl AppLayout {
    /// Compute layout regions from the terminal area and sidebar state.
    ///
    /// `user_collapsed`: user has toggled collapse with Ctrl+B.
    /// Returns the layout and effective sidebar visibility.
    pub fn compute(area: Rect, user_collapsed: bool) -> (Self, SidebarVisibility) {
        let visibility = if area.width < HIDE_SIDEBAR_THRESHOLD {
            SidebarVisibility::Hidden
        } else if user_collapsed || area.width < AUTO_COLLAPSE_THRESHOLD {
            SidebarVisibility::Collapsed
        } else {
            SidebarVisibility::Expanded
        };

        // Split vertically: content rows + status bar
        let rows = Layout::vertical([
            Constraint::Min(1),    // Content (sidebar + main)
            Constraint::Length(1), // Status bar
        ])
        .split(area);

        let content_area = rows[0];
        let status = rows[1];

        let (sidebar, main) = match visibility {
            SidebarVisibility::Hidden => (None, content_area),
            SidebarVisibility::Collapsed => {
                let cols = Layout::horizontal([
                    Constraint::Length(SIDEBAR_COLLAPSED_WIDTH),
                    Constraint::Min(1),
                ])
                .split(content_area);
                (Some(cols[0]), cols[1])
            }
            SidebarVisibility::Expanded => {
                let cols = Layout::horizontal([
                    Constraint::Length(SIDEBAR_EXPANDED_WIDTH),
                    Constraint::Min(1),
                ])
                .split(content_area);
                (Some(cols[0]), cols[1])
            }
        };

        (AppLayout { sidebar, main, status }, visibility)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expanded_layout() {
        let area = Rect::new(0, 0, 120, 40);
        let (layout, vis) = AppLayout::compute(area, false);
        assert_eq!(vis, SidebarVisibility::Expanded);
        assert!(layout.sidebar.is_some());
        assert_eq!(layout.sidebar.unwrap().width, SIDEBAR_EXPANDED_WIDTH);
        assert_eq!(layout.status.height, 1);
    }

    #[test]
    fn test_collapsed_by_user() {
        let area = Rect::new(0, 0, 120, 40);
        let (layout, vis) = AppLayout::compute(area, true);
        assert_eq!(vis, SidebarVisibility::Collapsed);
        assert_eq!(layout.sidebar.unwrap().width, SIDEBAR_COLLAPSED_WIDTH);
    }

    #[test]
    fn test_auto_collapse_narrow() {
        let area = Rect::new(0, 0, 55, 40);
        let (_, vis) = AppLayout::compute(area, false);
        assert_eq!(vis, SidebarVisibility::Collapsed);
    }

    #[test]
    fn test_hidden_very_narrow() {
        let area = Rect::new(0, 0, 18, 40);
        let (layout, vis) = AppLayout::compute(area, false);
        assert_eq!(vis, SidebarVisibility::Hidden);
        assert!(layout.sidebar.is_none());
        // Main gets full width minus status bar
        assert_eq!(layout.main.width, 18);
    }

    #[test]
    fn test_main_plus_sidebar_fills_width() {
        let area = Rect::new(0, 0, 100, 30);
        let (layout, _) = AppLayout::compute(area, false);
        let sidebar_w = layout.sidebar.map(|s| s.width).unwrap_or(0);
        assert_eq!(sidebar_w + layout.main.width, area.width);
    }
}
