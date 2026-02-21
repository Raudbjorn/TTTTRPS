//! Search Suggestions Commands
//!
//! Commands for autocomplete, query hints, query expansion, and spell correction.
//!
//! TODO: Phase 3 Migration - HybridSearchEngine needs to be updated to work with
//! EmbeddedSearch/MeilisearchLib. Currently stubbed out.

use tauri::State;

use crate::commands::AppState;
// TODO: Re-enable when HybridSearchEngine is migrated to EmbeddedSearch
// use crate::core::search::HybridSearchEngine;

// ============================================================================
// Search Suggestions and Hints
// ============================================================================

/// Get search suggestions for autocomplete
///
/// TODO: Phase 3 Migration - Update HybridSearchEngine to work with EmbeddedSearch
#[tauri::command]
#[allow(unused_variables)]
pub fn get_search_suggestions(
    partial: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    // TODO: Migrate to embedded MeilisearchLib
    // HybridSearchEngine::with_defaults expects Arc<SearchClient> (HTTP SDK).
    // Need to update HybridSearchEngine to work with EmbeddedSearch/MeilisearchLib.
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "get_search_suggestions() called but not yet migrated to embedded MeilisearchLib. Partial: {}",
        partial
    );

    // Return empty suggestions with explicit Ok - migration in Phase 3 Task 6
    Ok(Vec::new())
}

/// Get search hints for a query
///
/// TODO: Phase 3 Migration - Update HybridSearchEngine to work with EmbeddedSearch
#[tauri::command]
#[allow(unused_variables)]
pub fn get_search_hints(
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    // TODO: Migrate to embedded MeilisearchLib
    // HybridSearchEngine::with_defaults expects Arc<SearchClient> (HTTP SDK).
    // Need to update HybridSearchEngine to work with EmbeddedSearch/MeilisearchLib.
    //
    // Access via: state.embedded_search.inner()
    let _meili = state.embedded_search.inner();

    log::warn!(
        "get_search_hints() called but not yet migrated to embedded MeilisearchLib. Query: {}",
        query
    );

    // Return empty hints with explicit Ok - migration in Phase 3 Task 6
    Ok(Vec::new())
}

/// Expand a query with TTRPG synonyms
#[tauri::command]
pub fn expand_query(query: String) -> crate::core::search::synonyms::QueryExpansionResult {
    let synonyms = crate::core::search::TTRPGSynonyms::new();
    synonyms.expand_query(&query)
}

/// Correct spelling in a query
#[tauri::command]
pub fn correct_query(query: String) -> crate::core::spell_correction::CorrectionResult {
    let corrector = crate::core::spell_correction::SpellCorrector::new();
    corrector.correct(&query)
}
