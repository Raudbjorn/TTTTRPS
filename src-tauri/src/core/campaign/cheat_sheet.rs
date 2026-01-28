//! Cheat Sheet Builder
//!
//! Phase 9 of the Campaign Generation Overhaul.
//!
//! Provides cheat sheet generation for game sessions, aggregating relevant
//! content from session plans with priority-based truncation and print-friendly
//! HTML rendering.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                       CheatSheetBuilder                          │
//! │  ┌───────────────┐  ┌──────────────────┐  ┌──────────────────┐  │
//! │  │ Content       │  │ Priority-Based   │  │ HTML             │  │
//! │  │ Aggregator    │  │ Truncation       │  │ Exporter         │  │
//! │  └───────────────┘  └──────────────────┘  └──────────────────┘  │
//! └──────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                        Data Sources                              │
//! │  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐ │
//! │  │ Session    │  │ NPCs       │  │ Locations  │  │ Plot       │ │
//! │  │ Plan       │  │            │  │            │  │ Points     │ │
//! │  └────────────┘  └────────────┘  └────────────┘  └────────────┘ │
//! └──────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

use crate::database::{
    CampaignOps, Database, DisclosureLevel, IncludeStatus, CheatSheetPreferenceRecord,
    CardEntityType, LocationOps, NpcOps, QuickReferenceOps,
};
use super::quick_reference::{QuickReferenceCardManager, RenderedCard};

/// Maximum character count for a cheat sheet section
pub const DEFAULT_MAX_SECTION_CHARS: usize = 5000;

/// Maximum total character count for a cheat sheet
pub const DEFAULT_MAX_TOTAL_CHARS: usize = 25000;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during cheat sheet operations
#[derive(Debug, Clone, Error)]
pub enum CheatSheetError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Campaign not found: {campaign_id}")]
    CampaignNotFound { campaign_id: String },

    #[error("No content available for cheat sheet")]
    NoContent,

    #[error("Export error: {0}")]
    Export(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Quick reference error: {0}")]
    QuickReference(String),
}

impl From<sqlx::Error> for CheatSheetError {
    fn from(err: sqlx::Error) -> Self {
        CheatSheetError::Database(err.to_string())
    }
}

impl From<super::quick_reference::QuickReferenceError> for CheatSheetError {
    fn from(err: super::quick_reference::QuickReferenceError) -> Self {
        CheatSheetError::QuickReference(err.to_string())
    }
}

// ============================================================================
// Cheat Sheet Types
// ============================================================================

/// Section type for cheat sheet organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionType {
    /// Key NPCs for the session
    KeyNpcs,
    /// Important locations
    Locations,
    /// Active plot points
    PlotPoints,
    /// Session objectives/goals
    Objectives,
    /// Combat/encounter notes
    Encounters,
    /// Scheduled scenes
    Scenes,
    /// Important rules/mechanics
    Rules,
    /// Player character reminders
    PartyReminders,
    /// Custom/miscellaneous
    Custom,
}

impl SectionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SectionType::KeyNpcs => "key_npcs",
            SectionType::Locations => "locations",
            SectionType::PlotPoints => "plot_points",
            SectionType::Objectives => "objectives",
            SectionType::Encounters => "encounters",
            SectionType::Scenes => "scenes",
            SectionType::Rules => "rules",
            SectionType::PartyReminders => "party_reminders",
            SectionType::Custom => "custom",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SectionType::KeyNpcs => "Key NPCs",
            SectionType::Locations => "Locations",
            SectionType::PlotPoints => "Plot Points",
            SectionType::Objectives => "Objectives",
            SectionType::Encounters => "Encounters",
            SectionType::Scenes => "Scenes",
            SectionType::Rules => "Rules Reference",
            SectionType::PartyReminders => "Party Reminders",
            SectionType::Custom => "Notes",
        }
    }

    /// Default priority for section type (0-100, higher = more important)
    pub fn default_priority(&self) -> i32 {
        match self {
            SectionType::Objectives => 90,
            SectionType::KeyNpcs => 85,
            SectionType::Locations => 80,
            SectionType::Scenes => 75,
            SectionType::PlotPoints => 70,
            SectionType::Encounters => 65,
            SectionType::PartyReminders => 60,
            SectionType::Rules => 50,
            SectionType::Custom => 40,
        }
    }
}

