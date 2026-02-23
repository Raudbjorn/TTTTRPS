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
use super::views::chat::{ChatInputMode, ChatState};
use super::views::command_palette::{
    build_command_registry, CommandPaletteState, PaletteResult,
};
use super::views::campaign::{CampaignResult, CampaignState};
use super::views::library::LibraryState;
use super::views::personality::PersonalityState;
use super::views::generation::GenerationState;
use super::views::settings::SettingsState;

/// Central application state (Elm architecture).
pub struct AppState {
    /// Whether the app is still running.
    pub running: bool,
    /// Currently focused top-level view.
    pub focus: Focus,
    /// Chat view state.
    pub chat: ChatState,
    /// Library view state.
    pub library: LibraryState,
    /// Campaign/session management view state.
    pub campaign: CampaignState,
    /// Settings view state.
    pub settings: SettingsState,
    /// Character generation view state.
    pub generation: GenerationState,
    /// Personality view state.
    pub personality: PersonalityState,
    /// Active notifications (max 3 visible).
    pub notifications: Vec<Notification>,
    /// Monotonic counter for notification IDs.
    notification_counter: u64,
    /// Whether the help modal is open.
    pub show_help: bool,
    /// Command palette state (Some when open).
    pub command_palette: Option<CommandPaletteState>,
    /// Receiver for backend events.
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    /// Sender for pushing events from within the app.
    #[allow(dead_code)]
    event_tx: mpsc::UnboundedSender<AppEvent>,
    /// Backend services handle.
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
            chat: ChatState::new(),
            library: LibraryState::new(),
            campaign: CampaignState::new(),
            settings: SettingsState::new(),
            generation: GenerationState::new(),
            personality: PersonalityState::new(),
            notifications: Vec::new(),
            notification_counter: 0,
            show_help: false,
            command_palette: None,
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

