//! Combat Condition Commands
//!
//! Commands for managing conditions on combatants: add, remove, tick, and templates.

use serde::{Deserialize, Serialize};
use tauri::State;
use crate::commands::AppState;
use crate::core::session::conditions::{
    AdvancedCondition, ConditionDuration, ConditionTemplates, SaveTiming,
};

// ============================================================================
// Request Types
// ============================================================================

/// Request payload for adding a condition with full options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddConditionRequest {
    pub session_id: String,
    pub combatant_id: String,
    pub condition_name: String,
    pub duration_type: Option<String>,
    pub duration_value: Option<u32>,
    pub source_id: Option<String>,
    pub source_name: Option<String>,
    pub save_type: Option<String>,
    pub save_dc: Option<u32>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse duration from request parameters
fn parse_condition_duration(
    duration_type: Option<String>,
    duration_value: Option<u32>,
    save_type: Option<String>,
    save_dc: Option<u32>,
) -> Option<ConditionDuration> {
    let duration_type = duration_type?;
    match duration_type.as_str() {
        "turns" => Some(ConditionDuration::Turns(duration_value.unwrap_or(1))),
        "rounds" => Some(ConditionDuration::Rounds(duration_value.unwrap_or(1))),
        "minutes" => Some(ConditionDuration::Minutes(duration_value.unwrap_or(1))),
        "hours" => Some(ConditionDuration::Hours(duration_value.unwrap_or(1))),
        "end_of_next_turn" => Some(ConditionDuration::EndOfNextTurn),
        "start_of_next_turn" => Some(ConditionDuration::StartOfNextTurn),
        "end_of_source_turn" => Some(ConditionDuration::EndOfSourceTurn),
        "until_save" => Some(ConditionDuration::UntilSave {
            save_type: save_type.unwrap_or_else(|| "CON".to_string()),
            dc: save_dc.unwrap_or(10),
            timing: SaveTiming::EndOfTurn,
        }),
        "until_removed" => Some(ConditionDuration::UntilRemoved),
        "permanent" => Some(ConditionDuration::Permanent),
        _ => None,
    }
}

// ============================================================================
// Basic Condition Commands
// ============================================================================

/// Add a pre-defined condition to a combatant
#[tauri::command]
pub fn add_condition(
    session_id: String,
    combatant_id: String,
    condition_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager
        .add_condition_by_name(&session_id, &combatant_id, &condition_name, None, None, None)
        .map_err(|e| e.to_string())
}

/// Remove a condition by name from a combatant
#[tauri::command]
pub fn remove_condition(
    session_id: String,
    combatant_id: String,
    condition_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager
        .remove_advanced_condition_by_name(&session_id, &combatant_id, &condition_name)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ============================================================================
// Advanced Condition Commands
// ============================================================================

/// Add a condition with full control over duration and save mechanics
#[tauri::command]
pub fn add_condition_advanced(
    request: AddConditionRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let duration = parse_condition_duration(
        request.duration_type,
        request.duration_value,
        request.save_type,
        request.save_dc,
    );

    // Try to get a standard condition template, or create a custom one
    let mut condition = ConditionTemplates::by_name(&request.condition_name)
        .unwrap_or_else(|| {
            AdvancedCondition::new(
                &request.condition_name,
                format!("Custom condition: {}", request.condition_name),
                duration.clone().unwrap_or(ConditionDuration::UntilRemoved),
            )
        });

    // Override duration if specified
    if let Some(dur) = duration {
        condition.duration = dur.clone();
        condition.remaining = match &dur {
            ConditionDuration::Turns(n) => Some(*n),
            ConditionDuration::Rounds(n) => Some(*n),
            ConditionDuration::Minutes(n) => Some(*n),
            ConditionDuration::Hours(n) => Some(*n),
            _ => None,
        };
    }

    // Set source if provided
    if let (Some(src_id), Some(src_name)) = (request.source_id, request.source_name) {
        condition.source_id = Some(src_id);
        condition.source_name = Some(src_name);
    }

    state.session_manager.add_advanced_condition(
        &request.session_id,
        &request.combatant_id,
        condition,
    ).map_err(|e| e.to_string())
}

/// Remove a condition by its unique ID
#[tauri::command]
pub fn remove_condition_by_id(
    session_id: String,
    combatant_id: String,
    condition_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager.remove_advanced_condition(&session_id, &combatant_id, &condition_id)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Apply an advanced condition to a combatant (alternative API)
#[tauri::command]
pub fn apply_advanced_condition(
    session_id: String,
    combatant_id: String,
    condition_name: String,
    duration_type: Option<String>,
    duration_value: Option<u32>,
    source_id: Option<String>,
    source_name: Option<String>,
    state: State<'_, AppState>,
) -> Result<AdvancedCondition, String> {
    // Try to get a template condition first
    let mut condition = ConditionTemplates::by_name(&condition_name)
        .unwrap_or_else(|| {
            // Create a custom condition
            let duration = match duration_type.as_deref() {
                Some("turns") => ConditionDuration::Turns(duration_value.unwrap_or(1)),
                Some("rounds") => ConditionDuration::Rounds(duration_value.unwrap_or(1)),
                Some("minutes") => ConditionDuration::Minutes(duration_value.unwrap_or(1)),
                Some("hours") => ConditionDuration::Hours(duration_value.unwrap_or(1)),
                Some("end_of_turn") => ConditionDuration::EndOfNextTurn,
                Some("start_of_turn") => ConditionDuration::StartOfNextTurn,
                _ => ConditionDuration::UntilRemoved,
            };
            AdvancedCondition::new(&condition_name, "Custom condition", duration)
        });

    // Set source if provided
    if let (Some(sid), Some(sname)) = (source_id, source_name) {
        condition = condition.from_source(sid, sname);
    }

    // Apply to combatant
    state.session_manager.add_advanced_condition(&session_id, &combatant_id, condition.clone())
        .map_err(|e| e.to_string())?;

    Ok(condition)
}

/// Remove an advanced condition from a combatant
#[tauri::command]
pub fn remove_advanced_condition(
    session_id: String,
    combatant_id: String,
    condition_id: String,
    state: State<'_, AppState>,
) -> Result<Option<AdvancedCondition>, String> {
    state.session_manager.remove_advanced_condition(&session_id, &combatant_id, &condition_id)
        .map_err(|e| e.to_string())
}

/// Get all conditions for a combatant
#[tauri::command]
pub fn get_combatant_conditions(
    session_id: String,
    combatant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<AdvancedCondition>, String> {
    state.session_manager.get_combatant_conditions(&session_id, &combatant_id)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Condition Tick Commands
// ============================================================================

/// Tick conditions at end of turn (decrements durations, removes expired)
#[tauri::command]
pub fn tick_conditions_end_of_turn(
    session_id: String,
    combatant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state.session_manager.tick_conditions_end_of_turn(&session_id, &combatant_id)
        .map_err(|e| e.to_string())
}

/// Tick conditions at start of turn (processes start-of-turn effects)
#[tauri::command]
pub fn tick_conditions_start_of_turn(
    session_id: String,
    combatant_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state.session_manager.tick_conditions_start_of_turn(&session_id, &combatant_id)
        .map_err(|e| e.to_string())
}

// ============================================================================
// Condition Templates
// ============================================================================

/// List all available condition template names
#[tauri::command]
pub fn list_condition_templates() -> Vec<String> {
    ConditionTemplates::list_names().iter().map(|s| s.to_string()).collect()
}
