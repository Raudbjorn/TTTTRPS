//! NPC Management view — list, create, edit, and delete NPCs.
//!
//! Master-detail layout: left side shows NPC list, right side shows detail panel.
//! Press `a` to add, `e` to edit, `d` to delete, `Enter` toggles detail panel.
//! Press `/` to filter by name, `Ctrl+G` to generate a random name (in form).
//! Press `v` for voice mode, `t` for talk-about mode.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::core::name_gen::{NameCulture, NameGender, NameGenerator, NameOptions, NameType};
use crate::database::{NpcOps, NpcRecord};
use crate::tui::app::centered_rect;
use crate::tui::services::Services;
use crate::tui::theme;
use crate::tui::widgets::input_buffer::InputBuffer;

// ── Internal async data events ─────────────────────────────────────────────

enum NpcDataEvent {
    NpcsLoaded(Vec<NpcRecord>),
    NpcSaved,
    NpcDeleted,
    LoadError(String),
}

// ── Modal types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NpcModal {
    Create,
    Edit,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormField {
    Name,
    Role,
    Notes,
}

const FORM_FIELDS: [FormField; 3] = [FormField::Name, FormField::Role, FormField::Notes];

// ── State ──────────────────────────────────────────────────────────────────

pub struct NpcViewState {
    // NPC list
    npcs: Vec<NpcRecord>,
    selected: usize,
    show_detail: bool,

    // Filter
    filter: InputBuffer,
    filter_active: bool,

    // Modal
    modal: Option<NpcModal>,
    form_focus: usize,
    form_name: InputBuffer,
    form_role: InputBuffer,
    form_notes: InputBuffer,
    editing_id: Option<String>,

    // Name generation
    name_gen: NameGenerator,

    // Error/status
    error: Option<String>,

    // Async channel
    data_tx: mpsc::UnboundedSender<NpcDataEvent>,
    data_rx: mpsc::UnboundedReceiver<NpcDataEvent>,
}

