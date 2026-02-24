//! World-building asset browser — read-only three-panel view for
//! archetypes, setting packs, and vocabulary banks.
//!
//! Displays YAML-based archetype, setting pack, and vocabulary data
//! with category tree navigation (left), item list (middle), and a
//! detail pane (right). Data loads from `InMemoryArchetypeRegistry`
//! and `AssetLoader` via Services.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use tokio::sync::mpsc;

use super::super::theme;
use crate::core::assets::AssetLoader;
use crate::tui::services::Services;

// ── Asset category ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssetCategory {
    Archetypes,
    SettingPacks,
    Vocabulary,
}

impl AssetCategory {
    const ALL: &'static [AssetCategory] = &[
        AssetCategory::Archetypes,
        AssetCategory::SettingPacks,
        AssetCategory::Vocabulary,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Archetypes => "Archetypes",
            Self::SettingPacks => "Setting Packs",
            Self::Vocabulary => "Vocabulary",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Archetypes => "\u{25ce}",
            Self::SettingPacks => "\u{25c8}",
            Self::Vocabulary => "\u{25c7}",
        }
    }
}

// ── Panel focus ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssetPanel {
    Categories,
    Items,
    Detail,
}

impl AssetPanel {
    fn next(self) -> Self {
        match self {
            Self::Categories => Self::Items,
            Self::Items => Self::Detail,
            Self::Detail => Self::Categories,
        }
    }
}

// ── Asset item ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct AssetItem {
    name: String,
    item_type: String,
    description: String,
    details: Vec<(String, String)>,
}

// ── Loaded data ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct AssetData {
    archetype_items: Vec<AssetItem>,
    setting_items: Vec<AssetItem>,
    vocabulary_items: Vec<AssetItem>,
}

