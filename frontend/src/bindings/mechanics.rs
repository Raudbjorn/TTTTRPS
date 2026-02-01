use serde::{Deserialize, Serialize};
use super::core::{invoke, invoke_void, invoke_no_args};

// ============================================================================
// Combat Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatState {
    pub id: String,
    pub round: u32,
    pub current_turn: usize,
    pub combatants: Vec<Combatant>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combatant {
    pub id: String,
    pub name: String,
    pub initiative: i32,
    #[serde(alias = "current_hp")]
    pub hp_current: i32,
    #[serde(alias = "max_hp")]
    pub hp_max: i32,
    #[serde(alias = "armor_class")]
    pub ac: Option<i32>,
    #[serde(alias = "temp_hp")]
    pub hp_temp: Option<i32>,
    pub combatant_type: String,
    pub conditions: Vec<String>,
    pub is_active: bool,
}

// ============================================================================
// Combat Commands
// ============================================================================

pub async fn start_combat(session_id: String) -> Result<CombatState, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("start_combat", &Args { session_id }).await
}

pub async fn end_combat(session_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke_void("end_combat", &Args { session_id }).await
}

pub async fn get_combat(session_id: String) -> Result<Option<CombatState>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("get_combat", &Args { session_id }).await
}

pub async fn add_combatant(
    session_id: String,
    name: String,
    initiative: i32,
    combatant_type: String,
) -> Result<Combatant, String> {
    add_combatant_full(session_id, name, initiative, combatant_type, None, None, None).await
}

pub async fn add_combatant_full(
    session_id: String,
    name: String,
    initiative: i32,
    combatant_type: String,
    hp_current: Option<i32>,
    hp_max: Option<i32>,
    armor_class: Option<i32>,
) -> Result<Combatant, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        name: String,
        initiative: i32,
        combatant_type: String,
        hp_current: Option<i32>,
        hp_max: Option<i32>,
        armor_class: Option<i32>,
    }
    invoke("add_combatant", &Args { session_id, name, initiative, combatant_type, hp_current, hp_max, armor_class }).await
}

pub async fn remove_combatant(session_id: String, combatant_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
    }
    invoke_void("remove_combatant", &Args { session_id, combatant_id }).await
}

pub async fn next_turn(session_id: String) -> Result<Option<Combatant>, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
    }
    invoke("next_turn", &Args { session_id }).await
}

pub async fn damage_combatant(session_id: String, combatant_id: String, amount: i32) -> Result<i32, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
        amount: i32,
    }
    invoke("damage_combatant", &Args { session_id, combatant_id, amount }).await
}

pub async fn heal_combatant(session_id: String, combatant_id: String, amount: i32) -> Result<i32, String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
        amount: i32,
    }
    invoke("heal_combatant", &Args { session_id, combatant_id, amount }).await
}

pub async fn add_condition(session_id: String, combatant_id: String, condition_name: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
        condition_name: String,
    }
    invoke_void("add_condition", &Args { session_id, combatant_id, condition_name }).await
}

pub async fn remove_condition(session_id: String, combatant_id: String, condition_name: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args {
        session_id: String,
        combatant_id: String,
        condition_name: String,
    }
    invoke_void("remove_condition", &Args { session_id, combatant_id, condition_name }).await
}

// ============================================================================
// Advanced Conditions
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConditionDurationType {
    Turns,
    Rounds,
    Minutes,
    Hours,
    #[serde(alias = "EndOfNextTurn")]
    EndOfNextTurn,
    #[serde(alias = "StartOfNextTurn")]
    StartOfNextTurn,
    #[serde(alias = "EndOfSourceTurn")]
    EndOfSourceTurn,
    #[serde(alias = "UntilSave")]
    UntilSave,
    #[default]
    #[serde(alias = "UntilRemoved")]
    UntilRemoved,
    #[serde(alias = "Permanent")]
    Permanent,
}

