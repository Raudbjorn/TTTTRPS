//! Chat view — primary interface for LLM conversation.
//!
//! Handles message display, input mode switching, LLM streaming,
//! session persistence, and slash commands.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::core::llm::router::{ChatMessage, ChatRequest};
use crate::database::{ChatMessageRecord, MessageRole};
use crate::tui::events::{AppEvent, Notification, NotificationLevel};
use crate::tui::services::Services;
use crate::tui::widgets::input_buffer::InputBuffer;
use crate::tui::widgets::markdown::markdown_to_lines;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatInputMode {
    Normal,
    Insert,
}

struct DisplayMessage {
    #[allow(dead_code)]
    id: String,
    role: MessageRole,
    raw_content: String,
    rendered_lines: Vec<Line<'static>>,
    #[allow(dead_code)]
    created_at: String,
    is_streaming: bool,
}

impl DisplayMessage {
    fn from_record(record: &ChatMessageRecord) -> Self {
        let role = record.role_enum().unwrap_or(MessageRole::User);
        let rendered = markdown_to_lines(&record.content);
        Self {
            id: record.id.clone(),
            role,
            raw_content: record.content.clone(),
            rendered_lines: rendered,
            created_at: record.created_at.clone(),
            is_streaming: record.is_streaming != 0,
        }
    }

    fn from_user_input(session_id: &str, content: &str) -> (Self, ChatMessageRecord) {
        let record = ChatMessageRecord::with_role(
            session_id.to_string(),
            MessageRole::User,
            content.to_string(),
        );
        let display = Self {
            id: record.id.clone(),
            role: MessageRole::User,
            raw_content: content.to_string(),
            rendered_lines: markdown_to_lines(content),
            created_at: record.created_at.clone(),
            is_streaming: false,
        };
        (display, record)
    }

    fn new_streaming(session_id: &str) -> (Self, ChatMessageRecord) {
        let record = ChatMessageRecord::with_role(
            session_id.to_string(),
            MessageRole::Assistant,
            String::new(),
        )
        .streaming();
        let display = Self {
            id: record.id.clone(),
            role: MessageRole::Assistant,
            raw_content: String::new(),
            rendered_lines: vec![Line::styled(
                "▍",
                Style::default().fg(Color::DarkGray),
            )],
            created_at: record.created_at.clone(),
            is_streaming: true,
        };
        (display, record)
    }

    fn role_header(&self) -> Line<'static> {
        let (label, color) = match self.role {
            MessageRole::User => ("You", Color::Green),
            MessageRole::Assistant => ("Assistant", Color::Cyan),
            MessageRole::System => ("System", Color::Yellow),
            MessageRole::Error => ("Error", Color::Red),
        };
        Line::from(Span::styled(
            format!("── {label} ──"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
    }

    fn all_lines(&self) -> Vec<Line<'static>> {
        let mut out = vec![self.role_header()];
        out.extend(self.rendered_lines.clone());
        out.push(Line::raw(""));
        out
    }
}

// ============================================================================
// Chat-specific input rendering
// ============================================================================

fn render_chat_input(
    input: &InputBuffer,
    mode: ChatInputMode,
    is_streaming: bool,
) -> Paragraph<'static> {
    let (border_color, title) = match mode {
        ChatInputMode::Insert => (Color::Yellow, " Message (Esc to exit) "),
        ChatInputMode::Normal => (Color::DarkGray, " Message "),
    };

    let text = input.text();
    let cursor = input.cursor_position();

    let display = if text.is_empty() {
        Line::styled(
            "Type a message... (i to enter insert mode)",
            Style::default().fg(Color::DarkGray),
        )
    } else {
        let before = &text[..cursor];
        let cursor_char = text[cursor..]
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_else(|| " ".to_string());
        let after_cursor = if cursor < text.len() {
            let char_len = cursor_char.len();
            &text[cursor + char_len..]
        } else {
            ""
        };

        if mode == ChatInputMode::Insert {
            Line::from(vec![
                Span::raw(before.to_string()),
                Span::styled(
                    cursor_char,
                    Style::default().bg(Color::White).fg(Color::Black),
                ),
                Span::raw(after_cursor.to_string()),
            ])
        } else {
            Line::raw(text.to_string())
        }
    };

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(title);

    if is_streaming {
        block = block.title_bottom(Line::styled(
            " streaming... ",
            Style::default().fg(Color::Cyan),
        ));
    }

    Paragraph::new(display).block(block)
}

// ============================================================================
// ChatState
// ============================================================================

