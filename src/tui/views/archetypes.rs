//! Archetype browser — read-only tree view of archetype registry.
//!
//! Displays archetype categories, individual archetypes, and their
//! personality affinities, NPC role mappings, and naming cultures.
//! Data loads asynchronously from `InMemoryArchetypeRegistry` via Services.

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
use crate::core::archetype::types::ArchetypeCategory;
use crate::tui::services::Services;

// ── Snapshot types ──────────────────────────────────────────────────────────

/// Lightweight snapshot of an archetype for display (avoids holding Arc locks).
#[derive(Clone, Debug)]
struct ArchetypeSnapshot {
    id: String,
    display_name: String,
    description: Option<String>,
    category: ArchetypeCategory,
    vocabulary_bank_id: Option<String>,
    personality_affinities: Vec<(String, f32, u8)>, // (trait_id, weight, intensity)
    npc_role_mappings: Vec<(String, f32, Option<String>)>, // (role, weight, context)
    naming_cultures: Vec<(String, f32)>,             // (culture, weight)
}

/// Loaded registry data.
#[derive(Clone, Debug)]
struct RegistryData {
    categories: Vec<CategorySnapshot>,
}

#[derive(Clone, Debug)]
struct CategorySnapshot {
    category: ArchetypeCategory,
    label: String,
    icon: &'static str,
    archetypes: Vec<ArchetypeSnapshot>,
}

fn category_icon(cat: &ArchetypeCategory) -> &'static str {
    match cat {
        ArchetypeCategory::Role => "◎",
        ArchetypeCategory::Race => "◆",
        ArchetypeCategory::Class => "⚔",
        ArchetypeCategory::Setting => "◈",
        ArchetypeCategory::Custom(_) => "◇",
    }
}

fn category_label(cat: &ArchetypeCategory) -> String {
    match cat {
        ArchetypeCategory::Role => "Roles".to_string(),
        ArchetypeCategory::Race => "Races".to_string(),
        ArchetypeCategory::Class => "Classes".to_string(),
        ArchetypeCategory::Setting => "Settings".to_string(),
        ArchetypeCategory::Custom(label) => format!("Custom: {label}"),
    }
}

// ── State ──────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq)]
enum Panel {
    Categories,
    Archetypes,
}

pub struct ArchetypeViewState {
    data: Option<RegistryData>,
    loading: bool,
    selected_category: usize,
    selected_archetype: usize,
    focus_panel: Panel,
    scroll: usize,
    data_rx: mpsc::UnboundedReceiver<RegistryData>,
    data_tx: mpsc::UnboundedSender<RegistryData>,
}

