//! Random Table Commands
//!
//! Phase 8 of the Campaign Generation Overhaul.
//!
//! Tauri IPC commands for random table management and dice rolling.
//!
//! # Commands
//!
//! ## Table Management
//! - [`create_random_table`]: Create a new random table
//! - [`get_random_table`]: Get a table by ID
//! - [`list_random_tables`]: List tables for a campaign
//! - [`update_random_table`]: Update an existing table
//! - [`delete_random_table`]: Delete a table
//!
//! ## Rolling
//! - [`roll_on_table`]: Roll on a random table
//! - [`roll_dice`]: Roll dice without a table
//! - [`quick_roll`]: Quick roll on a table by ID
//!
//! ## History
//! - [`get_roll_history`]: Get roll history for a session/campaign
//! - [`clear_roll_history`]: Clear old roll history

use std::sync::Arc;
use tauri::State;
use tracing::{info, debug, error};

use crate::commands::AppState;
use crate::core::campaign::{
    RandomTableEngine, RandomTable, TableRollResult,
    CreateTableRequest, TableEntryInput, RollRequest,
    DiceNotation, DiceRoller, RollResult,
    RandomTableError,
};
use crate::database::{RollHistoryRecord, RandomTableType};

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a RandomTableEngine from AppState
fn get_table_engine(state: &State<'_, AppState>) -> RandomTableEngine {
    let pool = Arc::new(state.database.pool().clone());
    RandomTableEngine::new(pool)
}

/// Convert RandomTableError to String for Tauri IPC
fn table_err_to_string(err: RandomTableError) -> String {
    error!(error = %err, "Random table command error");
    err.to_string()
}

// ============================================================================
// Table Management Commands
// ============================================================================

