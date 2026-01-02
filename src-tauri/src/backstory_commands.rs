//! Backstory Generation Commands (TASK-019)
//!
//! Tauri commands for AI-powered character backstory generation.

use tauri::State;
use serde::{Deserialize, Serialize};

use crate::commands::AppState;
use crate::core::character_gen::{
    Character as CharGen, GameSystem, CharacterBackground, CharacterTrait, TraitType,
    BackstoryLength,
};
use crate::core::character_gen::backstory::{
    BackstoryGenerator, BackstoryRequest, GeneratedBackstory,
    BackstoryStyle, RegenerationOptions, BackstoryNPC,
};

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request payload for generating a backstory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateBackstoryRequest {
    /// Character data (can be a full character or minimal info)
    pub character: BackstoryCharacterInfo,
    /// Desired length: "brief", "medium", "detailed"
    #[serde(default = "default_length")]
    pub length: String,
    /// Campaign setting description for style matching
    pub campaign_setting: Option<String>,
    /// Style preferences
    #[serde(default)]
    pub style: BackstoryStylePayload,
    /// Elements to include in the backstory
    #[serde(default)]
    pub include_elements: Vec<String>,
    /// Elements to avoid in the backstory
    #[serde(default)]
    pub exclude_elements: Vec<String>,
}

fn default_length() -> String {
    "medium".to_string()
}

/// Simplified character info for backstory generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackstoryCharacterInfo {
    pub id: Option<String>,
    pub name: String,
    pub system: String,
    pub race: Option<String>,
    pub class: Option<String>,
    pub level: Option<u32>,
    pub concept: Option<String>,
    pub background: Option<String>,
    pub motivation: Option<String>,
    pub traits: Option<Vec<String>>,
}

/// Style payload from frontend
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackstoryStylePayload {
    /// Tone: "heroic", "tragic", "comedic", "mysterious", "gritty", "dark", "epic"
    pub tone: Option<String>,
    /// Perspective: "first_person", "third_person", "journal"
    pub perspective: Option<String>,
    /// Focus: "personal", "political", "adventurous", "philosophical", "professional"
    pub focus: Option<String>,
    /// Custom instructions
    pub custom_instructions: Option<String>,
}