impl ArchetypeViewState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            data: None,
            loading: false,
            selected_category: 0,
            selected_archetype: 0,
            focus_panel: Panel::Categories,
            scroll: 0,
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
            let all = registry.list_full(None).await;

            // Group by category, ordered: Role, Race, Class, Setting, Custom
            let category_order = [
                ArchetypeCategory::Role,
                ArchetypeCategory::Race,
                ArchetypeCategory::Class,
                ArchetypeCategory::Setting,
            ];

            let mut categories: Vec<CategorySnapshot> = Vec::new();

            for cat in &category_order {
                let mut archetypes: Vec<ArchetypeSnapshot> = all
                    .iter()
                    .filter(|a| &a.category == cat)
                    .map(|a| ArchetypeSnapshot {
                        id: a.id.to_string(),
                        display_name: a.display_name.to_string(),
                        description: a.description.clone(),
                        category: a.category.clone(),
                        vocabulary_bank_id: a.vocabulary_bank_id.clone(),
                        personality_affinities: a
                            .personality_affinity
                            .iter()
                            .map(|pa| (pa.trait_id.clone(), pa.weight, pa.default_intensity))
                            .collect(),
                        npc_role_mappings: a
                            .npc_role_mapping
                            .iter()
                            .map(|rm| (rm.role.clone(), rm.weight, rm.context.clone()))
                            .collect(),
                        naming_cultures: a
                            .naming_cultures
                            .iter()
                            .map(|nc| (nc.culture.clone(), nc.weight))
                            .collect(),
                    })
                    .collect();

                archetypes.sort_by(|a, b| a.display_name.cmp(&b.display_name));

                if !archetypes.is_empty() {
                    categories.push(CategorySnapshot {
                        label: category_label(cat),
                        icon: category_icon(cat),
                        category: cat.clone(),
                        archetypes,
                    });
                }
            }

            // Collect any custom categories
            let mut custom_cats: Vec<String> = all
                .iter()
                .filter_map(|a| {
                    if let ArchetypeCategory::Custom(label) = &a.category {
                        Some(label.clone())
                    } else {
                        None
                    }
                })
                .collect();
            custom_cats.sort();
            custom_cats.dedup();

            for label in custom_cats {
                let cat = ArchetypeCategory::Custom(label.clone());
                let mut archetypes: Vec<ArchetypeSnapshot> = all
                    .iter()
                    .filter(|a| a.category == cat)
                    .map(|a| ArchetypeSnapshot {
                        id: a.id.to_string(),
                        display_name: a.display_name.to_string(),
                        description: a.description.clone(),
                        category: a.category.clone(),
                        vocabulary_bank_id: a.vocabulary_bank_id.clone(),
                        personality_affinities: a
                            .personality_affinity
                            .iter()
                            .map(|pa| (pa.trait_id.clone(), pa.weight, pa.default_intensity))
                            .collect(),
                        npc_role_mappings: a
                            .npc_role_mapping
                            .iter()
                            .map(|rm| (rm.role.clone(), rm.weight, rm.context.clone()))
                            .collect(),
                        naming_cultures: a
                            .naming_cultures
                            .iter()
                            .map(|nc| (nc.culture.clone(), nc.weight))
                            .collect(),
                    })
                    .collect();
                archetypes.sort_by(|a, b| a.display_name.cmp(&b.display_name));
                if !archetypes.is_empty() {
                    categories.push(CategorySnapshot {
                        label: category_label(&cat),
                        icon: category_icon(&cat),
                        category: cat,
                        archetypes,
                    });
                }
            }

            let _ = tx.send(RegistryData { categories });
        });
    }

    pub fn poll(&mut self) {
        if let Ok(data) = self.data_rx.try_recv() {
            self.data = Some(data);
            self.loading = false;
            // Clamp selections
            self.selected_category = 0;
            self.selected_archetype = 0;
        }
    }

    fn current_category(&self) -> Option<&CategorySnapshot> {
        self.data
            .as_ref()
            .and_then(|d| d.categories.get(self.selected_category))
    }

    fn current_archetype(&self) -> Option<&ArchetypeSnapshot> {
        self.current_category()
            .and_then(|cat| cat.archetypes.get(self.selected_archetype))
    }

    fn category_count(&self) -> usize {
        self.data
            .as_ref()
            .map(|d| d.categories.len())
            .unwrap_or(0)
    }

    fn archetype_count(&self) -> usize {
        self.current_category()
            .map(|c| c.archetypes.len())
            .unwrap_or(0)
    }

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
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.focus_panel = match self.focus_panel {
                    Panel::Categories => Panel::Archetypes,
                    Panel::Archetypes => Panel::Categories,
                };
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('l') | KeyCode::Enter) => {
                if self.focus_panel == Panel::Categories {
                    self.focus_panel = Panel::Archetypes;
                    self.selected_archetype = 0;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('h') | KeyCode::Esc) => {
                if self.focus_panel == Panel::Archetypes {
                    self.focus_panel = Panel::Categories;
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                match self.focus_panel {
                    Panel::Categories => {
                        let max = self.category_count();
                        if max > 0 && self.selected_category + 1 < max {
                            self.selected_category += 1;
                            self.selected_archetype = 0;
                            self.scroll = 0;
                        }
                    }
                    Panel::Archetypes => {
                        let max = self.archetype_count();
                        if max > 0 && self.selected_archetype + 1 < max {
                            self.selected_archetype += 1;
                            self.scroll = 0;
                        }
                    }
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                match self.focus_panel {
                    Panel::Categories => {
                        self.selected_category = self.selected_category.saturating_sub(1);
                        self.selected_archetype = 0;
                        self.scroll = 0;
                    }
                    Panel::Archetypes => {
                        self.selected_archetype = self.selected_archetype.saturating_sub(1);
                        self.scroll = 0;
                    }
                }
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(30),
                Constraint::Percentage(45),
            ])
            .split(area);

        self.render_categories(frame, chunks[0]);
        self.render_archetypes(frame, chunks[1]);
        self.render_detail(frame, chunks[2]);
    }

    fn render_categories(&self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus_panel == Panel::Categories;
        let block = if is_focused {
            theme::block_focused("Categories")
        } else {
            theme::block_default("Categories")
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading && self.data.is_none() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Loading...",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        }

        let Some(ref data) = self.data else {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Press 'r' to load archetypes",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        };

        let mut lines: Vec<Line<'static>> = Vec::new();
        for (i, cat) in data.categories.iter().enumerate() {
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

            lines.push(Line::from(vec![
                Span::styled(marker.to_string(), style),
                Span::styled(format!("{} ", cat.icon), style),
                Span::styled(cat.label.clone(), style),
                Span::styled(
                    format!(" ({})", cat.archetypes.len()),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_archetypes(&self, frame: &mut Frame, area: Rect) {
        let is_focused = self.focus_panel == Panel::Archetypes;
        let cat = self.current_category();
        let title = cat.map(|c| c.label.as_str()).unwrap_or("Archetypes");
        let block = if is_focused {
            theme::block_focused(title)
        } else {
            theme::block_default(title)
        };
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(cat) = cat else {
            return;
        };

        let mut lines: Vec<Line<'static>> = Vec::new();
        for (i, arch) in cat.archetypes.iter().enumerate() {
            let is_selected = i == self.selected_archetype;
            let marker = if is_selected && is_focused {
                "\u{25b8} "
            } else {
                "  "
            };

            let style = if is_selected {
                Style::default()
                    .fg(theme::PRIMARY_LIGHT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            lines.push(Line::from(Span::styled(
                format!("{marker}{}", arch.display_name),
                style,
            )));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_detail(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_default("Detail");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(arch) = self.current_archetype() else {
            let cat = self.current_category();
            let hint = if cat.is_some() {
                "Select an archetype to view details."
            } else if self.data.is_some() {
                "No archetypes loaded."
            } else {
                "Press 'r' to load archetypes."
            };
            frame.render_widget(
                Paragraph::new(vec![
                    Line::raw(""),
                    Line::from(Span::styled(
                        format!("  {hint}"),
                        Style::default().fg(theme::TEXT_MUTED),
                    )),
                ]),
                inner,
            );
            return;
        };

        let cat = self.current_category().unwrap();
        let mut lines: Vec<Line<'static>> = Vec::new();

        // Header
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            format!("  {} {}", cat.icon, arch.display_name),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            format!("  Category: {}  |  ID: {}", cat.label, arch.id),
            Style::default().fg(theme::TEXT_MUTED),
        )));

        // Description
        if let Some(ref desc) = arch.description {
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                format!("  {desc}"),
                Style::default().fg(theme::TEXT),
            )));
        }

        // Vocabulary bank
        if let Some(ref bank) = arch.vocabulary_bank_id {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "Vocabulary: ",
                    Style::default().fg(theme::TEXT_MUTED),
                ),
                Span::styled(
                    bank.clone(),
                    Style::default().fg(theme::PRIMARY_LIGHT),
                ),
            ]));
        }

        // Personality affinities
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  PERSONALITY AFFINITIES",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        if arch.personality_affinities.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (none defined)",
                Style::default().fg(theme::TEXT_DIM),
            )));
        } else {
            for (trait_id, weight, intensity) in &arch.personality_affinities {
                let bar_len = (*weight * 10.0) as usize;
                let bar = format!(
                    "{}{}",
                    "\u{2588}".repeat(bar_len),
                    "\u{2591}".repeat(10 - bar_len)
                );
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<16}", trait_id),
                        Style::default().fg(theme::TEXT),
                    ),
                    Span::styled(bar, Style::default().fg(theme::PRIMARY_LIGHT)),
                    Span::styled(
                        format!(" {:.0}%  i:{}", weight * 100.0, intensity),
                        Style::default().fg(theme::TEXT_DIM),
                    ),
                ]));
            }
        }

        // NPC role mappings
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  NPC ROLE MAPPINGS",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        if arch.npc_role_mappings.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (none defined)",
                Style::default().fg(theme::TEXT_DIM),
            )));
        } else {
            for (role, weight, context) in &arch.npc_role_mappings {
                let ctx = context
                    .as_deref()
                    .map(|c| format!(" \u{2014} {c}"))
                    .unwrap_or_default();
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<16}", role),
                        Style::default().fg(theme::TEXT),
                    ),
                    Span::styled(
                        format!("{:.0}%", weight * 100.0),
                        Style::default().fg(theme::PRIMARY_LIGHT),
                    ),
                    Span::styled(ctx, Style::default().fg(theme::TEXT_DIM)),
                ]));
            }
        }

        // Naming cultures
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  NAMING CULTURES",
            Style::default()
                .fg(theme::PRIMARY)
                .add_modifier(Modifier::BOLD),
        )));
        if arch.naming_cultures.is_empty() {
            lines.push(Line::from(Span::styled(
                "  (none defined)",
                Style::default().fg(theme::TEXT_DIM),
            )));
        } else {
            for (culture, weight) in &arch.naming_cultures {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<16}", culture),
                        Style::default().fg(theme::TEXT),
                    ),
                    Span::styled(
                        format!("{:.0}%", weight * 100.0),
                        Style::default().fg(theme::PRIMARY_LIGHT),
                    ),
                ]));
            }
        }

        // Keybindings
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [Tab] panel  [h/l] navigate  [j/k] select  [r] refresh",
            Style::default().fg(theme::TEXT_DIM),
        )));

        // Apply scroll
        let visible: Vec<Line<'static>> = lines.into_iter().skip(self.scroll).collect();
        frame.render_widget(Paragraph::new(visible), inner);
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = ArchetypeViewState::new();
        assert_eq!(state.selected_category, 0);
        assert_eq!(state.selected_archetype, 0);
        assert_eq!(state.focus_panel, Panel::Categories);
        assert!(state.data.is_none());
    }

    #[test]
    fn test_category_icons_and_labels() {
        assert_eq!(category_icon(&ArchetypeCategory::Role), "\u{25ce}");
        assert_eq!(category_icon(&ArchetypeCategory::Race), "\u{25c6}");
        assert_eq!(category_icon(&ArchetypeCategory::Class), "\u{2694}");
        assert_eq!(category_label(&ArchetypeCategory::Role), "Roles");
        assert_eq!(category_label(&ArchetypeCategory::Race), "Races");
        assert_eq!(category_label(&ArchetypeCategory::Class), "Classes");
        assert_eq!(category_label(&ArchetypeCategory::Setting), "Settings");
        assert_eq!(
            category_label(&ArchetypeCategory::Custom("faction".into())),
            "Custom: faction"
        );
    }

    #[test]
    fn test_panel_toggle() {
        let mut state = ArchetypeViewState::new();
        assert_eq!(state.focus_panel, Panel::Categories);
        state.focus_panel = Panel::Archetypes;
        assert_eq!(state.focus_panel, Panel::Archetypes);
    }

    #[test]
    fn test_empty_data_accessors() {
        let state = ArchetypeViewState::new();
        assert_eq!(state.category_count(), 0);
        assert_eq!(state.archetype_count(), 0);
        assert!(state.current_category().is_none());
        assert!(state.current_archetype().is_none());
    }

    #[test]
    fn test_snapshot_with_data() {
        let mut state = ArchetypeViewState::new();
        state.data = Some(RegistryData {
            categories: vec![CategorySnapshot {
                category: ArchetypeCategory::Role,
                label: "Roles".into(),
                icon: "\u{25ce}",
                archetypes: vec![
                    ArchetypeSnapshot {
                        id: "merchant".into(),
                        display_name: "Merchant".into(),
                        description: Some("A trader of goods".into()),
                        category: ArchetypeCategory::Role,
                        vocabulary_bank_id: Some("mercantile".into()),
                        personality_affinities: vec![("shrewd".into(), 0.8, 7)],
                        npc_role_mappings: vec![("merchant".into(), 0.9, None)],
                        naming_cultures: vec![("common".into(), 1.0)],
                    },
                ],
            }],
        });
        assert_eq!(state.category_count(), 1);
        assert_eq!(state.archetype_count(), 1);
        assert_eq!(
            state.current_archetype().unwrap().display_name,
            "Merchant"
        );
    }
}
