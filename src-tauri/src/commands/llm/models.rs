//! Model Listing Commands
//!
//! Commands for listing available models from various LLM providers.

use crate::core::llm::{OllamaModel, ModelInfo};

// ============================================================================
// Commands
// ============================================================================

/// List available models from an Ollama instance
#[tauri::command]
pub async fn list_ollama_models(host: String) -> Result<Vec<OllamaModel>, String> {
    crate::core::llm::LLMClient::list_ollama_models(&host)
        .await
        .map_err(|e| e.to_string())
}

/// List available Anthropic models (API Key based)
#[tauri::command]
pub async fn list_anthropic_models(api_key: Option<String>) -> Result<Vec<ModelInfo>, String> {
    if let Some(key) = api_key {
        if !key.is_empty() && !key.starts_with("*") {
            match crate::core::llm::LLMClient::list_anthropic_models(&key).await {
                Ok(models) if !models.is_empty() => return Ok(models),
                _ => {} // Fall through to fallback
            }
        }
    }
    Ok(crate::core::llm::get_fallback_models("anthropic"))
}

/// List available OpenAI models (with fallback)
#[tauri::command]
pub async fn list_openai_models(api_key: Option<String>) -> Result<Vec<ModelInfo>, String> {
    // First try OpenAI API if we have a valid key
    if let Some(key) = api_key {
        if !key.is_empty() && !key.starts_with("*") {
            match crate::core::llm::LLMClient::list_openai_models(&key, None).await {
                Ok(models) if !models.is_empty() => return Ok(models),
                _ => {} // Fall through to GitHub fallback
            }
        }
    }

    // Second try: fetch from GitHub community list
    match crate::core::llm::LLMClient::fetch_openai_models_from_github().await {
        Ok(models) if !models.is_empty() => return Ok(models),
        _ => {} // Fall through to hardcoded fallback
    }

    // Final fallback: hardcoded list
    Ok(crate::core::llm::get_fallback_models("openai"))
}

/// List available Gemini models (with fallback)
#[tauri::command]
pub async fn list_gemini_models(api_key: Option<String>) -> Result<Vec<ModelInfo>, String> {
    if let Some(key) = api_key {
        if !key.is_empty() && !key.starts_with("*") {
            match crate::core::llm::LLMClient::list_gemini_models(&key).await {
                Ok(models) if !models.is_empty() => return Ok(models),
                _ => {} // Fall through to fallback
            }
        }
    }
    Ok(crate::core::llm::get_fallback_models("gemini"))
}

/// List available OpenRouter models (no auth required - uses public API)
#[tauri::command]
pub async fn list_openrouter_models() -> Result<Vec<ModelInfo>, String> {
    // OpenRouter has a public models endpoint
    match crate::core::llm::fetch_openrouter_models().await {
        Ok(models) => Ok(models.into_iter().collect()),
        Err(_) => Ok(crate::core::llm::get_extended_fallback_models("openrouter")),
    }
}

/// List available models for any provider via LiteLLM catalog
#[tauri::command]
pub async fn list_provider_models(provider: String) -> Result<Vec<ModelInfo>, String> {
    // First try LiteLLM catalog (comprehensive, no auth)
    match crate::core::llm::fetch_litellm_models_for_provider(&provider).await {
        Ok(models) if !models.is_empty() => return Ok(models),
        _ => {} // Fall through
    }
    // Fallback to extended hardcoded list
    Ok(crate::core::llm::get_extended_fallback_models(&provider))
}
