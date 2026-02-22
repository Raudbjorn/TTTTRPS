use std::io;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame, Terminal,
};
use tokio::sync::mpsc;

use super::events::{Action, AppEvent, Focus, Notification, NotificationLevel};
use super::services::Services;

/// Central application state (Elm architecture).
pub struct AppState {
    /// Whether the app is still running.
    pub running: bool,
    /// Currently focused top-level view.
    pub focus: Focus,
    /// Active notifications (max 3 visible).
    pub notifications: Vec<Notification>,
    /// Monotonic counter for notification IDs.
    notification_counter: u64,
    /// Whether the help modal is open.
    pub show_help: bool,
    /// Whether the command palette is open.
    pub show_command_palette: bool,
    /// Receiver for backend events.
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    /// Sender for pushing events from within the app.
    #[allow(dead_code)]
    event_tx: mpsc::UnboundedSender<AppEvent>,
    /// Backend services handle.
    #[allow(dead_code)]
    services: Services,
}

impl AppState {
    pub fn new(
        event_rx: mpsc::UnboundedReceiver<AppEvent>,
        event_tx: mpsc::UnboundedSender<AppEvent>,
        services: Services,
    ) -> Self {
        Self {
            running: true,
            focus: Focus::Chat,
            notifications: Vec::new(),
            notification_counter: 0,
            show_help: false,
            show_command_palette: false,
            event_rx,
            event_tx,
            services,
        }
    }

    // ── Elm event loop ──────────────────────────────────────────────────

