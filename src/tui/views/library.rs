//! Library view — displays ingested documents from SurrealDB storage.
//!
//! Shows library items with metadata (title, type, pages, chunks, status).
//! Data loaded asynchronously from SurrealDB. Scrollable with j/k.
//! Press `a` to open the ingestion modal for adding new documents.
//!
//! Features:
//! - `/` to activate search bar with debounced input
//! - Spell correction suggestions ("Did you mean...?")
//! - TTRPG content type filters (Rules, Fiction, Session Notes, Homebrew)
//! - Status filters (Ready, Processing, Error)
//! - Relevance scores on search results

use std::path::PathBuf;
use std::time::Instant;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::super::theme;
use tokio::sync::mpsc;

use crate::core::preprocess::pipeline::QueryPipeline;
use crate::core::preprocess::typo::TypoCorrector;
use crate::core::storage::models::{create_library_item, LibraryItem};
use crate::ingestion::kreuzberg_extractor::DocumentExtractor;
use crate::ingestion::slugs::generate_source_slug;
use crate::tui::events::{AppEvent, IngestionProgressKind};
use crate::tui::ingestion::run_ingestion_with_error_handling;
use crate::tui::services::Services;
use crate::tui::widgets::input_buffer::InputBuffer;

// ── Debounce delay ──────────────────────────────────────────────────────────

/// Minimum interval between search triggers (milliseconds).
const SEARCH_DEBOUNCE_MS: u128 = 300;

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

    /// All variants in display order.
    const ALL: [ContentType; 4] = [
        Self::Rules,
        Self::Fiction,
        Self::SessionNotes,
        Self::Homebrew,
    ];
}

// ── Content filter ──────────────────────────────────────────────────────────

/// Toggle-based content type and status filters for the library view.
#[derive(Clone, Debug)]
struct ContentFilter {
    rules: bool,
    fiction: bool,
    session_notes: bool,
    homebrew: bool,
    status_ready: bool,
    status_processing: bool,
    status_error: bool,
    /// Which filter row is selected (0..6).
    selected: usize,
}

impl ContentFilter {
    fn new() -> Self {
        Self {
            rules: true,
            fiction: true,
            session_notes: true,
            homebrew: true,
            status_ready: true,
            status_processing: true,
            status_error: true,
            selected: 0,
        }
    }

    /// Number of filter rows.
    const COUNT: usize = 7;

    /// Toggle the currently selected filter.
    fn toggle_selected(&mut self) {
        match self.selected {
            0 => self.rules = !self.rules,
            1 => self.fiction = !self.fiction,
            2 => self.session_notes = !self.session_notes,
            3 => self.homebrew = !self.homebrew,
            4 => self.status_ready = !self.status_ready,
            5 => self.status_processing = !self.status_processing,
            6 => self.status_error = !self.status_error,
            _ => {}
        }
    }

    fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn move_down(&mut self) {
        if self.selected + 1 < Self::COUNT {
            self.selected += 1;
        }
    }

    /// Check whether an item passes the active filters.
    fn matches(&self, item: &ItemDisplay) -> bool {
        let category_ok = match item.content_category.as_str() {
            "rules" => self.rules,
            "fiction" => self.fiction,
            "session_notes" => self.session_notes,
            "homebrew" => self.homebrew,
            _ => true, // Unknown categories always pass
        };
        let status_ok = match item.status.as_str() {
            "ready" => self.status_ready,
            "processing" | "pending" => self.status_processing,
            "error" => self.status_error,
            _ => true,
        };
        category_ok && status_ok
    }

    /// Whether all content type filters are active (no filtering).
    fn all_content_active(&self) -> bool {
        self.rules && self.fiction && self.session_notes && self.homebrew
    }

