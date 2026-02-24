//! Collapsible left sidebar with grouped navigation.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::events::{AreaFocus, Focus, SidebarGroup};
use super::layout::SidebarVisibility;
use super::theme;

/// Sidebar navigation state.
pub struct SidebarState {
    /// Whether the user has toggled collapse (Ctrl+B).
    pub user_collapsed: bool,
    /// Currently highlighted item index (into Focus::ALL).
    pub selected: usize,
}

impl SidebarState {
    pub fn new() -> Self {
        Self {
            user_collapsed: false,
            selected: 0,
        }
    }

    /// Toggle user collapse preference.
    pub fn toggle_collapse(&mut self) {
        self.user_collapsed = !self.user_collapsed;
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1) % Focus::ALL.len();
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if self.selected == 0 {
            self.selected = Focus::ALL.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    /// Get the currently highlighted Focus.
    pub fn selected_focus(&self) -> Focus {
        Focus::ALL[self.selected]
    }

    /// Sync selection to match the active focus (e.g., after Tab navigation).
    pub fn sync_to_focus(&mut self, focus: Focus) {
        if let Some(idx) = Focus::ALL.iter().position(|&f| f == focus) {
            self.selected = idx;
        }
    }

    /// Render the sidebar.
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        visibility: SidebarVisibility,
        current_focus: Focus,
        area_focus: AreaFocus,
    ) {
        match visibility {
            SidebarVisibility::Hidden => {}
            SidebarVisibility::Collapsed => {
                self.render_collapsed(frame, area, current_focus);
            }
            SidebarVisibility::Expanded => {
                self.render_expanded(frame, area, current_focus, area_focus);
            }
        }
    }

    fn render_collapsed(&self, frame: &mut Frame, area: Rect, current_focus: Focus) {
        let mut lines: Vec<Line> = Vec::new();

        for group in SidebarGroup::ALL {
            for &view in group.views() {
                if lines.len() >= area.height as usize {
                    break;
                }
                let style = if view == current_focus {
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT_MUTED)
                };
                lines.push(Line::from(Span::styled(
                    format!(" {}", view.icon()),
                    style,
                )));
            }
        }

        frame.render_widget(
            Paragraph::new(lines).style(Style::default().bg(theme::BG_SURFACE)),
            area,
        );
    }

    fn render_expanded(
        &self,
        frame: &mut Frame,
        area: Rect,
        current_focus: Focus,
        area_focus: AreaFocus,
    ) {
        let mut lines: Vec<Line> = Vec::new();
        let sidebar_focused = area_focus == AreaFocus::Sidebar;

        // Track which index in Focus::ALL we're at
        let mut focus_idx = 0usize;

        for group in SidebarGroup::ALL {
            if lines.len() >= area.height as usize {
                break;
            }

            // Group header
            lines.push(Line::from(Span::styled(
                format!(" {}", group.label()),
                Style::default()
                    .fg(theme::PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));

            for &view in group.views() {
                if lines.len() >= area.height as usize {
                    break;
                }

                let is_current = view == current_focus;
                let is_selected = sidebar_focused && focus_idx == self.selected;

                let (prefix, style) = if is_selected && is_current {
                    (
                        "▸ ",
                        Style::default()
                            .fg(theme::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if is_selected {
                    (
                        "▸ ",
                        Style::default()
                            .fg(theme::TEXT)
                            .add_modifier(Modifier::BOLD),
                    )
                } else if is_current {
                    (
                        "  ",
                        Style::default()
                            .fg(theme::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ("  ", Style::default().fg(theme::TEXT_MUTED))
                };

                let label = format!("{prefix}{} {}", view.icon(), view.label());
                // Pad to fill sidebar width
                let padded = format!("{:<width$}", label, width = area.width as usize);
                lines.push(Line::from(Span::styled(padded, style)));

                focus_idx += 1;
            }
        }

        frame.render_widget(
            Paragraph::new(lines).style(Style::default().bg(theme::BG_SURFACE)),
            area,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = SidebarState::new();
        assert!(!state.user_collapsed);
        assert_eq!(state.selected, 0);
        assert_eq!(state.selected_focus(), Focus::Chat);
    }

    #[test]
    fn test_select_next_wraps() {
        let mut state = SidebarState::new();
        for _ in 0..Focus::ALL.len() {
            state.select_next();
        }
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn test_select_prev_wraps() {
        let mut state = SidebarState::new();
        state.select_prev();
        assert_eq!(state.selected, Focus::ALL.len() - 1);
    }

    #[test]
    fn test_toggle_collapse() {
        let mut state = SidebarState::new();
        assert!(!state.user_collapsed);
        state.toggle_collapse();
        assert!(state.user_collapsed);
        state.toggle_collapse();
        assert!(!state.user_collapsed);
    }

    #[test]
    fn test_sync_to_focus() {
        let mut state = SidebarState::new();
        state.sync_to_focus(Focus::Settings);
        assert_eq!(state.selected_focus(), Focus::Settings);
    }
}