    /// Main event loop: render → select → update → loop.
    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
        tick_rate: Duration,
    ) -> io::Result<()> {
        let mut tick_interval = tokio::time::interval(tick_rate);
        let mut event_stream = EventStream::new();

        while self.running {
            // Render
            terminal.draw(|frame| self.render(frame))?;

            // Select next event
            tokio::select! {
                _ = tick_interval.tick() => {
                    self.on_tick();
                }
                Some(event) = self.event_rx.recv() => {
                    self.handle_event(event);
                }
                Some(Ok(crossterm_event)) = event_stream.next() => {
                    self.handle_event(AppEvent::Input(crossterm_event));
                }
            }
        }

        Ok(())
    }

    // ── Event handling ──────────────────────────────────────────────────

    fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Input(crossterm_event) => {
                if let Some(action) = self.map_input_to_action(crossterm_event) {
                    self.handle_action(action);
                }
            }
            AppEvent::Action(action) => self.handle_action(action),
            AppEvent::Tick => self.on_tick(),
            AppEvent::LlmToken(_token) => {
                // Will be handled when chat view is implemented
            }
            AppEvent::LlmDone => {
                // Will be handled when chat view is implemented
            }
            AppEvent::AudioFinished => {
                // Will be handled when voice UI is implemented
            }
            AppEvent::Notification(notification) => {
                self.push_notification(notification.message, notification.level);
            }
            AppEvent::Quit => {
                self.running = false;
            }
        }
    }

    // ── Input mapping ───────────────────────────────────────────────────

    fn map_input_to_action(&self, event: Event) -> Option<Action> {
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        else {
            return None;
        };

        // When help is open, only Esc/? closes it
        if self.show_help {
            return match code {
                KeyCode::Esc | KeyCode::Char('?') => Some(Action::CloseHelp),
                _ => None,
            };
        }

        // When command palette is open, Esc closes it
        if self.show_command_palette {
            return match code {
                KeyCode::Esc => Some(Action::CloseCommandPalette),
                _ => None,
            };
        }

        // Global keybindings
        match (modifiers, code) {
            // Ctrl+P → command palette
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => Some(Action::OpenCommandPalette),
            // Ctrl+C → quit
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => Some(Action::Quit),
            // No modifiers
            (KeyModifiers::NONE | KeyModifiers::SHIFT, _) => match code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Char('?') => Some(Action::ShowHelp),
                KeyCode::Tab => Some(Action::TabNext),
                KeyCode::BackTab => Some(Action::TabPrev),
                // Number keys → jump to view
                KeyCode::Char('1') => Some(Action::FocusChat),
                KeyCode::Char('2') => Some(Action::FocusLibrary),
                KeyCode::Char('3') => Some(Action::FocusCampaign),
                KeyCode::Char('4') => Some(Action::FocusSettings),
                KeyCode::Char('5') => Some(Action::FocusGeneration),
                KeyCode::Char('6') => Some(Action::FocusPersonality),
                _ => None,
            },
            _ => None,
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::FocusChat => self.focus = Focus::Chat,
            Action::FocusLibrary => self.focus = Focus::Library,
            Action::FocusCampaign => self.focus = Focus::Campaign,
            Action::FocusSettings => self.focus = Focus::Settings,
            Action::FocusGeneration => self.focus = Focus::Generation,
            Action::FocusPersonality => self.focus = Focus::Personality,
            Action::TabNext => self.focus = self.focus.next(),
            Action::TabPrev => self.focus = self.focus.prev(),
            Action::ShowHelp => self.show_help = true,
            Action::CloseHelp => self.show_help = false,
            Action::OpenCommandPalette => self.show_command_palette = true,
            Action::CloseCommandPalette => self.show_command_palette = false,
            Action::SendMessage(_msg) => {
                // Will be handled when chat view is implemented
            }
        }
    }

    // ── Notifications ───────────────────────────────────────────────────

    /// Push a notification (dedup by message, max 3).
    pub fn push_notification(&mut self, message: String, level: NotificationLevel) {
        if self.notifications.iter().any(|n| n.message == message) {
            return;
        }

        self.notification_counter += 1;
        self.notifications.push(Notification {
            id: self.notification_counter,
            message,
            level,
            ttl_ticks: 100,
        });

        while self.notifications.len() > 3 {
            self.notifications.remove(0);
        }
    }

    /// Tick: decrement notification TTLs and dismiss expired.
    fn on_tick(&mut self) {
        for n in &mut self.notifications {
            n.ttl_ticks = n.ttl_ticks.saturating_sub(1);
        }
        self.notifications.retain(|n| n.ttl_ticks > 0);
    }

    // ── Rendering ───────────────────────────────────────────────────────

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let chunks = Layout::vertical([
            Constraint::Length(1), // Tab bar
            Constraint::Min(1),   // Content
            Constraint::Length(1), // Status bar
        ])
        .split(area);

        self.render_tab_bar(frame, chunks[0]);
        self.render_content(frame, chunks[1]);
        self.render_status_bar(frame, chunks[2]);
        self.render_notifications(frame, area);

        if self.show_help {
            self.render_help_modal(frame, area);
        }

        if self.show_command_palette {
            self.render_command_palette(frame, area);
        }
    }

    fn render_tab_bar(&self, frame: &mut Frame, area: Rect) {
        let titles: Vec<Line> = Focus::ALL
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let num = format!("{}", i + 1);
                Line::from(vec![
                    Span::styled(num, Style::default().fg(Color::DarkGray)),
                    Span::raw(":"),
                    Span::raw(f.label()),
                ])
            })
            .collect();

        let selected = Focus::ALL
            .iter()
            .position(|&f| f == self.focus)
            .unwrap_or(0);

        let tabs = Tabs::new(titles)
            .select(selected)
            .style(Style::default().fg(Color::Gray))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .divider("│");

        frame.render_widget(tabs, area);
    }

    fn render_content(&self, frame: &mut Frame, area: Rect) {
        let title = format!(" {} ", self.focus.label());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let placeholder = Paragraph::new(vec![
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    self.focus.label(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" view — coming soon"),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  Press "),
                Span::styled("?", Style::default().fg(Color::Cyan).bold()),
                Span::raw(" for help, "),
                Span::styled("Tab", Style::default().fg(Color::Cyan).bold()),
                Span::raw(" to switch views, "),
                Span::styled("q", Style::default().fg(Color::Red).bold()),
                Span::raw(" to quit"),
            ]),
        ]);
        frame.render_widget(placeholder, inner);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let status = Line::from(vec![
            Span::styled(
                " TTTTRPS ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .bold(),
            ),
            Span::raw(" "),
            Span::styled(self.focus.label(), Style::default().fg(Color::Cyan)),
            Span::raw(" │ "),
            Span::styled("LLM:", Style::default().fg(Color::DarkGray)),
            Span::raw(" ready"),
            Span::raw(" │ "),
            Span::styled("Tab", Style::default().fg(Color::DarkGray)),
            Span::raw(":nav "),
            Span::styled("?", Style::default().fg(Color::DarkGray)),
            Span::raw(":help "),
            Span::styled("Ctrl+P", Style::default().fg(Color::DarkGray)),
            Span::raw(":cmd "),
            Span::styled("q", Style::default().fg(Color::DarkGray)),
            Span::raw(":quit"),
        ]);

        frame.render_widget(Paragraph::new(status), area);
    }

    fn render_notifications(&self, frame: &mut Frame, area: Rect) {
        if self.notifications.is_empty() {
            return;
        }

        let max_width = 50.min(area.width.saturating_sub(2));
        let height = self.notifications.len() as u16;
        let x = area.width.saturating_sub(max_width + 1);
        let y = 1; // Below tab bar

        let notification_area = Rect::new(x, y, max_width, height);

        let lines: Vec<Line> = self
            .notifications
            .iter()
            .map(|n| {
                let (prefix, color) = match n.level {
                    NotificationLevel::Info => ("ℹ", Color::Blue),
                    NotificationLevel::Success => ("✓", Color::Green),
                    NotificationLevel::Warning => ("⚠", Color::Yellow),
                    NotificationLevel::Error => ("✗", Color::Red),
                };
                Line::from(vec![
                    Span::styled(format!(" {prefix} "), Style::default().fg(color).bold()),
                    Span::raw(&n.message),
                ])
            })
            .collect();

        frame.render_widget(Clear, notification_area);
        frame.render_widget(Paragraph::new(lines), notification_area);
    }

    fn render_help_modal(&self, frame: &mut Frame, area: Rect) {
        let modal = centered_rect(60, 70, area);

        let keybindings = vec![
            ("q", "Quit application"),
            ("?", "Toggle this help"),
            ("Tab / Shift+Tab", "Next / previous view"),
            ("1-6", "Jump to view by number"),
            ("Ctrl+P", "Open command palette"),
            ("Ctrl+C", "Force quit"),
            ("Esc", "Close modal / palette"),
        ];

        let mut lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                " Keybindings",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
        ];

        for (key, desc) in &keybindings {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<22}", key),
                    Style::default().fg(Color::Cyan).bold(),
                ),
                Span::raw(*desc),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  Press "),
            Span::styled("?", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" or "),
            Span::styled("Esc", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" to close"),
        ]));

        let block = Block::default()
            .title(" Help ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        frame.render_widget(Clear, modal);
        frame.render_widget(Paragraph::new(lines).block(block), modal);
    }

    fn render_command_palette(&self, frame: &mut Frame, area: Rect) {
        let modal = centered_rect(50, 40, area);

        let lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                " Command Palette",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "  (fuzzy search coming in Phase 2)",
                Style::default().fg(Color::DarkGray),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  Press "),
                Span::styled("Esc", Style::default().fg(Color::Cyan).bold()),
                Span::raw(" to close"),
            ]),
        ];

        let block = Block::default()
            .title(" Commands ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        frame.render_widget(Clear, modal);
        frame.render_widget(Paragraph::new(lines).block(block), modal);
    }
}

/// Calculate a centered rect using percentage of parent area.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::vertical([
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
    .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_next() {
        assert_eq!(Focus::Chat.next(), Focus::Library);
        assert_eq!(Focus::Personality.next(), Focus::Chat);
    }

    #[test]
    fn test_focus_prev() {
        assert_eq!(Focus::Chat.prev(), Focus::Personality);
        assert_eq!(Focus::Library.prev(), Focus::Chat);
    }

    #[test]
    fn test_focus_all_labels() {
        for f in Focus::ALL {
            assert!(!f.label().is_empty());
        }
    }

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(50, 50, area);
        // Should be roughly centered
        assert!(centered.x > 0);
        assert!(centered.y > 0);
        assert!(centered.width > 0);
        assert!(centered.height > 0);
        assert!(centered.x + centered.width <= area.width);
        assert!(centered.y + centered.height <= area.height);
    }
}