/// Response payload for generated backstory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedBackstoryPayload {
    pub text: String,
    pub summary: String,
    pub key_events: Vec<String>,
    pub mentioned_npcs: Vec<BackstoryNPCPayload>,
    pub mentioned_locations: Vec<String>,
    pub plot_hooks: Vec<String>,
    pub suggested_traits: Vec<String>,
    pub model: Option<String>,
    pub tokens_used: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackstoryNPCPayload {
    pub name: String,
    pub relationship: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackstoryLengthOption {
    pub id: String,
    pub name: String,
    pub description: String,
    pub word_range: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackstoryStyleOptions {
    pub tones: Vec<StyleOption>,
    pub perspectives: Vec<StyleOption>,
    pub focuses: Vec<StyleOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleOption {
    pub id: String,
    pub name: String,
    pub description: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

fn convert_to_character(info: &BackstoryCharacterInfo) -> CharGen {
    let system = GameSystem::from_str(&info.system);

    let mut traits = Vec::new();
    if let Some(trait_list) = &info.traits {
        for trait_name in trait_list {
            traits.push(CharacterTrait {
                name: trait_name.clone(),
                trait_type: TraitType::Personality,
                description: trait_name.clone(),
                mechanical_effect: None,
            });
        }
    }

    CharGen {
        id: info.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        name: info.name.clone(),
        system,
        concept: info.concept.clone().unwrap_or_default(),
        race: info.race.clone(),
        class: info.class.clone(),
        level: info.level.unwrap_or(1),
        attributes: std::collections::HashMap::new(),
        skills: std::collections::HashMap::new(),
        traits,
        equipment: vec![],
        background: CharacterBackground {
            origin: info.background.clone().unwrap_or_default(),
            occupation: None,
            motivation: info.motivation.clone().unwrap_or_default(),
            connections: vec![],
            secrets: vec![],
            history: String::new(),
        },
        backstory: None,
        notes: String::new(),
        portrait_prompt: None,
    }
}

fn payload_to_backstory(payload: GeneratedBackstoryPayload) -> GeneratedBackstory {
    GeneratedBackstory {
        text: payload.text,
        summary: payload.summary,
        key_events: payload.key_events,
        mentioned_npcs: payload.mentioned_npcs.into_iter()
            .map(|npc| BackstoryNPC {
                name: npc.name,
                relationship: npc.relationship,
                status: npc.status,
            })
            .collect(),
        mentioned_locations: payload.mentioned_locations,
        plot_hooks: payload.plot_hooks,
        suggested_traits: payload.suggested_traits,
        metadata: Default::default(),
    }
}

fn backstory_to_payload(result: GeneratedBackstory) -> GeneratedBackstoryPayload {
    GeneratedBackstoryPayload {
        text: result.text,
        summary: result.summary,
        key_events: result.key_events,
        mentioned_npcs: result.mentioned_npcs.into_iter()
            .map(|npc| BackstoryNPCPayload {
                name: npc.name,
                relationship: npc.relationship,
                status: npc.status,
            })
            .collect(),
        mentioned_locations: result.mentioned_locations,
        plot_hooks: result.plot_hooks,
        suggested_traits: result.suggested_traits,
        model: result.metadata.model,
        tokens_used: result.metadata.tokens_used,
    }
}

fn parse_length(length: &str) -> BackstoryLength {
    match length.to_lowercase().as_str() {
        "brief" | "short" => BackstoryLength::Brief,
        "detailed" | "long" => BackstoryLength::Detailed,
        _ => BackstoryLength::Medium,
    }
}

// ============================================================================
// Tauri Commands
// ============================================================================

/// Generate a backstory for a character
#[tauri::command]
pub async fn generate_backstory(
    request: GenerateBackstoryRequest,
    state: State<'_, AppState>,
) -> Result<GeneratedBackstoryPayload, String> {
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured. Please configure in Settings.")?;

    let generator = BackstoryGenerator::new(config);
    let length = parse_length(&request.length);
    let character = convert_to_character(&request.character);

    let backstory_request = BackstoryRequest {
        character,
        length,
        campaign_setting: request.campaign_setting,
        style: BackstoryStyle {
            tone: request.style.tone,
            perspective: request.style.perspective,
            focus: request.style.focus,
            custom_instructions: request.style.custom_instructions,
        },
        include_elements: request.include_elements,
        exclude_elements: request.exclude_elements,
    };

    let result = generator.generate(&backstory_request).await
        .map_err(|e| e.to_string())?;

    Ok(backstory_to_payload(result))
}

/// Generate multiple backstory variations for selection
#[tauri::command]
pub async fn generate_backstory_variations(
    request: GenerateBackstoryRequest,
    count: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<GeneratedBackstoryPayload>, String> {
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured. Please configure in Settings.")?;

    let generator = BackstoryGenerator::new(config);
    let count = count.unwrap_or(3).min(5);
    let length = parse_length(&request.length);
    let character = convert_to_character(&request.character);

    let backstory_request = BackstoryRequest {
        character,
        length,
        campaign_setting: request.campaign_setting,
        style: BackstoryStyle {
            tone: request.style.tone,
            perspective: request.style.perspective,
            focus: request.style.focus,
            custom_instructions: request.style.custom_instructions,
        },
        include_elements: request.include_elements,
        exclude_elements: request.exclude_elements,
    };

    let results = generator.generate_variations(&backstory_request, count).await
        .map_err(|e| e.to_string())?;

    Ok(results.into_iter().map(backstory_to_payload).collect())
}

/// Regenerate a section of an existing backstory
#[tauri::command]
pub async fn regenerate_backstory_section(
    original: GeneratedBackstoryPayload,
    section: Option<String>,
    feedback: Option<String>,
    state: State<'_, AppState>,
) -> Result<GeneratedBackstoryPayload, String> {
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured. Please configure in Settings.")?;

    let generator = BackstoryGenerator::new(config);
    let original_backstory = payload_to_backstory(original);

    let options = RegenerationOptions {
        section,
        feedback,
        seed: None,
        preserve: vec![],
    };

    let result = generator.regenerate_section(&original_backstory, &options).await
        .map_err(|e| e.to_string())?;

    Ok(backstory_to_payload(result))
}

/// Edit an existing backstory based on instructions
#[tauri::command]
pub async fn edit_backstory(
    original: GeneratedBackstoryPayload,
    edit_instructions: String,
    state: State<'_, AppState>,
) -> Result<GeneratedBackstoryPayload, String> {
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured. Please configure in Settings.")?;

    let generator = BackstoryGenerator::new(config);
    let original_backstory = payload_to_backstory(original);

    let result = generator.edit_backstory(&original_backstory, &edit_instructions).await
        .map_err(|e| e.to_string())?;

    Ok(backstory_to_payload(result.backstory))
}

/// Expand a brief backstory into a longer version
#[tauri::command]
pub async fn expand_backstory(
    original: GeneratedBackstoryPayload,
    target_length: String,
    state: State<'_, AppState>,
) -> Result<GeneratedBackstoryPayload, String> {
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured. Please configure in Settings.")?;

    let generator = BackstoryGenerator::new(config);
    let target = parse_length(&target_length);
    let original_backstory = payload_to_backstory(original);

    let result = generator.expand_backstory(&original_backstory, target).await
        .map_err(|e| e.to_string())?;

    Ok(backstory_to_payload(result))
}

/// Generate additional plot hooks for a backstory
#[tauri::command]
pub async fn generate_plot_hooks(
    backstory: GeneratedBackstoryPayload,
    character_info: BackstoryCharacterInfo,
    count: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let config = state.llm_config.read().unwrap()
        .clone()
        .ok_or("LLM not configured. Please configure in Settings.")?;

    let generator = BackstoryGenerator::new(config);
    let count = count.unwrap_or(3).min(10);
    let backstory_struct = payload_to_backstory(backstory);
    let character = convert_to_character(&character_info);

    generator.generate_plot_hooks(&backstory_struct, &character, count).await
        .map_err(|e| e.to_string())
}

/// Get available backstory length options
#[tauri::command]
pub fn get_backstory_length_options() -> Vec<BackstoryLengthOption> {
    vec![
        BackstoryLengthOption {
            id: "brief".to_string(),
            name: "Brief".to_string(),
            description: "A short summary (~50-100 words, 1 paragraph)".to_string(),
            word_range: (50, 100),
        },
        BackstoryLengthOption {
            id: "medium".to_string(),
            name: "Medium".to_string(),
            description: "A standard backstory (~150-300 words, 2-3 paragraphs)".to_string(),
            word_range: (150, 300),
        },
        BackstoryLengthOption {
            id: "detailed".to_string(),
            name: "Detailed".to_string(),
            description: "A comprehensive history (~400-600 words, about 1 page)".to_string(),
            word_range: (400, 600),
        },
    ]
}

/// Get available style options for backstory generation
#[tauri::command]
pub fn get_backstory_style_options() -> BackstoryStyleOptions {
    BackstoryStyleOptions {
        tones: vec![
            StyleOption { id: "heroic".to_string(), name: "Heroic".to_string(), description: "Inspiring, triumphant narrative".to_string() },
            StyleOption { id: "tragic".to_string(), name: "Tragic".to_string(), description: "Marked by loss and sacrifice".to_string() },
            StyleOption { id: "comedic".to_string(), name: "Comedic".to_string(), description: "Lighthearted with humor".to_string() },
            StyleOption { id: "mysterious".to_string(), name: "Mysterious".to_string(), description: "Hints at secrets and unknowns".to_string() },
            StyleOption { id: "gritty".to_string(), name: "Gritty".to_string(), description: "Realistic and grounded".to_string() },
            StyleOption { id: "dark".to_string(), name: "Dark".to_string(), description: "Brooding and morally complex".to_string() },
            StyleOption { id: "epic".to_string(), name: "Epic".to_string(), description: "Grand and sweeping narrative".to_string() },
        ],
        perspectives: vec![
            StyleOption { id: "third_person".to_string(), name: "Third Person".to_string(), description: "Narrator describes the character".to_string() },
            StyleOption { id: "first_person".to_string(), name: "First Person".to_string(), description: "Character tells their own story".to_string() },
            StyleOption { id: "journal".to_string(), name: "Journal Entries".to_string(), description: "Written as diary or letters".to_string() },
        ],
        focuses: vec![
            StyleOption { id: "personal".to_string(), name: "Personal".to_string(), description: "Family and relationships".to_string() },
            StyleOption { id: "political".to_string(), name: "Political".to_string(), description: "Factions and power dynamics".to_string() },
            StyleOption { id: "adventurous".to_string(), name: "Adventurous".to_string(), description: "Action and exploration".to_string() },
            StyleOption { id: "philosophical".to_string(), name: "Philosophical".to_string(), description: "Beliefs and moral struggles".to_string() },
            StyleOption { id: "professional".to_string(), name: "Professional".to_string(), description: "Career and training".to_string() },
        ],
    }
}
