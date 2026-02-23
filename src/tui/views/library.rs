//! Library view — displays ingested documents from SurrealDB storage.
//!
//! Shows library items with metadata (title, type, pages, chunks, status).
//! Data loaded asynchronously from SurrealDB. Scrollable with j/k.
//! Press `a` to open the ingestion modal for adding new documents.

use std::path::PathBuf;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::core::storage::models::{create_library_item, LibraryItem};
use crate::ingestion::kreuzberg_extractor::DocumentExtractor;
use crate::ingestion::slugs::generate_source_slug;
use crate::tui::events::{AppEvent, IngestionProgressKind};
use crate::tui::ingestion::run_ingestion_with_error_handling;
use crate::tui::services::Services;
use crate::tui::widgets::input_buffer::InputBuffer;

// ── Content type ────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContentType {
    Rules,
    Fiction,
    SessionNotes,
    Homebrew,
}

impl ContentType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Rules => "rules",
            Self::Fiction => "fiction",
            Self::SessionNotes => "session_notes",
            Self::Homebrew => "homebrew",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Rules => "Rules",
            Self::Fiction => "Fiction",
            Self::SessionNotes => "Session Notes",
            Self::Homebrew => "Homebrew",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Rules => Self::Fiction,
            Self::Fiction => Self::SessionNotes,
            Self::SessionNotes => Self::Homebrew,
            Self::Homebrew => Self::Rules,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Rules => Self::Homebrew,
            Self::Fiction => Self::Rules,
            Self::SessionNotes => Self::Fiction,
            Self::Homebrew => Self::SessionNotes,
        }
    }
}

// ── Modal types ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IngestionField {
    FilePath,
    TitleOverride,
    ContentType,
}

impl IngestionField {
    fn next(self) -> Self {
        match self {
            Self::FilePath => Self::TitleOverride,
            Self::TitleOverride => Self::ContentType,
            Self::ContentType => Self::FilePath,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::FilePath => Self::ContentType,
            Self::TitleOverride => Self::FilePath,
            Self::ContentType => Self::TitleOverride,
        }
    }
}

#[derive(Clone, Debug)]
enum IngestionPhase {
    Extracting { progress: f32, status: String },
    Chunking { chunk_count: usize },
    Storing { stored: usize, total: usize },
    Done { chunk_count: usize },
    Error(String),
}

#[derive(Clone, Debug)]
enum IngestionModal {
    InputForm {
        focused_field: IngestionField,
        content_type: ContentType,
        error: Option<String>,
    },
    Progress {
        file_name: String,
        phase: IngestionPhase,
        library_item_id: Option<String>,
    },
}

// ── Display types ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct ItemDisplay {
    title: String,
    file_type: String,
    page_count: Option<i32>,
    chunk_count: i64,
    status: String,
    game_system: String,
}

#[derive(Clone, Debug)]
struct LibraryData {
    items: Vec<ItemDisplay>,
    total_count: usize,
    ready_count: usize,
    pending_count: usize,
    error_count: usize,
}

// ── State ────────────────────────────────────────────────────────────────────

pub struct LibraryState {
    data: Option<LibraryData>,
    lines_cache: Vec<Line<'static>>,
    scroll: usize,
    loading: bool,
    data_rx: mpsc::UnboundedReceiver<LibraryData>,
    data_tx: mpsc::UnboundedSender<LibraryData>,
    modal: Option<IngestionModal>,
    input: InputBuffer,
    title_input: InputBuffer,
}