fn build_items_from_registry(data: &AssetData, category: AssetCategory) -> &[AssetItem] {
    match category {
        AssetCategory::Archetypes => &data.archetype_items,
        AssetCategory::SettingPacks => &data.setting_items,
        AssetCategory::Vocabulary => &data.vocabulary_items,
    }
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct AssetBrowserState {
    category: AssetCategory,
    data: Option<AssetData>,
    loading: bool,
    selected_category: usize,
    selected_item: usize,
    focus_panel: AssetPanel,
    detail_scroll: usize,
    data_rx: mpsc::UnboundedReceiver<AssetData>,
    data_tx: mpsc::UnboundedSender<AssetData>,
}

impl AssetBrowserState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            category: AssetCategory::Archetypes,
            data: None,
            loading: false,
            selected_category: 0,
            selected_item: 0,
            focus_panel: AssetPanel::Categories,
            detail_scroll: 0,
            data_rx,
            data_tx,
        }
    }

    pub fn load(&mut self, services: &Services) {
        if self.loading {
            return;
        }
        self.loading = true;
        let tx = self.data_tx.clone();
        let registry = services.archetype_registry.clone();

        tokio::spawn(async move {
            // Archetypes from registry (full detail)
            let all_archetypes = registry.list_full(None).await;
            let archetype_items: Vec<AssetItem> = all_archetypes
                .iter()
                .map(|a| {
                    let mut details = Vec::new();
                    details.push(("Category".into(), format!("{}", a.category)));
                    details.push(("ID".into(), a.id.to_string()));

                    if !a.personality_affinity.is_empty() {
                        let traits: Vec<String> = a
                            .personality_affinity
                            .iter()
                            .map(|pa| format!("{} ({:.0}%)", pa.trait_id, pa.weight * 100.0))
                            .collect();
                        details.push(("Personality".into(), traits.join(", ")));
                    }

                    if !a.npc_role_mapping.is_empty() {
                        let roles: Vec<String> = a
                            .npc_role_mapping
                            .iter()
                            .map(|rm| format!("{} ({:.0}%)", rm.role, rm.weight * 100.0))
                            .collect();
                        details.push(("NPC Roles".into(), roles.join(", ")));
                    }

                    if !a.naming_cultures.is_empty() {
                        let cultures: Vec<String> = a
                            .naming_cultures
                            .iter()
                            .map(|nc| format!("{} ({:.0}%)", nc.culture, nc.weight * 100.0))
                            .collect();
                        details.push(("Naming Cultures".into(), cultures.join(", ")));
                    }

                    if let Some(ref bank) = a.vocabulary_bank_id {
                        details.push(("Vocabulary Bank".into(), bank.clone()));
                    }

                    AssetItem {
                        name: a.display_name.to_string(),
                        item_type: format!("{}", a.category),
                        description: a
                            .description
                            .clone()
                            .unwrap_or_else(|| format!("{} archetype", a.category)),
                        details,
                    }
                })
                .collect();

            // Setting packs from registry
            let all_packs = registry.list_setting_packs().await;
            let setting_items: Vec<AssetItem> = all_packs
                .iter()
                .map(|summary| {
                    let mut details = Vec::new();
                    details.push(("ID".into(), summary.id.clone()));
                    details.push(("Game System".into(), summary.game_system.clone()));
                    details.push(("Version".into(), summary.version.clone()));
                    if let Some(ref author) = summary.author {
                        details.push(("Author".into(), author.clone()));
                    }
                    if !summary.tags.is_empty() {
                        details.push(("Tags".into(), summary.tags.join(", ")));
                    }

                    AssetItem {
                        name: summary.name.clone(),
                        item_type: "setting_pack".into(),
                        description: format!(
                            "{} setting pack (v{})",
                            summary.game_system, summary.version
                        ),
                        details,
                    }
                })
                .collect();

            // Vocabulary banks from AssetLoader
            let vocab_banks = AssetLoader::load_vocabulary_banks();
            let vocabulary_items: Vec<AssetItem> = vocab_banks
                .iter()
                .map(|bank| {
                    let phrase_count = bank.phrase_count();
                    let mut details = Vec::new();
                    details.push(("Bank ID".into(), bank.id.clone()));
                    if let Some(ref culture) = bank.culture {
                        details.push(("Culture".into(), culture.clone()));
                    }
                    if let Some(ref role) = bank.role {
                        details.push(("Role".into(), role.clone()));
                    }
                    details.push(("Phrase Count".into(), format!("{phrase_count}")));

                    // Show phrase categories
                    let categories: Vec<String> = bank
                        .phrases
                        .keys()
                        .cloned()
                        .collect();
                    if !categories.is_empty() {
                        details.push((
                            "Categories".into(),
                            categories.join(", "),
                        ));
                    }

                    // Show sample phrases (up to 3 from first category)
                    if let Some(first_phrases) = bank.phrases.values().next() {
                        for (i, phrase) in first_phrases.iter().take(3).enumerate() {
                            details.push((
                                format!("Sample {}", i + 1),
                                format!("\"{}\"", phrase.text),
                            ));
                        }
                    }

                    AssetItem {
                        name: bank.display_name.clone(),
                        item_type: "vocabulary_bank".into(),
                        description: format!(
                            "{} vocabulary bank with {} phrases",
                            bank.culture.as_deref().unwrap_or("general"),
                            phrase_count
                        ),
                        details,
                    }
                })
                .collect();

            let _ = tx.send(AssetData {
                archetype_items,
                setting_items,
                vocabulary_items,
            });
        });
    }

    pub fn poll(&mut self) {
        if let Ok(data) = self.data_rx.try_recv() {
            self.data = Some(data);
            self.loading = false;
            self.selected_item = 0;
            self.detail_scroll = 0;
        }
    }

    fn items(&self) -> &[AssetItem] {
        match &self.data {
            Some(data) => build_items_from_registry(data, self.category),
            None => &[],
        }
    }

    fn current_item(&self) -> Option<&AssetItem> {
        self.items().get(self.selected_item)
    }

    fn item_count_for_category(&self, cat: AssetCategory) -> usize {
        match &self.data {
            Some(data) => build_items_from_registry(data, cat).len(),
            None => 0,
        }
    }

    // ── Input ───────────────────────────────────────────────────────────

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            ..
        }) = event
        else {
            return false;
        };

        match (*modifiers, *code) {
            // Panel switching
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.focus_panel = self.focus_panel.next();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('l') | KeyCode::Right) => {
                match self.focus_panel {
                    AssetPanel::Categories => {
                        self.focus_panel = AssetPanel::Items;
                        self.selected_item = 0;
                    }
                    AssetPanel::Items => {
                        self.focus_panel = AssetPanel::Detail;
                        self.detail_scroll = 0;
                    }
                    AssetPanel::Detail => {}
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('h') | KeyCode::Left) => {
                match self.focus_panel {
                    AssetPanel::Categories => {}
                    AssetPanel::Items => {
                        self.focus_panel = AssetPanel::Categories;
                    }
                    AssetPanel::Detail => {
                        self.focus_panel = AssetPanel::Items;
                    }
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                match self.focus_panel {
                    AssetPanel::Categories => {
                        self.focus_panel = AssetPanel::Items;
                        self.selected_item = 0;
                    }
                    AssetPanel::Items => {
                        self.focus_panel = AssetPanel::Detail;
                        self.detail_scroll = 0;
                    }
                    AssetPanel::Detail => {}
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Esc) => {
                match self.focus_panel {
                    AssetPanel::Categories => return false,
                    AssetPanel::Items => {
                        self.focus_panel = AssetPanel::Categories;
                    }
                    AssetPanel::Detail => {
                        self.focus_panel = AssetPanel::Items;
                    }
                }
                true
            }
            // Refresh
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }
            // Vertical navigation
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                match self.focus_panel {
                    AssetPanel::Categories => {
                        let max = AssetCategory::ALL.len().saturating_sub(1);
                        if self.selected_category < max {
                            self.selected_category += 1;
                            self.category = AssetCategory::ALL[self.selected_category];
                            self.selected_item = 0;
                            self.detail_scroll = 0;
                        }
                    }
                    AssetPanel::Items => {
                        let max = self.items().len().saturating_sub(1);
                        if self.selected_item < max {
                            self.selected_item += 1;
                            self.detail_scroll = 0;
                        }
                    }
                    AssetPanel::Detail => {
                        self.detail_scroll = self.detail_scroll.saturating_add(1);
                    }
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                match self.focus_panel {
                    AssetPanel::Categories => {
                        if self.selected_category > 0 {
                            self.selected_category -= 1;
                            self.category = AssetCategory::ALL[self.selected_category];
                            self.selected_item = 0;
                            self.detail_scroll = 0;
                        }
                    }
                    AssetPanel::Items => {
                        self.selected_item = self.selected_item.saturating_sub(1);
                        self.detail_scroll = 0;
                    }
                    AssetPanel::Detail => {
                        self.detail_scroll = self.detail_scroll.saturating_sub(1);
                    }
                }
                true
            }
            _ => false,
        }
    }

    // ── Rendering ───────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(35),
                Constraint::Percentage(40),
            ])
            .split(area);

        self.render_categories(frame, chunks[0]);
        self.render_items(frame, chunks[1]);
        self.render_detail(frame, chunks[2]);
    }

    fn render_categories(&self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus_panel == AssetPanel::Categories;
        let block = if is_focused {
            theme::block_focused("Asset Categories")
        } else {
            theme::block_default("Asset Categories")
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        for (i, cat) in AssetCategory::ALL.iter().enumerate() {
            let is_selected = i == self.selected_category;
            let marker = if is_selected && is_focused {
                "\u{25b8} "
            } else {
                "  "
            };

            let style = if is_selected {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            let item_count = self.item_count_for_category(*cat);

            lines.push(Line::from(vec![
                Span::styled(marker.to_string(), style),
                Span::styled(format!("{} ", cat.icon()), style),
                Span::styled(cat.label().to_string(), style),
                Span::styled(
                    format!(" ({})", item_count),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!(
                "  {}",
                "\u{2500}".repeat(inner.width.saturating_sub(4) as usize)
            ),
            Style::default().fg(theme::TEXT_DIM),
        )));
        lines.push(Line::raw(""));

        if self.data.is_some() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "YAML-based assets",
                    Style::default().fg(theme::TEXT_MUTED),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "loaded from registry",
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        } else if self.loading {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Loading...", Style::default().fg(theme::TEXT_MUTED)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "Press 'r' to load assets",
                    Style::default().fg(theme::TEXT_MUTED),
                ),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_items(&self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus_panel == AssetPanel::Items;
        let block = if is_focused {
            theme::block_focused(self.category.label())
        } else {
            theme::block_default(self.category.label())
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let items = self.items();
        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        if items.is_empty() {
            let msg = if self.data.is_some() {
                "No items in this category."
            } else {
                "Press 'r' to load."
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(msg, Style::default().fg(theme::TEXT_MUTED)),
            ]));
        } else {
            for (i, item) in items.iter().enumerate() {
                let is_selected = i == self.selected_item;
                let marker = if is_selected && is_focused {
                    "\u{25b8} "
                } else {
                    "  "
                };

                let name_style = if is_selected {
                    Style::default()
                        .fg(theme::PRIMARY_LIGHT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                };

                lines.push(Line::from(vec![
                    Span::styled(marker.to_string(), name_style),
                    Span::styled(item.name.clone(), name_style),
                ]));

                // Show type label below
                let type_label = match item.item_type.as_str() {
                    "role" => "role archetype",
                    "race" => "race archetype",
                    "class" => "class archetype",
                    "setting" => "setting archetype",
                    "setting_pack" => "setting pack",
                    "vocabulary_bank" => "vocabulary bank",
                    other => other,
                };

                let desc_style = if is_selected {
                    Style::default().fg(theme::TEXT_MUTED)
                } else {
                    Style::default().fg(theme::TEXT_DIM)
                };

                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(type_label.to_string(), desc_style),
                ]));

                if i < items.len() - 1 {
                    lines.push(Line::raw(""));
                }
            }
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        let block = if self.focus_panel == AssetPanel::Detail {
            theme::block_focused("Detail")
        } else {
            theme::block_default("Detail")
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = match self.current_item() {
            Some(item) => self.build_detail_lines(item, inner.width),
            None => vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Select an item to view details.",
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                ]),
            ],
        };

        let content = Paragraph::new(lines).scroll((self.detail_scroll as u16, 0));
        frame.render_widget(content, inner);
    }

    fn build_detail_lines(&self, item: &AssetItem, width: u16) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();
        let sep_width = width.saturating_sub(4) as usize;

        // Header
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{} {}", self.category.icon(), item.name),
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Type
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("Type: {}", item.item_type),
                Style::default().fg(theme::TEXT_MUTED),
            ),
        ]));

        lines.push(Line::raw(""));

        // Description
        lines.push(Line::from(Span::styled(
            "  DESCRIPTION",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        for wrapped in wrap_text(&item.description, sep_width) {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(wrapped, Style::default().fg(theme::TEXT)),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {}", "\u{2500}".repeat(sep_width)),
            Style::default().fg(theme::TEXT_DIM),
        )));
        lines.push(Line::raw(""));

        // Key-value details
        for (key, value) in &item.details {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    key.clone(),
                    Style::default()
                        .fg(theme::PRIMARY)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));

            for wrapped in wrap_text(value, sep_width) {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(wrapped, Style::default().fg(theme::TEXT)),
                ]));
            }

            lines.push(Line::raw(""));
        }

        // Footer
        lines.push(Line::from(Span::styled(
            format!("  {}", "\u{2500}".repeat(sep_width)),
            Style::default().fg(theme::TEXT_DIM),
        )));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab", Style::default().fg(theme::TEXT_DIM)),
            Span::raw(":panel  "),
            Span::styled("h/l", Style::default().fg(theme::TEXT_DIM)),
            Span::raw(":navigate  "),
            Span::styled("j/k", Style::default().fg(theme::TEXT_DIM)),
            Span::raw(":select  "),
            Span::styled("r", Style::default().fg(theme::TEXT_DIM)),
            Span::raw(":refresh"),
        ]));

        lines
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Simple word-wrap: splits text into lines that fit within `max_width` chars.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() > max_width {
            lines.push(current);
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_defaults() {
        let state = AssetBrowserState::new();
        assert_eq!(state.category, AssetCategory::Archetypes);
        assert_eq!(state.selected_category, 0);
        assert_eq!(state.selected_item, 0);
        assert_eq!(state.focus_panel, AssetPanel::Categories);
        assert_eq!(state.detail_scroll, 0);
        assert!(state.data.is_none());
    }

    #[test]
    fn test_panel_cycling() {
        assert_eq!(AssetPanel::Categories.next(), AssetPanel::Items);
        assert_eq!(AssetPanel::Items.next(), AssetPanel::Detail);
        assert_eq!(AssetPanel::Detail.next(), AssetPanel::Categories);
    }

    #[test]
    fn test_category_labels_and_icons() {
        for cat in AssetCategory::ALL {
            assert!(!cat.label().is_empty());
            assert!(!cat.icon().is_empty());
        }
    }

    #[test]
    fn test_wrap_text_short() {
        let lines = wrap_text("hello world", 80);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn test_wrap_text_long() {
        let text = "This is a somewhat longer description that should wrap across multiple lines when the width is small.";
        let lines = wrap_text(text, 30);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.len() <= 30 + 20, "Line too long: {}", line);
        }
    }

    #[test]
    fn test_wrap_text_empty() {
        let lines = wrap_text("", 40);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_detail_scroll_no_underflow() {
        let mut state = AssetBrowserState::new();
        state.focus_panel = AssetPanel::Detail;
        state.detail_scroll = 0;
        state.detail_scroll = state.detail_scroll.saturating_sub(1);
        assert_eq!(state.detail_scroll, 0);
    }

    #[test]
    fn test_empty_items_accessor() {
        let state = AssetBrowserState::new();
        assert!(state.items().is_empty());
        assert!(state.current_item().is_none());
    }

    #[test]
    fn test_item_count_without_data() {
        let state = AssetBrowserState::new();
        assert_eq!(
            state.item_count_for_category(AssetCategory::Archetypes),
            0
        );
    }
}
