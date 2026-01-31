//! Combatant Management Commands
//!
//! Commands for managing combatants: add, remove, damage, heal, and initiative.

use tauri::State;
use crate::commands::AppState;
use crate::core::session_manager::{Combatant, CombatantType};

/// Add a combatant to the current combat
#[tauri::command]
pub fn add_combatant(
    session_id: String,
    name: String,
    initiative: i32,
    combatant_type: String,
    hp_current: Option<i32>,
    hp_max: Option<i32>,
    armor_class: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Combatant, String> {
    let ctype = match combatant_type.as_str() {
        "player" => CombatantType::Player,
        "npc" => CombatantType::NPC,
        "monster" => CombatantType::Monster,
        "ally" => CombatantType::Ally,
        _ => return Err(format!(
            "Unknown combatant type: '{}'. Valid types: player, npc, monster, ally",
            combatant_type
        )),
    };

    // Create full combatant with optional HP/AC
    let mut combatant = Combatant::new(name.clone(), initiative, ctype);
    combatant.current_hp = hp_current.or(hp_max);
    combatant.max_hp = hp_max;
    combatant.armor_class = armor_class;

    state.session_manager.add_combatant(&session_id, combatant.clone())
        .map_err(|e| e.to_string())?;

    Ok(combatant)
}

/// Remove a combatant from combat
#[tauri::command]
pub fn remove_combatant(
    session_id: String,
    combatant_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.session_manager.remove_combatant(&session_id, &combatant_id)
        .map_err(|e| e.to_string())
}

/// Advance to the next turn in initiative order
#[tauri::command]
pub fn next_turn(session_id: String, state: State<'_, AppState>) -> Result<Option<Combatant>, String> {
    state.session_manager.next_turn(&session_id)
        .map_err(|e| e.to_string())
}

/// Get the current combatant (whose turn it is)
#[tauri::command]
pub fn get_current_combatant(session_id: String, state: State<'_, AppState>) -> Result<Option<Combatant>, String> {
    Ok(state.session_manager.get_current_combatant(&session_id))
}

/// Apply damage to a combatant
#[tauri::command]
pub fn damage_combatant(
    session_id: String,
    combatant_id: String,
    amount: i32,
    state: State<'_, AppState>,
) -> Result<i32, String> {
    if amount < 0 {
        return Err("Damage amount cannot be negative. Use heal_combatant for healing.".to_string());
    }
    state.session_manager.damage_combatant(&session_id, &combatant_id, amount)
        .map_err(|e| e.to_string())
}

/// Heal a combatant
#[tauri::command]
pub fn heal_combatant(
    session_id: String,
    combatant_id: String,
    amount: i32,
    state: State<'_, AppState>,
) -> Result<i32, String> {
    if amount < 0 {
        return Err("Heal amount cannot be negative. Use damage_combatant for damage.".to_string());
    }
    state.session_manager.heal_combatant(&session_id, &combatant_id, amount)
        .map_err(|e| e.to_string())
}