impl LibraryState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            data: None,
            lines_cache: Vec::new(),
            scroll: 0,
            loading: false,
            data_rx,
            data_tx,
            modal: None,
            input: InputBuffer::new(),
            title_input: InputBuffer::new(),
        }
    }

    /// Whether an ingestion modal is currently open.
    pub fn has_modal(&self) -> bool {
        self.modal.is_some()
    }

    /// Open the ingestion modal with the file path field focused.
    pub fn open_ingest_modal(&mut self) {
        self.input.clear();
        self.title_input.clear();
        self.modal = Some(IngestionModal::InputForm {
            focused_field: IngestionField::FilePath,
            content_type: ContentType::Rules,
            error: None,
        });
    }

    /// Trigger async data load from SurrealDB storage.
    pub fn load(&mut self, services: &Services) {
        if self.loading {
            return;
        }
        self.loading = true;

        let storage = services.storage.clone();
        let tx = self.data_tx.clone();

        tokio::spawn(async move {
            let db = storage.db();

            // Load all items (up to 200)
            let items_result =
                crate::core::storage::get_library_items(db, None, 200, 0).await;

            let items = match items_result {
                Ok(items) => items,
                Err(e) => {
                    log::warn!("Failed to load library items: {e}");
                    Vec::new()
                }
            };

            let total_count = items.len();
            let ready_count = items.iter().filter(|i| i.item.status == "ready").count();
            let pending_count = items
                .iter()
                .filter(|i| i.item.status == "pending" || i.item.status == "processing")
                .count();
            let error_count = items.iter().filter(|i| i.item.status == "error").count();

            let display_items: Vec<ItemDisplay> = items
                .into_iter()
                .map(|iwc| ItemDisplay {
                    title: iwc.item.title,
                    file_type: iwc
                        .item
                        .file_type
                        .unwrap_or_else(|| "—".to_string()),
                    page_count: iwc.item.page_count,
                    chunk_count: iwc.chunk_count,
                    status: iwc.item.status,
                    game_system: iwc
                        .item
                        .game_system
                        .unwrap_or_else(|| "—".to_string()),
                })
                .collect();

            let data = LibraryData {
                items: display_items,
                total_count,
                ready_count,
                pending_count,
                error_count,
            };

            let _ = tx.send(data);
        });
    }

    /// Poll for async data completion. Call from on_tick.
    pub fn poll(&mut self) {
        if let Ok(data) = self.data_rx.try_recv() {
            self.lines_cache = build_lines(&data);
            self.data = Some(data);
            self.loading = false;
        }
    }

    /// Handle an ingestion progress event from the pipeline.
    pub fn handle_ingestion_event(&mut self, event: &AppEvent, services: &Services) {
        let AppEvent::IngestionProgress {
            library_item_id,
            phase,
        } = event
        else {
            return;
        };

        let mut should_refresh = false;

        // Update modal phase if it matches the current progress modal
        if let Some(IngestionModal::Progress {
            library_item_id: ref modal_id,
            phase: ref mut modal_phase,
            ..
        }) = self.modal
        {
            if modal_id.as_deref() == Some(library_item_id.as_str()) {
                *modal_phase = match phase {
                    IngestionProgressKind::Extracting { progress, status } => {
                        IngestionPhase::Extracting {
                            progress: *progress,
                            status: status.clone(),
                        }
                    }
                    IngestionProgressKind::Chunking { chunk_count } => {
                        IngestionPhase::Chunking {
                            chunk_count: *chunk_count,
                        }
                    }
                    IngestionProgressKind::Storing { stored, total } => {
                        IngestionPhase::Storing {
                            stored: *stored,
                            total: *total,
                        }
                    }
                    IngestionProgressKind::Complete { chunk_count } => {
                        should_refresh = true;
                        IngestionPhase::Done {
                            chunk_count: *chunk_count,
                        }
                    }
                    IngestionProgressKind::Error(msg) => IngestionPhase::Error(msg.clone()),
                };
            }
        } else if matches!(phase, IngestionProgressKind::Complete { .. }) {
            // Modal was closed during progress — still refresh the list
            should_refresh = true;
        }

        if should_refresh {
            self.load(services);
        }
    }

    // ── Input ────────────────────────────────────────────────────────────

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

        // Modal consumes all input when open
        if self.modal.is_some() {
            return self.handle_modal_input(*code, *modifiers, services);
        }

        match (*modifiers, *code) {
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.scroll_down(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.scroll_up(1);
                true
            }
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                self.scroll = self.lines_cache.len().saturating_sub(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                self.scroll = 0;
                true
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.scroll_down(15);
                true
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.scroll_up(15);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                self.open_ingest_modal();
                true
            }
            _ => false,
        }
    }

    fn handle_modal_input(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        services: &Services,
    ) -> bool {
        let modal = match self.modal {
            Some(ref modal) => modal.clone(),
            None => return false,
        };

        match modal {
            IngestionModal::InputForm {
                focused_field,
                content_type,
                ..
            } => self.handle_form_input(code, modifiers, focused_field, content_type, services),
            IngestionModal::Progress { ref phase, .. } => {
                self.handle_progress_input(code, phase)
            }
        }
    }

    fn handle_form_input(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        focused_field: IngestionField,
        content_type: ContentType,
        services: &Services,
    ) -> bool {
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.modal = None;
                true
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                if let Some(IngestionModal::InputForm {
                    focused_field: ref mut f,
                    ..
                }) = self.modal
                {
                    *f = focused_field.next();
                }
                true
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                if let Some(IngestionModal::InputForm {
                    focused_field: ref mut f,
                    ..
                }) = self.modal
                {
                    *f = focused_field.prev();
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                self.start_ingestion(services);
                true
            }
            // Content type field: left/right to cycle
            (KeyModifiers::NONE, KeyCode::Left)
                if focused_field == IngestionField::ContentType =>
            {
                if let Some(IngestionModal::InputForm {
                    content_type: ref mut ct,
                    ..
                }) = self.modal
                {
                    *ct = content_type.prev();
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Right)
                if focused_field == IngestionField::ContentType =>
            {
                if let Some(IngestionModal::InputForm {
                    content_type: ref mut ct,
                    ..
                }) = self.modal
                {
                    *ct = content_type.next();
                }
                true
            }
            // Text input for the focused field
            _ if focused_field == IngestionField::FilePath
                || focused_field == IngestionField::TitleOverride =>
            {
                self.handle_text_input(focused_field, code);
                true
            }
            _ => true, // Consume all input when modal is open
        }
    }

    fn handle_text_input(&mut self, field: IngestionField, code: KeyCode) {
        let buf = match field {
            IngestionField::FilePath => &mut self.input,
            IngestionField::TitleOverride => &mut self.title_input,
            IngestionField::ContentType => return,
        };

        match code {
            KeyCode::Char(c) => buf.insert_char(c),
            KeyCode::Backspace => buf.backspace(),
            KeyCode::Delete => buf.delete(),
            KeyCode::Left => buf.move_left(),
            KeyCode::Right => buf.move_right(),
            KeyCode::Home => buf.move_home(),
            KeyCode::End => buf.move_end(),
            _ => {}
        }
    }

    fn handle_progress_input(&mut self, code: KeyCode, phase: &IngestionPhase) -> bool {
        match code {
            // Esc closes modal (task continues in background)
            KeyCode::Esc => {
                self.modal = None;
                true
            }
            // Enter closes on Done or Error
            KeyCode::Enter
                if matches!(phase, IngestionPhase::Done { .. } | IngestionPhase::Error(_)) =>
            {
                self.modal = None;
                true
            }
            _ => true, // Consume all input when progress modal is open
        }
    }

    fn start_ingestion(&mut self, services: &Services) {
        let file_path_str = self.input.text().trim().to_string();
        let path = PathBuf::from(&file_path_str);

        // Validate file exists
        if !path.exists() {
            if let Some(IngestionModal::InputForm { ref mut error, .. }) = self.modal {
                *error = Some("File not found".to_string());
            }
            return;
        }

        // Validate supported format
        if !DocumentExtractor::is_supported(&path) {
            if let Some(IngestionModal::InputForm { ref mut error, .. }) = self.modal {
                *error = Some("Unsupported file format".to_string());
            }
            return;
        }

        // Extract modal state
        let (content_type, title_override_text) = match self.modal {
            Some(IngestionModal::InputForm {
                content_type,
                ..
            }) => (content_type, self.title_input.text().trim().to_string()),
            _ => return,
        };

        // Generate slug and title
        let title_override = if title_override_text.is_empty() {
            None
        } else {
            Some(title_override_text.as_str())
        };
        let slug = generate_source_slug(&path, title_override);
        let title = title_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string()
            });

        let file_ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let file_size = std::fs::metadata(&path).map(|m| m.len() as i64).ok();

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document")
            .to_string();

        // Create library item in SurrealDB (status: "processing")
        let item = LibraryItem::builder(slug.clone(), title)
            .file_path(file_path_str)
            .file_type(file_ext)
            .status("processing")
            .content_category(content_type.as_str());

        let item = if let Some(size) = file_size {
            item.file_size(size)
        } else {
            item
        };

        let library_item = item.build();
        let storage = services.storage.clone();
        let event_tx = services.event_tx.clone();
        let ct = content_type.as_str().to_string();
        let file_path_for_spawn = PathBuf::from(library_item.file_path.as_deref().unwrap_or(""));
        let slug_for_spawn = slug.clone();

        // Switch to progress modal (use slug as temporary ID for event matching)
        self.modal = Some(IngestionModal::Progress {
            file_name,
            phase: IngestionPhase::Extracting {
                progress: 0.0,
                status: "Starting...".to_string(),
            },
            library_item_id: Some(slug),
        });

        // Spawn the create + ingest pipeline
        tokio::spawn(async move {
            let db = storage.db();

            // Create library item first
            let item_id = match create_library_item(db, &library_item).await {
                Ok(id) => id,
                Err(e) => {
                    let error_msg = format!("Failed to create library item: {e}");
                    log::error!("{error_msg}");
                    let _ = event_tx.send(AppEvent::IngestionProgress {
                        library_item_id: slug_for_spawn,
                        phase: IngestionProgressKind::Error(error_msg),
                    });
                    return;
                }
            };

            // Extract just the ID portion (SurrealDB returns "library_item:xxx")
            let clean_id = item_id
                .strip_prefix("library_item:")
                .unwrap_or(&item_id)
                .to_string();

            run_ingestion_with_error_handling(
                file_path_for_spawn,
                clean_id,
                ct,
                storage,
                event_tx,
            )
            .await;
        });
    }

    fn scroll_down(&mut self, n: usize) {
        self.scroll = self
            .scroll
            .saturating_add(n)
            .min(self.lines_cache.len().saturating_sub(1));
    }

    fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    // ── Rendering ────────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Library ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading && self.data.is_none() {
            let loading = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Loading library...",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ]);
            frame.render_widget(loading, inner);
            return;
        }

        if self.lines_cache.is_empty() {
            let empty = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "No data loaded. Press r to refresh, a to ingest.",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ]);
            frame.render_widget(empty, inner);
        } else {
            let content =
                Paragraph::new(self.lines_cache.clone()).scroll((self.scroll as u16, 0));
            frame.render_widget(content, inner);
        }

        // Render modal overlay
        if let Some(ref modal) = self.modal {
            match modal {
                IngestionModal::InputForm {
                    focused_field,
                    content_type,
                    error,
                    ..
                } => self.render_form_modal(frame, area, *focused_field, *content_type, error.as_deref()),
                IngestionModal::Progress {
                    file_name, phase, ..
                } => self.render_progress_modal(frame, area, file_name, phase),
            }
        }
    }

    fn render_form_modal(
        &self,
        frame: &mut Frame,
        area: Rect,
        focused_field: IngestionField,
        content_type: ContentType,
        error: Option<&str>,
    ) {
        let modal_area = centered_fixed(60, 16, area);
        let block = Block::default()
            .title(" Ingest Document ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        // Build lines
        let mut lines = vec![Line::raw("")];

        // File Path field
        let fp_style = if focused_field == IngestionField::FilePath {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("File Path:", fp_style),
        ]));

        let cursor_char = if focused_field == IngestionField::FilePath {
            "_"
        } else {
            ""
        };
        let path_text = self.input.text();
        let path_display = if path_text.is_empty() && focused_field != IngestionField::FilePath {
            "(enter file path)".to_string()
        } else {
            format!("{path_text}{cursor_char}")
        };
        let path_style = if focused_field == IngestionField::FilePath {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                if focused_field == IngestionField::FilePath {
                    "▸ "
                } else {
                    "  "
                },
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(path_display, path_style),
        ]));
        lines.push(Line::raw(""));

        // Title override field
        let title_style = if focused_field == IngestionField::TitleOverride {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Title (optional):", title_style),
        ]));

        let title_cursor = if focused_field == IngestionField::TitleOverride {
            "_"
        } else {
            ""
        };
        let title_text = self.title_input.text();
        let title_display = if title_text.is_empty() && focused_field != IngestionField::TitleOverride {
            "(auto from filename)".to_string()
        } else {
            format!("{title_text}{title_cursor}")
        };
        let title_val_style = if focused_field == IngestionField::TitleOverride {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                if focused_field == IngestionField::TitleOverride {
                    "▸ "
                } else {
                    "  "
                },
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(title_display, title_val_style),
        ]));
        lines.push(Line::raw(""));

        // Content Type selector
        let ct_style = if focused_field == IngestionField::ContentType {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let arrows = if focused_field == IngestionField::ContentType {
            format!("  ◀ {} ▶", content_type.label())
        } else {
            format!("    {}", content_type.label())
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Content Type:", ct_style),
            Span::styled(arrows, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::raw(""));

        // Error message
        if let Some(err) = error {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(err, Style::default().fg(Color::Red).bold()),
            ]));
        } else {
            lines.push(Line::raw(""));
        }

        // Footer
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab", Style::default().fg(Color::DarkGray)),
            Span::raw(":next  "),
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(":ingest  "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":cancel"),
        ]));

        frame.render_widget(Clear, modal_area);
        frame.render_widget(Paragraph::new(lines).block(block), modal_area);
    }

    fn render_progress_modal(
        &self,
        frame: &mut Frame,
        area: Rect,
        file_name: &str,
        phase: &IngestionPhase,
    ) {
        let modal_area = centered_fixed(52, 10, area);

        let title_text = if file_name.len() > 30 {
            format!(" Ingesting: {}... ", &file_name[..27])
        } else {
            format!(" Ingesting: {file_name} ")
        };

        let border_color = match phase {
            IngestionPhase::Done { .. } => Color::Green,
            IngestionPhase::Error(_) => Color::Red,
            _ => Color::Yellow,
        };

        let block = Block::default()
            .title(title_text)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let mut lines = vec![Line::raw("")];

        match phase {
            IngestionPhase::Extracting { progress, status } => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Extracting text...", Style::default().fg(Color::Cyan)),
                ]));
                // Progress bar
                let bar_width = 30;
                let filled = (progress * bar_width as f32) as usize;
                let empty = bar_width - filled;
                let pct = (progress * 100.0) as u32;
                lines.push(Line::from(vec![
                    Span::raw("  ["),
                    Span::styled(
                        "█".repeat(filled),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(
                        "░".repeat(empty),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(format!("] {pct}%")),
                ]));
                // Status text (truncated)
                let status_display = if status.len() > 44 {
                    format!("{}...", &status[..41])
                } else {
                    status.clone()
                };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(status_display, Style::default().fg(Color::DarkGray)),
                ]));
            }
            IngestionPhase::Chunking { chunk_count } => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Chunking text...", Style::default().fg(Color::Cyan)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::raw(format!("{chunk_count} chunks created")),
                ]));
            }
            IngestionPhase::Storing { stored, total } => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Storing in database...", Style::default().fg(Color::Cyan)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::raw(format!("{stored}/{total} chunks stored")),
                ]));
            }
            IngestionPhase::Done { chunk_count } => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("Done! {chunk_count} chunks ingested."),
                        Style::default().fg(Color::Green).bold(),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Press Enter or Esc to close", Style::default().fg(Color::DarkGray)),
                ]));
            }
            IngestionPhase::Error(msg) => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Error:", Style::default().fg(Color::Red).bold()),
                ]));
                let error_display = if msg.len() > 44 {
                    format!("{}...", &msg[..41])
                } else {
                    msg.clone()
                };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(error_display, Style::default().fg(Color::Red)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Press Enter or Esc to close", Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        // Footer (only for active progress)
        if matches!(
            phase,
            IngestionPhase::Extracting { .. }
                | IngestionPhase::Chunking { .. }
                | IngestionPhase::Storing { .. }
        ) {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Esc", Style::default().fg(Color::DarkGray)),
                Span::raw(":dismiss (continues in background)"),
            ]));
        }

        frame.render_widget(Clear, modal_area);
        frame.render_widget(Paragraph::new(lines).block(block), modal_area);
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Compute a centered rectangle with fixed dimensions.
fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

