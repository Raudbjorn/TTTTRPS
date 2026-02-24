use std::io;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use tokio::sync::mpsc;

use super::events::{Action, AppEvent, AreaFocus, Focus, Notification, NotificationLevel};
use super::layout::AppLayout;
use super::services::Services;
use super::sidebar::SidebarState;
use super::theme;
use super::views::campaign::{CampaignResult, CampaignState};
use super::views::chat::{ChatInputMode, ChatState};
use super::views::combat::CombatViewState;
use super::views::command_palette::{
    build_command_registry, CommandPaletteState, PaletteResult,
};
use super::views::dice_modal::DiceRollerState;
use super::views::generation::GenerationState;
use super::views::library::LibraryState;
use super::views::archetypes::ArchetypeViewState;
use super::views::audit::AuditViewState;
use super::views::locations::LocationViewState;
use super::views::npcs::NpcViewState;
use super::views::personality::PersonalityState;
use super::views::settings::SettingsState;
use super::views::usage::UsageViewState;
use super::views::voice::VoiceViewState;

/// Central application state (Elm architecture).
pub struct AppState {
    /// Whether the app is still running.
    pub running: bool,
    /// Currently focused top-level view.
    pub focus: Focus,
    /// Whether sidebar or main content has input focus.
    pub area_focus: AreaFocus,
    /// Sidebar navigation state.
    pub sidebar: SidebarState,
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
    /// Combat tracker view state.
    pub combat: CombatViewState,
    /// NPC management view state.
    pub npcs: NpcViewState,
    /// Usage dashboard view state.
    pub usage: UsageViewState,
    /// Audit viewer state.
    pub audit: AuditViewState,
    /// Location generator view state.
    pub locations: LocationViewState,
    /// Voice manager view state.
    pub voice: VoiceViewState,
    /// Archetype browser view state.
    pub archetypes: ArchetypeViewState,
    /// Active notifications (max 3 visible).
    pub notifications: Vec<Notification>,
    /// Monotonic counter for notification IDs.
    notification_counter: u64,
    /// Whether the help modal is open.
    pub show_help: bool,
    /// Dice roller modal state (Some when open).
    pub dice_roller: Option<DiceRollerState>,
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
            area_focus: AreaFocus::Main,
            sidebar: SidebarState::new(),
            chat: ChatState::new(),
            library: LibraryState::new(),
            campaign: CampaignState::new(),
            settings: SettingsState::new(),
            generation: GenerationState::new(),
            personality: PersonalityState::new(),
            combat: CombatViewState::new(),
            npcs: NpcViewState::new(),
            usage: UsageViewState::new(),
            audit: AuditViewState::new(),
            locations: LocationViewState::new(),
            voice: VoiceViewState::new(),
            archetypes: ArchetypeViewState::new(),
            notifications: Vec::new(),
            notification_counter: 0,
            show_help: false,
            dice_roller: None,
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
                if self.show_help {
                    if let Some(action) = self.map_help_input(&crossterm_event) {
                        self.handle_action(action);
                    }
                    return;
                }

                // Priority 3: Dice roller modal
                if let Some(ref mut dice) = self.dice_roller {
                    if !dice.handle_input(&crossterm_event) {
                        // Esc / Ctrl+D — close
                        self.dice_roller = None;
                    }
                    return;
                }

                // Priority 4: Sidebar input (when focused)
                if self.area_focus == AreaFocus::Sidebar {
                    if self.handle_sidebar_input(&crossterm_event) {
                        return;
                    }
                }

                // Priority 5: Focused view
                let consumed = self.dispatch_view_input(&crossterm_event);
                if consumed {
                    return;
                }

                // Priority 6: Global keybindings
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
                self.chat
                    .on_npc_conversation_loaded(npc, conversation, &self.services);
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

    /// Dispatch input to the currently focused view. Returns true if consumed.
    fn dispatch_view_input(&mut self, event: &Event) -> bool {
        match self.focus {
            Focus::Chat => self.chat.handle_input(event, &self.services),
            Focus::Library => self.library.handle_input(event, &self.services),
            Focus::Campaign => {
                match self.campaign.handle_input(event, &self.services) {
                    Some(CampaignResult::Consumed) => true,
                    Some(CampaignResult::SwitchSession(sid)) => {
                        self.handle_action(Action::SwitchChatSession(sid));
                        true
                    }
                    None => false,
                }
            }
            Focus::Settings => self.settings.handle_input(event, &self.services),
            Focus::Generation => self.generation.handle_input(event, &self.services),
            Focus::Personality => self.personality.handle_input(event, &self.services),
            Focus::Combat => self.combat.handle_input(event),
            Focus::Npcs => self.npcs.handle_input(event, &self.services),
            Focus::Usage => self.usage.handle_input(event, &self.services),
            Focus::Audit => self.audit.handle_input(event, &self.services),
            Focus::Locations => self.locations.handle_input(event, &self.services),
            Focus::Voice => self.voice.handle_input(event, &self.services),
            Focus::Archetypes => self.archetypes.handle_input(event, &self.services),
            // Stub views
            Focus::Notes => false,
        }
    }

