//! Personality Template Commands
//!
//! Commands for managing personality templates.

use tauri::State;

use crate::commands::AppState;
use crate::core::personality::{PersonalityId, SettingTemplate, TemplateId};

use super::types::{
    ApplyTemplateRequest, CreateTemplateFromPersonalityRequest, TemplatePreviewResponse,
};

// ============================================================================
// Template Commands
// ============================================================================

/// List all personality templates
#[tauri::command]
pub async fn list_personality_templates(
    state: State<'_, AppState>,
) -> Result<Vec<TemplatePreviewResponse>, String> {
    let templates = state.template_store.list_with_limit(1000).await.map_err(|e| e.to_string())?;
    Ok(templates.into_iter().map(TemplatePreviewResponse::from).collect())
}

/// Filter templates by game system
#[tauri::command]
pub async fn filter_templates_by_game_system(
    game_system: String,
    state: State<'_, AppState>,
) -> Result<Vec<TemplatePreviewResponse>, String> {
    let templates = state.template_store.filter_by_game_system(&game_system).map_err(|e| e.to_string())?;
    Ok(templates.into_iter().map(TemplatePreviewResponse::from).collect())
}

/// Filter templates by setting name
#[tauri::command]
pub async fn filter_templates_by_setting(
    setting_name: String,
    state: State<'_, AppState>,
) -> Result<Vec<TemplatePreviewResponse>, String> {
    let templates = state.template_store.filter_by_setting(&setting_name).map_err(|e| e.to_string())?;
    Ok(templates.into_iter().map(TemplatePreviewResponse::from).collect())
}

/// Search personality templates by keyword
#[tauri::command]
pub async fn search_personality_templates(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<TemplatePreviewResponse>, String> {
    let templates = state.template_store.search_with_limit(&query, 100).map_err(|e| e.to_string())?;
    Ok(templates.into_iter().map(TemplatePreviewResponse::from).collect())
}

/// Get template preview by ID
#[tauri::command]
pub async fn get_template_preview(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<Option<TemplatePreviewResponse>, String> {
    let id = TemplateId::new(template_id);
    let template = state.template_store.get(&id).await.map_err(|e| e.to_string())?;
    Ok(template.map(TemplatePreviewResponse::from))
}

/// Apply a template to a campaign
#[tauri::command]
pub async fn apply_template_to_campaign(
    request: ApplyTemplateRequest,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let id = TemplateId::new(&request.template_id);

    // Get the template
    let template = state.template_store.get(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Template not found: {}", request.template_id))?;

    // Get the base profile that the template extends
    let base_profile = state.personality_store.get(template.base_profile.as_str())
        .map_err(|e| format!("Base profile not found: {}", e))?;

    // Convert template to profile by applying overrides to base
    let profile = template.to_personality_profile(&base_profile);
    let profile_id = profile.id.clone();

    // Store the generated profile
    state.personality_store.create(profile)
        .map_err(|e| format!("Failed to store profile: {}", e))?;

    log::info!(
        "Applied template '{}' to campaign '{}', created profile '{}'",
        template.name,
        request.campaign_id,
        profile_id
    );

    Ok(profile_id)
}

/// Create a template from an existing personality profile
#[tauri::command]
pub async fn create_template_from_personality(
    request: CreateTemplateFromPersonalityRequest,
    state: State<'_, AppState>,
) -> Result<TemplatePreviewResponse, String> {
    // Get the source personality
    let profile = state.personality_store.get(&request.personality_id)
        .map_err(|e| format!("Personality not found: {}", e))?;

    // Create template from profile using the builder
    let mut template = SettingTemplate::new(&request.name, PersonalityId::new(&request.personality_id));
    template.description = request.description;
    template.game_system = request.game_system;
    template.setting_name = request.setting_name;

    // Copy common phrases from the profile
    template.common_phrases = profile.speech_patterns.common_phrases.clone();

    // Copy tags
    template.tags = profile.tags.clone();

    // Save template
    state.template_store.save(&template).await.map_err(|e| e.to_string())?;

    log::info!("Created template '{}' from personality '{}'", template.name, request.personality_id);

    Ok(TemplatePreviewResponse::from(template))
}

/// Export a personality template to YAML
#[tauri::command]
pub async fn export_personality_template(
    template_id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let id = TemplateId::new(template_id);

    let template = state.template_store.get(&id).await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())?;

    // Convert to YAML
    serde_yaml_ng::to_string(&template).map_err(|e| format!("YAML serialization failed: {}", e))
}

/// Import a personality template from YAML
#[tauri::command]
pub async fn import_personality_template(
    yaml_content: String,
    state: State<'_, AppState>,
) -> Result<TemplatePreviewResponse, String> {
    // Parse YAML
    let template: SettingTemplate = serde_yaml_ng::from_str(&yaml_content)
        .map_err(|e| format!("YAML parse failed: {}", e))?;

    // Save template
    state.template_store.save(&template).await.map_err(|e| e.to_string())?;

    log::info!("Imported template '{}'", template.name);

    Ok(TemplatePreviewResponse::from(template))
}
