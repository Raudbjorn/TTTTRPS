//! Vocabulary Bank Commands
//!
//! Commands for managing NPC dialogue vocabulary banks.

use tauri::State;

use crate::core::archetype::{
    VocabularyBank, BankListFilter, PhraseFilterOptions,
    setting_pack::{VocabularyBankDefinition, PhraseDefinition},
};
use crate::commands::AppState;
use super::types::{
    CreateVocabularyBankRequest, VocabularyBankResponse, VocabularyBankSummaryResponse,
    PhraseOutput, PhraseFilterRequest, get_vocabulary_manager,
};

// ============================================================================
// TASK-ARCH-061: Vocabulary Bank Commands
// ============================================================================

/// Create a new vocabulary bank.
///
/// # Arguments
/// * `request` - Vocabulary bank creation request
///
/// # Returns
/// The ID of the created vocabulary bank.
#[tauri::command]
pub async fn create_vocabulary_bank(
    request: CreateVocabularyBankRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let manager = get_vocabulary_manager(&state).await?;

    // Build VocabularyBankDefinition
    let mut definition = VocabularyBankDefinition::new(&request.id, &request.name);

    if let Some(desc) = request.description {
        definition.description = Some(desc);
    }

    if let Some(culture) = request.culture {
        definition.culture = Some(culture);
    }

    if let Some(role) = request.role {
        definition.role = Some(role);
    }

    // Group phrases by category and add to definition
    let mut phrase_groups: std::collections::HashMap<String, Vec<PhraseDefinition>> = std::collections::HashMap::new();
    for phrase in request.phrases {
        let mut phrase_def = PhraseDefinition::new(&phrase.text);
        phrase_def.formality = phrase.formality;
        if let Some(tones) = phrase.tones {
            phrase_def.tone_markers = tones;
        }
        phrase_def.context_tags = phrase.tags;

        phrase_groups
            .entry(phrase.category)
            .or_default()
            .push(phrase_def);
    }
    definition.phrases = phrase_groups;

    // Create VocabularyBank from definition
    let bank = VocabularyBank::from_definition(definition);

    let id = manager.register(bank).await
        .map_err(|e| e.to_string())?;

    log::info!("Created vocabulary bank: {}", id);
    Ok(id)
}

/// Get a vocabulary bank by ID.
///
/// # Arguments
/// * `id` - The vocabulary bank ID
///
/// # Returns
/// The full vocabulary bank data.
#[tauri::command]
pub async fn get_vocabulary_bank(
    id: String,
    state: State<'_, AppState>,
) -> Result<VocabularyBankResponse, String> {
    let manager = get_vocabulary_manager(&state).await?;

    let bank = manager.get_bank(&id).await
        .map_err(|e| e.to_string())?;

    // Flatten phrases from HashMap<String, Vec<PhraseDefinition>> to Vec<PhraseOutput>
    let phrases: Vec<PhraseOutput> = bank.definition.phrases
        .iter()
        .flat_map(|(category, phrase_list)| {
            phrase_list.iter().map(move |p| PhraseOutput {
                text: p.text.clone(),
                category: category.clone(),
                formality: p.formality,
                tones: p.tone_markers.clone(),
                tags: p.context_tags.clone(),
            })
        })
        .collect();

    Ok(VocabularyBankResponse {
        id: bank.definition.id.clone(),
        name: bank.definition.display_name.clone(),
        description: bank.definition.description.clone(),
        culture: bank.definition.culture.clone(),
        role: bank.definition.role.clone(),
        phrases,
    })
}