        // Load initial chat session
        if self.focus == Focus::Chat {
            self.chat.load_session(&self.services);
        }

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
                // Priority 1: Command palette consumes all input when open
                if let Some(ref mut palette) = self.command_palette {
                    match palette.handle_input(&crossterm_event) {
                        PaletteResult::Consumed => return,
                        PaletteResult::Execute(action) => {
                            self.command_palette = None;
                            self.handle_action(action);
                            return;
                        }
                        PaletteResult::Close => {
                            self.command_palette = None;
                            return;
                        }
                    }
                }
                // Priority 2: Help modal
                // Priority 3: Focused view
                if !self.show_help {
                    let consumed = match self.focus {
                        Focus::Chat => {
                            self.chat.handle_input(&crossterm_event, &self.services)
                        }
                        Focus::Library => {
                            self.library.handle_input(&crossterm_event, &self.services)
                        }
                        Focus::Campaign => {
                            match self.campaign.handle_input(&crossterm_event, &self.services)
                            {
                                Some(CampaignResult::Consumed) => true,
                                Some(CampaignResult::SwitchSession(sid)) => {
                                    self.handle_action(Action::SwitchChatSession(sid));
                                    true
                                }
                                None => false,
                            }
                        }
                        Focus::Settings => {
                            self.settings.handle_input(&crossterm_event, &self.services)
                        }
                        Focus::Generation => {
                            self.generation.handle_input(&crossterm_event, &self.services)
                        }
                        Focus::Personality => {
                            self.personality.handle_input(&crossterm_event, &self.services)
                        }
                    };
                    if consumed {
                        return;
                    }
                }
                // Priority 4: Global keybindings
                if let Some(action) = self.map_input_to_action(crossterm_event) {
                    self.handle_action(action);
                }
            }
            AppEvent::Action(action) => self.handle_action(action),
            AppEvent::Tick => self.on_tick(),
            AppEvent::LlmToken(token) => {
                self.chat.append_token(&token);
            }
            AppEvent::LlmDone => {
                self.chat.finalize_and_persist(&self.services);
            }
            AppEvent::LlmError(error) => {
                self.chat.handle_stream_error(&error, &self.services);
            }
            AppEvent::ChatSessionLoaded {
                session_id,
                messages,
            } => {
                self.chat.on_session_loaded(session_id, messages);
            }
            AppEvent::NpcConversationLoaded { npc, conversation } => {
                self.chat.on_npc_conversation_loaded(npc, conversation, &self.services);
            }
            AppEvent::AudioPlayback(ref event) => {
                self.services.audio.update_state(event);
                self.chat.on_audio_event(event);
            }
            AppEvent::AudioFinished => {
                // Legacy variant — no-op, AudioPlayback(Finished) handles it.
            }
            ref ingest @ AppEvent::IngestionProgress { .. } => {
                self.library
                    .handle_ingestion_event(ingest, &self.services);
            }
            ref oauth_event @ (AppEvent::OAuthFlowResult { .. }
            | AppEvent::DeviceFlowUpdate { .. }) => {
                self.settings
                    .handle_oauth_event(oauth_event, &self.services);
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

        // Command palette is handled before map_input_to_action is called,
        // so we don't need to check for it here.

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
            Action::FocusChat => {
                self.focus = Focus::Chat;
                // Trigger session loading on first focus
                self.chat.load_session(&self.services);
            }
            Action::FocusLibrary => {
                self.focus = Focus::Library;
                self.library.load(&self.services);
            }
            Action::FocusCampaign => {
                self.focus = Focus::Campaign;
                self.campaign.load(&self.services);
            }
            Action::FocusSettings => {
                self.focus = Focus::Settings;
                self.settings.load(&self.services);
            }
            Action::FocusGeneration => {
                self.focus = Focus::Generation;
                self.generation.load(&self.services);
            }
            Action::FocusPersonality => {
                self.focus = Focus::Personality;
                self.personality.load(&self.services);
            }
            Action::TabNext => {
                self.focus = self.focus.next();
                self.on_focus_changed();
            }
            Action::TabPrev => {
                self.focus = self.focus.prev();
                self.on_focus_changed();
            }
            Action::ShowHelp => self.show_help = true,
            Action::CloseHelp => self.show_help = false,
            Action::NewChatSession => {
                self.chat.cmd_new_session(&self.services);
            }
            Action::ClearChat => {
                self.chat.cmd_clear(&self.services);
            }
            Action::RefreshSettings => {
                self.settings.load(&self.services);
            }
            Action::AddProvider => {
                self.settings.open_add_modal();
            }
            Action::EditProvider(ref id) => {
                self.settings.open_edit_modal(id, &self.services);
            }
            Action::DeleteProvider(ref id) => {
                self.settings.open_delete_modal(id);
            }
            Action::RefreshLibrary => {
                self.library.load(&self.services);
            }
            Action::IngestDocument => {
                self.library.open_ingest_modal();
                self.focus = Focus::Library;
            }
            Action::RefreshCampaign => {
                self.campaign.load(&self.services);
            }
            Action::SwitchChatSession(session_id) => {
                self.chat.switch_to_session(session_id, &self.services);
                self.focus = Focus::Chat;
            }
            Action::OpenCommandPalette => {
                self.command_palette =
                    Some(CommandPaletteState::new(build_command_registry()));
            }
            Action::CloseCommandPalette => {
                self.command_palette = None;
            }
            Action::SendMessage(_msg) => {
                // Handled directly by ChatState via input handling
            }
        }
    }

    fn on_focus_changed(&mut self) {
        match self.focus {
            Focus::Chat => self.chat.load_session(&self.services),
            Focus::Library => self.library.load(&self.services),
            Focus::Campaign => self.campaign.load(&self.services),
            Focus::Settings => self.settings.load(&self.services),
            Focus::Generation => self.generation.load(&self.services),
            Focus::Personality => self.personality.load(&self.services),
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

    /// Tick: decrement notification TTLs, dismiss expired, poll async data.
    fn on_tick(&mut self) {
        for n in &mut self.notifications {
            n.ttl_ticks = n.ttl_ticks.saturating_sub(1);
        }
        self.notifications.retain(|n| n.ttl_ticks > 0);

        // Poll async view data
        self.library.poll();
        self.campaign.poll();
        self.settings.poll();
        self.generation.poll();
        self.personality.poll();
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

        if let Some(ref palette) = self.command_palette {
            palette.render(frame, area);
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
        match self.focus {
            Focus::Chat => self.chat.render(frame, area),
            Focus::Library => self.library.render(frame, area),
            Focus::Campaign => self.campaign.render(frame, area),
            Focus::Settings => self.settings.render(frame, area),
            Focus::Generation => self.generation.render(frame, area),
            Focus::Personality => self.personality.render(frame, area),
        }
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let llm_status = if self.chat.is_streaming() {
            Span::styled("streaming", Style::default().fg(Color::Cyan))
        } else {
            Span::raw("ready")
        };

        let mode_indicator = match self.chat.input_mode() {
            ChatInputMode::Insert if self.focus == Focus::Chat => {
                Span::styled(" INSERT ", Style::default().fg(Color::Black).bg(Color::Yellow))
            }
            _ => Span::raw(""),
        };

        let status = Line::from(vec![
            Span::styled(
                " TTTTRPS ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .bold(),
            ),
            Span::raw(" "),
            mode_indicator,
            Span::raw(" "),
            Span::styled(self.focus.label(), Style::default().fg(Color::Cyan)),
            Span::raw(" │ "),
            Span::styled("LLM:", Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            llm_status,
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
            ("", ""),
            ("Chat View:", ""),
            ("i / Enter / a", "Enter insert mode"),
            ("Esc", "Exit insert mode"),
            ("j/k", "Scroll messages"),
            ("G / g", "Jump to bottom / top"),
            ("/clear", "Clear messages"),
            ("/new", "New session"),
            ("/speak <text>", "Speak text (TTS)"),
            ("/pause /resume /stop", "Playback controls"),
            ("/volume <0-100>", "Set volume"),
            ("/voices", "List voice providers"),
            ("", ""),
            ("Library View:", ""),
            ("a", "Ingest document"),
            ("r", "Refresh data"),
            ("j/k", "Scroll list"),
            ("", ""),
            ("Settings View:", ""),
            ("a", "Add LLM provider"),
            ("e", "Edit selected provider"),
            ("d", "Delete selected provider"),
            ("r", "Refresh data"),
            ("j/k", "Navigate provider list"),
            ("", ""),
            ("Generation View:", ""),
            ("j/k", "Navigate systems / scroll"),
            ("Enter", "Select system / generate"),
            ("Esc", "Go back one phase"),
            ("b", "Generate backstory (LLM)"),
            ("s", "Save character"),
            ("r", "Regenerate character"),
            ("n", "New character"),
            ("l", "Toggle saved characters"),
            ("", ""),
            ("Personality View:", ""),
            ("a", "Add personality (preset/manual)"),
            ("e", "Edit selected personality"),
            ("d", "Delete selected personality"),
            ("p", "Preview system prompt"),
            ("Enter", "Toggle detail panel"),
            ("r", "Refresh data"),
            ("j/k", "Navigate list"),
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
            if key.is_empty() {
                lines.push(Line::raw(""));
            } else if desc.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("  {key}"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<22}", key),
                        Style::default().fg(Color::Cyan).bold(),
                    ),
                    Span::raw(*desc),
                ]));
            }
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

}

/// Calculate a centered rect using percentage of parent area.
pub(super) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
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