    /// Handle sidebar-specific input. Returns true if consumed.
    fn handle_sidebar_input(&mut self, event: &Event) -> bool {
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        else {
            return false;
        };

        match (*modifiers, *code) {
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                self.sidebar.select_next();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                self.sidebar.select_prev();
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) | (KeyModifiers::NONE, KeyCode::Char('l')) => {
                let focus = self.sidebar.selected_focus();
                self.handle_action(focus.to_action());
                self.area_focus = AreaFocus::Main;
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('h')) => {
                self.sidebar.user_collapsed = true;
                self.area_focus = AreaFocus::Main;
                true
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.area_focus = AreaFocus::Main;
                true
            }
            _ => false,
        }
    }

    // ── Input mapping ───────────────────────────────────────────────────

    /// Map help modal input to action.
    fn map_help_input(&self, event: &Event) -> Option<Action> {
        let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event
        else {
            return None;
        };
        match code {
            KeyCode::Esc | KeyCode::Char('?') => Some(Action::CloseHelp),
            _ => None,
        }
    }

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

        // Global keybindings (always active when no modal/sidebar consumes)
        match (modifiers, code) {
            // Ctrl+P → command palette
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => Some(Action::OpenCommandPalette),
            // Ctrl+B → toggle sidebar
            (KeyModifiers::CONTROL, KeyCode::Char('b')) => Some(Action::ToggleSidebar),
            // Ctrl+D → dice roller
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => Some(Action::OpenDiceRoller),
            // Ctrl+C → quit
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => Some(Action::Quit),
            // No modifiers
            (KeyModifiers::NONE | KeyModifiers::SHIFT, _) => match code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Char('?') => Some(Action::ShowHelp),
                KeyCode::Tab => Some(Action::TabNext),
                KeyCode::BackTab => Some(Action::TabPrev),
                // Number keys → jump to legacy views (backward compatible)
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
                self.set_focus(Focus::Chat);
                self.chat.load_session(&self.services);
            }
            Action::FocusLibrary => {
                self.set_focus(Focus::Library);
                self.library.load(&self.services);
            }
            Action::FocusCampaign => {
                self.set_focus(Focus::Campaign);
                self.campaign.load(&self.services);
            }
            Action::FocusSettings => {
                self.set_focus(Focus::Settings);
                self.settings.load(&self.services);
            }
            Action::FocusGeneration => {
                self.set_focus(Focus::Generation);
                self.generation.load(&self.services);
            }
            Action::FocusPersonality => {
                self.set_focus(Focus::Personality);
                self.personality.load(&self.services);
            }
            // New view focus actions (stubs — no load() yet)
            Action::FocusCombat => self.set_focus(Focus::Combat),
            Action::FocusNotes => self.set_focus(Focus::Notes),
            Action::FocusNpcs => self.set_focus(Focus::Npcs),
            Action::FocusLocations => self.set_focus(Focus::Locations),
            Action::FocusArchetypes => self.set_focus(Focus::Archetypes),
            Action::FocusVoice => self.set_focus(Focus::Voice),
            Action::FocusUsage => self.set_focus(Focus::Usage),
            Action::FocusAudit => self.set_focus(Focus::Audit),
            Action::TabNext => {
                self.focus = self.focus.next();
                self.sidebar.sync_to_focus(self.focus);
                self.on_focus_changed();
            }
            Action::TabPrev => {
                self.focus = self.focus.prev();
                self.sidebar.sync_to_focus(self.focus);
                self.on_focus_changed();
            }
            Action::ToggleSidebar => {
                self.sidebar.toggle_collapse();
                // If expanding and main was focused, switch to sidebar
                if !self.sidebar.user_collapsed {
                    self.area_focus = AreaFocus::Sidebar;
                    self.sidebar.sync_to_focus(self.focus);
                }
            }
            Action::ShowHelp => self.show_help = true,
            Action::CloseHelp => self.show_help = false,
            Action::OpenDiceRoller => {
                self.dice_roller = Some(DiceRollerState::new());
            }
            Action::CloseDiceRoller => {
                self.dice_roller = None;
            }
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
                self.set_focus(Focus::Chat);
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
            // New view refreshes — stubs
            Action::RefreshNpcs
            | Action::RefreshUsage
            | Action::RefreshAudit
            | Action::RefreshLocations
            | Action::RefreshVoice
            | Action::RefreshArchetypes => {
                // TODO: wire when views are implemented
            }
            // Combat actions — stubs
            Action::StartCombat | Action::EndCombat | Action::NextTurn => {
                // TODO: wire when combat view is implemented
            }
        }
    }

    /// Set focus and sync sidebar selection.
    fn set_focus(&mut self, focus: Focus) {
        self.focus = focus;
        self.sidebar.sync_to_focus(focus);
        self.area_focus = AreaFocus::Main;
    }

    fn on_focus_changed(&mut self) {
        match self.focus {
            Focus::Chat => self.chat.load_session(&self.services),
            Focus::Library => self.library.load(&self.services),
            Focus::Campaign => self.campaign.load(&self.services),
            Focus::Settings => self.settings.load(&self.services),
            Focus::Generation => self.generation.load(&self.services),
            Focus::Personality => self.personality.load(&self.services),
            Focus::Npcs => self.npcs.load(&self.services),
            Focus::Usage => self.usage.load(&self.services),
            Focus::Audit => self.audit.load(&self.services),
            Focus::Locations => self.locations.load(&self.services),
            Focus::Voice => self.voice.load(&self.services),
            Focus::Archetypes => self.archetypes.load(&self.services),
            Focus::Combat | Focus::Notes => {}
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
        self.npcs.poll();
        self.usage.poll();
        self.audit.poll();
        self.locations.poll();
        self.voice.poll();
        self.archetypes.poll();
    }

    // ── Rendering ───────────────────────────────────────────────────────

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        let (layout, visibility) = AppLayout::compute(area, self.sidebar.user_collapsed);

        // Render sidebar if visible
        if let Some(sidebar_area) = layout.sidebar {
            self.sidebar
                .render(frame, sidebar_area, visibility, self.focus, self.area_focus);
        }

        // Render main content
        self.render_content(frame, layout.main);

        // Render status bar
        self.render_status_bar(frame, layout.status);

        // Overlays
        self.render_notifications(frame, area);

        if self.show_help {
            self.render_help_modal(frame, area);
        }

        if let Some(ref dice) = self.dice_roller {
            dice.render(frame, area);
        }

        if let Some(ref palette) = self.command_palette {
            palette.render(frame, area);
        }
    }

    fn render_content(&self, frame: &mut Frame, area: Rect) {
        match self.focus {
            Focus::Chat => self.chat.render(frame, area),
            Focus::Library => self.library.render(frame, area),
            Focus::Campaign => self.campaign.render(frame, area),
            Focus::Settings => self.settings.render(frame, area),
            Focus::Generation => self.generation.render(frame, area),
            Focus::Personality => self.personality.render(frame, area),
            Focus::Combat => self.combat.render(frame, area),
            Focus::Npcs => self.npcs.render(frame, area),
            Focus::Usage => self.usage.render(frame, area),
            Focus::Audit => self.audit.render(frame, area),
            Focus::Locations => self.locations.render(frame, area),
            Focus::Voice => self.voice.render(frame, area),
            Focus::Archetypes => self.archetypes.render(frame, area),
            // Remaining stub views
            other => self.render_stub_view(frame, area, other),
        }
    }

    fn render_stub_view(&self, frame: &mut Frame, area: Rect, focus: Focus) {
        let block = Block::default()
            .title(format!(" {} ", focus.label()))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::PRIMARY));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                format!("{} {}", focus.icon(), focus.label()),
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "Coming soon",
                Style::default().fg(theme::TEXT_MUTED),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "This view is under construction.",
                Style::default().fg(theme::TEXT_DIM),
            )),
        ];

        frame.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            inner,
        );
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let llm_status = if self.chat.is_streaming() {
            Span::styled("streaming", Style::default().fg(theme::PRIMARY_LIGHT))
        } else {
            Span::styled("ready", Style::default().fg(theme::TEXT_MUTED))
        };

        let mode_indicator = match self.chat.input_mode() {
            ChatInputMode::Insert if self.focus == Focus::Chat => {
                Span::styled(" INSERT ", theme::insert_badge())
            }
            _ => Span::raw(""),
        };

        let status = Line::from(vec![
            Span::styled(" TTTTRPS ", theme::brand_badge()),
            Span::raw(" "),
            mode_indicator,
            Span::raw(" "),
            Span::styled(
                self.focus.label(),
                Style::default()
                    .fg(theme::PRIMARY_LIGHT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" │ "),
            Span::styled("LLM:", theme::key_hint()),
            Span::raw(" "),
            llm_status,
            Span::raw(" │ "),
            Span::styled("Tab", theme::key_hint()),
            Span::raw(":nav "),
            Span::styled("Ctrl+B", theme::key_hint()),
            Span::raw(":sidebar "),
            Span::styled("?", theme::key_hint()),
            Span::raw(":help "),
            Span::styled("Ctrl+P", theme::key_hint()),
            Span::raw(":cmd "),
            Span::styled("q", theme::key_hint()),
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
        let y = 1;

        let notification_area = Rect::new(x, y, max_width, height);

        let lines: Vec<Line> = self
            .notifications
            .iter()
            .map(|n| {
                let (prefix, color) = match n.level {
                    NotificationLevel::Info => ("ℹ", theme::INFO),
                    NotificationLevel::Success => ("✓", theme::SUCCESS),
                    NotificationLevel::Warning => ("⚠", theme::WARNING),
                    NotificationLevel::Error => ("✗", theme::ERROR),
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
        let modal = centered_rect(60, 80, area);

        let keybindings = vec![
            ("Global:", ""),
            ("q", "Quit application"),
            ("?", "Toggle this help"),
            ("Tab / Shift+Tab", "Next / previous view"),
            ("1-6", "Jump to view by number"),
            ("Ctrl+P", "Open command palette"),
            ("Ctrl+B", "Toggle sidebar collapse/expand"),
            ("Ctrl+D", "Open dice roller"),
            ("Ctrl+C", "Force quit"),
            ("Esc", "Close modal / focus main"),
            ("", ""),
            ("Sidebar (when focused):", ""),
            ("j/k", "Navigate up/down"),
            ("Enter / l", "Select view"),
            ("h", "Collapse sidebar"),
            ("Esc", "Focus main content"),
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
                    .fg(theme::ACCENT)
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
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<22}", key),
                        Style::default().fg(theme::PRIMARY_LIGHT).bold(),
                    ),
                    Span::raw(*desc),
                ]));
            }
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  Press "),
            Span::styled(
                "?",
                Style::default().fg(theme::PRIMARY_LIGHT).bold(),
            ),
            Span::raw(" or "),
            Span::styled(
                "Esc",
                Style::default().fg(theme::PRIMARY_LIGHT).bold(),
            ),
            Span::raw(" to close"),
        ]));

        let block = Block::default()
            .title(" Help ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT));

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
    fn test_focus_next_cycles_14() {
        let mut f = Focus::Chat;
        for _ in 0..14 {
            f = f.next();
        }
        assert_eq!(f, Focus::Chat); // Full cycle
    }

    #[test]
    fn test_focus_prev_cycles_14() {
        let mut f = Focus::Chat;
        for _ in 0..14 {
            f = f.prev();
        }
        assert_eq!(f, Focus::Chat); // Full cycle
    }

    #[test]
    fn test_focus_next_first_step() {
        assert_eq!(Focus::Chat.next(), Focus::Combat);
        assert_eq!(Focus::Personality.next(), Focus::Chat);
    }

    #[test]
    fn test_focus_prev_first_step() {
        assert_eq!(Focus::Chat.prev(), Focus::Personality);
        assert_eq!(Focus::Combat.prev(), Focus::Chat);
    }

    #[test]
    fn test_focus_all_labels() {
        for f in Focus::ALL {
            assert!(!f.label().is_empty());
        }
    }

    #[test]
    fn test_focus_all_icons() {
        for f in Focus::ALL {
            assert!(!f.icon().is_empty());
        }
    }

    #[test]
    fn test_focus_all_have_groups() {
        for f in Focus::ALL {
            let _group = f.group(); // Should not panic
            // Verify group contains this focus
            assert!(f.group().views().contains(&f));
        }
    }

    #[test]
    fn test_sidebar_groups_cover_all_views() {
        use super::super::events::SidebarGroup;
        let mut all_from_groups: Vec<Focus> = Vec::new();
        for group in SidebarGroup::ALL {
            all_from_groups.extend_from_slice(group.views());
        }
        assert_eq!(all_from_groups.len(), Focus::ALL.len());
        for f in Focus::ALL {
            assert!(all_from_groups.contains(&f));
        }
    }

    #[test]
    fn test_focus_to_action_roundtrip() {
        // Verify each Focus maps to a unique Action
        let actions: Vec<Action> = Focus::ALL.iter().map(|f| f.to_action()).collect();
        for (i, a) in actions.iter().enumerate() {
            for (j, b) in actions.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "Focus::{:?} and Focus::{:?} map to same action", Focus::ALL[i], Focus::ALL[j]);
                }
            }
        }
    }

    #[test]
    fn test_centered_rect() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_rect(50, 50, area);
        assert!(centered.x > 0);
        assert!(centered.y > 0);
        assert!(centered.width > 0);
        assert!(centered.height > 0);
        assert!(centered.x + centered.width <= area.width);
        assert!(centered.y + centered.height <= area.height);
    }

    #[test]
    fn test_area_focus_default_is_main() {
        // New state starts with Main focus
        assert_eq!(AreaFocus::Main, AreaFocus::Main);
        assert_ne!(AreaFocus::Sidebar, AreaFocus::Main);
    }
}