/// List all vocabulary banks with optional filtering.
///
/// # Arguments
/// * `culture` - Optional culture filter
/// * `role` - Optional role filter
///
/// # Returns
/// List of vocabulary bank summaries.
#[tauri::command]
pub async fn list_vocabulary_banks(
    culture: Option<String>,
    role: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<VocabularyBankSummaryResponse>, String> {
    let manager = get_vocabulary_manager(&state).await?;

    let filter = BankListFilter {
        culture,
        role,
        race: None,
        builtin_only: None,
    };

    let summaries = manager.list_banks(Some(filter)).await;

    Ok(summaries.into_iter().map(VocabularyBankSummaryResponse::from).collect())
}

/// Update an existing vocabulary bank.
///
/// # Arguments
/// * `request` - Vocabulary bank update request (must have existing ID)
#[tauri::command]
pub async fn update_vocabulary_bank(
    request: CreateVocabularyBankRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = get_vocabulary_manager(&state).await?;

    // Build VocabularyBankDefinition
    let mut definition = VocabularyBankDefinition::new(&request.id, &request.name);

    if let Some(desc) = request.description {
        definition.description = Some(desc);
    }

    if let Some(culture) = request.culture {
        definition.culture = Some(culture);
    }

    if let Some(role) = request.role {
        definition.role = Some(role);
    }

    // Group phrases by category and add to definition
    let mut phrase_groups: std::collections::HashMap<String, Vec<PhraseDefinition>> = std::collections::HashMap::new();
    for phrase in request.phrases {
        let mut phrase_def = PhraseDefinition::new(&phrase.text);
        phrase_def.formality = phrase.formality;
        if let Some(tones) = phrase.tones {
            phrase_def.tone_markers = tones;
        }
        phrase_def.context_tags = phrase.tags;

        phrase_groups
            .entry(phrase.category)
            .or_default()
            .push(phrase_def);
    }
    definition.phrases = phrase_groups;

    // Create VocabularyBank from definition
    let bank = VocabularyBank::from_definition(definition);

    manager.update(bank).await
        .map_err(|e| e.to_string())?;

    log::info!("Updated vocabulary bank: {}", request.id);
    Ok(())
}

/// Delete a vocabulary bank.
///
/// # Arguments
/// * `id` - The vocabulary bank ID to delete
///
/// # Errors
/// - If vocabulary bank doesn't exist
/// - If vocabulary bank is in use by archetypes
#[tauri::command]
pub async fn delete_vocabulary_bank(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = get_vocabulary_manager(&state).await?;

    manager.delete_bank(&id).await
        .map_err(|e| e.to_string())?;

    log::info!("Deleted vocabulary bank: {}", id);
    Ok(())
}

/// Get phrases from a vocabulary bank with optional filtering.
///
/// This command returns just the phrase text strings, filtered by category,
/// formality range, and tone. It uses session-based tracking to avoid
/// repeating the same phrase.
///
/// # Arguments
/// * `bank_id` - The vocabulary bank ID
/// * `category` - Required category to filter by (e.g., "greetings")
/// * `filter` - Optional additional filters for formality and tone
/// * `session_id` - Session ID for usage tracking (prevents repeating phrases)
///
/// # Returns
/// List of matching phrase texts.
#[tauri::command]
pub async fn get_phrases(
    bank_id: String,
    category: String,
    filter: Option<PhraseFilterRequest>,
    session_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let manager = get_vocabulary_manager(&state).await?;

    // Build filter options starting with category (required)
    let mut opts = PhraseFilterOptions::for_category(&category);

    if let Some(f) = filter {
        if let (Some(min), Some(max)) = (f.formality_min, f.formality_max) {
            opts = opts.with_formality(min, max);
        }
        if let Some(tone) = f.tone {
            opts = opts.with_tone(&tone);
        }
    }

    // Use provided session_id or a sentinel value for ephemeral/one-off requests.
    // NOTE: "ephemeral" indicates no session tracking - phrase usage won't be tracked
    // across requests. Use a real session_id for consistent phrase avoidance.
    let session = session_id.unwrap_or_else(|| "ephemeral".to_string());

    let phrases = manager.get_phrases(&bank_id, opts, &session).await
        .map_err(|e| e.to_string())?;

    Ok(phrases)
}