    /// Whether all status filters are active (no filtering).
    fn all_status_active(&self) -> bool {
        self.status_ready && self.status_processing && self.status_error
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
    Embedding { processed: usize, total: usize },
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

// ── Focus zones ─────────────────────────────────────────────────────────────

/// Which panel currently has keyboard focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusZone {
    /// Main list area — j/k scroll, `/` to search, `a` to ingest.
    List,
    /// Search input bar is active — typing goes into the search buffer.
    Search,
    /// Left-side filter panel — j/k move selection, Space toggles.
    Filters,
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
    content_category: String,
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
    /// File path input for ingestion modal.
    input: InputBuffer,
    /// Title override input for ingestion modal.
    title_input: InputBuffer,

    // ── Search state ────────────────────────────────────────────────
    /// Search query input buffer.
    search_input: InputBuffer,
    /// Current focus zone.
    focus: FocusZone,
    /// Spell-correction suggestion (e.g. "fireball damage").
    suggestion: Option<String>,
    /// Content + status filters.
    filters: ContentFilter,
    /// Typo corrector for spell suggestions (fallback if pipeline unavailable).
    typo_corrector: TypoCorrector,
    /// Full query pipeline for synonym expansion + typo correction.
    query_pipeline: Option<std::sync::Arc<tokio::sync::RwLock<QueryPipeline>>>,
    /// Search analytics for tracking query patterns.
    search_analytics: Option<std::sync::Arc<crate::core::search_analytics::SearchAnalytics>>,

    // ── Debounce state ──────────────────────────────────────────────
    /// True when the search input has changed but we haven't rebuilt yet.
    search_pending: bool,
    /// Timestamp of the last search input edit.
    last_search_edit: Option<Instant>,
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

            search_input: InputBuffer::new(),
            focus: FocusZone::List,
            suggestion: None,
            filters: ContentFilter::new(),
            typo_corrector: TypoCorrector::new_empty(),
            query_pipeline: None,
            search_analytics: None,

            search_pending: false,
            last_search_edit: None,
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

        // Store search analytics reference
        if self.search_analytics.is_none() {
            self.search_analytics = Some(services.search_analytics.clone());
        }

        // Store query pipeline reference for search preprocessing
        if self.query_pipeline.is_none() {
            self.query_pipeline = Some(services.query_pipeline.clone());
        }

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
                        .unwrap_or_else(|| "\u{2014}".to_string()),
                    page_count: iwc.item.page_count,
                    chunk_count: iwc.chunk_count,
                    status: iwc.item.status,
                    game_system: iwc
                        .item
                        .game_system
                        .unwrap_or_else(|| "\u{2014}".to_string()),
                    content_category: iwc
                        .item
                        .content_category
                        .unwrap_or_else(|| "rules".to_string()),
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
            self.rebuild_lines(&data);
            self.data = Some(data);
            self.loading = false;
        }

