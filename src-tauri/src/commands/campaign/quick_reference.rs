//! Quick Reference Commands Module
//!
//! Tauri commands for quick reference cards and cheat sheets.
//! Phase 9 of the Campaign Generation Overhaul.
//!
//! Provides commands for:
//! - Rendering entity cards at different disclosure levels
//! - Managing the pinned card tray (pin/unpin/reorder)
//! - Building and exporting cheat sheets
//! - Managing cheat sheet preferences

use std::collections::HashMap;
use tauri::State;
use tracing::{debug, error, info};

use crate::commands::AppState;
use crate::core::campaign::quick_reference::{
    QuickReferenceCardManager, QuickReferenceError,
    RenderedCard, HoverPreview, CardTray, PinnedCard,
    MAX_PINNED_CARDS,
};
use crate::core::campaign::cheat_sheet::{
    CheatSheetBuilder, CheatSheet, CheatSheetOptions, CheatSheetError,
    SectionType, HtmlExporter,
};
use crate::database::{
    CardEntityType, DisclosureLevel, CheatSheetPreferenceRecord,
    PreferenceType, IncludeStatus, QuickReferenceOps,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert quick reference errors to String for Tauri IPC
fn qr_err_to_string(err: QuickReferenceError) -> String {
    let msg = err.to_string();
    error!(error = %msg, "Quick reference command error");
    msg
}

/// Convert cheat sheet errors to String for Tauri IPC
fn cs_err_to_string(err: CheatSheetError) -> String {
    let msg = err.to_string();
    error!(error = %msg, "Cheat sheet command error");
    msg
}

// ============================================================================
// Card Rendering Commands
// ============================================================================

/// Render an entity card for display.
///
/// # Arguments
/// * `entity_type` - Type of entity (npc, location, item, plot_point, scene, character)
/// * `entity_id` - Entity ID
/// * `disclosure_level` - Detail level (minimal, summary, complete)
/// * `session_id` - Optional session ID for pin status
///
/// # Returns
/// Rendered card with HTML and text content
#[tauri::command]
pub async fn get_entity_card(
    entity_type: String,
    entity_id: String,
    disclosure_level: String,
    session_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<RenderedCard, String> {
    debug!(
        entity_type = %entity_type,
        entity_id = %entity_id,
        disclosure_level = %disclosure_level,
        "Getting entity card"
    );

    let entity_type = CardEntityType::try_from(entity_type.as_str())
        .map_err(|e| format!("Invalid entity type: {}", e))?;
    let disclosure = DisclosureLevel::try_from(disclosure_level.as_str())
        .map_err(|e| format!("Invalid disclosure level: {}", e))?;

    let manager = QuickReferenceCardManager::new(&state.database);

    manager
        .render_entity_card(entity_type, &entity_id, disclosure, session_id.as_deref())
        .await
        .map_err(qr_err_to_string)
}

/// Generate a hover preview for an entity.
///
/// # Arguments
/// * `entity_type` - Type of entity
/// * `entity_id` - Entity ID
///
/// # Returns
/// Minimal preview suitable for tooltips
#[tauri::command]
pub async fn get_hover_preview(
    entity_type: String,
    entity_id: String,
    state: State<'_, AppState>,
) -> Result<HoverPreview, String> {
    debug!(
        entity_type = %entity_type,
        entity_id = %entity_id,
        "Getting hover preview"
    );

    let entity_type = CardEntityType::try_from(entity_type.as_str())
        .map_err(|e| format!("Invalid entity type: {}", e))?;

    let manager = QuickReferenceCardManager::new(&state.database);

    manager
        .generate_hover_preview(entity_type, &entity_id)
        .await
        .map_err(qr_err_to_string)
}

// ============================================================================
// Card Tray Commands
// ============================================================================

/// Get the card tray for a session.
///
/// # Arguments
/// * `session_id` - Session ID
///
/// # Returns
/// Card tray with all pinned cards (max 6)
#[tauri::command]
pub async fn get_pinned_cards(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<CardTray, String> {
    debug!(session_id = %session_id, "Getting pinned cards");

    let manager = QuickReferenceCardManager::new(&state.database);

    manager
        .get_card_tray(&session_id)
        .await
        .map_err(qr_err_to_string)
}

/// Pin a card to the session tray.
///
/// # Arguments
/// * `session_id` - Session ID
/// * `entity_type` - Type of entity to pin
/// * `entity_id` - Entity ID
/// * `disclosure_level` - Optional disclosure level (defaults to summary)
///
/// # Returns
/// The pinned card with rendered content
#[tauri::command]
pub async fn pin_card(
    session_id: String,
    entity_type: String,
    entity_id: String,
    disclosure_level: Option<String>,
    state: State<'_, AppState>,
) -> Result<PinnedCard, String> {
    info!(
        session_id = %session_id,
        entity_type = %entity_type,
        entity_id = %entity_id,
        "Pinning card"
    );

    let entity_type = CardEntityType::try_from(entity_type.as_str())
        .map_err(|e| format!("Invalid entity type: {}", e))?;

    let disclosure = disclosure_level
        .as_deref()
        .map(DisclosureLevel::try_from)
        .transpose()
        .map_err(|e| format!("Invalid disclosure level: {}", e))?;

    let manager = QuickReferenceCardManager::new(&state.database);

    manager
        .pin_card(&session_id, entity_type, &entity_id, disclosure)
        .await
        .map_err(qr_err_to_string)
}

/// Unpin a card from the session tray.
///
/// # Arguments
/// * `session_id` - Session ID
/// * `entity_type` - Type of entity
/// * `entity_id` - Entity ID
#[tauri::command]
pub async fn unpin_card(
    session_id: String,
    entity_type: String,
    entity_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!(
        session_id = %session_id,
        entity_type = %entity_type,
        entity_id = %entity_id,
        "Unpinning card"
    );

    let entity_type = CardEntityType::try_from(entity_type.as_str())
        .map_err(|e| format!("Invalid entity type: {}", e))?;

    let manager = QuickReferenceCardManager::new(&state.database);

    manager
        .unpin_card(&session_id, entity_type, &entity_id)
        .await
        .map_err(qr_err_to_string)
}

/// Reorder pinned cards in the tray.
///
/// # Arguments
/// * `session_id` - Session ID
/// * `card_ids_in_order` - Card IDs in desired order
///
/// # Returns
/// Updated card tray
#[tauri::command]
pub async fn reorder_pinned_cards(
    session_id: String,
    card_ids_in_order: Vec<String>,
    state: State<'_, AppState>,
) -> Result<CardTray, String> {
    debug!(
        session_id = %session_id,
        card_count = card_ids_in_order.len(),
        "Reordering pinned cards"
    );

    let manager = QuickReferenceCardManager::new(&state.database);

    manager
        .reorder_cards(&session_id, card_ids_in_order)
        .await
        .map_err(qr_err_to_string)
}

/// Update the disclosure level of a pinned card.
///
/// # Arguments
/// * `pin_id` - Pin record ID
/// * `disclosure_level` - New disclosure level
#[tauri::command]
pub async fn update_card_disclosure(
    pin_id: String,
    disclosure_level: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    debug!(pin_id = %pin_id, level = %disclosure_level, "Updating card disclosure");

    let disclosure = DisclosureLevel::try_from(disclosure_level.as_str())
        .map_err(|e| format!("Invalid disclosure level: {}", e))?;

    let manager = QuickReferenceCardManager::new(&state.database);

    manager
        .update_card_disclosure(&pin_id, disclosure)
        .await
        .map_err(qr_err_to_string)
}

/// Get the maximum number of pinned cards allowed.
#[tauri::command]
pub fn get_max_pinned_cards() -> usize {
    MAX_PINNED_CARDS
}

// ============================================================================
// Cheat Sheet Commands
// ============================================================================

/// Build a cheat sheet for a session.
///
/// # Arguments
/// * `campaign_id` - Campaign ID
/// * `session_id` - Session ID
/// * `options` - Optional cheat sheet options
///
/// # Returns
/// Generated cheat sheet with sections and items
#[tauri::command]
pub async fn build_cheat_sheet(
    campaign_id: String,
    session_id: String,
    options: Option<CheatSheetOptionsInput>,
    state: State<'_, AppState>,
) -> Result<CheatSheet, String> {
    info!(
        campaign_id = %campaign_id,
        session_id = %session_id,
        "Building cheat sheet"
    );

    let builder = CheatSheetBuilder::new(&state.database);
    let opts = options.map(|o| o.into()).unwrap_or_default();

    builder
        .build_for_session(&campaign_id, &session_id, opts)
        .await
        .map_err(cs_err_to_string)
}

/// Build a custom cheat sheet with specific entities.
///
/// # Arguments
/// * `campaign_id` - Campaign ID
/// * `title` - Cheat sheet title
/// * `entities` - List of (entity_type, entity_id) pairs
/// * `options` - Optional cheat sheet options
///
/// # Returns
/// Generated custom cheat sheet
#[tauri::command]
pub async fn build_custom_cheat_sheet(
    campaign_id: String,
    title: String,
    entities: Vec<EntityRef>,
    options: Option<CheatSheetOptionsInput>,
    state: State<'_, AppState>,
) -> Result<CheatSheet, String> {
    info!(
        campaign_id = %campaign_id,
        entity_count = entities.len(),
        "Building custom cheat sheet"
    );

    let builder = CheatSheetBuilder::new(&state.database);
    let opts = options.map(|o| o.into()).unwrap_or_default();

    let entity_ids: Result<Vec<(CardEntityType, String)>, String> = entities
        .into_iter()
        .map(|e| {
            let entity_type = CardEntityType::try_from(e.entity_type.as_str())
                .map_err(|err| format!("Invalid entity type: {}", err))?;
            Ok((entity_type, e.entity_id))
        })
        .collect();

    builder
        .build_custom(&campaign_id, title, entity_ids?, opts)
        .await
        .map_err(cs_err_to_string)
}

/// Export a cheat sheet to print-friendly HTML.
///
/// # Arguments
/// * `cheat_sheet` - Cheat sheet to export
///
/// # Returns
/// HTML string suitable for printing
#[tauri::command]
pub fn export_cheat_sheet_html(
    cheat_sheet: CheatSheet,
) -> Result<String, String> {
    debug!("Exporting cheat sheet to HTML");

    HtmlExporter::export(&cheat_sheet)
        .map_err(cs_err_to_string)
}

// ============================================================================
// Cheat Sheet Preferences Commands
// ============================================================================

/// Save a cheat sheet preference.
///
/// # Arguments
/// * `preference` - Preference to save
#[tauri::command]
pub async fn save_cheat_sheet_preference(
    preference: CheatSheetPreferenceInput,
    state: State<'_, AppState>,
) -> Result<CheatSheetPreferenceRecord, String> {
    info!(
        campaign_id = %preference.campaign_id,
        preference_type = %preference.preference_type,
        "Saving cheat sheet preference"
    );

    let pref_type = PreferenceType::try_from(preference.preference_type.as_str())
        .map_err(|e| format!("Invalid preference type: {}", e))?;

    let mut record = CheatSheetPreferenceRecord::new(
        preference.campaign_id.clone(),
        pref_type,
    );

    if let Some(session_id) = preference.session_id {
        record = record.with_session(session_id);
    }

    if let Some(entity_type_str) = &preference.entity_type {
        let entity_type = CardEntityType::try_from(entity_type_str.as_str())
            .map_err(|e| format!("Invalid entity type: {}", e))?;

        if let Some(entity_id) = preference.entity_id {
            record = record.with_entity(entity_type, entity_id);
        } else {
            record = record.with_entity_type(entity_type);
        }
    }

    if let Some(status_str) = preference.include_status {
        let status = IncludeStatus::try_from(status_str.as_str())
            .map_err(|e| format!("Invalid include status: {}", e))?;
        record = record.with_include_status(status);
    }

    if let Some(level_str) = preference.default_disclosure_level {
        let level = DisclosureLevel::try_from(level_str.as_str())
            .map_err(|e| format!("Invalid disclosure level: {}", e))?;
        record = record.with_disclosure_level(level);
    }

    if let Some(priority) = preference.priority {
        record = record.with_priority(priority);
    }

    state.database
        .save_cheat_sheet_preference(&record)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    Ok(record)
}

/// Get cheat sheet preferences for a campaign.
///
/// # Arguments
/// * `campaign_id` - Campaign ID
///
/// # Returns
/// List of preferences ordered by priority
#[tauri::command]
pub async fn get_cheat_sheet_preferences(
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<CheatSheetPreferenceRecord>, String> {
    debug!(campaign_id = %campaign_id, "Getting cheat sheet preferences");

    state.database
        .get_cheat_sheet_preferences(&campaign_id)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

/// Delete a cheat sheet preference.
///
/// # Arguments
/// * `preference_id` - Preference record ID
#[tauri::command]
pub async fn delete_cheat_sheet_preference(
    preference_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    debug!(preference_id = %preference_id, "Deleting cheat sheet preference");

    state.database
        .delete_cheat_sheet_preference(&preference_id)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

/// Invalidate card cache for an entity.
///
/// Call this when an entity is updated to ensure fresh card rendering.
///
/// # Arguments
/// * `entity_type` - Type of entity
/// * `entity_id` - Entity ID
///
/// # Returns
/// Number of cache entries removed
#[tauri::command]
pub async fn invalidate_card_cache(
    entity_type: String,
    entity_id: String,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    // Validate entity type like other commands
    let validated_type = CardEntityType::try_from(entity_type.as_str())
        .map_err(|e| format!("Invalid entity type: {}", e))?;

    debug!(
        entity_type = %validated_type,
        entity_id = %entity_id,
        "Invalidating card cache"
    );

    state.database
        .invalidate_card_cache(&validated_type.to_string(), &entity_id)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

/// Clean up expired card cache entries.
///
/// # Returns
/// Number of cache entries removed
#[tauri::command]
pub async fn cleanup_card_cache(
    state: State<'_, AppState>,
) -> Result<u64, String> {
    debug!("Cleaning up expired card cache");

    state.database
        .cleanup_expired_card_cache()
        .await
        .map_err(|e| format!("Database error: {}", e))
}

// ============================================================================
// Info Commands
// ============================================================================

/// List available entity types for cards.
#[tauri::command]
pub fn list_card_entity_types() -> Vec<EntityTypeInfo> {
    vec![
        EntityTypeInfo {
            value: "npc".to_string(),
            name: "NPC".to_string(),
            description: "Non-player characters".to_string(),
        },
        EntityTypeInfo {
            value: "location".to_string(),
            name: "Location".to_string(),
            description: "Places and environments".to_string(),
        },
        EntityTypeInfo {
            value: "item".to_string(),
            name: "Item".to_string(),
            description: "Equipment, treasures, and objects".to_string(),
        },
        EntityTypeInfo {
            value: "plot_point".to_string(),
            name: "Plot Point".to_string(),
            description: "Story elements and events".to_string(),
        },
        EntityTypeInfo {
            value: "scene".to_string(),
            name: "Scene".to_string(),
            description: "Planned scenes and encounters".to_string(),
        },
        EntityTypeInfo {
            value: "character".to_string(),
            name: "Character".to_string(),
            description: "Player characters".to_string(),
        },
    ]
}

/// List available disclosure levels.
#[tauri::command]
pub fn list_disclosure_levels() -> Vec<DisclosureLevelInfo> {
    vec![
        DisclosureLevelInfo {
            value: "minimal".to_string(),
            name: "Minimal".to_string(),
            description: "Name and type only - for hover previews".to_string(),
        },
        DisclosureLevelInfo {
            value: "summary".to_string(),
            name: "Summary".to_string(),
            description: "Key details for quick reference".to_string(),
        },
        DisclosureLevelInfo {
            value: "complete".to_string(),
            name: "Complete".to_string(),
            description: "Full entity details".to_string(),
        },
    ]
}

/// List available cheat sheet section types.
#[tauri::command]
pub fn list_cheat_sheet_sections() -> Vec<SectionTypeInfo> {
    vec![
        SectionTypeInfo {
            value: "key_npcs".to_string(),
            name: "Key NPCs".to_string(),
            description: "Important NPCs for the session".to_string(),
            default_priority: SectionType::KeyNpcs.default_priority(),
        },
        SectionTypeInfo {
            value: "locations".to_string(),
            name: "Locations".to_string(),
            description: "Relevant locations".to_string(),
            default_priority: SectionType::Locations.default_priority(),
        },
        SectionTypeInfo {
            value: "plot_points".to_string(),
            name: "Plot Points".to_string(),
            description: "Active plot threads".to_string(),
            default_priority: SectionType::PlotPoints.default_priority(),
        },
        SectionTypeInfo {
            value: "objectives".to_string(),
            name: "Objectives".to_string(),
            description: "Session goals".to_string(),
            default_priority: SectionType::Objectives.default_priority(),
        },
        SectionTypeInfo {
            value: "encounters".to_string(),
            name: "Encounters".to_string(),
            description: "Combat and encounter notes".to_string(),
            default_priority: SectionType::Encounters.default_priority(),
        },
        SectionTypeInfo {
            value: "scenes".to_string(),
            name: "Scenes".to_string(),
            description: "Scheduled scenes".to_string(),
            default_priority: SectionType::Scenes.default_priority(),
        },
        SectionTypeInfo {
            value: "rules".to_string(),
            name: "Rules Reference".to_string(),
            description: "Important rules and mechanics".to_string(),
            default_priority: SectionType::Rules.default_priority(),
        },
        SectionTypeInfo {
            value: "party_reminders".to_string(),
            name: "Party Reminders".to_string(),
            description: "Player character notes".to_string(),
            default_priority: SectionType::PartyReminders.default_priority(),
        },
        SectionTypeInfo {
            value: "custom".to_string(),
            name: "Notes".to_string(),
            description: "Custom notes and miscellaneous".to_string(),
            default_priority: SectionType::Custom.default_priority(),
        },
    ]
}

// ============================================================================
// Input/Output Types for Tauri IPC
// ============================================================================

/// Entity reference for custom cheat sheets
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityRef {
    pub entity_type: String,
    pub entity_id: String,
}

/// Cheat sheet options input for Tauri commands
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheatSheetOptionsInput {
    pub max_section_chars: Option<usize>,
    pub max_total_chars: Option<usize>,
    pub include_sections: Option<Vec<String>>,
    pub exclude_sections: Option<Vec<String>>,
    pub default_disclosure: Option<String>,
    pub include_collapsed: Option<bool>,
    pub section_priorities: Option<HashMap<String, i32>>,
}

impl From<CheatSheetOptionsInput> for CheatSheetOptions {
    fn from(input: CheatSheetOptionsInput) -> Self {
        let mut options = CheatSheetOptions::default();

        if let Some(max) = input.max_section_chars {
            options.max_section_chars = max;
        }
        if let Some(max) = input.max_total_chars {
            options.max_total_chars = max;
        }
        if let Some(sections) = input.include_sections {
            options.include_sections = sections.iter()
                .filter_map(|s| section_type_from_str(s))
                .collect();
        }
        if let Some(sections) = input.exclude_sections {
            options.exclude_sections = sections.iter()
                .filter_map(|s| section_type_from_str(s))
                .collect();
        }
        if let Some(disclosure) = input.default_disclosure {
            if let Ok(level) = DisclosureLevel::try_from(disclosure.as_str()) {
                options.default_disclosure = level;
            }
        }
        if let Some(collapsed) = input.include_collapsed {
            options.include_collapsed = collapsed;
        }
        if let Some(priorities) = input.section_priorities {
            for (key, priority) in priorities {
                if let Some(section_type) = section_type_from_str(&key) {
                    options.section_priorities.insert(section_type, priority);
                }
            }
        }

        options
    }
}

fn section_type_from_str(s: &str) -> Option<SectionType> {
    match s {
        "key_npcs" => Some(SectionType::KeyNpcs),
        "locations" => Some(SectionType::Locations),
        "plot_points" => Some(SectionType::PlotPoints),
        "objectives" => Some(SectionType::Objectives),
        "encounters" => Some(SectionType::Encounters),
        "scenes" => Some(SectionType::Scenes),
        "rules" => Some(SectionType::Rules),
        "party_reminders" => Some(SectionType::PartyReminders),
        "custom" => Some(SectionType::Custom),
        _ => None,
    }
}

/// Cheat sheet preference input for Tauri commands
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheatSheetPreferenceInput {
    pub campaign_id: String,
    pub session_id: Option<String>,
    pub preference_type: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub include_status: Option<String>,
    pub default_disclosure_level: Option<String>,
    pub priority: Option<i32>,
}

/// Entity type info for frontend
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EntityTypeInfo {
    pub value: String,
    pub name: String,
    pub description: String,
}

/// Disclosure level info for frontend
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DisclosureLevelInfo {
    pub value: String,
    pub name: String,
    pub description: String,
}

/// Section type info for frontend
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SectionTypeInfo {
    pub value: String,
    pub name: String,
    pub description: String,
    pub default_priority: i32,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_card_entity_types() {
        let types = list_card_entity_types();
        assert_eq!(types.len(), 6);
        assert!(types.iter().any(|t| t.value == "npc"));
        assert!(types.iter().any(|t| t.value == "location"));
    }

    #[test]
    fn test_list_disclosure_levels() {
        let levels = list_disclosure_levels();
        assert_eq!(levels.len(), 3);
        assert!(levels.iter().any(|l| l.value == "minimal"));
        assert!(levels.iter().any(|l| l.value == "summary"));
        assert!(levels.iter().any(|l| l.value == "complete"));
    }

    #[test]
    fn test_list_cheat_sheet_sections() {
        let sections = list_cheat_sheet_sections();
        assert!(!sections.is_empty());
        assert!(sections.iter().any(|s| s.value == "key_npcs"));
    }

    #[test]
    fn test_get_max_pinned_cards() {
        assert_eq!(get_max_pinned_cards(), 6);
    }

    #[test]
    fn test_section_type_from_str() {
        assert_eq!(section_type_from_str("key_npcs"), Some(SectionType::KeyNpcs));
        assert_eq!(section_type_from_str("locations"), Some(SectionType::Locations));
        assert_eq!(section_type_from_str("invalid"), None);
    }

    #[test]
    fn test_cheat_sheet_options_input_conversion() {
        let input = CheatSheetOptionsInput {
            max_section_chars: Some(3000),
            max_total_chars: Some(15000),
            include_sections: Some(vec!["key_npcs".to_string(), "locations".to_string()]),
            exclude_sections: None,
            default_disclosure: Some("summary".to_string()),
            include_collapsed: Some(false),
            section_priorities: None,
        };

        let options: CheatSheetOptions = input.into();

        assert_eq!(options.max_section_chars, 3000);
        assert_eq!(options.max_total_chars, 15000);
        assert_eq!(options.include_sections.len(), 2);
        assert_eq!(options.default_disclosure, DisclosureLevel::Summary);
        assert!(!options.include_collapsed);
    }
}
