//! Command palette — fuzzy-searchable registry of all TUI actions.
//!
//! Opens on Ctrl+P, provides nucleo-powered fuzzy matching with
//! match highlighting, category grouping, and keybinding hints.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use nucleo::{
    pattern::{Atom, AtomKind, CaseMatching, Normalization},
    Matcher, Utf32Str,
};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::tui::events::Action;
use crate::tui::widgets::input_buffer::InputBuffer;

// ============================================================================
// Command types
// ============================================================================

/// A command that can be invoked from the palette.
#[derive(Clone)]
pub struct Command {
    pub label: &'static str,
    pub description: &'static str,
    pub category: CommandCategory,
    pub keybinding: Option<&'static str>,
    pub action: Action,
}

/// Command grouping for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommandCategory {
    Navigation,
    Chat,
    System,
}

impl CommandCategory {
    fn label(self) -> &'static str {
        match self {
            Self::Navigation => "Navigation",
            Self::Chat => "Chat",
            Self::System => "System",
        }
    }

    fn color(self) -> Color {
        match self {
            Self::Navigation => Color::Blue,
            Self::Chat => Color::Green,
            Self::System => Color::Magenta,
        }
    }
}

/// A command that matched the current filter, with score and indices.
struct FilteredCommand {
    command_index: usize,
    score: u16,
    indices: Vec<u32>,
}

/// Result of handling a palette input event.
pub enum PaletteResult {
    /// Event consumed, palette stays open.
    Consumed,
    /// User selected a command — close and dispatch this action.
    Execute(Action),
    /// User pressed Esc — close without action.
    Close,
}

// ============================================================================
// Command registry
// ============================================================================

pub fn build_command_registry() -> Vec<Command> {
    vec![
        Command {
            label: "Go to Chat",
            description: "Switch to the Chat view",
            category: CommandCategory::Navigation,
            keybinding: Some("1"),
            action: Action::FocusChat,
        },
        Command {
            label: "Go to Library",
            description: "Switch to the Library view",
            category: CommandCategory::Navigation,
            keybinding: Some("2"),
            action: Action::FocusLibrary,
        },
        Command {
            label: "Go to Campaign",
            description: "Switch to the Campaign view",
            category: CommandCategory::Navigation,
            keybinding: Some("3"),
            action: Action::FocusCampaign,
        },
        Command {
            label: "Go to Settings",
            description: "Switch to the Settings view",
            category: CommandCategory::Navigation,
            keybinding: Some("4"),
            action: Action::FocusSettings,
        },
        Command {
            label: "Go to Generation",
            description: "Switch to the Generation view",
            category: CommandCategory::Navigation,
            keybinding: Some("5"),
            action: Action::FocusGeneration,
        },
        Command {
            label: "Go to Personality",
            description: "Switch to the Personality view",
            category: CommandCategory::Navigation,
            keybinding: Some("6"),
            action: Action::FocusPersonality,
        },
        Command {
            label: "New Chat Session",
            description: "Archive current session and start fresh",
            category: CommandCategory::Chat,
            keybinding: None,
            action: Action::NewChatSession,
        },
        Command {
            label: "Clear Chat",
            description: "Clear all messages in current session",
            category: CommandCategory::Chat,
            keybinding: None,
            action: Action::ClearChat,
        },
        Command {
            label: "Refresh Settings",
            description: "Reload settings data from backend",
            category: CommandCategory::System,
            keybinding: Some("r"),
            action: Action::RefreshSettings,
        },
        Command {
            label: "Show Help",
            description: "Open the keybindings help modal",
            category: CommandCategory::System,
            keybinding: Some("?"),
            action: Action::ShowHelp,
        },
        Command {
            label: "Quit",
            description: "Exit the application",
            category: CommandCategory::System,
            keybinding: Some("q"),
            action: Action::Quit,
        },
    ]
}

// ============================================================================
// Palette state
// ============================================================================

pub struct CommandPaletteState {
    input: InputBuffer,
    commands: Vec<Command>,
    filtered: Vec<FilteredCommand>,
    selected: usize,
    matcher: Matcher,
}

impl CommandPaletteState {
    pub fn new(commands: Vec<Command>) -> Self {
        let len = commands.len();
        let mut state = Self {
            input: InputBuffer::new(),
            commands,
            filtered: Vec::with_capacity(len),
            selected: 0,
            matcher: Matcher::default(),
        };
        state.refilter();
        state
    }