impl ConditionDurationType {
    pub fn to_string_key(&self) -> &'static str {
        match self {
            Self::Turns => "turns",
            Self::Rounds => "rounds",
            Self::Minutes => "minutes",
            Self::Hours => "hours",
            Self::EndOfNextTurn => "end_of_next_turn",
            Self::StartOfNextTurn => "start_of_next_turn",
            Self::EndOfSourceTurn => "end_of_source_turn",
            Self::UntilSave => "until_save",
            Self::UntilRemoved => "until_removed",
            Self::Permanent => "permanent",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Turns => "Turns",
            Self::Rounds => "Rounds",
            Self::Minutes => "Minutes",
            Self::Hours => "Hours",
            Self::EndOfNextTurn => "End of Next Turn",
            Self::StartOfNextTurn => "Start of Next Turn",
            Self::EndOfSourceTurn => "End of Source's Turn",
            Self::UntilSave => "Until Save",
            Self::UntilRemoved => "Until Removed",
            Self::Permanent => "Permanent",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Turns,
            Self::Rounds,
            Self::Minutes,
            Self::EndOfNextTurn,
            Self::StartOfNextTurn,
            Self::UntilSave,
            Self::UntilRemoved,
            Self::Permanent,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionEffect {
    pub description: String,
    pub mechanic: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedCondition {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub effects: Vec<ConditionEffect>,
    #[serde(default)]
    pub duration_type: ConditionDurationType,
    pub remaining: Option<u32>,
    pub source_id: Option<String>,
    pub source_name: Option<String>,
    pub save_type: Option<String>,
    pub save_dc: Option<u32>,
    pub applied_at_round: Option<u32>,
    pub applied_at_turn: Option<usize>,
}

impl AdvancedCondition {
    pub fn duration_display(&self) -> String {
        match self.remaining {
            Some(n) if n > 0 => format!("{} {}", n, self.duration_type.display_name()),
            None => match self.duration_type {
                ConditionDurationType::UntilSave => {
                    if let (Some(save_type), Some(dc)) = (&self.save_type, self.save_dc) {
                        format!("Until {} save (DC {})", save_type, dc)
                    } else {
                        "Until Save".to_string()
                    }
                }
                _ => self.duration_type.display_name().to_string(),
            },
            _ => "Expired".to_string(),
        }
    }
}

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

pub async fn add_condition_advanced(request: AddConditionRequest) -> Result<(), String> {
    invoke_void("add_condition_advanced", &request).await
}

pub async fn remove_condition_by_id(session_id: String, combatant_id: String, condition_id: String) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args { session_id: String, combatant_id: String, condition_id: String }
    invoke_void("remove_condition_by_id", &Args { session_id, combatant_id, condition_id }).await
}

pub async fn get_combatant_conditions(session_id: String, combatant_id: String) -> Result<Vec<AdvancedCondition>, String> {
    #[derive(Serialize)]
    struct Args { session_id: String, combatant_id: String }
    invoke("get_combatant_conditions", &Args { session_id, combatant_id }).await
}

pub async fn tick_conditions_end_of_turn(session_id: String, combatant_id: String) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { session_id: String, combatant_id: String }
    invoke("tick_conditions_end_of_turn", &Args { session_id, combatant_id }).await
}

pub async fn tick_conditions_start_of_turn(session_id: String, combatant_id: String) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args { session_id: String, combatant_id: String }
    invoke("tick_conditions_start_of_turn", &Args { session_id, combatant_id }).await
}

pub async fn list_condition_templates() -> Result<Vec<String>, String> {
    invoke_no_args("list_condition_templates").await
}

pub const STANDARD_CONDITIONS: &[&str] = &[
    "Blinded", "Charmed", "Deafened", "Exhaustion", "Frightened", "Grappled",
    "Incapacitated", "Invisible", "Paralyzed", "Petrified", "Poisoned",
    "Prone", "Restrained", "Stunned", "Unconscious",
];

