//! Dice Roller modal overlay.
//!
//! Global overlay activated by `Ctrl+D` or `Action::OpenDiceRoller`.
//! Uses the backend `DiceNotation` / `DiceRoller` / `RollResult` types.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::core::campaign::dice::{DiceNotation, DiceRoller, RollResult};
use crate::tui::theme;
use crate::tui::widgets::input_buffer::InputBuffer;

/// Maximum number of history entries to keep.
const MAX_HISTORY: usize = 20;

/// Quick-roll key mappings: (key, notation).
const QUICK_ROLLS: &[(char, &str)] = &[
    ('4', "d4"),
    ('6', "d6"),
    ('8', "d8"),
    ('0', "d10"),
    ('2', "d12"),
    ('d', "d20"),
];

/// State for the dice roller modal.
pub struct DiceRollerState {
    input: InputBuffer,
    history: Vec<(String, RollResult)>,
    error: Option<String>,
    roller: DiceRoller,
    /// Scroll offset for history (0 = most recent at bottom).
    history_scroll: usize,
}

impl DiceRollerState {
    pub fn new() -> Self {
        Self {
            input: InputBuffer::new(),
            history: Vec::new(),
            error: None,
            roller: DiceRoller::new(),
            history_scroll: 0,
        }
    }

    /// Handle input events. Returns `true` if the event was consumed.
    /// Returns `false` for Esc (caller should close the modal).
    pub fn handle_input(&mut self, event: &Event) -> bool {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            ..
        }) = event
        {
            // Ctrl+D closes (toggle behavior)
            if *modifiers == KeyModifiers::CONTROL && *code == KeyCode::Char('d') {
                return false;
            }

            match code {
                KeyCode::Esc => return false,
                KeyCode::Enter => {
                    self.submit();
                }
                KeyCode::Backspace => {
                    self.input.backspace();
                    self.error = None;
                }
                KeyCode::Delete => {
                    self.input.delete();
                    self.error = None;
                }
                KeyCode::Left => self.input.move_left(),
                KeyCode::Right => self.input.move_right(),
                KeyCode::Home => self.input.move_home(),
                KeyCode::End => self.input.move_end(),
                KeyCode::Up => {
                    // Scroll history up
                    if self.history_scroll < self.history.len().saturating_sub(1) {
                        self.history_scroll += 1;
                    }
                }
                KeyCode::Down => {
                    // Scroll history down
                    self.history_scroll = self.history_scroll.saturating_sub(1);
                }
                KeyCode::Char(c) => {
                    // Check quick-roll keys when input is empty
                    if self.input.is_empty() && !modifiers.contains(KeyModifiers::CONTROL) {
                        if let Some((_, notation)) = QUICK_ROLLS.iter().find(|(k, _)| k == c) {
                            self.roll_notation(notation);
                            return true;
                        }
                    }
                    self.input.insert_char(*c);
                    self.error = None;
                }
                _ => {}
            }
            true
        } else {
            false
        }
    }

    fn submit(&mut self) {
        let text = self.input.text().trim().to_string();
        if text.is_empty() {
            return;
        }
        self.roll_notation(&text);
        self.input.clear();
    }

    fn roll_notation(&mut self, notation: &str) {
        match DiceNotation::parse(notation) {
            Ok(parsed) => {
                let result = self.roller.roll(&parsed);
                self.history.push((notation.to_string(), result));
                if self.history.len() > MAX_HISTORY {
                    self.history.remove(0);
                }
                self.history_scroll = 0;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(format!("{e}"));
            }
        }
    }

    /// Render the dice roller as a centered modal overlay.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let modal = centered_modal(55, 50, area);
        frame.render_widget(Clear, modal);

        let block = Block::default()
            .title(" 🎲 Dice Roller ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT))
            .style(Style::default().bg(theme::BG_BASE));

        let inner = block.inner(modal);
        frame.render_widget(block, modal);

        // Split inner into: input (3) + error (1) + history (flex) + quick_keys (2) + hint (1)
        let chunks = Layout::vertical([
            Constraint::Length(3), // Input
            Constraint::Length(1), // Error / spacer
            Constraint::Min(3),   // History
            Constraint::Length(2), // Quick keys
            Constraint::Length(1), // Hint line
        ])
        .split(inner);

        self.render_input(frame, chunks[0]);
        self.render_error(frame, chunks[1]);
        self.render_history(frame, chunks[2]);
        self.render_quick_keys(frame, chunks[3]);
        self.render_hint(frame, chunks[4]);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let input_block = Block::default()
            .title(" Notation ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::PRIMARY_LIGHT));

        let input_inner = input_block.inner(area);
        frame.render_widget(input_block, area);

        let cursor_pos = self.input.cursor_position();
        let text = self.input.text();

        // Show placeholder if empty
        let line = if text.is_empty() {
            Line::from(Span::styled(
                "Type notation (e.g. 2d6+3) or press a quick key...",
                Style::default().fg(theme::TEXT_DIM),
            ))
        } else {
            Line::from(Span::styled(
                text.to_string(),
                Style::default().fg(theme::TEXT),
            ))
        };

        frame.render_widget(Paragraph::new(line), input_inner);

        // Place cursor
        frame.set_cursor_position((
            input_inner.x + cursor_pos as u16,
            input_inner.y,
        ));
    }

    fn render_error(&self, frame: &mut Frame, area: Rect) {
        if let Some(ref err) = self.error {
            let line = Line::from(Span::styled(
                format!(" {err}"),
                Style::default().fg(theme::ERROR),
            ));
            frame.render_widget(Paragraph::new(line), area);
        }
    }

    fn render_history(&self, frame: &mut Frame, area: Rect) {
        if self.history.is_empty() {
            let empty = vec![
                Line::raw(""),
                Line::from(Span::styled(
                    "No rolls yet",
                    Style::default().fg(theme::TEXT_DIM),
                )),
            ];
            frame.render_widget(
                Paragraph::new(empty).alignment(Alignment::Center),
                area,
            );
            return;
        }

        let visible_height = area.height as usize;
        let total = self.history.len();

        // Build lines from history (newest at bottom)
        let mut lines: Vec<Line<'static>> = Vec::with_capacity(total * 2);
        for (i, (notation, result)) in self.history.iter().enumerate() {
            let is_latest = i == total - 1;

            // Roll label
            let label_style = if is_latest {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_MUTED)
            };

            // Build result line
            let rolls_str: String = result
                .rolls
                .iter()
                .map(|r| r.value.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let mut spans: Vec<Span<'static>> = vec![
                Span::styled(format!(" {notation}: "), label_style),
                Span::styled(format!("[{rolls_str}]"), Style::default().fg(theme::TEXT)),
            ];

            if result.notation.modifier != 0 {
                spans.push(Span::styled(
                    format!(" ({}) ", result.subtotal),
                    Style::default().fg(theme::TEXT_DIM),
                ));
            }

            // Total
            let total_style = if result.is_critical() {
                Style::default()
                    .fg(theme::SUCCESS)
                    .add_modifier(Modifier::BOLD)
            } else if result.is_critical_fail() {
                Style::default()
                    .fg(theme::ERROR)
                    .add_modifier(Modifier::BOLD)
            } else if is_latest {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            spans.push(Span::styled(format!("= {}", result.total), total_style));

            // Critical/fumble indicator
            if result.is_critical() {
                spans.push(Span::styled(
                    " NAT 20!",
                    Style::default()
                        .fg(theme::SUCCESS)
                        .add_modifier(Modifier::BOLD),
                ));
            } else if result.is_critical_fail() {
                spans.push(Span::styled(
                    " NAT 1!",
                    Style::default()
                        .fg(theme::ERROR)
                        .add_modifier(Modifier::BOLD),
                ));
            }

            lines.push(Line::from(spans));
        }

        // Calculate scroll: history_scroll=0 means show bottom
        let scroll_offset = if lines.len() > visible_height {
            lines.len() - visible_height - self.history_scroll.min(lines.len() - visible_height)
        } else {
            0
        };

        let paragraph = Paragraph::new(lines.clone()).scroll((scroll_offset as u16, 0));
        frame.render_widget(paragraph, area);

        // Scrollbar if needed
        if lines.len() > visible_height {
            let mut scrollbar_state =
                ScrollbarState::new(lines.len().saturating_sub(visible_height))
                    .position(scroll_offset);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .thumb_style(Style::default().fg(theme::PRIMARY_LIGHT))
                    .track_style(Style::default().fg(theme::TEXT_DIM)),
                area,
                &mut scrollbar_state,
            );
        }
    }

    fn render_quick_keys(&self, frame: &mut Frame, area: Rect) {
        let keys_line = Line::from(vec![
            Span::styled(" Quick: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled("4", theme::key_hint()),
            Span::styled(":d4 ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("6", theme::key_hint()),
            Span::styled(":d6 ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("8", theme::key_hint()),
            Span::styled(":d8 ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("0", theme::key_hint()),
            Span::styled(":d10 ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("2", theme::key_hint()),
            Span::styled(":d12 ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("d", theme::key_hint()),
            Span::styled(":d20", Style::default().fg(theme::TEXT_DIM)),
        ]);

        let adv_line = Line::from(vec![
            Span::styled(
                " (quick keys work when input is empty)",
                Style::default().fg(theme::TEXT_DIM),
            ),
        ]);

        frame.render_widget(Paragraph::new(vec![keys_line, adv_line]), area);
    }

    fn render_hint(&self, frame: &mut Frame, area: Rect) {
        let hint = Line::from(vec![
            Span::styled(" Esc", theme::key_hint()),
            Span::styled(":close ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("Enter", theme::key_hint()),
            Span::styled(":roll ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled("↑/↓", theme::key_hint()),
            Span::styled(":scroll", Style::default().fg(theme::TEXT_DIM)),
        ]);
        frame.render_widget(Paragraph::new(hint), area);
    }
}

/// Center a modal of given percentage within the area.
fn centered_modal(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(v[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = DiceRollerState::new();
        assert!(state.history.is_empty());
        assert!(state.error.is_none());
        assert!(state.input.is_empty());
        assert_eq!(state.history_scroll, 0);
    }

    #[test]
    fn test_roll_valid_notation() {
        let mut state = DiceRollerState::new();
        state.roll_notation("2d6+3");
        assert_eq!(state.history.len(), 1);
        assert!(state.error.is_none());
        let (notation, result) = &state.history[0];
        assert_eq!(notation, "2d6+3");
        assert!(result.total >= 5 && result.total <= 15);
    }

    #[test]
    fn test_roll_invalid_notation() {
        let mut state = DiceRollerState::new();
        state.roll_notation("not_dice");
        assert!(state.history.is_empty());
        assert!(state.error.is_some());
    }

    #[test]
    fn test_history_cap() {
        let mut state = DiceRollerState::new();
        for _ in 0..25 {
            state.roll_notation("d20");
        }
        assert_eq!(state.history.len(), MAX_HISTORY);
    }

    #[test]
    fn test_submit_clears_input() {
        let mut state = DiceRollerState::new();
        state.input.insert_char('d');
        state.input.insert_char('2');
        state.input.insert_char('0');
        state.submit();
        assert!(state.input.is_empty());
        assert_eq!(state.history.len(), 1);
    }

    #[test]
    fn test_submit_empty_does_nothing() {
        let mut state = DiceRollerState::new();
        state.submit();
        assert!(state.history.is_empty());
        assert!(state.error.is_none());
    }

    #[test]
    fn test_esc_returns_false() {
        let mut state = DiceRollerState::new();
        let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!state.handle_input(&event));
    }

    #[test]
    fn test_ctrl_d_returns_false() {
        let mut state = DiceRollerState::new();
        let event = Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert!(!state.handle_input(&event));
    }

    #[test]
    fn test_quick_roll_when_empty() {
        let mut state = DiceRollerState::new();
        // Press 'd' for d20 when input is empty
        let event = Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        let consumed = state.handle_input(&event);
        assert!(consumed);
        assert_eq!(state.history.len(), 1);
        let (notation, result) = &state.history[0];
        assert_eq!(notation, "d20");
        assert!(result.total >= 1 && result.total <= 20);
    }

    #[test]
    fn test_quick_roll_disabled_with_input() {
        let mut state = DiceRollerState::new();
        // Type something first
        state.input.insert_char('2');
        // Now 'd' should be typed, not trigger quick roll
        let event = Event::Key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        state.handle_input(&event);
        assert!(state.history.is_empty());
        assert_eq!(state.input.text(), "2d");
    }

    #[test]
    fn test_scroll_up_down() {
        let mut state = DiceRollerState::new();
        // Add enough history to scroll
        for _ in 0..10 {
            state.roll_notation("d20");
        }

        let up = Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        state.handle_input(&up);
        assert_eq!(state.history_scroll, 1);
        state.handle_input(&up);
        assert_eq!(state.history_scroll, 2);

        let down = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        state.handle_input(&down);
        assert_eq!(state.history_scroll, 1);
    }

    #[test]
    fn test_scroll_resets_on_new_roll() {
        let mut state = DiceRollerState::new();
        for _ in 0..5 {
            state.roll_notation("d20");
        }
        state.history_scroll = 3;
        state.roll_notation("d20");
        assert_eq!(state.history_scroll, 0);
    }
}