    /// Re-run fuzzy matching against all commands using current input.
    fn refilter(&mut self) {
        self.filtered.clear();
        let query = self.input.text();

        if query.trim().is_empty() {
            // Empty input: show all commands, no highlighting
            for (i, _) in self.commands.iter().enumerate() {
                self.filtered.push(FilteredCommand {
                    command_index: i,
                    score: 0,
                    indices: Vec::new(),
                });
            }
        } else {
            let atom = Atom::new(
                query,
                CaseMatching::Ignore,
                Normalization::Smart,
                AtomKind::Fuzzy,
                false,
            );

            let mut buf = Vec::new();
            for (i, cmd) in self.commands.iter().enumerate() {
                let haystack = Utf32Str::new(cmd.label, &mut buf);
                let mut indices = Vec::new();
                if let Some(score) = atom.indices(haystack, &mut self.matcher, &mut indices) {
                    self.filtered.push(FilteredCommand {
                        command_index: i,
                        score,
                        indices,
                    });
                }
            }

            // Sort by score descending
            self.filtered.sort_by(|a, b| b.score.cmp(&a.score));
        }

        // Clamp selection
        if self.filtered.is_empty() {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(self.filtered.len() - 1);
        }
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + 1) % self.filtered.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = (self.selected + self.filtered.len() - 1) % self.filtered.len();
        }
    }

    pub fn selected_action(&self) -> Option<Action> {
        self.filtered
            .get(self.selected)
            .map(|fc| self.commands[fc.command_index].action.clone())
    }

    // ── Input handling ──────────────────────────────────────────────

    pub fn handle_input(&mut self, event: &Event) -> PaletteResult {
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        else {
            return PaletteResult::Consumed;
        };

        match (*modifiers, *code) {
            (KeyModifiers::NONE, KeyCode::Esc) => PaletteResult::Close,
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if let Some(action) = self.selected_action() {
                    PaletteResult::Execute(action)
                } else {
                    PaletteResult::Close
                }
            }
            (KeyModifiers::NONE, KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                self.select_prev();
                PaletteResult::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                self.select_next();
                PaletteResult::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.input.backspace();
                self.refilter();
                PaletteResult::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Delete) => {
                self.input.delete();
                self.refilter();
                PaletteResult::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Left) => {
                self.input.move_left();
                PaletteResult::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                self.input.move_right();
                PaletteResult::Consumed
            }
            (KeyModifiers::NONE, KeyCode::Home) => {
                self.input.move_home();
                PaletteResult::Consumed
            }
            (KeyModifiers::NONE, KeyCode::End) => {
                self.input.move_end();
                PaletteResult::Consumed
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.input.clear();
                self.refilter();
                PaletteResult::Consumed
            }
            (_, KeyCode::Char(c)) => {
                self.input.insert_char(c);
                self.refilter();
                PaletteResult::Consumed
            }
            _ => PaletteResult::Consumed, // Swallow everything else
        }
    }

    // ── Rendering ───────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let modal = centered_palette_rect(area);

        frame.render_widget(Clear, modal);

        let block = Block::default()
            .title(" Command Palette ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        let chunks = Layout::vertical([
            Constraint::Length(1), // Input
            Constraint::Length(1), // Separator
            Constraint::Min(1),   // Results
        ])
        .split(inner);

        // Input field with cursor
        self.render_input(frame, chunks[0]);

        // Separator
        let sep = Line::styled(
            "─".repeat(chunks[1].width as usize),
            Style::default().fg(Color::DarkGray),
        );
        frame.render_widget(Paragraph::new(sep), chunks[1]);

        // Results
        self.render_results(frame, chunks[2]);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let text = self.input.text();
        let cursor = self.input.cursor_position();

        let line = if text.is_empty() {
            Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    "Type to search...",
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        } else {
            let before = &text[..cursor];
            let cursor_char = text[cursor..]
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_else(|| " ".to_string());
            let after_cursor = if cursor < text.len() {
                &text[cursor + cursor_char.len()..]
            } else {
                ""
            };

            Line::from(vec![
                Span::styled("> ", Style::default().fg(Color::Cyan)),
                Span::raw(before.to_string()),
                Span::styled(
                    cursor_char,
                    Style::default().bg(Color::White).fg(Color::Black),
                ),
                Span::raw(after_cursor.to_string()),
            ])
        };

        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_results(&self, frame: &mut Frame, area: Rect) {
        if self.filtered.is_empty() {
            let no_match = Line::styled(
                "  No matching commands",
                Style::default().fg(Color::DarkGray),
            );
            frame.render_widget(Paragraph::new(no_match), area);
            return;
        }

        let visible_height = area.height as usize;
        let is_filtered = !self.input.text().trim().is_empty();

        let mut lines: Vec<Line> = Vec::new();

        if is_filtered {
            // Flat list sorted by score
            for (i, fc) in self.filtered.iter().enumerate() {
                if lines.len() >= visible_height {
                    break;
                }
                let cmd = &self.commands[fc.command_index];
                let is_selected = i == self.selected;
                lines.push(self.render_command_line(cmd, &fc.indices, is_selected, area.width));
            }
        } else {
            // Grouped by category
            let mut current_category: Option<CommandCategory> = None;
            for (i, fc) in self.filtered.iter().enumerate() {
                if lines.len() >= visible_height {
                    break;
                }
                let cmd = &self.commands[fc.command_index];

                // Category header
                if current_category != Some(cmd.category) {
                    current_category = Some(cmd.category);
                    if !lines.is_empty() && lines.len() < visible_height {
                        lines.push(Line::raw("")); // Spacer between groups
                    }
                    if lines.len() < visible_height {
                        lines.push(Line::styled(
                            format!("  {}", cmd.category.label()),
                            Style::default()
                                .fg(cmd.category.color())
                                .add_modifier(Modifier::BOLD),
                        ));
                    }
                }

                if lines.len() < visible_height {
                    let is_selected = i == self.selected;
                    lines.push(self.render_command_line(
                        cmd,
                        &fc.indices,
                        is_selected,
                        area.width,
                    ));
                }
            }
        }

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_command_line(
        &self,
        cmd: &Command,
        match_indices: &[u32],
        is_selected: bool,
        width: u16,
    ) -> Line<'static> {
        let mut spans: Vec<Span> = Vec::new();

        // Selection indicator
        let prefix = if is_selected { "▸ " } else { "  " };
        let prefix_style = if is_selected {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default()
        };
        spans.push(Span::styled(prefix.to_string(), prefix_style));

        // Label with match highlighting
        let base_style = if is_selected {
            Style::default().fg(Color::White).bold()
        } else {
            Style::default().fg(Color::White)
        };
        let highlight_style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        };

        for (i, ch) in cmd.label.chars().enumerate() {
            let style = if match_indices.contains(&(i as u32)) {
                highlight_style
            } else {
                base_style
            };
            spans.push(Span::styled(ch.to_string(), style));
        }

        // Keybinding hint (right-aligned)
        if let Some(key) = cmd.keybinding {
            let label_len = cmd.label.len() + 2; // prefix
            let key_display = format!(" [{key}]");
            let padding_needed = (width as usize)
                .saturating_sub(label_len)
                .saturating_sub(key_display.len());
            if padding_needed > 0 {
                spans.push(Span::raw(" ".repeat(padding_needed)));
            }
            spans.push(Span::styled(
                key_display,
                Style::default().fg(Color::DarkGray),
            ));
        }

        Line::from(spans)
    }
}

