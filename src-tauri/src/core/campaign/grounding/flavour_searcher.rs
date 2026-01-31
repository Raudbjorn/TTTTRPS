//! Flavour Searcher - Lore and setting content retrieval
//!
//! Part of Phase 3: Content Grounding Layer (Task 3.5)
//!
//! Searches indexed flavour sources (setting books, adventure modules, lore documents)
//! for setting-appropriate content including names, locations, and lore.

use crate::core::search::{SearchClient, SearchResult, INDEX_FICTION, INDEX_RULES};
use crate::database::Citation;
use std::sync::Arc;

use super::citation_builder::CitationBuilder;

/// Result type for flavour search operations.
pub type FlavourResult<T> = Result<T, FlavourSearchError>;

/// Errors that can occur in flavour search operations.
#[derive(Debug, thiserror::Error)]
pub enum FlavourSearchError {
    #[error("Search error: {0}")]
    Search(String),

    #[error("No results found for query: {0}")]
    NoResults(String),

    #[error("Invalid filter: {0}")]
    InvalidFilter(String),
}

/// A piece of lore or setting content found in flavour sources.
#[derive(Debug, Clone)]
pub struct LoreResult {
    /// The search result from the index
    pub result: SearchResult,
    /// Citation for the lore
    pub citation: Citation,
    /// Category of lore (history, geography, culture, etc.)
    pub category: LoreCategory,
    /// Relevance score (raw Meilisearch score, typically 0.0-1.0 but may exceed 1.0 for strong matches)
    pub relevance: f64,
}

/// Category of lore content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoreCategory {
    /// Historical events, timelines
    History,
    /// Geographic information, maps, regions
    Geography,
    /// Cultural information, customs, religions
    Culture,
    /// Faction information, organizations
    Faction,
    /// Character backgrounds, notable figures
    Character,
    /// Cosmology, planes, deities
    Cosmology,
    /// General lore that doesn't fit other categories
    General,
}

impl LoreCategory {
    /// Get keywords associated with this category for search boosting.
    pub fn keywords(&self) -> &[&str] {
        match self {
            LoreCategory::History => &["history", "timeline", "year", "era", "ancient", "war", "founded"],
            LoreCategory::Geography => &["region", "city", "town", "mountain", "river", "forest", "location", "map"],
            LoreCategory::Culture => &["culture", "tradition", "custom", "religion", "festival", "language"],
            LoreCategory::Faction => &["faction", "organization", "guild", "order", "alliance", "group"],
            LoreCategory::Character => &["character", "hero", "villain", "notable", "famous", "legendary"],
            LoreCategory::Cosmology => &["plane", "deity", "god", "goddess", "divine", "celestial", "infernal"],
            LoreCategory::General => &[],
        }
    }

    /// Infer category from content.
    pub fn infer_from_content(content: &str) -> Self {
        let content_lower = content.to_lowercase();

        // Check for category-specific keywords
        for category in [
            LoreCategory::History,
            LoreCategory::Geography,
            LoreCategory::Culture,
            LoreCategory::Faction,
            LoreCategory::Character,
            LoreCategory::Cosmology,
        ] {
            let matches = category
                .keywords()
                .iter()
                .filter(|kw| content_lower.contains(*kw))
                .count();
            if matches >= 2 {
                return category;
            }
        }

        LoreCategory::General
    }
}

/// A name found in setting content.
#[derive(Debug, Clone)]
pub struct NameResult {
    /// The name itself
    pub name: String,
    /// Type of name (person, place, organization)
    pub name_type: NameType,
    /// Setting or source the name comes from
    pub source: String,
    /// Citation for the name
    pub citation: Citation,
    /// Additional context about the name
    pub context: Option<String>,
}

/// Type of name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameType {
    Person,
    Place,
    Organization,
    Item,
    Creature,
    Event,
    Other,
}

/// A location found in setting content.
#[derive(Debug, Clone)]
pub struct LocationResult {
    /// Name of the location
    pub name: String,
    /// Type of location (city, region, dungeon, etc.)
    pub location_type: LocationType,
    /// Description or lore about the location
    pub description: String,
    /// Parent location (e.g., region contains city)
    pub parent: Option<String>,
    /// Setting the location belongs to
    pub setting: String,
    /// Citation for the location
    pub citation: Citation,
}

/// Type of location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationType {
    Continent,
    Region,
    Country,
    City,
    Town,
    Village,
    Landmark,
    Dungeon,
    Building,
    Wilderness,
    Other,
}