impl NpcViewState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            npcs: Vec::new(),
            selected: 0,
            show_detail: false,
            filter: InputBuffer::new(),
            filter_active: false,
            modal: None,
            form_focus: 0,
            form_name: InputBuffer::new(),
            form_role: InputBuffer::new(),
            form_notes: InputBuffer::new(),
            editing_id: None,
            name_gen: NameGenerator::new(),
            error: None,
            data_tx,
            data_rx,
        }
    }

    pub fn load(&self, services: &Services) {
        let db = services.database.clone();
        let tx = self.data_tx.clone();
        tokio::spawn(async move {
            match db.list_npcs(None).await {
                Ok(npcs) => { let _ = tx.send(NpcDataEvent::NpcsLoaded(npcs)); }
                Err(e) => { let _ = tx.send(NpcDataEvent::LoadError(format!("{e}"))); }
            }
        });
    }

    pub fn poll(&mut self) {
        while let Ok(event) = self.data_rx.try_recv() {
            match event {
                NpcDataEvent::NpcsLoaded(npcs) => {
                    self.npcs = npcs;
                    if !self.npcs.is_empty() {
                        self.selected = self.selected.min(self.npcs.len() - 1);
                    } else {
                        self.selected = 0;
                    }
                    self.error = None;
                }
                NpcDataEvent::NpcSaved | NpcDataEvent::NpcDeleted => {
                    self.error = None;
                    // Reload list — we need a Services ref, but we only have the channel.
                    // The next load() call from the app will refresh.
                }
                NpcDataEvent::LoadError(msg) => {
                    self.error = Some(msg);
                }
            }
        }
    }

    // ── Filtered NPC list ────────────────────────────────────────────────

    /// Returns indices into `self.npcs` that match the current filter text.
    fn filtered_indices(&self) -> Vec<usize> {
        let query = self.filter.text().trim().to_lowercase();
        if query.is_empty() {
            return (0..self.npcs.len()).collect();
        }
        self.npcs
            .iter()
            .enumerate()
            .filter(|(_, npc)| npc.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect()
    }

    // ── Input handling ─────────────────────────────────────────────────────

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        let Event::Key(KeyEvent { code, modifiers, kind: KeyEventKind::Press, .. }) = event else {
            return false;
        };

        if let Some(modal) = self.modal {
            return self.handle_modal_input(modal, *code, *modifiers, services);
        }

        // When filter is active, route input to filter first
        if self.filter_active {
            return self.handle_filter_input(*code, *modifiers, services);
        }

        self.handle_list_input(*code, *modifiers, services)
    }

    fn handle_list_input(&mut self, code: KeyCode, modifiers: KeyModifiers, services: &Services) -> bool {
        let filtered = self.filtered_indices();
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Char('/')) => {
                self.filter_active = true;
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                if !filtered.is_empty() {
                    // Find current position in filtered list and move forward
                    let pos = filtered.iter().position(|&i| i == self.selected).unwrap_or(0);
                    let next = (pos + 1).min(filtered.len() - 1);
                    self.selected = filtered[next];
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                if !filtered.is_empty() {
                    let pos = filtered.iter().position(|&i| i == self.selected).unwrap_or(0);
                    let prev = pos.saturating_sub(1);
                    self.selected = filtered[prev];
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                self.show_detail = !self.show_detail;
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                self.open_create_modal();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                self.open_edit_modal();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('d')) => {
                if !self.npcs.is_empty() {
                    self.modal = Some(NpcModal::Delete);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }
            _ => false,
        }
    }

    fn handle_filter_input(&mut self, code: KeyCode, modifiers: KeyModifiers, services: &Services) -> bool {
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                // Clear filter and deactivate
                self.filter.clear();
                self.filter_active = false;
                // Reset selection to first item if current selection is not visible
                if !self.npcs.is_empty() {
                    self.selected = self.selected.min(self.npcs.len() - 1);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                // Confirm filter — deactivate but keep text
                self.filter_active = false;
                // Snap selection to first filtered result
                let filtered = self.filtered_indices();
                if !filtered.is_empty() && !filtered.contains(&self.selected) {
                    self.selected = filtered[0];
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Down) | (KeyModifiers::NONE, KeyCode::Up) => {
                // Allow navigation while filter is active
                self.filter_active = false; // hand off to list navigation
                self.handle_list_input(code, modifiers, services)
            }
            _ => {
                route_text_input(&mut self.filter, code, modifiers);
                // After typing, snap selection to first visible match
                let filtered = self.filtered_indices();
                if !filtered.is_empty() && !filtered.contains(&self.selected) {
                    self.selected = filtered[0];
                }
                true
            }
        }
    }

    fn handle_modal_input(
        &mut self,
        modal: NpcModal,
        code: KeyCode,
        modifiers: KeyModifiers,
        services: &Services,
    ) -> bool {
        match modal {
            NpcModal::Create | NpcModal::Edit => self.handle_form_input(code, modifiers, services),
            NpcModal::Delete => self.handle_delete_input(code, services),
        }
    }

    fn handle_form_input(&mut self, code: KeyCode, modifiers: KeyModifiers, services: &Services) -> bool {
        match (modifiers, code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.modal = None;
                true
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.form_focus = (self.form_focus + 1) % FORM_FIELDS.len();
                true
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.form_focus = if self.form_focus == 0 { FORM_FIELDS.len() - 1 } else { self.form_focus - 1 };
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Enter) => {
                self.submit_form(services);
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
                self.generate_name_into_active_field();
                true
            }
            _ => {
                let buf = match FORM_FIELDS[self.form_focus] {
                    FormField::Name => &mut self.form_name,
                    FormField::Role => &mut self.form_role,
                    FormField::Notes => &mut self.form_notes,
                };
                route_text_input(buf, code, modifiers);
                true
            }
        }
    }

    fn handle_delete_input(&mut self, code: KeyCode, services: &Services) -> bool {
        match code {
            KeyCode::Char('y') | KeyCode::Enter => {
                if let Some(npc) = self.npcs.get(self.selected) {
                    let id = npc.id.clone();
                    let db = services.database.clone();
                    let tx = self.data_tx.clone();
                    tokio::spawn(async move {
                        match db.delete_npc(&id).await {
                            Ok(()) => { let _ = tx.send(NpcDataEvent::NpcDeleted); }
                            Err(e) => { let _ = tx.send(NpcDataEvent::LoadError(format!("{e}"))); }
                        }
                    });
                    // Remove locally for instant feedback
                    self.npcs.remove(self.selected);
                    if !self.npcs.is_empty() {
                        self.selected = self.selected.min(self.npcs.len() - 1);
                    } else {
                        self.selected = 0;
                        self.show_detail = false;
                    }
                }
                self.modal = None;
                true
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.modal = None;
                true
            }
            _ => true,
        }
    }

    // ── Form helpers ───────────────────────────────────────────────────────

    fn open_create_modal(&mut self) {
        self.modal = Some(NpcModal::Create);
        self.editing_id = None;
        self.form_focus = 0;
        self.form_name.clear();
        self.form_role.clear();
        self.form_notes.clear();
    }

    fn open_edit_modal(&mut self) {
        if let Some(npc) = self.npcs.get(self.selected) {
            self.modal = Some(NpcModal::Edit);
            self.editing_id = Some(npc.id.clone());
            self.form_focus = 0;
            self.form_name.clear();
            for c in npc.name.chars() { self.form_name.insert_char(c); }
            self.form_role.clear();
            for c in npc.role.chars() { self.form_role.insert_char(c); }
            self.form_notes.clear();
            if let Some(ref notes) = npc.notes {
                for c in notes.chars() { self.form_notes.insert_char(c); }
            }
        }
    }

    fn generate_name_into_active_field(&mut self) {
        let options = NameOptions {
            culture: Some(NameCulture::Fantasy),
            gender: Some(NameGender::Neutral),
            name_type: NameType::FullName,
            ..Default::default()
        };
        let generated = self.name_gen.generate(&options);

        let buf = match FORM_FIELDS[self.form_focus] {
            FormField::Name => &mut self.form_name,
            FormField::Role => &mut self.form_role,
            FormField::Notes => &mut self.form_notes,
        };
        buf.clear();
        for c in generated.name.chars() {
            buf.insert_char(c);
        }
    }

    fn submit_form(&mut self, services: &Services) {
        let name = self.form_name.take();
        let role = self.form_role.take();
        let notes_text = self.form_notes.take();

        if name.trim().is_empty() {
            self.error = Some("Name is required.".to_string());
            return;
        }

        let mut npc = if let Some(ref id) = self.editing_id {
            // Editing existing — preserve fields
            self.npcs.iter().find(|n| &n.id == id).cloned().unwrap_or_else(|| {
                NpcRecord::new(id.clone(), name.clone(), role.clone())
            })
        } else {
            NpcRecord::new(uuid::Uuid::new_v4().to_string(), name.clone(), role.clone())
        };

        npc.name = name;
        npc.role = role;
        npc.notes = if notes_text.trim().is_empty() { None } else { Some(notes_text) };

        let db = services.database.clone();
        let tx = self.data_tx.clone();
        let npc_clone = npc.clone();
        tokio::spawn(async move {
            match db.save_npc(&npc_clone).await {
                Ok(()) => {
                    let _ = tx.send(NpcDataEvent::NpcSaved);
                    // Reload
                    if let Ok(npcs) = db.list_npcs(None).await {
                        let _ = tx.send(NpcDataEvent::NpcsLoaded(npcs));
                    }
                }
                Err(e) => { let _ = tx.send(NpcDataEvent::LoadError(format!("{e}"))); }
            }
        });

        self.modal = None;
    }

    // ── Rendering ──────────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        if self.show_detail && !self.npcs.is_empty() {
            let chunks = Layout::horizontal([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ]).split(area);
            self.render_list(frame, chunks[0]);
            self.render_detail(frame, chunks[1]);
        } else {
            self.render_list(frame, area);
        }

        // Modal overlay
        if let Some(modal) = self.modal {
            match modal {
                NpcModal::Create | NpcModal::Edit => self.render_form_modal(frame, area, modal),
                NpcModal::Delete => self.render_delete_modal(frame, area),
            }
        }
    }

    fn render_list(&self, frame: &mut Frame, area: Rect) {
        let filtered = self.filtered_indices();
        let filter_text = self.filter.text();
        let title = if filter_text.trim().is_empty() {
            format!(" NPCs ({}) ", self.npcs.len())
        } else {
            format!(" NPCs ({}/{}) ", filtered.len(), self.npcs.len())
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_MUTED));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();

        // Filter bar
        if self.filter_active || !filter_text.is_empty() {
            let filter_style = if self.filter_active {
                Style::default().fg(theme::ACCENT)
            } else {
                Style::default().fg(theme::TEXT_MUTED)
            };
            let display_text = if self.filter_active {
                format!("{}▎", filter_text)
            } else {
                filter_text.to_string()
            };
            lines.push(Line::from(vec![
                Span::styled("  / ", filter_style),
                Span::styled(display_text, Style::default().fg(theme::TEXT)),
            ]));
            lines.push(Line::from(Span::styled(
                format!("  {}", "─".repeat(inner.width.saturating_sub(4) as usize)),
                Style::default().fg(theme::TEXT_DIM),
            )));
        }

        if self.npcs.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("No NPCs yet. Press ", Style::default().fg(theme::TEXT_MUTED)),
                Span::styled("a", Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
                Span::styled(" to create one.", Style::default().fg(theme::TEXT_MUTED)),
            ]));
        } else if filtered.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("No matches.", Style::default().fg(theme::TEXT_MUTED)),
            ]));
        } else {
            lines.push(Line::raw(""));
            for &idx in &filtered {
                let npc = &self.npcs[idx];
                let is_selected = idx == self.selected;
                let cursor = if is_selected { "▸ " } else { "  " };

                let name_style = if is_selected {
                    Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let role_display = if npc.role.is_empty() { "(no role)" } else { &npc.role };

                lines.push(Line::from(vec![
                    Span::styled(cursor.to_string(), if is_selected { Style::default().fg(theme::ACCENT) } else { Style::default() }),
                    Span::styled(truncate(&npc.name, 20), name_style),
                    Span::raw("  "),
                    Span::styled(truncate(role_display, 20), Style::default().fg(theme::TEXT_MUTED)),
                ]));
            }
        }

        // Footer
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(inner.width.saturating_sub(4) as usize)),
            Style::default().fg(theme::TEXT_MUTED),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("/", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":filter "),
            Span::styled("a", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":add "),
            Span::styled("e", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":edit "),
            Span::styled("d", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":del "),
            Span::styled("Enter", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":detail "),
            Span::styled("r", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":refresh"),
        ]));

        if let Some(ref err) = self.error {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("✗ {err}"), Style::default().fg(theme::ERROR)),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        let npc = match self.npcs.get(self.selected) {
            Some(n) => n,
            None => return,
        };

        let block = Block::default()
            .title(format!(" {} ", npc.name))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::PRIMARY_LIGHT));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        // Name
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(npc.name.clone(), Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
        ]));

        // Role
        if !npc.role.is_empty() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Role: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::raw(npc.role.clone()),
            ]));
        }

        // Campaign
        if let Some(ref cid) = npc.campaign_id {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Campaign: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::raw(truncate(cid, 30)),
            ]));
        }

        // Location
        if let Some(ref loc) = npc.location_id {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Location: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::raw(truncate(loc, 30)),
            ]));
        }

        // Voice
        if let Some(ref voice) = npc.voice_profile_id {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Voice: ", Style::default().fg(theme::TEXT_MUTED)),
                Span::raw(truncate(voice, 30)),
            ]));
        }

        // Stats (parsed from stats_json)
        if let Some(ref stats_str) = npc.stats_json {
            if let Ok(stats) = serde_json::from_str::<serde_json::Value>(stats_str) {
                if let Some(obj) = stats.as_object() {
                    if !obj.is_empty() {
                        lines.push(Line::raw(""));
                        lines.push(Line::from(Span::styled(
                            "  STATS",
                            Style::default().fg(theme::PRIMARY).add_modifier(Modifier::BOLD),
                        )));
                        for (key, val) in obj {
                            let display_val = match val {
                                serde_json::Value::Number(n) => n.to_string(),
                                serde_json::Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(format!("{}: ", capitalize(key)), Style::default().fg(theme::TEXT_MUTED)),
                                Span::styled(display_val, Style::default().fg(theme::TEXT)),
                            ]));
                        }
                    }
                }
            }
        }

        // Personality traits (parsed from personality_json)
        {
            if let Ok(personality) = serde_json::from_str::<serde_json::Value>(&npc.personality_json) {
                if let Some(obj) = personality.as_object() {
                    // Show personality if it has meaningful data (not just "{}")
                    let has_data = obj.values().any(|v| !v.is_null() && v != "");
                    if has_data && !obj.is_empty() {
                        lines.push(Line::raw(""));
                        lines.push(Line::from(Span::styled(
                            "  PERSONALITY",
                            Style::default().fg(theme::NPC).add_modifier(Modifier::BOLD),
                        )));
                        for (key, val) in obj {
                            match val {
                                serde_json::Value::Array(arr) => {
                                    let items: Vec<String> = arr
                                        .iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect();
                                    if !items.is_empty() {
                                        lines.push(Line::from(vec![
                                            Span::raw("  "),
                                            Span::styled(
                                                format!("{}: ", capitalize(key)),
                                                Style::default().fg(theme::TEXT_MUTED),
                                            ),
                                            Span::styled(
                                                items.join(", "),
                                                Style::default().fg(theme::TEXT),
                                            ),
                                        ]));
                                    }
                                }
                                serde_json::Value::String(s) if !s.is_empty() => {
                                    lines.push(Line::from(vec![
                                        Span::raw("  "),
                                        Span::styled(
                                            format!("{}: ", capitalize(key)),
                                            Style::default().fg(theme::TEXT_MUTED),
                                        ),
                                        Span::styled(
                                            truncate(s, 50),
                                            Style::default().fg(theme::TEXT),
                                        ),
                                    ]));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // Quest hooks
        let hooks = npc.quest_hooks_vec();
        if !hooks.is_empty() {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  QUEST HOOKS",
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
            )));
            for hook in &hooks {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("• ", Style::default().fg(theme::PRIMARY_LIGHT)),
                    Span::raw(truncate(hook, 50)),
                ]));
            }
        }

        // Notes
        if let Some(ref notes) = npc.notes {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  NOTES",
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
            )));
            for line in notes.lines().take(10) {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::raw(line.to_string()),
                ]));
            }
        }

        // Created
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Created: ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(format_datetime(&npc.created_at), Style::default().fg(theme::TEXT_DIM)),
        ]));

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_form_modal(&self, frame: &mut Frame, area: Rect, modal: NpcModal) {
        let modal_area = centered_rect(50, 50, area);
        frame.render_widget(Clear, modal_area);

        let title = match modal {
            NpcModal::Create => " Create NPC ",
            NpcModal::Edit => " Edit NPC ",
            _ => "",
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ACCENT));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        for (i, field) in FORM_FIELDS.iter().enumerate() {
            let is_focused = i == self.form_focus;
            let marker = if is_focused { "▸" } else { " " };
            let label_style = if is_focused {
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_MUTED)
            };

            let (label, value) = match field {
                FormField::Name => {
                    let val = if is_focused {
                        format!("{}▎", self.form_name.text())
                    } else {
                        let t = self.form_name.text().to_string();
                        if t.is_empty() { "(required)".to_string() } else { t }
                    };
                    ("Name", val)
                }
                FormField::Role => {
                    let val = if is_focused {
                        format!("{}▎", self.form_role.text())
                    } else {
                        let t = self.form_role.text().to_string();
                        if t.is_empty() { "(optional)".to_string() } else { t }
                    };
                    ("Role", val)
                }
                FormField::Notes => {
                    let val = if is_focused {
                        format!("{}▎", self.form_notes.text())
                    } else {
                        let t = self.form_notes.text().to_string();
                        if t.is_empty() { "(optional)".to_string() } else { t }
                    };
                    ("Notes", val)
                }
            };

            let val_style = if is_focused {
                Style::default().fg(theme::TEXT)
            } else {
                Style::default()
            };

            lines.push(Line::from(vec![
                Span::raw(format!("  {marker} ")),
                Span::styled(format!("{:<8}", format!("{label}:")), label_style),
                Span::styled(value, val_style),
            ]));
        }

        // Footer
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "─".repeat(inner.width.saturating_sub(4) as usize)),
            Style::default().fg(theme::TEXT_MUTED),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":field "),
            Span::styled("Ctrl+G", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":gen name "),
            Span::styled("Ctrl+Enter", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":save "),
            Span::styled("Esc", Style::default().fg(theme::TEXT_MUTED)),
            Span::raw(":cancel"),
        ]));

        if let Some(ref err) = self.error {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("✗ {err}"), Style::default().fg(theme::ERROR)),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_delete_modal(&self, frame: &mut Frame, area: Rect) {
        let modal_area = centered_rect(40, 20, area);
        frame.render_widget(Clear, modal_area);

        let name = self.npcs.get(self.selected).map(|n| n.name.as_str()).unwrap_or("?");

        let block = Block::default()
            .title(" Delete NPC ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::ERROR));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        let lines = vec![
            Line::raw(""),
            Line::from(vec![
                Span::raw("  Delete "),
                Span::styled(name.to_string(), Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)),
                Span::raw("?"),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("y/Enter", Style::default().fg(theme::SUCCESS)),
                Span::raw(" to confirm, "),
                Span::styled("n/Esc", Style::default().fg(theme::ERROR)),
                Span::raw(" to cancel"),
            ]),
        ];

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