pub fn get_condition_description(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "blinded" => Some("Can't see. Auto-fails sight checks. Attacks have advantage against, disadvantage on attacks."),
        "charmed" => Some("Can't attack charmer. Charmer has advantage on social checks."),
        "deafened" => Some("Can't hear. Auto-fails hearing checks."),
        "exhaustion" => Some("Cumulative levels with increasing penalties."),
        "frightened" => Some("Disadvantage on checks/attacks while fear source visible. Can't move closer."),
        "grappled" => Some("Speed 0. Ends if grappler incapacitated or removed from reach."),
        "incapacitated" => Some("Can't take actions or reactions."),
        "invisible" => Some("Can't be seen. Attacks against have disadvantage, attacks have advantage."),
        "paralyzed" => Some("Incapacitated, can't move/speak. Auto-fail STR/DEX saves. Attacks have advantage, crits in 5ft."),
        "petrified" => Some("Transformed to stone. Incapacitated, resistant to damage, immune to poison/disease."),
        "poisoned" => Some("Disadvantage on attacks and ability checks."),
        "prone" => Some("Can only crawl. Disadvantage on attacks. Advantage/disadvantage based on distance."),
        "restrained" => Some("Speed 0. Attacks against have advantage. Disadvantage on attacks and DEX saves."),
        "stunned" => Some("Incapacitated, can't move. Auto-fail STR/DEX saves. Attacks have advantage."),
        "unconscious" => Some("Incapacitated, drops items, falls prone. Auto-fail STR/DEX. Attacks have advantage, crits in 5ft."),
        _ => None,
    }
}

// ============================================================================
// Character Generation
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub system: String,
    pub concept: String,
    pub race: Option<String>,
    #[serde(rename = "class")]
    pub character_class: Option<String>,
    pub level: u32,
    pub attributes: std::collections::HashMap<String, CharacterAttributeValue>,
    pub skills: std::collections::HashMap<String, i32>,
    pub traits: Vec<CharacterTrait>,
    pub equipment: Vec<CharacterEquipment>,
    pub background: CharacterBackground,
    pub backstory: Option<String>,
    pub notes: String,
    pub portrait_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterAttributeValue {
    pub base: i32,
    pub modifier: i32,
    pub temp_bonus: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTrait {
    pub name: String,
    pub trait_type: String,
    pub description: String,
    pub mechanical_effect: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterEquipment {
    pub name: String,
    pub category: String,
    pub description: String,
    pub stats: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CharacterBackground {
    pub origin: String,
    pub occupation: Option<String>,
    pub motivation: String,
    pub connections: Vec<String>,
    pub secrets: Vec<String>,
    pub history: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeValue {
    pub name: String,
    pub value: i32,
    pub modifier: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GenerationOptions {
    pub system: Option<String>,
    pub name: Option<String>,
    pub concept: Option<String>,
    pub race: Option<String>,
    #[serde(rename = "class")]
    pub character_class: Option<String>,
    pub background: Option<String>,
    pub level: Option<u32>,
    pub point_buy: Option<u32>,
    pub random_stats: bool,
    pub include_equipment: bool,
    pub include_backstory: bool,
    pub backstory_length: Option<String>,
    pub theme: Option<String>,
    pub campaign_setting: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSystemInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub races: Vec<String>,
    pub classes: Vec<String>,
    pub backgrounds: Vec<String>,
    pub attributes: Vec<String>,
    pub has_levels: bool,
    pub max_level: Option<u32>,
}

pub async fn generate_character(options: GenerationOptions) -> Result<Character, String> {
    #[derive(Serialize)]
    struct Args {
        options: GenerationOptions,
    }
    invoke("generate_character_advanced", &Args { options }).await
}

pub async fn generate_character_advanced(options: GenerationOptions) -> Result<Character, String> {
    #[derive(Serialize)]
    struct Args {
        options: GenerationOptions,
    }
    invoke("generate_character_advanced", &Args { options }).await
}

pub async fn get_supported_systems() -> Result<Vec<String>, String> {
    invoke_no_args("get_supported_systems").await
}

pub async fn list_system_info() -> Result<Vec<GameSystemInfo>, String> {
    invoke_no_args("list_system_info").await
}

pub async fn get_game_system_info(system: String) -> Result<Option<GameSystemInfo>, String> {
    #[derive(Serialize)]
    struct Args {
        system: String,
    }
    invoke("get_system_info", &Args { system }).await
}