/// Filters for flavour searches.
#[derive(Debug, Clone, Default)]
pub struct FlavourFilters {
    /// Filter by setting name
    pub setting: Option<String>,
    /// Filter by campaign ID
    pub campaign_id: Option<String>,
    /// Filter by source document
    pub source: Option<String>,
    /// Filter by game system
    pub game_system: Option<String>,
    /// Filter by content category
    pub content_category: Option<String>,
}

impl FlavourFilters {
    /// Create filters for a specific setting.
    pub fn for_setting(setting: impl Into<String>) -> Self {
        Self {
            setting: Some(setting.into()),
            ..Default::default()
        }
    }

    /// Create filters for a specific campaign.
    pub fn for_campaign(campaign_id: impl Into<String>) -> Self {
        Self {
            campaign_id: Some(campaign_id.into()),
            ..Default::default()
        }
    }

    /// Add game system filter.
    pub fn with_game_system(mut self, system: impl Into<String>) -> Self {
        self.game_system = Some(system.into());
        self
    }

    /// Escape a value for use in Meilisearch filter strings.
    ///
    /// Meilisearch filter syntax uses double-quoted strings, so we must escape:
    /// - Backslashes: `\` -> `\\`
    /// - Double quotes: `"` -> `\"`
    fn escape_filter_value(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    }