// ── Free helpers ───────────────────────────────────────────────────────────

fn route_text_input(buf: &mut InputBuffer, code: KeyCode, modifiers: KeyModifiers) {
    match (modifiers, code) {
        (KeyModifiers::NONE, KeyCode::Char(c)) | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            buf.insert_char(c);
        }
        (KeyModifiers::NONE, KeyCode::Backspace) => buf.backspace(),
        (KeyModifiers::NONE, KeyCode::Delete) => buf.delete(),
        (KeyModifiers::NONE, KeyCode::Left) => buf.move_left(),
        (KeyModifiers::NONE, KeyCode::Right) => buf.move_right(),
        (KeyModifiers::NONE, KeyCode::Home) => buf.move_home(),
        (KeyModifiers::NONE, KeyCode::End) => buf.move_end(),
        _ => {}
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max.saturating_sub(1)])
    } else {
        s.to_string()
    }
}

fn format_datetime(s: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| s.to_string())
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npc_view_new() {
        let state = NpcViewState::new();
        assert!(state.npcs.is_empty());
        assert_eq!(state.selected, 0);
        assert!(!state.show_detail);
        assert!(state.modal.is_none());
    }

    #[test]
    fn test_npc_view_selection() {
        let mut state = NpcViewState::new();
        state.npcs = vec![
            NpcRecord::new("1".into(), "Aldric".into(), "Merchant".into()),
            NpcRecord::new("2".into(), "Brynn".into(), "Guard".into()),
            NpcRecord::new("3".into(), "Cora".into(), "Healer".into()),
        ];
        assert_eq!(state.selected, 0);

        // Move down
        state.selected = (state.selected + 1).min(state.npcs.len() - 1);
        assert_eq!(state.selected, 1);

        // Move down again
        state.selected = (state.selected + 1).min(state.npcs.len() - 1);
        assert_eq!(state.selected, 2);

        // Can't go past end
        state.selected = (state.selected + 1).min(state.npcs.len() - 1);
        assert_eq!(state.selected, 2);

        // Move up
        state.selected = state.selected.saturating_sub(1);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn test_npc_view_open_create() {
        let mut state = NpcViewState::new();
        state.open_create_modal();
        assert_eq!(state.modal, Some(NpcModal::Create));
        assert!(state.editing_id.is_none());
        assert_eq!(state.form_focus, 0);
    }

    #[test]
    fn test_npc_view_open_edit() {
        let mut state = NpcViewState::new();
        state.npcs = vec![
            NpcRecord::new("1".into(), "Aldric".into(), "Merchant".into()),
        ];
        state.npcs[0].notes = Some("Sells potions".to_string());
        state.open_edit_modal();

        assert_eq!(state.modal, Some(NpcModal::Edit));
        assert_eq!(state.editing_id.as_deref(), Some("1"));
        assert_eq!(state.form_name.text(), "Aldric");
        assert_eq!(state.form_role.text(), "Merchant");
        assert_eq!(state.form_notes.text(), "Sells potions");
    }

    #[test]
    fn test_npc_view_detail_toggle() {
        let mut state = NpcViewState::new();
        assert!(!state.show_detail);
        state.show_detail = !state.show_detail;
        assert!(state.show_detail);
        state.show_detail = !state.show_detail;
        assert!(!state.show_detail);
    }

    #[test]
    fn test_truncate_helper() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hell…");
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn test_format_datetime_valid() {
        let dt = format_datetime("2026-02-24T12:00:00+00:00");
        assert_eq!(dt, "2026-02-24 12:00");
    }

    #[test]
    fn test_format_datetime_invalid() {
        let dt = format_datetime("not-a-date");
        assert_eq!(dt, "not-a-date");
    }

    #[test]
    fn test_filter_matching() {
        let mut state = NpcViewState::new();
        state.npcs = vec![
            NpcRecord::new("1".into(), "Aldric the Bold".into(), "Merchant".into()),
            NpcRecord::new("2".into(), "Brynn Silverleaf".into(), "Guard".into()),
            NpcRecord::new("3".into(), "Cora Nightshade".into(), "Healer".into()),
            NpcRecord::new("4".into(), "Aldric Junior".into(), "Squire".into()),
        ];

        // Empty filter matches all
        assert_eq!(state.filtered_indices(), vec![0, 1, 2, 3]);

        // Type "aldric" into filter — case-insensitive match
        for c in "aldric".chars() {
            state.filter.insert_char(c);
        }
        assert_eq!(state.filtered_indices(), vec![0, 3]);

        // "brynn" — single match
        state.filter.clear();
        for c in "brynn".chars() {
            state.filter.insert_char(c);
        }
        assert_eq!(state.filtered_indices(), vec![1]);

        // "zzz" — no match
        state.filter.clear();
        for c in "zzz".chars() {
            state.filter.insert_char(c);
        }
        assert!(state.filtered_indices().is_empty());
    }

    #[test]
    fn test_name_generation_produces_non_empty() {
        let mut state = NpcViewState::new();
        state.modal = Some(NpcModal::Create);
        state.form_focus = 0; // Name field

        state.generate_name_into_active_field();

        let name_text = state.form_name.text().to_string();
        assert!(!name_text.is_empty(), "generated name must not be empty");
        assert!(name_text.contains(' '), "FullName should have first and last name: {name_text}");
    }

    #[test]
    fn test_filter_clear() {
        let mut state = NpcViewState::new();
        state.npcs = vec![
            NpcRecord::new("1".into(), "Aldric".into(), "Merchant".into()),
            NpcRecord::new("2".into(), "Brynn".into(), "Guard".into()),
        ];

        // Activate filter and type something
        state.filter_active = true;
        for c in "aldric".chars() {
            state.filter.insert_char(c);
        }
        assert_eq!(state.filtered_indices().len(), 1);

        // Clear the filter
        state.filter.clear();
        state.filter_active = false;

        // All NPCs should be visible again
        assert_eq!(state.filtered_indices(), vec![0, 1]);
        assert!(!state.filter_active);
        assert!(state.filter.text().is_empty());
    }

    #[test]
    fn test_capitalize_helper() {
        assert_eq!(capitalize("strength"), "Strength");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("a"), "A");
        assert_eq!(capitalize("ABC"), "ABC");
    }
}