/// Calculate palette modal position — top-center, ~50% wide, ~40% tall.
fn centered_palette_rect(area: Rect) -> Rect {
    let width = (area.width * 50 / 100).max(30).min(area.width);
    let height = (area.height * 45 / 100).max(10).min(area.height);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = area.height / 6; // Slightly above center

    Rect::new(x, y, width, height)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_palette() -> CommandPaletteState {
        CommandPaletteState::new(build_command_registry())
    }

    #[test]
    fn test_empty_input_shows_all() {
        let palette = make_palette();
        assert_eq!(palette.filtered.len(), palette.commands.len());
        assert_eq!(palette.filtered.len(), 11);
    }

    #[test]
    fn test_typing_filters() {
        let mut palette = make_palette();
        palette.input.insert_char('q');
        palette.input.insert_char('u');
        palette.input.insert_char('i');
        palette.refilter();

        assert!(!palette.filtered.is_empty());
        // "Quit" should be the top result
        let top = &palette.commands[palette.filtered[0].command_index];
        assert_eq!(top.label, "Quit");
    }

    #[test]
    fn test_no_match() {
        let mut palette = make_palette();
        for c in "zzzzzz".chars() {
            palette.input.insert_char(c);
        }
        palette.refilter();
        assert!(palette.filtered.is_empty());
    }

    #[test]
    fn test_selection_wraps_forward() {
        let mut palette = make_palette();
        let len = palette.filtered.len();
        for _ in 0..len {
            palette.select_next();
        }
        assert_eq!(palette.selected, 0); // Wrapped
    }

    #[test]
    fn test_selection_wraps_backward() {
        let mut palette = make_palette();
        palette.select_prev(); // From 0 → last
        assert_eq!(palette.selected, palette.filtered.len() - 1);
    }

    #[test]
    fn test_selected_action_returns_correct_action() {
        let palette = make_palette();
        let action = palette.selected_action().unwrap();
        // First command is "Go to Chat" (Navigation category, index 0)
        assert_eq!(action, Action::FocusChat);
    }

    #[test]
    fn test_fuzzy_match_highlights() {
        let mut palette = make_palette();
        // Type "gochat" which should fuzzy-match "Go to Chat"
        for c in "gochat".chars() {
            palette.input.insert_char(c);
        }
        palette.refilter();

        assert!(!palette.filtered.is_empty());
        let top = &palette.filtered[0];
        let cmd = &palette.commands[top.command_index];
        assert_eq!(cmd.label, "Go to Chat");
        // Should have match indices
        assert!(!top.indices.is_empty());
    }

    #[test]
    fn test_category_ordering() {
        let registry = build_command_registry();
        // Navigation commands come first, then Chat, then System
        let categories: Vec<_> = registry.iter().map(|c| c.category).collect();
        let mut sorted = categories.clone();
        sorted.sort();
        assert_eq!(categories, sorted);
    }

    #[test]
    fn test_clear_input_restores_all() {
        let mut palette = make_palette();
        palette.input.insert_char('q');
        palette.refilter();
        let filtered_count = palette.filtered.len();
        assert!(filtered_count < 11);

        palette.input.clear();
        palette.refilter();
        assert_eq!(palette.filtered.len(), 11);
    }
}