    /// Build Meilisearch filter string.
    fn to_filter_string(&self) -> Option<String> {
        let mut filters = Vec::new();

        if let Some(setting) = &self.setting {
            filters.push(format!(r#"setting = "{}""#, Self::escape_filter_value(setting)));
        }
        if let Some(campaign_id) = &self.campaign_id {
            filters.push(format!(r#"campaign_id = "{}""#, Self::escape_filter_value(campaign_id)));
        }
        if let Some(source) = &self.source {
            filters.push(format!(r#"source = "{}""#, Self::escape_filter_value(source)));
        }
        if let Some(game_system) = &self.game_system {
            filters.push(format!(r#"game_system = "{}""#, Self::escape_filter_value(game_system)));
        }
        if let Some(category) = &self.content_category {
            filters.push(format!(r#"content_category = "{}""#, Self::escape_filter_value(category)));
        }

        if filters.is_empty() {
            None
        } else {
            Some(filters.join(" AND "))
        }
    }
}

/// Searches indexed flavour sources for setting-appropriate content.
pub struct FlavourSearcher {
    search: Arc<SearchClient>,
}

impl FlavourSearcher {
    /// Create a new FlavourSearcher with the given SearchClient.
    pub fn new(search: Arc<SearchClient>) -> Self {
        Self { search }
    }

    /// Search for setting lore.
    ///
    /// # Arguments
    /// * `query` - Search query (e.g., "history of Waterdeep", "Forgotten Realms pantheon")
    /// * `filters` - Optional filters to narrow results
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    /// Lore results with citations and categories.
    pub async fn search_setting_lore(
        &self,
        query: &str,
        filters: Option<FlavourFilters>,
        limit: usize,
    ) -> FlavourResult<Vec<LoreResult>> {
        let filter_str = filters.as_ref().and_then(|f| f.to_filter_string());

        // Search fiction index first (primary source for lore)
        let fiction_results = self
            .search
            .search(
                INDEX_FICTION,
                query,
                limit,
                filter_str.as_deref(),
            )
            .await
            .map_err(|e| FlavourSearchError::Search(e.to_string()))?;

        // Also search rules index for setting info in rulebooks
        let rules_results = self
            .search
            .search(
                INDEX_RULES,
                query,
                limit / 2,
                filter_str.as_deref(),
            )
            .await
            .map_err(|e| FlavourSearchError::Search(e.to_string()))?;

        // Combine and deduplicate results by document ID
        let mut seen_ids = std::collections::HashSet::new();
        let mut all_results: Vec<SearchResult> = Vec::with_capacity(fiction_results.len() + rules_results.len());

        for result in fiction_results.into_iter().chain(rules_results.into_iter()) {
            if seen_ids.insert(result.document.id.clone()) {
                all_results.push(result);
            }
        }

        // Convert to LoreResults
        let lore_results: Vec<LoreResult> = all_results
            .into_iter()
            .take(limit)
            .map(|result| self.to_lore_result(result))
            .collect();

        if lore_results.is_empty() {
            return Err(FlavourSearchError::NoResults(query.to_string()));
        }

        Ok(lore_results)
    }

    /// Search for setting-appropriate names.
    ///
    /// # Arguments
    /// * `name_type` - Type of name to search for (person, place, etc.)
    /// * `filters` - Filters including setting
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    /// Names found in the setting with context.
    pub async fn search_names(
        &self,
        name_type: NameType,
        filters: Option<FlavourFilters>,
        limit: usize,
    ) -> FlavourResult<Vec<NameResult>> {
        // Build query based on name type
        let query = match name_type {
            NameType::Person => "character name person hero",
            NameType::Place => "location city town region",
            NameType::Organization => "guild order faction organization",
            NameType::Item => "artifact weapon item magic",
            NameType::Creature => "creature monster beast",
            NameType::Event => "battle war event",
            NameType::Other => "name",
        };

        let filter_str = filters.as_ref().and_then(|f| f.to_filter_string());

        let results = self
            .search
            .search(
                INDEX_FICTION,
                query,
                limit * 2, // Search more to find actual names
                filter_str.as_deref(),
            )
            .await
            .map_err(|e| FlavourSearchError::Search(e.to_string()))?;

        // Extract names from results
        let names: Vec<NameResult> = results
            .into_iter()
            .filter_map(|result| self.extract_name(&result, name_type))
            .take(limit)
            .collect();

        Ok(names)
    }

    /// Search for canonical setting locations.
    ///
    /// # Arguments
    /// * `query` - Location search query (e.g., "city", "dungeon", specific name)
    /// * `filters` - Filters including setting
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    /// Locations found in the setting.
    pub async fn search_locations(
        &self,
        query: &str,
        filters: Option<FlavourFilters>,
        limit: usize,
    ) -> FlavourResult<Vec<LocationResult>> {
        // Enhance query with location-related terms
        let enhanced_query = format!("{} location region city town", query);
        let filter_str = filters.as_ref().and_then(|f| f.to_filter_string());

        let results = self
            .search
            .search(
                INDEX_FICTION,
                &enhanced_query,
                limit * 2,
                filter_str.as_deref(),
            )
            .await
            .map_err(|e| FlavourSearchError::Search(e.to_string()))?;

        // Extract locations from results
        let locations: Vec<LocationResult> = results
            .into_iter()
            .filter_map(|result| self.extract_location(&result, filters.as_ref()))
            .take(limit)
            .collect();

        Ok(locations)
    }

    /// Convert a search result to a LoreResult.
    fn to_lore_result(&self, result: SearchResult) -> LoreResult {
        let category = LoreCategory::infer_from_content(&result.document.content);
        let relevance = result.score as f64;

        // Build citation
        let source_name = result
            .document
            .book_title
            .clone()
            .unwrap_or_else(|| result.document.source.clone());

        let mut builder = CitationBuilder::from_flavour_source(&source_name)
            .source_id(&result.document.id)
            .confidence(relevance);

        if let Some(page) = result.document.page_number {
            builder = builder.page(page);
        }
        if let Some(section) = &result.document.section_title {
            builder = builder.section(section);
        }

        // Add excerpt (first 200 chars)
        let excerpt: String = result.document.content.chars().take(200).collect();
        if !excerpt.is_empty() {
            builder = builder.excerpt(excerpt);
        }

        LoreResult {
            result,
            citation: builder.build(),
            category,
            relevance,
        }
    }

    /// Extract a name from a search result.
    fn extract_name(&self, result: &SearchResult, name_type: NameType) -> Option<NameResult> {
        // Simple name extraction - look for capitalized words
        let content = &result.document.content;

        // Find the first capitalized proper noun (simplified)
        let words: Vec<&str> = content.split_whitespace().collect();
        let name = words
            .iter()
            .find(|w| {
                w.len() > 2
                    && w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && !["The", "A", "An", "And", "Or", "But", "In", "On", "At", "To", "For"]
                        .contains(w)
            })
            .map(|s| s.to_string())?;

        let source_name = result
            .document
            .book_title
            .clone()
            .unwrap_or_else(|| result.document.source.clone());

        let citation = CitationBuilder::from_flavour_source(&source_name)
            .source_id(&result.document.id)
            .confidence(result.score as f64)
            .build();

        Some(NameResult {
            name,
            name_type,
            source: source_name,
            citation,
            context: Some(content.chars().take(100).collect()),
        })
    }

    /// Extract a location from a search result.
    fn extract_location(
        &self,
        result: &SearchResult,
        filters: Option<&FlavourFilters>,
    ) -> Option<LocationResult> {
        let content = &result.document.content;
        let content_lower = content.to_lowercase();

        // Infer location type from content
        let location_type = if content_lower.contains("continent") {
            LocationType::Continent
        } else if content_lower.contains("region") || content_lower.contains("realm") {
            LocationType::Region
        } else if content_lower.contains("country") || content_lower.contains("kingdom") {
            LocationType::Country
        } else if content_lower.contains("city") || content_lower.contains("metropolis") {
            LocationType::City
        } else if content_lower.contains("town") {
            LocationType::Town
        } else if content_lower.contains("village") {
            LocationType::Village
        } else if content_lower.contains("dungeon") || content_lower.contains("cave") {
            LocationType::Dungeon
        } else if content_lower.contains("building") || content_lower.contains("tower") || content_lower.contains("temple") {
            LocationType::Building
        } else if content_lower.contains("forest") || content_lower.contains("mountain") || content_lower.contains("wilderness") {
            LocationType::Wilderness
        } else if content_lower.contains("landmark") {
            LocationType::Landmark
        } else {
            LocationType::Other
        };

        // Extract name (first capitalized word or phrase)
        let name = result
            .document
            .section_title
            .clone()
            .or_else(|| {
                content
                    .split_whitespace()
                    .find(|w| {
                        w.len() > 2
                            && w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    })
                    .map(String::from)
            })
            .unwrap_or_else(|| "Unknown Location".to_string());

        let source_name = result
            .document
            .book_title
            .clone()
            .unwrap_or_else(|| result.document.source.clone());

        let setting = filters
            .and_then(|f| f.setting.clone())
            .unwrap_or_else(|| "Unknown Setting".to_string());

        let citation = CitationBuilder::from_flavour_source(&source_name)
            .source_id(&result.document.id)
            .confidence(result.score as f64)
            .excerpt(content.chars().take(150).collect::<String>())
            .build();

        Some(LocationResult {
            name,
            location_type,
            description: content.chars().take(300).collect(),
            parent: None,
            setting,
            citation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lore_category_keywords() {
        assert!(!LoreCategory::History.keywords().is_empty());
        assert!(!LoreCategory::Geography.keywords().is_empty());
        assert!(LoreCategory::General.keywords().is_empty());
    }

    #[test]
    fn test_lore_category_inference() {
        let history_text = "In the year 1492, a great war was fought that founded the kingdom.";
        assert_eq!(LoreCategory::infer_from_content(history_text), LoreCategory::History);

        let geography_text = "The city of Waterdeep lies on the river, surrounded by mountains.";
        assert_eq!(LoreCategory::infer_from_content(geography_text), LoreCategory::Geography);

        let general_text = "Something happened somewhere.";
        assert_eq!(LoreCategory::infer_from_content(general_text), LoreCategory::General);
    }

    #[test]
    fn test_flavour_filters_to_string() {
        let filters = FlavourFilters {
            setting: Some("Forgotten Realms".to_string()),
            game_system: Some("dnd5e".to_string()),
            ..Default::default()
        };

        let filter_str = filters.to_filter_string();
        assert!(filter_str.is_some());
        let s = filter_str.unwrap();
        assert!(s.contains("setting"));
        assert!(s.contains("game_system"));
        assert!(s.contains("AND"));
    }

    #[test]
    fn test_flavour_filters_empty() {
        let filters = FlavourFilters::default();
        assert!(filters.to_filter_string().is_none());
    }

    #[test]
    fn test_flavour_filters_builder() {
        let filters = FlavourFilters::for_setting("Eberron")
            .with_game_system("dnd5e");

        assert_eq!(filters.setting, Some("Eberron".to_string()));
        assert_eq!(filters.game_system, Some("dnd5e".to_string()));
    }

    #[test]
    fn test_name_type_variants() {
        assert_ne!(NameType::Person, NameType::Place);
        assert_eq!(NameType::Person, NameType::Person);
    }

    #[test]
    fn test_location_type_variants() {
        assert_ne!(LocationType::City, LocationType::Town);
        assert_eq!(LocationType::Dungeon, LocationType::Dungeon);
    }

    #[test]
    fn test_lore_result_structure() {
        // Test that LoreResult can hold all necessary data
        let _category = LoreCategory::History;
        assert_eq!(LoreCategory::History.keywords().len(), 7);
    }

    // Integration tests would require a mock SearchClient
    #[test]
    #[ignore]
    fn test_search_setting_lore_integration() {
        // Would need async runtime and mock SearchClient
    }

    #[test]
    #[ignore]
    fn test_search_names_integration() {
        // Would need async runtime and mock SearchClient
    }

    #[test]
    #[ignore]
    fn test_search_locations_integration() {
        // Would need async runtime and mock SearchClient
    }
}