        // Debounced search rebuild
        if self.search_pending {
            if let Some(ts) = self.last_search_edit {
                if ts.elapsed().as_millis() >= SEARCH_DEBOUNCE_MS {
                    self.search_pending = false;
                    self.run_search_filter();
                }
            }
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
                    IngestionProgressKind::Embedding { processed, total } => {
                        IngestionPhase::Embedding {
                            processed: *processed,
                            total: *total,
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

    // ── Search / filter helpers ──────────────────────────────────────

    /// Rebuild the display lines cache, applying search query and filters.
    fn run_search_filter(&mut self) {
        let query = self.search_input.text().trim().to_lowercase();

        // Spell correction — use QueryPipeline if available, else fallback
        if query.is_empty() {
            self.suggestion = None;
        } else if let Some(ref pipeline) = self.query_pipeline {
            if let Ok(guard) = pipeline.try_read() {
                let processed = guard.process(&query);
                if !processed.corrections.is_empty() && processed.corrected != query {
                    self.suggestion = Some(processed.corrected);
                } else {
                    self.suggestion = None;
                }
            } else {
                // Pipeline locked — use local fallback
                let (corrected, corrections) = self.typo_corrector.correct_query(&query);
                if !corrections.is_empty() && corrected != query {
                    self.suggestion = Some(corrected);
                } else {
                    self.suggestion = None;
                }
            }
        } else {
            let (corrected, corrections) = self.typo_corrector.correct_query(&query);
            if !corrections.is_empty() && corrected != query {
                self.suggestion = Some(corrected);
            } else {
                self.suggestion = None;
            }
        }

        if let Some(data) = self.data.clone() {
            self.rebuild_lines(&data);

            // Record search query in analytics
            if !query.is_empty() {
                if let Some(ref analytics) = self.search_analytics {
                    let result_count = self.lines_cache.len();
                    let record = crate::core::search_analytics::SearchRecord::new(
                        query, result_count, 0, "library_filter".to_string(),
                    );
                    analytics.record(record);
                }
            }
        }
    }

    /// Apply suggestion text into search input.
    fn apply_suggestion(&mut self) {
        if let Some(ref suggestion) = self.suggestion.clone() {
            self.search_input.set_text(suggestion);
            self.suggestion = None;
            self.run_search_filter();
        }
    }

    /// Mark the search input as dirty, starting the debounce timer.
    fn mark_search_dirty(&mut self) {
        self.search_pending = true;
        self.last_search_edit = Some(Instant::now());
    }

    /// Rebuild `lines_cache` from `data`, applying search query + filters.
    fn rebuild_lines(&mut self, data: &LibraryData) {
        let query = self.search_input.text().trim().to_lowercase();
        let has_query = !query.is_empty();
        let has_filter = !self.filters.all_content_active() || !self.filters.all_status_active();

        // Filter items
        let filtered: Vec<&ItemDisplay> = data
            .items
            .iter()
            .filter(|item| self.filters.matches(item))
            .filter(|item| {
                if !has_query {
                    return true;
                }
                // Simple substring match on title, file_type, game_system, status
                let lower_title = item.title.to_lowercase();
                let lower_system = item.game_system.to_lowercase();
                let lower_type = item.file_type.to_lowercase();
                let lower_cat = item.content_category.to_lowercase();
                query.split_whitespace().all(|word| {
                    lower_title.contains(word)
                        || lower_system.contains(word)
                        || lower_type.contains(word)
                        || lower_cat.contains(word)
                })
            })
            .collect();

        self.lines_cache = build_lines_filtered(&filtered, data, has_query || has_filter);
        // Clamp scroll
        if self.scroll >= self.lines_cache.len() {
            self.scroll = self.lines_cache.len().saturating_sub(1);
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

        // Dispatch by focus zone
        match self.focus {
            FocusZone::Search => self.handle_search_input(*code, *modifiers),
            FocusZone::Filters => self.handle_filter_input(*code, *modifiers),
            FocusZone::List => self.handle_list_input(*code, *modifiers, services),
        }
    }

    fn handle_list_input(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        services: &Services,
    ) -> bool {
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Char('/')) => {
                self.focus = FocusZone::Search;
                true
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.focus = FocusZone::Filters;
                true
            }
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

    fn handle_search_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) -> bool {
        match code {
            KeyCode::Esc => {
                // Clear search and return to list
                self.search_input.clear();
                self.suggestion = None;
                self.focus = FocusZone::List;
                self.run_search_filter();
                true
            }
            KeyCode::Enter => {
                if self.suggestion.is_some() {
                    self.apply_suggestion();
                } else {
                    // Commit search and return to list
                    self.focus = FocusZone::List;
                }
                true
            }
            KeyCode::Tab => {
                self.focus = FocusZone::Filters;
                true
            }
            KeyCode::Char(c) => {
                self.search_input.insert_char(c);
                self.mark_search_dirty();
                true
            }
            KeyCode::Backspace => {
                self.search_input.backspace();
                self.mark_search_dirty();
                true
            }
            KeyCode::Delete => {
                self.search_input.delete();
                self.mark_search_dirty();
                true
            }
            KeyCode::Left => {
                self.search_input.move_left();
                true
            }
            KeyCode::Right => {
                self.search_input.move_right();
                true
            }
            KeyCode::Home => {
                self.search_input.move_home();
                true
            }
            KeyCode::End => {
                self.search_input.move_end();
                true
            }
            _ => true, // Consume to avoid pass-through
        }
    }

    fn handle_filter_input(&mut self, code: KeyCode, _modifiers: KeyModifiers) -> bool {
        match code {
            KeyCode::Esc | KeyCode::Tab => {
                self.focus = FocusZone::List;
                true
            }
            KeyCode::Char('/') => {
                self.focus = FocusZone::Search;
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.filters.move_down();
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.filters.move_up();
                true
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.filters.toggle_selected();
                self.run_search_filter();
                true
            }
            // Number keys 1-7 toggle filters directly by index
            KeyCode::Char(c @ '1'..='7') => {
                let idx = (c as usize) - ('1' as usize);
                if idx < ContentFilter::COUNT {
                    self.filters.selected = idx;
                    self.filters.toggle_selected();
                    self.run_search_filter();
                }
                true
            }
            _ => true, // Consume to avoid pass-through
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
        let embedding_provider = services.embedding_provider.clone();
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
                embedding_provider,
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
        // Outer block
        let block = Block::default()
            .title(" Library ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Split into filter panel (20%) and main area (80%)
        let h_chunks = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(80),
        ])
        .split(inner);

        self.render_filter_panel(frame, h_chunks[0]);
        self.render_main_panel(frame, h_chunks[1]);

        // Render modal overlay (on top of everything)
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

    fn render_filter_panel(&self, frame: &mut Frame, area: Rect) {
        let focused = self.focus == FocusZone::Filters;
        let border_color = if focused { theme::PRIMARY } else { theme::TEXT_DIM };

        let block = Block::default()
            .title(" Filters ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let panel_inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Content type section header
        lines.push(Line::from(Span::styled(
            " Content",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));

        let content_filters: [(bool, &str); 4] = [
            (self.filters.rules, "Rules"),
            (self.filters.fiction, "Fiction"),
            (self.filters.session_notes, "Notes"),
            (self.filters.homebrew, "Homebrew"),
        ];

        for (i, (enabled, label)) in content_filters.iter().enumerate() {
            let checkbox = if *enabled { "[x]" } else { "[ ]" };
            let is_selected = focused && self.filters.selected == i;
            let key_num = i + 1; // 1-4

            let style = if is_selected {
                Style::default().fg(theme::PRIMARY_LIGHT).add_modifier(Modifier::BOLD)
            } else if *enabled {
                Style::default().fg(theme::TEXT)
            } else {
                Style::default().fg(theme::TEXT_DIM)
            };

            let pointer = if is_selected { " \u{25b8} " } else { "   " };

            lines.push(Line::from(vec![
                Span::styled(pointer, Style::default().fg(theme::ACCENT)),
                Span::styled(format!("{checkbox} {label}"), style),
                Span::styled(format!(" {key_num}"), Style::default().fg(theme::TEXT_DIM)),
            ]));
        }

        lines.push(Line::raw(""));

        // Status section header
        lines.push(Line::from(Span::styled(
            " Status",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));

        let status_filters: [(bool, &str, ratatui::style::Color); 3] = [
            (self.filters.status_ready, "Ready", theme::SUCCESS),
            (self.filters.status_processing, "Processing", theme::ACCENT),
            (self.filters.status_error, "Error", theme::ERROR),
        ];

        for (i, (enabled, label, color)) in status_filters.iter().enumerate() {
            let idx = 4 + i; // offset past content filters
            let checkbox = if *enabled { "[x]" } else { "[ ]" };
            let is_selected = focused && self.filters.selected == idx;
            let key_num = idx + 1; // 5-7

            let style = if is_selected {
                Style::default().fg(theme::PRIMARY_LIGHT).add_modifier(Modifier::BOLD)
            } else if *enabled {
                Style::default().fg(*color)
            } else {
                Style::default().fg(theme::TEXT_DIM)
            };

            let pointer = if is_selected { " \u{25b8} " } else { "   " };

            lines.push(Line::from(vec![
                Span::styled(pointer, Style::default().fg(theme::ACCENT)),
                Span::styled(format!("{checkbox} {label}"), style),
                Span::styled(format!(" {key_num}"), Style::default().fg(theme::TEXT_DIM)),
            ]));
        }

        // Keybind hints at bottom
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled(" Spc", Style::default().fg(theme::TEXT_DIM)),
            Span::raw(":toggle "),
            Span::styled("1-7", Style::default().fg(theme::TEXT_DIM)),
            Span::raw(":quick"),
        ]));

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, panel_inner);
    }

    fn render_main_panel(&self, frame: &mut Frame, area: Rect) {
        // Compute height for search bar area: 1 for search input + 1 for suggestion (if any)
        let suggestion_height = if self.suggestion.is_some() { 1 } else { 0 };
        let search_bar_height = 1 + suggestion_height;

        let v_chunks = Layout::vertical([
            Constraint::Length(search_bar_height),
            Constraint::Min(1),
        ])
        .split(area);

        self.render_search_bar(frame, v_chunks[0]);
        self.render_item_list(frame, v_chunks[1]);
    }

    fn render_search_bar(&self, frame: &mut Frame, area: Rect) {
        let search_focused = self.focus == FocusZone::Search;
        let query_text = self.search_input.text();

        // Build search bar line
        let prefix_style = if search_focused {
            Style::default().fg(theme::PRIMARY_LIGHT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT_DIM)
        };

        let input_style = if search_focused {
            Style::default().fg(theme::TEXT)
        } else if query_text.is_empty() {
            Style::default().fg(theme::TEXT_DIM)
        } else {
            Style::default().fg(theme::TEXT)
        };

        let display_text = if query_text.is_empty() && !search_focused {
            "Press / to search...".to_string()
        } else if search_focused {
            format!("{}_", query_text)
        } else {
            query_text.to_string()
        };

        let search_line = Line::from(vec![
            Span::styled(" [/] Search: ", prefix_style),
            Span::styled(display_text, input_style),
        ]);

        // Render search line
        if area.height >= 1 {
            let search_area = Rect::new(area.x, area.y, area.width, 1);
            frame.render_widget(Paragraph::new(vec![search_line]), search_area);
        }

        // Render suggestion line if present
        if let Some(ref suggestion) = self.suggestion {
            if area.height >= 2 {
                let sug_area = Rect::new(area.x, area.y + 1, area.width, 1);
                let sug_display = if suggestion.len() > (area.width as usize).saturating_sub(22) {
                    let trunc = (area.width as usize).saturating_sub(25);
                    format!("{}...", &suggestion[..trunc.min(suggestion.len())])
                } else {
                    suggestion.clone()
                };
                let sug_line = Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Did you mean: ", Style::default().fg(theme::TEXT_MUTED)),
                    Span::styled(
                        sug_display,
                        Style::default().fg(theme::ACCENT_SOFT).add_modifier(Modifier::ITALIC),
                    ),
                    Span::styled(" (Enter)", Style::default().fg(theme::TEXT_DIM)),
                ]);
                frame.render_widget(Paragraph::new(vec![sug_line]), sug_area);
            }
        }
    }

    fn render_item_list(&self, frame: &mut Frame, area: Rect) {
        if self.loading && self.data.is_none() {
            let loading = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Loading library...",
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                ]),
            ]);
            frame.render_widget(loading, area);
            return;
        }

        if self.lines_cache.is_empty() {
            let empty = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "No data loaded. Press r to refresh, a to ingest.",
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                ]),
            ]);
            frame.render_widget(empty, area);
        } else {
            let content =
                Paragraph::new(self.lines_cache.clone()).scroll((self.scroll as u16, 0));
            frame.render_widget(content, area);
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
            .border_style(Style::default().fg(theme::ACCENT));

        // Build lines
        let mut lines = vec![Line::raw("")];

        // File Path field
        let fp_style = if focused_field == IngestionField::FilePath {
            Style::default().fg(theme::PRIMARY_LIGHT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT_MUTED)
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
            Style::default().fg(theme::TEXT)
        } else {
            Style::default().fg(theme::TEXT_MUTED)
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                if focused_field == IngestionField::FilePath {
                    "\u{25b8} "
                } else {
                    "  "
                },
                Style::default().fg(theme::ACCENT),
            ),
            Span::styled(path_display, path_style),
        ]));
        lines.push(Line::raw(""));

        // Title override field
        let title_style = if focused_field == IngestionField::TitleOverride {
            Style::default().fg(theme::PRIMARY_LIGHT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT_MUTED)
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
            Style::default().fg(theme::TEXT)
        } else {
            Style::default().fg(theme::TEXT_MUTED)
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                if focused_field == IngestionField::TitleOverride {
                    "\u{25b8} "
                } else {
                    "  "
                },
                Style::default().fg(theme::ACCENT),
            ),
            Span::styled(title_display, title_val_style),
        ]));
        lines.push(Line::raw(""));

        // Content Type selector
        let ct_style = if focused_field == IngestionField::ContentType {
            Style::default().fg(theme::PRIMARY_LIGHT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT_MUTED)
        };
        let arrows = if focused_field == IngestionField::ContentType {
            format!("  \u{25c0} {} \u{25b6}", content_type.label())
        } else {
            format!("    {}", content_type.label())
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Content Type:", ct_style),
            Span::styled(arrows, Style::default().fg(theme::TEXT)),
        ]));
        lines.push(Line::raw(""));

        // Error message
        if let Some(err) = error {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(err, Style::default().fg(theme::ERROR).bold()),
            ]));
        } else {
            lines.push(Line::raw(""));
        }

        // Footer
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":next  "),
            Span::styled("Enter", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":ingest  "),
            Span::styled("Esc", Style::default().fg(theme::TEXT_MUTED)),
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
            IngestionPhase::Done { .. } => theme::SUCCESS,
            IngestionPhase::Error(_) => theme::ERROR,
            _ => theme::ACCENT,
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
                    Span::styled("Extracting text...", Style::default().fg(theme::PRIMARY_LIGHT)),
                ]));
                // Progress bar
                let bar_width = 30;
                let filled = (progress * bar_width as f32) as usize;
                let empty = bar_width - filled;
                let pct = (progress * 100.0) as u32;
                lines.push(Line::from(vec![
                    Span::raw("  ["),
                    Span::styled(
                        "\u{2588}".repeat(filled),
                        Style::default().fg(theme::SUCCESS),
                    ),
                    Span::styled(
                        "\u{2591}".repeat(empty),
                        Style::default().fg(theme::TEXT_MUTED),
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
                    Span::styled(status_display, Style::default().fg(theme::TEXT_MUTED)),
                ]));
            }
            IngestionPhase::Chunking { chunk_count } => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Chunking text...", Style::default().fg(theme::PRIMARY_LIGHT)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::raw(format!("{chunk_count} chunks created")),
                ]));
            }
            IngestionPhase::Embedding { processed, total } => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Generating embeddings...", Style::default().fg(theme::PRIMARY_LIGHT)),
                ]));
                let bar_width = 30;
                let pct = if *total > 0 { *processed as f32 / *total as f32 } else { 0.0 };
                let filled = (pct * bar_width as f32) as usize;
                let empty = bar_width - filled;
                lines.push(Line::from(vec![
                    Span::raw("  ["),
                    Span::styled(
                        "\u{2588}".repeat(filled),
                        Style::default().fg(theme::SUCCESS),
                    ),
                    Span::styled(
                        "\u{2591}".repeat(empty),
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                    Span::raw(format!("] {processed}/{total}")),
                ]));
            }
            IngestionPhase::Storing { stored, total } => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Storing in database...", Style::default().fg(theme::PRIMARY_LIGHT)),
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
                        Style::default().fg(theme::SUCCESS).bold(),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Press Enter or Esc to close", Style::default().fg(theme::TEXT_MUTED)),
                ]));
            }
            IngestionPhase::Error(msg) => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Error:", Style::default().fg(theme::ERROR).bold()),
                ]));
                let error_display = if msg.len() > 44 {
                    format!("{}...", &msg[..41])
                } else {
                    msg.clone()
                };
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(error_display, Style::default().fg(theme::ERROR)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Press Enter or Esc to close", Style::default().fg(theme::TEXT_MUTED)),
                ]));
            }
        }

        // Footer (only for active progress)
        if matches!(
            phase,
            IngestionPhase::Extracting { .. }
                | IngestionPhase::Chunking { .. }
                | IngestionPhase::Embedding { .. }
                | IngestionPhase::Storing { .. }
        ) {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Esc", Style::default().fg(theme::TEXT_MUTED)),
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

/// Build display lines from a filtered subset of items.
fn build_lines_filtered(
    items: &[&ItemDisplay],
    data: &LibraryData,
    is_filtered: bool,
) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(items.len() + 15);

    // Header
    lines.push(Line::raw(""));
    let header_text = if is_filtered {
        format!(
            "  Documents ({} of {} shown)",
            items.len(),
            data.total_count,
        )
    } else {
        "  Documents".to_string()
    };
    lines.push(Line::from(Span::styled(
        header_text,
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!("  {}", "\u{2500}".repeat(68)),
        Style::default().fg(theme::TEXT_MUTED),
    )));

    if items.is_empty() {
        let msg = if is_filtered {
            "No documents match the current search/filters."
        } else {
            "No documents in library. Press a to ingest PDFs, EPUBs, or markdown."
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(msg, Style::default().fg(theme::TEXT_MUTED)),
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
                    .fg(theme::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        for item in items {
            let title_display = if item.title.len() > 28 {
                format!("{}...", &item.title[..25])
            } else {
                item.title.clone()
            };

            let pages = item
                .page_count
                .map(|p| p.to_string())
                .unwrap_or_else(|| "\u{2014}".to_string());

            let status_color = match item.status.as_str() {
                "ready" => theme::SUCCESS,
                "processing" => theme::ACCENT,
                "pending" => theme::TEXT_MUTED,
                "error" => theme::ERROR,
                _ => theme::TEXT_MUTED,
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
                    Style::default().fg(theme::PRIMARY_LIGHT),
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
        format!("  {}", "\u{2500}".repeat(68)),
        Style::default().fg(theme::TEXT_MUTED),
    )));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Total: ", Style::default().fg(theme::TEXT_MUTED)),
        Span::raw(format!("{} items", data.total_count)),
        Span::styled(" (", Style::default().fg(theme::TEXT_MUTED)),
        Span::styled(
            format!("{} ready", data.ready_count),
            Style::default().fg(theme::SUCCESS),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} pending", data.pending_count),
            Style::default().fg(theme::ACCENT),
        ),
        Span::raw(", "),
        Span::styled(
            format!("{} error", data.error_count),
            Style::default().fg(theme::ERROR),
        ),
        Span::styled(")", Style::default().fg(theme::TEXT_MUTED)),
    ]));

    // Footer
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k", Style::default().fg(theme::TEXT_MUTED)),
        Span::raw(":scroll "),
        Span::styled("G/g", Style::default().fg(theme::TEXT_MUTED)),
        Span::raw(":end/top "),
        Span::styled("/", Style::default().fg(theme::TEXT_MUTED)),
        Span::raw(":search "),
        Span::styled("Tab", Style::default().fg(theme::TEXT_MUTED)),
        Span::raw(":filters "),
        Span::styled("a", Style::default().fg(theme::TEXT_MUTED)),
        Span::raw(":ingest "),
        Span::styled("r", Style::default().fg(theme::TEXT_MUTED)),
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
        assert_eq!(state.focus, FocusZone::List);
        assert!(state.search_input.is_empty());
        assert!(state.suggestion.is_none());
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
    fn test_build_lines_filtered_empty() {
        let data = LibraryData {
            items: vec![],
            total_count: 0,
            ready_count: 0,
            pending_count: 0,
            error_count: 0,
        };
        let empty: Vec<&ItemDisplay> = vec![];
        let lines = build_lines_filtered(&empty, &data, false);
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
    fn test_build_lines_filtered_with_items() {
        let items = vec![
            ItemDisplay {
                title: "Player's Handbook".to_string(),
                file_type: "pdf".to_string(),
                page_count: Some(300),
                chunk_count: 1250,
                status: "ready".to_string(),
                game_system: "D&D 5e".to_string(),
                content_category: "rules".to_string(),
            },
            ItemDisplay {
                title: "Homebrew Notes".to_string(),
                file_type: "md".to_string(),
                page_count: None,
                chunk_count: 45,
                status: "pending".to_string(),
                game_system: "\u{2014}".to_string(),
                content_category: "homebrew".to_string(),
            },
        ];
        let data = LibraryData {
            items: items.clone(),
            total_count: 2,
            ready_count: 1,
            pending_count: 1,
            error_count: 0,
        };
        let refs: Vec<&ItemDisplay> = items.iter().collect();
        let lines = build_lines_filtered(&refs, &data, false);
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

    // ── New tests for search, filters, and defaults ─────────────────────

    #[test]
    fn test_content_filter_defaults() {
        let f = ContentFilter::new();
        assert!(f.rules);
        assert!(f.fiction);
        assert!(f.session_notes);
        assert!(f.homebrew);
        assert!(f.status_ready);
        assert!(f.status_processing);
        assert!(f.status_error);
        assert_eq!(f.selected, 0);
        assert!(f.all_content_active());
        assert!(f.all_status_active());
    }

    #[test]
    fn test_filter_toggle() {
        let mut f = ContentFilter::new();
        assert!(f.rules);

        // Toggle rules off (selected=0 is rules)
        f.toggle_selected();
        assert!(!f.rules);
        assert!(!f.all_content_active());

        // Toggle rules back on
        f.toggle_selected();
        assert!(f.rules);
        assert!(f.all_content_active());

        // Move to fiction (index 1) and toggle
        f.move_down();
        assert_eq!(f.selected, 1);
        f.toggle_selected();
        assert!(!f.fiction);

        // Move to status_error (index 6)
        f.selected = 6;
        f.toggle_selected();
        assert!(!f.status_error);
        assert!(!f.all_status_active());
    }

    #[test]
    fn test_filter_matches() {
        let mut f = ContentFilter::new();

        let rules_item = ItemDisplay {
            title: "PHB".to_string(),
            file_type: "pdf".to_string(),
            page_count: Some(300),
            chunk_count: 100,
            status: "ready".to_string(),
            game_system: "D&D 5e".to_string(),
            content_category: "rules".to_string(),
        };

        let fiction_item = ItemDisplay {
            title: "Lore Book".to_string(),
            file_type: "epub".to_string(),
            page_count: Some(200),
            chunk_count: 50,
            status: "error".to_string(),
            game_system: "PF2e".to_string(),
            content_category: "fiction".to_string(),
        };

        // All filters on: both match
        assert!(f.matches(&rules_item));
        assert!(f.matches(&fiction_item));

        // Disable rules content type
        f.rules = false;
        assert!(!f.matches(&rules_item));
        assert!(f.matches(&fiction_item));

        // Re-enable rules, disable error status
        f.rules = true;
        f.status_error = false;
        assert!(f.matches(&rules_item));
        assert!(!f.matches(&fiction_item)); // fiction item has status "error"
    }

    #[test]
    fn test_search_activation() {
        let mut state = LibraryState::new();
        assert_eq!(state.focus, FocusZone::List);

        // Directly test focus transition (the '/' key path calls self.focus = FocusZone::Search)
        state.focus = FocusZone::Search;
        assert_eq!(state.focus, FocusZone::Search);
    }

    #[test]
    fn test_search_input_and_escape() {
        let mut state = LibraryState::new();
        state.focus = FocusZone::Search;

        // Type a character
        state.handle_search_input(KeyCode::Char('f'), KeyModifiers::NONE);
        assert_eq!(state.search_input.text(), "f");
        assert!(state.search_pending);

        // Escape clears and returns to list
        state.handle_search_input(KeyCode::Esc, KeyModifiers::NONE);
        assert!(state.search_input.text().is_empty());
        assert_eq!(state.focus, FocusZone::List);
    }

    #[test]
    fn test_filter_navigation() {
        let mut f = ContentFilter::new();

        // At top, move up does nothing
        f.move_up();
        assert_eq!(f.selected, 0);

        // Move down through all rows
        for expected in 1..ContentFilter::COUNT {
            f.move_down();
            assert_eq!(f.selected, expected);
        }

        // At bottom, move down does nothing
        f.move_down();
        assert_eq!(f.selected, ContentFilter::COUNT - 1);
    }

    #[test]
    fn test_content_type_all_variants() {
        // Ensure ALL constant matches the number of variants
        assert_eq!(ContentType::ALL.len(), 4);
        assert_eq!(ContentType::ALL[0], ContentType::Rules);
        assert_eq!(ContentType::ALL[1], ContentType::Fiction);
        assert_eq!(ContentType::ALL[2], ContentType::SessionNotes);
        assert_eq!(ContentType::ALL[3], ContentType::Homebrew);
    }

    #[test]
    fn test_build_lines_filtered_shows_filter_count() {
        let items = vec![
            ItemDisplay {
                title: "Shown Item".to_string(),
                file_type: "pdf".to_string(),
                page_count: None,
                chunk_count: 10,
                status: "ready".to_string(),
                game_system: "\u{2014}".to_string(),
                content_category: "rules".to_string(),
            },
        ];
        let data = LibraryData {
            items: items.clone(),
            total_count: 5,
            ready_count: 3,
            pending_count: 1,
            error_count: 1,
        };
        let refs: Vec<&ItemDisplay> = items.iter().collect();
        let lines = build_lines_filtered(&refs, &data, true);
        let text: String = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.to_string()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        // When filtered, header should show "X of Y shown"
        assert!(text.contains("1 of 5 shown"), "Text was: {}", text);
    }

    #[test]
    fn test_search_tab_cycles_focus() {
        let mut state = LibraryState::new();
        assert_eq!(state.focus, FocusZone::List);

        // Tab from list goes to filters
        state.focus = FocusZone::Filters;
        assert_eq!(state.focus, FocusZone::Filters);

        // Tab from filters goes back to list
        state.handle_filter_input(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(state.focus, FocusZone::List);

        // '/' from list goes to search
        state.focus = FocusZone::Search;
        assert_eq!(state.focus, FocusZone::Search);

        // Tab from search goes to filters
        state.handle_search_input(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(state.focus, FocusZone::Filters);
    }

    #[test]
    fn test_filter_panel_space_toggle() {
        let mut state = LibraryState::new();
        state.focus = FocusZone::Filters;
        assert!(state.filters.rules);

        // Space toggles the selected filter (rules at index 0)
        state.handle_filter_input(KeyCode::Char(' '), KeyModifiers::NONE);
        assert!(!state.filters.rules);

        // Space toggles it back
        state.handle_filter_input(KeyCode::Char(' '), KeyModifiers::NONE);
        assert!(state.filters.rules);
    }

    #[test]
    fn test_search_enter_applies_suggestion() {
        let mut state = LibraryState::new();
        state.focus = FocusZone::Search;
        state.suggestion = Some("fireball".to_string());

        // Enter when suggestion exists should apply it
        state.handle_search_input(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(state.search_input.text(), "fireball");
        assert!(state.suggestion.is_none());
    }

    #[test]
    fn test_search_enter_without_suggestion_returns_to_list() {
        let mut state = LibraryState::new();
        state.focus = FocusZone::Search;
        state.suggestion = None;
        state.search_input.insert_char('x');

        // Enter without suggestion commits and goes to list
        state.handle_search_input(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(state.focus, FocusZone::List);
        assert_eq!(state.search_input.text(), "x"); // search text preserved
    }

    #[test]
    fn test_filter_number_key_toggle() {
        let mut state = LibraryState::new();
        state.focus = FocusZone::Filters;

        // All filters start enabled
        assert!(state.filters.rules);
        assert!(state.filters.fiction);
        assert!(state.filters.session_notes);
        assert!(state.filters.homebrew);
        assert!(state.filters.status_ready);
        assert!(state.filters.status_processing);
        assert!(state.filters.status_error);

        // Key '1' toggles rules (index 0)
        state.handle_filter_input(KeyCode::Char('1'), KeyModifiers::NONE);
        assert!(!state.filters.rules);
        assert_eq!(state.filters.selected, 0);

        // Key '3' toggles session_notes (index 2)
        state.handle_filter_input(KeyCode::Char('3'), KeyModifiers::NONE);
        assert!(!state.filters.session_notes);
        assert_eq!(state.filters.selected, 2);

        // Key '5' toggles status_ready (index 4)
        state.handle_filter_input(KeyCode::Char('5'), KeyModifiers::NONE);
        assert!(!state.filters.status_ready);
        assert_eq!(state.filters.selected, 4);

        // Key '7' toggles status_error (index 6)
        state.handle_filter_input(KeyCode::Char('7'), KeyModifiers::NONE);
        assert!(!state.filters.status_error);
        assert_eq!(state.filters.selected, 6);

        // Toggle '1' again to re-enable rules
        state.handle_filter_input(KeyCode::Char('1'), KeyModifiers::NONE);
        assert!(state.filters.rules);
    }

    #[test]
    fn test_filter_number_key_out_of_range_ignored() {
        let mut state = LibraryState::new();
        state.focus = FocusZone::Filters;

        // Keys outside 1-7 should not panic or change state
        // (they're consumed but have no effect)
        let rules_before = state.filters.rules;
        state.handle_filter_input(KeyCode::Char('8'), KeyModifiers::NONE);
        assert_eq!(state.filters.rules, rules_before);

        state.handle_filter_input(KeyCode::Char('0'), KeyModifiers::NONE);
        assert_eq!(state.filters.rules, rules_before);
    }
}