/// Create a new random table.
///
/// # Arguments
/// * `name` - Table name
/// * `dice_notation` - Dice notation (e.g., "d20", "2d6")
/// * `entries` - List of table entries with ranges
/// * `description` - Optional description
/// * `table_type` - Type of table (standard, weighted, d66, nested, oracle)
/// * `category` - Optional category for organization
/// * `tags` - Optional tags for searching
/// * `campaign_id` - Optional campaign association
///
/// # Returns
/// The created random table
#[tauri::command]
pub async fn create_random_table(
    name: String,
    dice_notation: String,
    entries: Vec<TableEntryInput>,
    description: Option<String>,
    table_type: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<RandomTable, String> {
    info!(name = %name, dice = %dice_notation, "Creating random table");

    let engine = get_table_engine(&state);

    let parsed_type = table_type
        .map(|t| RandomTableType::try_from(t.as_str()))
        .transpose()?
        .unwrap_or_default();

    let request = CreateTableRequest {
        name,
        description,
        dice_notation,
        table_type: parsed_type,
        category,
        tags: tags.unwrap_or_default(),
        campaign_id,
        entries,
        is_system: false,
    };

    engine
        .create_table(request)
        .await
        .map_err(table_err_to_string)
}

/// Get a random table by ID.
///
/// # Arguments
/// * `table_id` - The table's unique identifier
///
/// # Returns
/// The random table with all entries
#[tauri::command]
pub async fn get_random_table(
    table_id: String,
    state: State<'_, AppState>,
) -> Result<RandomTable, String> {
    debug!(table_id = %table_id, "Getting random table");

    let engine = get_table_engine(&state);
    engine
        .get_table(&table_id)
        .await
        .map_err(table_err_to_string)
}

/// List random tables for a campaign.
///
/// Returns both campaign-specific tables and system tables.
///
/// # Arguments
/// * `campaign_id` - Optional campaign to filter by
///
/// # Returns
/// List of random tables
#[tauri::command]
pub async fn list_random_tables(
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<RandomTable>, String> {
    debug!(campaign_id = ?campaign_id, "Listing random tables");

    let engine = get_table_engine(&state);
    engine
        .list_tables(campaign_id.as_deref())
        .await
        .map_err(table_err_to_string)
}

/// List random tables by category.
///
/// # Arguments
/// * `category` - Category to filter by
/// * `campaign_id` - Optional campaign to filter by
///
/// # Returns
/// List of random tables in the category
#[tauri::command]
pub async fn list_random_tables_by_category(
    category: String,
    campaign_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<RandomTable>, String> {
    debug!(category = %category, "Listing random tables by category");

    let engine = get_table_engine(&state);
    engine
        .list_tables_by_category(&category, campaign_id.as_deref())
        .await
        .map_err(table_err_to_string)
}

/// Update a random table.
///
/// Replaces all entries with the provided list.
///
/// # Arguments
/// * `table_id` - The table's unique identifier
/// * `name` - New table name
/// * `dice_notation` - New dice notation
/// * `entries` - New list of entries
/// * `description` - Optional description
/// * `table_type` - Type of table
/// * `category` - Optional category
/// * `tags` - Optional tags
///
/// # Returns
/// The updated random table
#[tauri::command]
pub async fn update_random_table(
    table_id: String,
    name: String,
    dice_notation: String,
    entries: Vec<TableEntryInput>,
    description: Option<String>,
    table_type: Option<String>,
    category: Option<String>,
    tags: Option<Vec<String>>,
    state: State<'_, AppState>,
) -> Result<RandomTable, String> {
    info!(table_id = %table_id, "Updating random table");

    let engine = get_table_engine(&state);

    let parsed_type = table_type
        .map(|t| RandomTableType::try_from(t.as_str()))
        .transpose()?
        .unwrap_or_default();

    let request = CreateTableRequest {
        name,
        description,
        dice_notation,
        table_type: parsed_type,
        category,
        tags: tags.unwrap_or_default(),
        campaign_id: None, // Can't change campaign
        entries,
        is_system: false,
    };

    engine
        .update_table(&table_id, request)
        .await
        .map_err(table_err_to_string)
}

/// Delete a random table.
///
/// Cannot delete system tables.
///
/// # Arguments
/// * `table_id` - The table's unique identifier
#[tauri::command]
pub async fn delete_random_table(
    table_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    info!(table_id = %table_id, "Deleting random table");

    let engine = get_table_engine(&state);
    engine
        .delete_table(&table_id)
        .await
        .map_err(table_err_to_string)
}

// ============================================================================
// Rolling Commands
// ============================================================================

/// Roll on a random table.
///
/// Resolves nested tables and records in history.
///
/// # Arguments
/// * `table_id` - The table to roll on
/// * `session_id` - Optional session for history
/// * `campaign_id` - Optional campaign for history
/// * `context` - Optional context description
/// * `forced_roll` - Optional forced result (for GM fiat)
///
/// # Returns
/// The roll result including nested results
#[tauri::command]
pub async fn roll_on_table(
    table_id: String,
    session_id: Option<String>,
    campaign_id: Option<String>,
    context: Option<String>,
    forced_roll: Option<i32>,
    state: State<'_, AppState>,
) -> Result<TableRollResult, String> {
    debug!(table_id = %table_id, forced = ?forced_roll, "Rolling on table");

    let engine = get_table_engine(&state);

    let request = RollRequest {
        table_id,
        session_id,
        campaign_id,
        context,
        forced_roll,
        max_depth: None,
    };

    engine
        .roll_on_table(request)
        .await
        .map_err(table_err_to_string)
}

/// Quick roll on a table by ID.
///
/// Simplified version without session tracking.
///
/// # Arguments
/// * `table_id` - The table to roll on
///
/// # Returns
/// The roll result
#[tauri::command]
pub async fn quick_table_roll(
    table_id: String,
    state: State<'_, AppState>,
) -> Result<TableRollResult, String> {
    debug!(table_id = %table_id, "Quick roll on table");

    let engine = get_table_engine(&state);
    engine
        .quick_roll(&table_id)
        .await
        .map_err(table_err_to_string)
}

/// Roll dice without a table.
///
/// # Arguments
/// * `notation` - Dice notation (e.g., "d20", "2d6+3")
/// * `session_id` - Optional session for history
/// * `campaign_id` - Optional campaign for history
/// * `context` - Optional context description
///
/// # Returns
/// The dice roll result
#[tauri::command]
pub async fn roll_dice(
    notation: String,
    session_id: Option<String>,
    campaign_id: Option<String>,
    context: Option<String>,
    state: State<'_, AppState>,
) -> Result<RollResult, String> {
    debug!(notation = %notation, "Rolling dice");

    let engine = get_table_engine(&state);
    engine
        .roll_dice(&notation, session_id.as_deref(), campaign_id.as_deref(), context.as_deref())
        .await
        .map_err(table_err_to_string)
}

/// Parse and validate dice notation.
///
/// # Arguments
/// * `notation` - Dice notation to parse
///
/// # Returns
/// Parsed notation details
#[tauri::command]
pub async fn parse_dice_notation(
    notation: String,
) -> Result<DiceNotationInfo, String> {
    let parsed = DiceNotation::parse(&notation)
        .map_err(|e| e.to_string())?;

    Ok(DiceNotationInfo {
        notation: parsed.to_string(),
        count: parsed.count,
        sides: parsed.sides(),
        modifier: parsed.modifier,
        min_result: parsed.min_result(),
        max_result: parsed.max_result(),
        average_result: parsed.average_result(),
    })
}

/// Parsed dice notation information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiceNotationInfo {
    pub notation: String,
    pub count: u32,
    pub sides: u32,
    pub modifier: i32,
    pub min_result: i32,
    pub max_result: i32,
    pub average_result: f64,
}

/// Roll with advantage (roll twice, take higher).
///
/// # Arguments
/// * `notation` - Dice notation
///
/// # Returns
/// Both rolls and the best result
#[tauri::command]
pub async fn roll_with_advantage(
    notation: String,
) -> Result<AdvantageRollResult, String> {
    let parsed = DiceNotation::parse(&notation)
        .map_err(|e| e.to_string())?;

    let roller = DiceRoller::new();
    let (roll1, roll2, best) = roller.roll_advantage(&parsed);

    Ok(AdvantageRollResult {
        roll1,
        roll2,
        best,
    })
}

/// Roll with disadvantage (roll twice, take lower).
///
/// # Arguments
/// * `notation` - Dice notation
///
/// # Returns
/// Both rolls and the worst result
#[tauri::command]
pub async fn roll_with_disadvantage(
    notation: String,
) -> Result<DisadvantageRollResult, String> {
    let parsed = DiceNotation::parse(&notation)
        .map_err(|e| e.to_string())?;

    let roller = DiceRoller::new();
    let (roll1, roll2, worst) = roller.roll_disadvantage(&parsed);

    Ok(DisadvantageRollResult {
        roll1,
        roll2,
        worst,
    })
}

/// Advantage roll result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdvantageRollResult {
    pub roll1: RollResult,
    pub roll2: RollResult,
    pub best: RollResult,
}

/// Disadvantage roll result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DisadvantageRollResult {
    pub roll1: RollResult,
    pub roll2: RollResult,
    pub worst: RollResult,
}