/// A single item in a cheat sheet section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheetItem {
    /// Unique identifier
    pub id: String,
    /// Item title/name
    pub title: String,
    /// Brief summary
    pub summary: String,
    /// Full content (may be truncated)
    pub content: String,
    /// Entity type if linked to an entity
    pub entity_type: Option<CardEntityType>,
    /// Entity ID if linked
    pub entity_id: Option<String>,
    /// Priority for truncation decisions (0-100)
    pub priority: i32,
    /// Whether this item was truncated
    pub was_truncated: bool,
    /// Original character count before truncation
    pub original_chars: usize,
}

/// A section of the cheat sheet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheetSection {
    /// Section type
    pub section_type: SectionType,
    /// Display title
    pub title: String,
    /// Items in this section
    pub items: Vec<CheatSheetItem>,
    /// Section priority (0-100)
    pub priority: i32,
    /// Whether this section was truncated
    pub was_truncated: bool,
    /// Number of items hidden due to truncation
    pub hidden_items: usize,
    /// Whether this section is collapsed by default
    pub collapsed: bool,
}

/// Truncation warning for the cheat sheet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruncationWarning {
    /// Section that was truncated
    pub section: SectionType,
    /// Number of characters removed
    pub chars_removed: usize,
    /// Number of items hidden
    pub items_hidden: usize,
    /// Reason for truncation
    pub reason: String,
}

/// Complete cheat sheet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheet {
    /// Campaign ID
    pub campaign_id: String,
    /// Session ID (if session-specific)
    pub session_id: Option<String>,
    /// Cheat sheet title
    pub title: String,
    /// Ordered sections
    pub sections: Vec<CheatSheetSection>,
    /// Total character count
    pub total_chars: usize,
    /// Maximum allowed characters
    pub max_chars: usize,
    /// Truncation warnings
    pub warnings: Vec<TruncationWarning>,
    /// When the cheat sheet was generated
    pub generated_at: String,
}

/// Options for cheat sheet generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheatSheetOptions {
    /// Maximum characters per section
    pub max_section_chars: usize,
    /// Maximum total characters
    pub max_total_chars: usize,
    /// Sections to include (empty = all)
    pub include_sections: Vec<SectionType>,
    /// Sections to exclude
    pub exclude_sections: Vec<SectionType>,
    /// Default disclosure level for entities
    pub default_disclosure: DisclosureLevel,
    /// Whether to include collapsed sections
    pub include_collapsed: bool,
    /// Custom section priorities (overrides defaults)
    pub section_priorities: HashMap<SectionType, i32>,
}

impl Default for CheatSheetOptions {
    fn default() -> Self {
        Self {
            max_section_chars: DEFAULT_MAX_SECTION_CHARS,
            max_total_chars: DEFAULT_MAX_TOTAL_CHARS,
            include_sections: vec![],
            exclude_sections: vec![],
            default_disclosure: DisclosureLevel::Summary,
            include_collapsed: true,
            section_priorities: HashMap::new(),
        }
    }
}

// ============================================================================
// CheatSheetBuilder
// ============================================================================

/// Builder for constructing cheat sheets from campaign data
pub struct CheatSheetBuilder<'a> {
    database: &'a Database,
    card_manager: QuickReferenceCardManager<'a>,
}

impl<'a> CheatSheetBuilder<'a> {
    /// Create a new CheatSheetBuilder
    pub fn new(database: &'a Database) -> Self {
        let card_manager = QuickReferenceCardManager::new(database);
        Self {
            database,
            card_manager,
        }
    }