// ── Line builders ────────────────────────────────────────────────────────────

fn build_lines(data: &LibraryData) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(data.items.len() + 15);

    // Header
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "  Documents",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!("  {}", "─".repeat(70)),
        Style::default().fg(Color::DarkGray),
    )));

    if data.items.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "No documents in library. Press a to ingest PDFs, EPUBs, or markdown.",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    } else {
        // Table header
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(
                    "{:<30} {:>5} {:>6} {:>7} {:>8}  {}",
                    "Title", "Type", "Pages", "Chunks", "Status", "System"
                ),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        for item in &data.items {
            let title_display = if item.title.len() > 28 {
                format!("{}...", &item.title[..25])
            } else {
                item.title.clone()
            };

            let pages = item
                .page_count
                .map(|p| p.to_string())
                .unwrap_or_else(|| "—".to_string());

            let status_color = match item.status.as_str() {
                "ready" => Color::Green,
                "processing" => Color::Yellow,
                "pending" => Color::DarkGray,
                "error" => Color::Red,
                _ => Color::DarkGray,
            };

            let system_display = if item.game_system.len() > 12 {
                format!("{}...", &item.game_system[..9])
            } else {
                item.game_system.clone()
            };

            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:<30}", title_display),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(format!(" {:>5} {:>6} {:>7} ", item.file_type, pages, item.chunk_count)),
                Span::styled(
                    format!("{:>8}", item.status),
                    Style::default().fg(status_color),
                ),
                Span::raw(format!("  {}", system_display)),
            ]));
        }
    }

    // Summary
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        format!("  {}", "─".repeat(70)),
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Total: ", Style::default().fg(Color::DarkGray)),
        Span::raw(format!("{} items", data.total_count)),
        Span::styled(" (", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{} ready", data.ready_count),
            Style::default().fg(Color::Green),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} pending", data.pending_count),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} error", data.error_count),
            Style::default().fg(Color::Red),
        ),
        Span::styled(")", Style::default().fg(Color::DarkGray)),
    ]));

    // Footer
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k", Style::default().fg(Color::DarkGray)),
        Span::raw(":scroll "),
        Span::styled("G/g", Style::default().fg(Color::DarkGray)),
        Span::raw(":bottom/top "),
        Span::styled("a", Style::default().fg(Color::DarkGray)),
        Span::raw(":ingest "),
        Span::styled("r", Style::default().fg(Color::DarkGray)),
        Span::raw(":refresh"),
    ]));
    lines.push(Line::raw(""));

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_state_new() {
        let state = LibraryState::new();
        assert!(state.data.is_none());
        assert!(state.lines_cache.is_empty());
        assert_eq!(state.scroll, 0);
        assert!(!state.loading);
        assert!(!state.has_modal());
    }

    #[test]
    fn test_open_ingest_modal() {
        let mut state = LibraryState::new();
        assert!(!state.has_modal());

        state.open_ingest_modal();
        assert!(state.has_modal());

        match &state.modal {
            Some(IngestionModal::InputForm {
                focused_field,
                content_type,
                error,
                ..
            }) => {
                assert_eq!(*focused_field, IngestionField::FilePath);
                assert_eq!(*content_type, ContentType::Rules);
                assert!(error.is_none());
            }
            _ => panic!("Expected InputForm modal"),
        }
    }

    #[test]
    fn test_content_type_cycle() {
        assert_eq!(ContentType::Rules.next(), ContentType::Fiction);
        assert_eq!(ContentType::Fiction.next(), ContentType::SessionNotes);
        assert_eq!(ContentType::SessionNotes.next(), ContentType::Homebrew);
        assert_eq!(ContentType::Homebrew.next(), ContentType::Rules);

        assert_eq!(ContentType::Rules.prev(), ContentType::Homebrew);
        assert_eq!(ContentType::Homebrew.prev(), ContentType::SessionNotes);
    }

    #[test]
    fn test_content_type_labels() {
        assert_eq!(ContentType::Rules.as_str(), "rules");
        assert_eq!(ContentType::Fiction.label(), "Fiction");
        assert_eq!(ContentType::SessionNotes.as_str(), "session_notes");
        assert_eq!(ContentType::Homebrew.label(), "Homebrew");
    }

    #[test]
    fn test_ingestion_field_cycle() {
        assert_eq!(IngestionField::FilePath.next(), IngestionField::TitleOverride);
        assert_eq!(IngestionField::TitleOverride.next(), IngestionField::ContentType);
        assert_eq!(IngestionField::ContentType.next(), IngestionField::FilePath);

        assert_eq!(IngestionField::FilePath.prev(), IngestionField::ContentType);
    }

    #[test]
    fn test_build_lines_empty() {
        let data = LibraryData {
            items: vec![],
            total_count: 0,
            ready_count: 0,
            pending_count: 0,
            error_count: 0,
        };
        let lines = build_lines(&data);
        let text: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Documents"));
        assert!(text.contains("No documents in library"));
        assert!(text.contains("0 items"));
    }

    #[test]
    fn test_build_lines_with_items() {
        let data = LibraryData {
            items: vec![
                ItemDisplay {
                    title: "Player's Handbook".to_string(),
                    file_type: "pdf".to_string(),
                    page_count: Some(300),
                    chunk_count: 1250,
                    status: "ready".to_string(),
                    game_system: "D&D 5e".to_string(),
                },
                ItemDisplay {
                    title: "Homebrew Notes".to_string(),
                    file_type: "md".to_string(),
                    page_count: None,
                    chunk_count: 45,
                    status: "pending".to_string(),
                    game_system: "—".to_string(),
                },
            ],
            total_count: 2,
            ready_count: 1,
            pending_count: 1,
            error_count: 0,
        };
        let lines = build_lines(&data);
        let text: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Player's Handbook"));
        assert!(text.contains("Homebrew Notes"));
        assert!(text.contains("2 items"));
        assert!(text.contains("1 ready"));
        assert!(text.contains("1 pending"));
    }

    #[test]
    fn test_scroll_bounds() {
        let mut state = LibraryState::new();
        state.scroll_down(10);
        assert_eq!(state.scroll, 0);

        state.lines_cache = vec![Line::raw(""); 20];
        state.scroll_down(5);
        assert_eq!(state.scroll, 5);
        state.scroll_down(100);
        assert_eq!(state.scroll, 19);
        state.scroll_up(100);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_centered_fixed() {
        let area = Rect::new(0, 0, 100, 50);
        let centered = centered_fixed(60, 16, area);
        assert_eq!(centered.x, 20);
        assert_eq!(centered.y, 17);
        assert_eq!(centered.width, 60);
        assert_eq!(centered.height, 16);
    }

    #[test]
    fn test_centered_fixed_small_area() {
        let area = Rect::new(0, 0, 30, 10);
        let centered = centered_fixed(60, 16, area);
        // Width and height clamped to area size
        assert_eq!(centered.width, 30);
        assert_eq!(centered.height, 10);
    }
}