// ============================================================================
// History Commands
// ============================================================================

/// Get roll history for a session.
///
/// # Arguments
/// * `session_id` - The session ID
/// * `limit` - Maximum number of records to return
///
/// # Returns
/// List of roll history records
#[tauri::command]
pub async fn get_session_roll_history(
    session_id: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<RollHistoryRecord>, String> {
    debug!(session_id = %session_id, "Getting session roll history");

    let engine = get_table_engine(&state);
    engine
        .get_session_roll_history(&session_id, limit.unwrap_or(50))
        .await
        .map_err(table_err_to_string)
}

/// Get roll history for a campaign.
///
/// # Arguments
/// * `campaign_id` - The campaign ID
/// * `limit` - Maximum number of records to return
///
/// # Returns
/// List of roll history records
#[tauri::command]
pub async fn get_campaign_roll_history(
    campaign_id: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<RollHistoryRecord>, String> {
    debug!(campaign_id = %campaign_id, "Getting campaign roll history");

    let engine = get_table_engine(&state);
    engine
        .get_campaign_roll_history(&campaign_id, limit.unwrap_or(100))
        .await
        .map_err(table_err_to_string)
}

/// Get roll history for a specific table.
///
/// # Arguments
/// * `table_id` - The table ID
/// * `limit` - Maximum number of records to return
///
/// # Returns
/// List of roll history records
#[tauri::command]
pub async fn get_table_roll_history(
    table_id: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<RollHistoryRecord>, String> {
    debug!(table_id = %table_id, "Getting table roll history");

    let engine = get_table_engine(&state);
    engine
        .get_table_roll_history(&table_id, limit.unwrap_or(100))
        .await
        .map_err(table_err_to_string)
}

/// Clear old roll history.
///
/// # Arguments
/// * `days` - Delete records older than this many days (must be non-negative)
///
/// # Returns
/// Number of records deleted
#[tauri::command]
pub async fn clear_old_roll_history(
    days: i64,
    state: State<'_, AppState>,
) -> Result<u64, String> {
    // Validate days parameter
    if days < 0 {
        return Err("Days parameter must be non-negative".to_string());
    }

    info!(days, "Clearing old roll history");

    let engine = get_table_engine(&state);
    engine
        .clear_old_history(days)
        .await
        .map_err(table_err_to_string)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dice_notation_info() {
        // This would need to run through parse_dice_notation command
        // For unit testing, we test the underlying types
        let notation = DiceNotation::parse("2d6+3").unwrap();
        assert_eq!(notation.count, 2);
        assert_eq!(notation.sides(), 6);
        assert_eq!(notation.modifier, 3);
    }
}