    /// Build a cheat sheet for a session
    pub async fn build_for_session(
        &self,
        campaign_id: &str,
        session_id: &str,
        options: CheatSheetOptions,
    ) -> Result<CheatSheet, CheatSheetError> {
        info!(
            campaign_id = %campaign_id,
            session_id = %session_id,
            "Building cheat sheet for session"
        );

        // Load preferences
        let preferences = self.database
            .get_session_cheat_sheet_preferences(campaign_id, session_id)
            .await?;

        // Build sections based on available data
        let mut sections = Vec::new();

        // Key NPCs section
        if self.should_include_section(SectionType::KeyNpcs, &options, &preferences) {
            if let Ok(section) = self.build_npc_section(campaign_id, &options, &preferences).await {
                sections.push(section);
            }
        }

        // Locations section
        if self.should_include_section(SectionType::Locations, &options, &preferences) {
            if let Ok(section) = self.build_location_section(campaign_id, &options, &preferences).await {
                sections.push(section);
            }
        }

        // Objectives section (from session plan if available)
        if self.should_include_section(SectionType::Objectives, &options, &preferences) {
            if let Ok(section) = self.build_objectives_section(session_id, &options).await {
                sections.push(section);
            }
        }

        // Sort sections by priority
        sections.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Apply truncation
        let (sections, warnings) = self.apply_truncation(sections, &options);

        // Calculate total characters
        let total_chars: usize = sections.iter()
            .flat_map(|s| &s.items)
            .map(|i| i.content.len())
            .sum();

        let campaign = self.database.get_campaign(campaign_id).await?
            .ok_or_else(|| CheatSheetError::CampaignNotFound {
                campaign_id: campaign_id.to_string(),
            })?;

        Ok(CheatSheet {
            campaign_id: campaign_id.to_string(),
            session_id: Some(session_id.to_string()),
            title: format!("{} - Session Cheat Sheet", campaign.name),
            sections,
            total_chars,
            max_chars: options.max_total_chars,
            warnings,
            generated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Build a custom cheat sheet with specific entities
    pub async fn build_custom(
        &self,
        campaign_id: &str,
        title: String,
        entity_ids: Vec<(CardEntityType, String)>,
        options: CheatSheetOptions,
    ) -> Result<CheatSheet, CheatSheetError> {
        info!(
            campaign_id = %campaign_id,
            entity_count = entity_ids.len(),
            "Building custom cheat sheet"
        );

        // Group entities by type
        let mut npc_ids = Vec::new();
        let mut location_ids = Vec::new();
        let mut other_ids = Vec::new();

        for (entity_type, entity_id) in entity_ids {
            match entity_type {
                CardEntityType::Npc => npc_ids.push(entity_id),
                CardEntityType::Location => location_ids.push(entity_id),
                _ => other_ids.push((entity_type, entity_id)),
            }
        }

        let mut sections = Vec::new();

        // Build NPC section
        if !npc_ids.is_empty() {
            let items = self.build_entity_items(
                CardEntityType::Npc,
                &npc_ids,
                options.default_disclosure,
            ).await;
            sections.push(CheatSheetSection {
                section_type: SectionType::KeyNpcs,
                title: SectionType::KeyNpcs.display_name().to_string(),
                items,
                priority: SectionType::KeyNpcs.default_priority(),
                was_truncated: false,
                hidden_items: 0,
                collapsed: false,
            });
        }

        // Build location section
        if !location_ids.is_empty() {
            let items = self.build_entity_items(
                CardEntityType::Location,
                &location_ids,
                options.default_disclosure,
            ).await;
            sections.push(CheatSheetSection {
                section_type: SectionType::Locations,
                title: SectionType::Locations.display_name().to_string(),
                items,
                priority: SectionType::Locations.default_priority(),
                was_truncated: false,
                hidden_items: 0,
                collapsed: false,
            });
        }

        // Build custom section for other entities
        if !other_ids.is_empty() {
            let mut items = Vec::new();
            for (entity_type, entity_id) in other_ids {
                if let Ok(card) = self.card_manager.render_entity_card(
                    entity_type,
                    &entity_id,
                    options.default_disclosure,
                    None,
                ).await {
                    items.push(self.card_to_item(card, 50));
                }
            }
            if !items.is_empty() {
                sections.push(CheatSheetSection {
                    section_type: SectionType::Custom,
                    title: "Other".to_string(),
                    items,
                    priority: SectionType::Custom.default_priority(),
                    was_truncated: false,
                    hidden_items: 0,
                    collapsed: false,
                });
            }
        }

        // Apply truncation
        let (sections, warnings) = self.apply_truncation(sections, &options);

        let total_chars: usize = sections.iter()
            .flat_map(|s| &s.items)
            .map(|i| i.content.len())
            .sum();

        Ok(CheatSheet {
            campaign_id: campaign_id.to_string(),
            session_id: None,
            title,
            sections,
            total_chars,
            max_chars: options.max_total_chars,
            warnings,
            generated_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    // =========================================================================
    // Section Builders
    // =========================================================================

    async fn build_npc_section(
        &self,
        campaign_id: &str,
        options: &CheatSheetOptions,
        preferences: &[CheatSheetPreferenceRecord],
    ) -> Result<CheatSheetSection, CheatSheetError> {
        let npcs = self.database.list_npcs(Some(campaign_id)).await?;

        let mut items = Vec::new();
        for npc in npcs {
            // Check if NPC should be included based on preferences
            let include_status = self.get_entity_include_status(
                CardEntityType::Npc,
                &npc.id,
                preferences,
            );

            if include_status == IncludeStatus::Never {
                continue;
            }

            let disclosure = self.get_entity_disclosure_level(
                CardEntityType::Npc,
                &npc.id,
                preferences,
                options.default_disclosure,
            );

            if let Ok(card) = self.card_manager.render_entity_card(
                CardEntityType::Npc,
                &npc.id,
                disclosure,
                None,
            ).await {
                let priority = self.get_entity_priority(
                    CardEntityType::Npc,
                    &npc.id,
                    preferences,
                );
                items.push(self.card_to_item(card, priority));
            }
        }

        // Sort by priority
        items.sort_by(|a, b| b.priority.cmp(&a.priority));

        let section_priority = options.section_priorities
            .get(&SectionType::KeyNpcs)
            .copied()
            .unwrap_or_else(|| SectionType::KeyNpcs.default_priority());

        Ok(CheatSheetSection {
            section_type: SectionType::KeyNpcs,
            title: SectionType::KeyNpcs.display_name().to_string(),
            items,
            priority: section_priority,
            was_truncated: false,
            hidden_items: 0,
            collapsed: false,
        })
    }

    async fn build_location_section(
        &self,
        campaign_id: &str,
        options: &CheatSheetOptions,
        preferences: &[CheatSheetPreferenceRecord],
    ) -> Result<CheatSheetSection, CheatSheetError> {
        let locations = self.database.list_locations(campaign_id).await?;

        let mut items = Vec::new();
        for location in locations {
            let include_status = self.get_entity_include_status(
                CardEntityType::Location,
                &location.id,
                preferences,
            );

            if include_status == IncludeStatus::Never {
                continue;
            }

            let disclosure = self.get_entity_disclosure_level(
                CardEntityType::Location,
                &location.id,
                preferences,
                options.default_disclosure,
            );

            if let Ok(card) = self.card_manager.render_entity_card(
                CardEntityType::Location,
                &location.id,
                disclosure,
                None,
            ).await {
                let priority = self.get_entity_priority(
                    CardEntityType::Location,
                    &location.id,
                    preferences,
                );
                items.push(self.card_to_item(card, priority));
            }
        }

        items.sort_by(|a, b| b.priority.cmp(&a.priority));

        let section_priority = options.section_priorities
            .get(&SectionType::Locations)
            .copied()
            .unwrap_or_else(|| SectionType::Locations.default_priority());

        Ok(CheatSheetSection {
            section_type: SectionType::Locations,
            title: SectionType::Locations.display_name().to_string(),
            items,
            priority: section_priority,
            was_truncated: false,
            hidden_items: 0,
            collapsed: false,
        })
    }

    async fn build_objectives_section(
        &self,
        _session_id: &str,
        options: &CheatSheetOptions,
    ) -> Result<CheatSheetSection, CheatSheetError> {
        // TODO: Load objectives from session plan when available
        // For now, return an empty section
        let section_priority = options.section_priorities
            .get(&SectionType::Objectives)
            .copied()
            .unwrap_or_else(|| SectionType::Objectives.default_priority());

        Ok(CheatSheetSection {
            section_type: SectionType::Objectives,
            title: SectionType::Objectives.display_name().to_string(),
            items: vec![],
            priority: section_priority,
            was_truncated: false,
            hidden_items: 0,
            collapsed: false,
        })
    }

    async fn build_entity_items(
        &self,
        entity_type: CardEntityType,
        entity_ids: &[String],
        disclosure: DisclosureLevel,
    ) -> Vec<CheatSheetItem> {
        let mut items = Vec::new();
        for entity_id in entity_ids {
            if let Ok(card) = self.card_manager.render_entity_card(
                entity_type,
                entity_id,
                disclosure,
                None,
            ).await {
                items.push(self.card_to_item(card, 50));
            }
        }
        items
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    fn should_include_section(
        &self,
        section_type: SectionType,
        options: &CheatSheetOptions,
        preferences: &[CheatSheetPreferenceRecord],
    ) -> bool {
        // Check explicit exclusions first
        if options.exclude_sections.contains(&section_type) {
            return false;
        }

        // Check explicit inclusions
        if !options.include_sections.is_empty() {
            return options.include_sections.contains(&section_type);
        }

        // Check preferences for category-level include status
        for pref in preferences {
            if pref.preference_type == "category" {
                if let Some(entity_type) = &pref.entity_type {
                    let matches = match section_type {
                        SectionType::KeyNpcs => entity_type == "npc",
                        SectionType::Locations => entity_type == "location",
                        _ => false,
                    };
                    if matches {
                        if let Ok(status) = pref.include_status_enum() {
                            return status != IncludeStatus::Never;
                        }
                    }
                }
            }
        }

        true
    }

    fn get_entity_include_status(
        &self,
        entity_type: CardEntityType,
        entity_id: &str,
        preferences: &[CheatSheetPreferenceRecord],
    ) -> IncludeStatus {
        // First check entity-specific preference
        for pref in preferences {
            if pref.preference_type == "entity" {
                if pref.entity_type.as_deref() == Some(entity_type.as_str()) &&
                   pref.entity_id.as_deref() == Some(entity_id) {
                    if let Ok(status) = pref.include_status_enum() {
                        return status;
                    }
                }
            }
        }

        // Then check category preference
        for pref in preferences {
            if pref.preference_type == "category" {
                if pref.entity_type.as_deref() == Some(entity_type.as_str()) {
                    if let Ok(status) = pref.include_status_enum() {
                        return status;
                    }
                }
            }
        }

        IncludeStatus::Auto
    }

    fn get_entity_disclosure_level(
        &self,
        entity_type: CardEntityType,
        entity_id: &str,
        preferences: &[CheatSheetPreferenceRecord],
        default: DisclosureLevel,
    ) -> DisclosureLevel {
        for pref in preferences {
            if pref.preference_type == "entity" {
                if pref.entity_type.as_deref() == Some(entity_type.as_str()) &&
                   pref.entity_id.as_deref() == Some(entity_id) {
                    if let Ok(level) = pref.disclosure_level_enum() {
                        return level;
                    }
                }
            }
        }

        for pref in preferences {
            if pref.preference_type == "category" {
                if pref.entity_type.as_deref() == Some(entity_type.as_str()) {
                    if let Ok(level) = pref.disclosure_level_enum() {
                        return level;
                    }
                }
            }
        }

        default
    }

    fn get_entity_priority(
        &self,
        entity_type: CardEntityType,
        entity_id: &str,
        preferences: &[CheatSheetPreferenceRecord],
    ) -> i32 {
        for pref in preferences {
            if pref.preference_type == "entity" {
                if pref.entity_type.as_deref() == Some(entity_type.as_str()) &&
                   pref.entity_id.as_deref() == Some(entity_id) {
                    return pref.priority;
                }
            }
        }

        50 // Default priority
    }

    fn card_to_item(&self, card: RenderedCard, priority: i32) -> CheatSheetItem {
        CheatSheetItem {
            id: card.entity_id.clone(),
            title: card.title,
            summary: card.subtitle.unwrap_or_default(),
            content: card.text_content.clone(),
            entity_type: Some(card.entity_type),
            entity_id: Some(card.entity_id),
            priority,
            was_truncated: false,
            original_chars: card.text_content.len(),
        }
    }

    fn apply_truncation(
        &self,
        mut sections: Vec<CheatSheetSection>,
        options: &CheatSheetOptions,
    ) -> (Vec<CheatSheetSection>, Vec<TruncationWarning>) {
        let mut warnings = Vec::new();
        let mut total_chars: usize = 0;

        // First pass: truncate individual sections
        for section in &mut sections {
            let section_chars: usize = section.items.iter()
                .map(|i| i.content.len())
                .sum();

            if section_chars > options.max_section_chars {
                let (truncated_items, mut section_warning) =
                    self.truncate_section_items(&mut section.items, options.max_section_chars);
                section.items = truncated_items;
                section.was_truncated = true;
                section.hidden_items = section_warning.items_hidden;
                // Update warning with the correct section type
                section_warning.section = section.section_type;
                warnings.push(section_warning);
            }

            total_chars += section.items.iter().map(|i| i.content.len()).sum::<usize>();
        }

        // Second pass: remove low-priority sections if still over budget
        while total_chars > options.max_total_chars && sections.len() > 1 {
            // Remove the lowest priority section
            if let Some(removed) = sections.pop() {
                let chars_removed: usize = removed.items.iter()
                    .map(|i| i.content.len())
                    .sum();
                total_chars = total_chars.saturating_sub(chars_removed);
                warnings.push(TruncationWarning {
                    section: removed.section_type,
                    chars_removed,
                    items_hidden: removed.items.len(),
                    reason: "Section removed due to total character limit".to_string(),
                });
            }
        }

        (sections, warnings)
    }

    fn truncate_section_items(
        &self,
        items: &mut Vec<CheatSheetItem>,
        max_chars: usize,
    ) -> (Vec<CheatSheetItem>, TruncationWarning) {
        // Sort by priority (highest first)
        items.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut result = Vec::new();
        let mut current_chars = 0;
        let mut hidden_count = 0;
        let mut chars_removed = 0;

        for item in items.drain(..) {
            // Use character count (not byte length) for consistent UTF-8 handling
            let item_char_count = item.content.chars().count();

            if current_chars + item_char_count <= max_chars {
                current_chars += item_char_count;
                result.push(item);
            } else if current_chars < max_chars {
                // Truncate this item to fit (using character-based boundary)
                // Reserve 3 chars for "..."
                let remaining_chars = max_chars.saturating_sub(current_chars).saturating_sub(3);
                let mut truncated_item = item.clone();

                // Take the first `remaining_chars` characters
                let truncated_content: String = item.content.chars().take(remaining_chars).collect();
                let truncated_char_count = truncated_content.chars().count();

                truncated_item.content = format!("{}...", truncated_content);
                truncated_item.was_truncated = true;
                chars_removed += item_char_count.saturating_sub(truncated_char_count);
                current_chars += truncated_char_count + 3; // +3 for "..."
                result.push(truncated_item);
            } else {
                hidden_count += 1;
                chars_removed += item_char_count;
            }
        }

        let warning = TruncationWarning {
            section: SectionType::Custom, // Will be updated by caller
            chars_removed,
            items_hidden: hidden_count,
            reason: format!("Section truncated to {} characters", max_chars),
        };

        (result, warning)
    }
}

// ============================================================================
// HTML Exporter
// ============================================================================

/// HTML exporter for print-friendly cheat sheets
pub struct HtmlExporter;

impl HtmlExporter {
    /// Export a cheat sheet to print-friendly HTML
    pub fn export(cheat_sheet: &CheatSheet) -> Result<String, CheatSheetError> {
        let mut html = String::new();

        // HTML header with print styles
        html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>"#);
        html.push_str(&Self::escape_html(&cheat_sheet.title));
        html.push_str(r#"</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            font-size: 11pt;
            line-height: 1.4;
            color: #1a1a1a;
            padding: 20px;
            max-width: 800px;
            margin: 0 auto;
        }
        h1 { font-size: 16pt; margin-bottom: 12px; border-bottom: 2px solid #333; padding-bottom: 8px; }
        h2 { font-size: 13pt; margin-top: 16px; margin-bottom: 8px; color: #444; }
        h3 { font-size: 11pt; margin-top: 12px; margin-bottom: 4px; }
        .section { margin-bottom: 20px; page-break-inside: avoid; }
        .item { margin-bottom: 12px; padding: 8px; background: #f9f9f9; border-left: 3px solid #666; }
        .item-title { font-weight: 600; margin-bottom: 4px; }
        .item-subtitle { font-size: 9pt; color: #666; margin-bottom: 4px; }
        .item-content { font-size: 10pt; }
        .warning { background: #fff3cd; border: 1px solid #ffc107; padding: 8px; margin-bottom: 12px; font-size: 9pt; }
        .truncated { opacity: 0.7; font-style: italic; }
        .meta { font-size: 9pt; color: #666; margin-top: 16px; border-top: 1px solid #ddd; padding-top: 8px; }
        @media print {
            body { padding: 0; }
            .section { page-break-inside: avoid; }
            .warning { display: none; }
        }
    </style>
</head>
<body>
"#);

        // Title
        html.push_str(&format!("<h1>{}</h1>\n", Self::escape_html(&cheat_sheet.title)));

        // Warnings
        if !cheat_sheet.warnings.is_empty() {
            html.push_str("<div class=\"warning\">\n");
            html.push_str("<strong>Note:</strong> Some content was truncated to fit the cheat sheet.\n");
            html.push_str("<ul>\n");
            for warning in &cheat_sheet.warnings {
                html.push_str(&format!(
                    "<li>{}: {} items hidden, {} characters removed</li>\n",
                    warning.section.display_name(),
                    warning.items_hidden,
                    warning.chars_removed
                ));
            }
            html.push_str("</ul>\n</div>\n");
        }

        // Sections
        for section in &cheat_sheet.sections {
            if section.items.is_empty() {
                continue;
            }

            html.push_str("<div class=\"section\">\n");
            html.push_str(&format!("<h2>{}</h2>\n", Self::escape_html(&section.title)));

            for item in &section.items {
                let truncated_class = if item.was_truncated { " truncated" } else { "" };
                html.push_str(&format!("<div class=\"item{}\">\n", truncated_class));
                html.push_str(&format!(
                    "<div class=\"item-title\">{}</div>\n",
                    Self::escape_html(&item.title)
                ));
                if !item.summary.is_empty() {
                    html.push_str(&format!(
                        "<div class=\"item-subtitle\">{}</div>\n",
                        Self::escape_html(&item.summary)
                    ));
                }
                html.push_str(&format!(
                    "<div class=\"item-content\">{}</div>\n",
                    Self::escape_html(&item.content)
                ));
                html.push_str("</div>\n");
            }

            if section.hidden_items > 0 {
                html.push_str(&format!(
                    "<p class=\"truncated\">+ {} more items not shown</p>\n",
                    section.hidden_items
                ));
            }

            html.push_str("</div>\n");
        }

        // Metadata
        html.push_str("<div class=\"meta\">\n");
        html.push_str(&format!(
            "Generated: {} | Total: {} characters\n",
            cheat_sheet.generated_at,
            cheat_sheet.total_chars
        ));
        html.push_str("</div>\n");

        html.push_str("</body>\n</html>");

        Ok(html)
    }

    /// Escape HTML special characters to prevent XSS attacks.
    ///
    /// Replaces: & < > " '
    pub fn escape_html(text: &str) -> String {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_type_properties() {
        assert_eq!(SectionType::KeyNpcs.as_str(), "key_npcs");
        assert_eq!(SectionType::KeyNpcs.display_name(), "Key NPCs");
        assert!(SectionType::Objectives.default_priority() > SectionType::Custom.default_priority());
    }

    #[test]
    fn test_cheat_sheet_options_default() {
        let options = CheatSheetOptions::default();
        assert_eq!(options.max_section_chars, DEFAULT_MAX_SECTION_CHARS);
        assert_eq!(options.max_total_chars, DEFAULT_MAX_TOTAL_CHARS);
        assert!(options.include_sections.is_empty());
    }

    #[test]
    fn test_cheat_sheet_item_structure() {
        let item = CheatSheetItem {
            id: "item-1".to_string(),
            title: "Test Item".to_string(),
            summary: "A test item".to_string(),
            content: "Full content here".to_string(),
            entity_type: Some(CardEntityType::Npc),
            entity_id: Some("npc-1".to_string()),
            priority: 75,
            was_truncated: false,
            original_chars: 17,
        };

        assert_eq!(item.title, "Test Item");
        assert!(!item.was_truncated);
    }

    #[test]
    fn test_cheat_sheet_section_structure() {
        let section = CheatSheetSection {
            section_type: SectionType::KeyNpcs,
            title: "Key NPCs".to_string(),
            items: vec![],
            priority: 85,
            was_truncated: false,
            hidden_items: 0,
            collapsed: false,
        };

        assert_eq!(section.section_type, SectionType::KeyNpcs);
        assert!(!section.collapsed);
    }

    #[test]
    fn test_html_exporter_escape() {
        assert_eq!(HtmlExporter::escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(HtmlExporter::escape_html("\"quotes\""), "&quot;quotes&quot;");
        assert_eq!(HtmlExporter::escape_html("Tom & Jerry"), "Tom &amp; Jerry");
    }

    #[test]
    fn test_html_exporter_basic() {
        let cheat_sheet = CheatSheet {
            campaign_id: "camp-1".to_string(),
            session_id: Some("sess-1".to_string()),
            title: "Test Cheat Sheet".to_string(),
            sections: vec![
                CheatSheetSection {
                    section_type: SectionType::KeyNpcs,
                    title: "Key NPCs".to_string(),
                    items: vec![
                        CheatSheetItem {
                            id: "npc-1".to_string(),
                            title: "Bob the Merchant".to_string(),
                            summary: "Friendly shopkeeper".to_string(),
                            content: "Sells general goods in the market square.".to_string(),
                            entity_type: Some(CardEntityType::Npc),
                            entity_id: Some("npc-1".to_string()),
                            priority: 75,
                            was_truncated: false,
                            original_chars: 40,
                        },
                    ],
                    priority: 85,
                    was_truncated: false,
                    hidden_items: 0,
                    collapsed: false,
                },
            ],
            total_chars: 100,
            max_chars: 25000,
            warnings: vec![],
            generated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let html = HtmlExporter::export(&cheat_sheet).unwrap();

        assert!(html.contains("Test Cheat Sheet"));
        assert!(html.contains("Key NPCs"));
        assert!(html.contains("Bob the Merchant"));
        assert!(html.contains("<!DOCTYPE html>"));
    }

    #[test]
    fn test_truncation_warning_structure() {
        let warning = TruncationWarning {
            section: SectionType::KeyNpcs,
            chars_removed: 500,
            items_hidden: 2,
            reason: "Section truncated".to_string(),
        };

        assert_eq!(warning.section, SectionType::KeyNpcs);
        assert_eq!(warning.items_hidden, 2);
    }
}