pub struct ChatState {
    input_mode: ChatInputMode,
    input: InputBuffer,
    messages: Vec<DisplayMessage>,
    session_id: Option<String>,
    session_loading: bool,
    scroll_offset: usize,
    auto_scroll: bool,
    streaming_buffer: String,
    streaming_record_id: Option<String>,
    active_stream_id: Option<String>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            input_mode: ChatInputMode::Normal,
            input: InputBuffer::new(),
            messages: Vec::new(),
            session_id: None,
            session_loading: false,
            scroll_offset: 0,
            auto_scroll: true,
            streaming_buffer: String::new(),
            streaming_record_id: None,
            active_stream_id: None,
        }
    }

    pub fn input_mode(&self) -> ChatInputMode {
        self.input_mode
    }

    pub fn is_streaming(&self) -> bool {
        self.active_stream_id.is_some()
    }

    // ── Session loading ──────────────────────────────────────────────

    pub fn load_session(&mut self, services: &Services) {
        if self.session_loading || self.session_id.is_some() {
            return;
        }
        self.session_loading = true;

        let db = services.database.clone();
        let tx = services.event_tx.clone();

        tokio::spawn(async move {
            use crate::database::ChatOps;

            match db.get_or_create_active_chat_session().await {
                Ok(session) => {
                    let messages = db
                        .get_chat_messages(&session.id, 200)
                        .await
                        .unwrap_or_default();
                    let _ = tx.send(AppEvent::ChatSessionLoaded {
                        session_id: session.id,
                        messages,
                    });
                }
                Err(e) => {
                    log::error!("Failed to load chat session: {e}");
                    let _ = tx.send(AppEvent::Notification(Notification {
                        id: 0,
                        message: format!("Session load failed: {e}"),
                        level: NotificationLevel::Error,
                        ttl_ticks: 100,
                    }));
                }
            }
        });
    }

    pub fn on_session_loaded(&mut self, session_id: String, records: Vec<ChatMessageRecord>) {
        self.session_id = Some(session_id);
        self.session_loading = false;
        self.messages = records.iter().map(DisplayMessage::from_record).collect();
        self.scroll_to_bottom();
    }

    // ── Input handling (two-phase) ───────────────────────────────────

    /// Returns true if the event was consumed (don't pass to global handler).
    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        else {
            return false;
        };

        match self.input_mode {
            ChatInputMode::Insert => self.handle_insert_input(*code, *modifiers, services),
            ChatInputMode::Normal => self.handle_normal_input(*code, *modifiers),
        }
    }

    fn handle_insert_input(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        services: &Services,
    ) -> bool {
        // These always fall through to global
        match (modifiers, code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => return false,
            (_, KeyCode::Tab) | (_, KeyCode::BackTab) => return false,
            _ => {}
        }

        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.input_mode = ChatInputMode::Normal;
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if !self.input.is_empty() {
                    let text = self.input.take();
                    self.send_or_command(&text, services);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.input.backspace();
                true
            }
            (KeyModifiers::NONE, KeyCode::Delete) => {
                self.input.delete();
                true
            }
            (KeyModifiers::NONE, KeyCode::Left) => {
                self.input.move_left();
                true
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                self.input.move_right();
                true
            }
            (KeyModifiers::NONE, KeyCode::Home) => {
                self.input.move_home();
                true
            }
            (KeyModifiers::NONE, KeyCode::End) => {
                self.input.move_end();
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.input.clear();
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                self.input.move_home();
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
                self.input.move_end();
                true
            }
            (_, KeyCode::Char(c)) => {
                self.input.insert_char(c);
                true
            }
            _ => true, // Consume but ignore other keys in insert mode
        }
    }

    fn handle_normal_input(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        if modifiers != KeyModifiers::NONE && modifiers != KeyModifiers::SHIFT {
            return false;
        }

        match code {
            // Enter insert mode
            KeyCode::Char('i') | KeyCode::Char('a') | KeyCode::Enter => {
                self.input_mode = ChatInputMode::Insert;
                true
            }
            // Scroll
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_down(1);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_up(1);
                true
            }
            KeyCode::Char('G') | KeyCode::End => {
                self.scroll_to_bottom();
                true
            }
            KeyCode::Char('g') | KeyCode::Home => {
                self.scroll_to_top();
                true
            }
            KeyCode::PageDown => {
                self.scroll_down(10);
                true
            }
            KeyCode::PageUp => {
                self.scroll_up(10);
                true
            }
            _ => false, // Fall through to global handler
        }
    }

    // ── Slash commands ───────────────────────────────────────────────

    fn send_or_command(&mut self, text: &str, services: &Services) {
        if let Some(cmd) = text.strip_prefix('/') {
            let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
            match parts[0] {
                "clear" => self.cmd_clear(services),
                "new" => self.cmd_new_session(services),
                "help" => self.cmd_help(services),
                unknown => {
                    let _ = services.event_tx.send(AppEvent::Notification(Notification {
                        id: 0,
                        message: format!("Unknown command: /{unknown}"),
                        level: NotificationLevel::Warning,
                        ttl_ticks: 80,
                    }));
                }
            }
        } else {
            self.send_message(text, services);
        }
    }

    pub fn cmd_clear(&mut self, services: &Services) {
        if let Some(ref sid) = self.session_id {
            let db = services.database.clone();
            let sid = sid.clone();
            tokio::spawn(async move {
                use crate::database::ChatOps;
                if let Err(e) = db.clear_chat_messages(&sid).await {
                    log::error!("Failed to clear messages: {e}");
                }
            });
        }
        self.messages.clear();
        self.scroll_offset = 0;
    }

    /// Switch to a specific chat session by ID.
    /// Clears current state and loads the target session's messages.
    pub fn switch_to_session(&mut self, session_id: String, services: &Services) {
        self.messages.clear();
        self.session_id = None;
        self.scroll_offset = 0;
        self.streaming_buffer.clear();
        self.streaming_record_id = None;
        self.active_stream_id = None;
        self.session_loading = true;

        let db = services.database.clone();
        let tx = services.event_tx.clone();
        let sid = session_id.clone();

        tokio::spawn(async move {
            use crate::database::ChatOps;

            match db.get_chat_messages(&sid, 200).await {
                Ok(messages) => {
                    let _ = tx.send(crate::tui::events::AppEvent::ChatSessionLoaded {
                        session_id: sid,
                        messages,
                    });
                }
                Err(e) => {
                    log::error!("Failed to load session {sid}: {e}");
                    let _ = tx.send(crate::tui::events::AppEvent::Notification(
                        crate::tui::events::Notification {
                            id: 0,
                            message: format!("Session load failed: {e}"),
                            level: crate::tui::events::NotificationLevel::Error,
                            ttl_ticks: 100,
                        },
                    ));
                }
            }
        });
    }

    pub fn cmd_new_session(&mut self, services: &Services) {
        if let Some(ref sid) = self.session_id {
            let db = services.database.clone();
            let sid = sid.clone();
            tokio::spawn(async move {
                use crate::database::ChatOps;
                if let Err(e) = db.archive_chat_session(&sid).await {
                    log::error!("Failed to archive session: {e}");
                }
            });
        }
        self.messages.clear();
        self.session_id = None;
        self.scroll_offset = 0;
        self.streaming_buffer.clear();
        self.streaming_record_id = None;
        self.active_stream_id = None;
        self.load_session(services);
    }

    fn cmd_help(&self, services: &Services) {
        let _ = services.event_tx.send(AppEvent::Notification(Notification {
            id: 0,
            message: "Commands: /clear /new /help".to_string(),
            level: NotificationLevel::Info,
            ttl_ticks: 120,
        }));
    }

    // ── Message sending + LLM streaming ──────────────────────────────

    fn send_message(&mut self, text: &str, services: &Services) {
        let session_id = match self.session_id {
            Some(ref s) => s.clone(),
            None => return,
        };

        // Cancel any active stream
        if let Some(ref stream_id) = self.active_stream_id {
            let llm = services.llm.clone();
            let sid = stream_id.clone();
            tokio::spawn(async move {
                llm.cancel_stream(&sid).await;
            });
            self.finalize_response();
        }

        // 1. Persist + display user message
        let (user_display, user_record) = DisplayMessage::from_user_input(&session_id, text);
        let db = services.database.clone();
        let user_rec = user_record.clone();
        tokio::spawn(async move {
            use crate::database::ChatOps;
            if let Err(e) = db.add_chat_message(&user_rec).await {
                log::error!("Failed to persist user message: {e}");
            }
        });
        self.messages.push(user_display);

        // 2. Create streaming assistant placeholder
        let (assistant_display, assistant_record) = DisplayMessage::new_streaming(&session_id);
        self.streaming_record_id = Some(assistant_record.id.clone());
        let db2 = services.database.clone();
        let asst_rec = assistant_record.clone();
        tokio::spawn(async move {
            use crate::database::ChatOps;
            if let Err(e) = db2.add_chat_message(&asst_rec).await {
                log::error!("Failed to persist assistant placeholder: {e}");
            }
        });
        self.messages.push(assistant_display);
        self.streaming_buffer.clear();
        self.scroll_to_bottom();

        // 3. Build ChatRequest from history
        let chat_messages: Vec<ChatMessage> = self
            .messages
            .iter()
            .filter(|m| !m.is_streaming && !m.raw_content.is_empty())
            .filter_map(|m| match m.role {
                MessageRole::User => Some(ChatMessage::user(m.raw_content.clone())),
                MessageRole::Assistant => Some(ChatMessage::assistant(m.raw_content.clone())),
                MessageRole::System => Some(ChatMessage::system(m.raw_content.clone())),
                MessageRole::Error => None,
            })
            .collect();

        let request = ChatRequest::new(chat_messages).with_system(
            "You are a knowledgeable TTRPG Game Master assistant. Help the GM with rules, \
             lore, encounter design, NPC roleplay, and session planning. Be concise and practical.",
        );

        // 4. Spawn streaming task
        let llm = services.llm.clone();
        let tx = services.event_tx.clone();

        tokio::spawn(async move {
            match llm.stream_chat(request).await {
                Ok(mut rx) => {
                    while let Some(chunk_result) = rx.recv().await {
                        match chunk_result {
                            Ok(chunk) => {
                                if !chunk.content.is_empty() {
                                    let _ = tx.send(AppEvent::LlmToken(chunk.content));
                                }
                                if chunk.is_final {
                                    let _ = tx.send(AppEvent::LlmDone);
                                    return;
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(AppEvent::LlmError(e.to_string()));
                                return;
                            }
                        }
                    }
                    // Channel closed without is_final
                    let _ = tx.send(AppEvent::LlmDone);
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::LlmError(e.to_string()));
                }
            }
        });

        self.active_stream_id = self.streaming_record_id.clone();
    }

    // ── Streaming event handlers (called by AppState) ────────────────

    pub fn append_token(&mut self, token: &str) {
        self.streaming_buffer.push_str(token);
        if let Some(last) = self.messages.last_mut() {
            if last.is_streaming {
                last.raw_content = self.streaming_buffer.clone();
                let mut rendered = markdown_to_lines(&self.streaming_buffer);
                if let Some(last_line) = rendered.last_mut() {
                    last_line
                        .spans
                        .push(Span::styled("▍", Style::default().fg(Color::DarkGray)));
                } else {
                    rendered.push(Line::styled("▍", Style::default().fg(Color::DarkGray)));
                }
                last.rendered_lines = rendered;
            }
        }
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    pub fn finalize_response(&mut self) {
        let final_content = self.streaming_buffer.clone();

        if let Some(last) = self.messages.last_mut() {
            if last.is_streaming {
                last.is_streaming = false;
                last.raw_content.clone_from(&final_content);
                last.rendered_lines = markdown_to_lines(&final_content);
            }
        }

        self.streaming_buffer.clear();
        self.active_stream_id = None;
    }

    pub fn finalize_and_persist(&mut self, services: &Services) {
        let final_content = self.streaming_buffer.clone();
        self.finalize_response();

        if let Some(ref record_id) = self.streaming_record_id {
            let db = services.database.clone();
            let rid = record_id.clone();
            let content = final_content;
            tokio::spawn(async move {
                use crate::database::ChatOps;
                if let Some(mut record) = db.get_chat_message(&rid).await.ok().flatten() {
                    record.content = content;
                    record.is_streaming = 0;
                    if let Err(e) = db.update_chat_message(&record).await {
                        log::error!("Failed to update assistant message: {e}");
                    }
                }
            });
        }
        self.streaming_record_id = None;
    }

    pub fn handle_stream_error(&mut self, error: &str, services: &Services) {
        if let Some(last) = self.messages.last_mut() {
            if last.is_streaming {
                last.is_streaming = false;
                last.role = MessageRole::Error;
                last.raw_content = error.to_string();
                last.rendered_lines = vec![Line::styled(
                    error.to_string(),
                    Style::default().fg(Color::Red),
                )];
            }
        }

        if let Some(ref record_id) = self.streaming_record_id {
            let db = services.database.clone();
            let rid = record_id.clone();
            let err = error.to_string();
            tokio::spawn(async move {
                use crate::database::ChatOps;
                if let Some(mut record) = db.get_chat_message(&rid).await.ok().flatten() {
                    record.content = err;
                    record.is_streaming = 0;
                    record.role = MessageRole::Error.to_string();
                    if let Err(e) = db.update_chat_message(&record).await {
                        log::error!("Failed to update error message: {e}");
                    }
                }
            });
        }

        self.streaming_buffer.clear();
        self.streaming_record_id = None;
        self.active_stream_id = None;

        let _ = services.event_tx.send(AppEvent::Notification(Notification {
            id: 0,
            message: format!("LLM error: {error}"),
            level: NotificationLevel::Error,
            ttl_ticks: 100,
        }));
    }

    // ── Scrolling ────────────────────────────────────────────────────

    fn total_content_lines(&self) -> usize {
        self.messages.iter().map(|m| m.all_lines().len()).sum()
    }

    fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
        self.auto_scroll = false;
    }

    fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
        self.auto_scroll = false;
    }

    fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.total_content_lines();
        self.auto_scroll = true;
    }

    fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = false;
    }

    // ── Rendering ────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([
            Constraint::Min(1),    // Messages
            Constraint::Length(4), // Mode indicator + input
        ])
        .split(area);

        self.render_messages(frame, chunks[0]);
        self.render_input(frame, chunks[1]);
    }

    fn render_messages(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Chat ");

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.session_loading {
            let loading = Paragraph::new(Line::styled(
                "  Loading session...",
                Style::default().fg(Color::DarkGray),
            ));
            frame.render_widget(loading, inner);
            return;
        }

        if self.messages.is_empty() {
            let welcome = Paragraph::new(vec![
                Line::raw(""),
                Line::styled(
                    "  Welcome to TTTTRPS Chat",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Line::raw(""),
                Line::styled(
                    "  Press i or Enter to start typing.",
                    Style::default().fg(Color::DarkGray),
                ),
                Line::styled(
                    "  Type /help for available commands.",
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            frame.render_widget(welcome, inner);
            return;
        }

        let all_lines: Vec<Line> = self
            .messages
            .iter()
            .flat_map(|m| m.all_lines())
            .collect();

        let visible_height = inner.height as usize;
        let total = all_lines.len();

        let max_scroll = total.saturating_sub(visible_height);
        let effective_scroll = if self.auto_scroll {
            max_scroll
        } else {
            self.scroll_offset.min(max_scroll)
        };

        let visible: Vec<Line> = all_lines
            .into_iter()
            .skip(effective_scroll)
            .take(visible_height)
            .collect();

        let paragraph = Paragraph::new(visible);
        frame.render_widget(paragraph, inner);

        // Scrollbar
        if total > visible_height {
            let mut scrollbar_state = ScrollbarState::new(total)
                .position(effective_scroll)
                .viewport_content_length(visible_height);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                area,
                &mut scrollbar_state,
            );
        }

        // "New messages below" indicator
        if !self.auto_scroll && effective_scroll < max_scroll {
            let indicator = Line::styled(
                " ↓ new messages below ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
            let indicator_area = Rect::new(
                inner.x + inner.width.saturating_sub(24),
                inner.y + inner.height.saturating_sub(1),
                24.min(inner.width),
                1,
            );
            frame.render_widget(Paragraph::new(indicator), indicator_area);
        }
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let mode_line = match self.input_mode {
            ChatInputMode::Insert => {
                if self.is_streaming() {
                    Line::from(vec![
                        Span::styled(
                            " -- INSERT -- ",
                            Style::default().fg(Color::Black).bg(Color::Yellow),
                        ),
                        Span::raw(" "),
                        Span::styled("streaming...", Style::default().fg(Color::Cyan)),
                    ])
                } else {
                    Line::from(Span::styled(
                        " -- INSERT -- ",
                        Style::default().fg(Color::Black).bg(Color::Yellow),
                    ))
                }
            }
            ChatInputMode::Normal => {
                if self.is_streaming() {
                    Line::from(vec![
                        Span::styled(
                            " -- NORMAL -- ",
                            Style::default().fg(Color::Black).bg(Color::DarkGray),
                        ),
                        Span::raw(" "),
                        Span::styled("streaming...", Style::default().fg(Color::Cyan)),
                    ])
                } else {
                    Line::from(Span::styled(
                        " -- NORMAL -- ",
                        Style::default().fg(Color::Black).bg(Color::DarkGray),
                    ))
                }
            }
        };

        let chunks = Layout::vertical([
            Constraint::Length(1), // Mode indicator
            Constraint::Min(1),   // Input box
        ])
        .split(area);

        frame.render_widget(Paragraph::new(mode_line), chunks[0]);
        frame.render_widget(
            render_chat_input(&self.input, self.input_mode, self.is_streaming()),
            chunks[1],
        );
    }
}
