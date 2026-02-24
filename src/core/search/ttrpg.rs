//! TTRPG-Specific Search Types
//!
//! Document types and operations specific to tabletop RPG content.

use serde::{Deserialize, Serialize};

use super::models::SearchDocument;

/// Index name for TTRPG content
pub const INDEX_TTRPG: &str = "ttrpg";

// ============================================================================
// TTRPG Document Types
// ============================================================================

/// TTRPG-specific filterable fields for faceted search
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TTRPGFilterableFields {
    /// Damage types mentioned (fire, cold, radiant, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_types: Vec<String>,

    /// Creature types (humanoid, undead, dragon, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creature_types: Vec<String>,

    /// Conditions (poisoned, frightened, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,

    /// Alignments (lawful good, chaotic evil, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alignments: Vec<String>,

    /// Item rarities (common, rare, legendary, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rarities: Vec<String>,

    /// Size categories (tiny, small, medium, large, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sizes: Vec<String>,

    /// Spell schools (evocation, necromancy, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spell_schools: Vec<String>,

    /// Challenge rating for monsters/encounters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge_rating: Option<f32>,

    /// Level (spell level, class level, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
}

/// TTRPG-specific searchable document with game metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TTRPGSearchDocument {
    /// Base document fields
    #[serde(flatten)]
    pub base: SearchDocument,

    // TTRPG-specific filterable fields
    /// Damage types mentioned (fire, cold, radiant, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_types: Vec<String>,

    /// Creature types (humanoid, undead, dragon, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creature_types: Vec<String>,

    /// Conditions (poisoned, frightened, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,

    /// Alignments (lawful good, chaotic evil, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alignments: Vec<String>,

    /// Item rarities (common, rare, legendary, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rarities: Vec<String>,

    /// Size categories (tiny, small, medium, large, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sizes: Vec<String>,

    /// Spell schools (evocation, necromancy, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spell_schools: Vec<String>,

    /// Challenge rating for monsters/encounters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge_rating: Option<f32>,

    /// Level (spell level, class level, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,

    /// TTRPG element type (stat_block, random_table, spell, etc.)
    ///
    /// Note: This is distinct from `base.source_type` which is set to "ttrpg"
    /// to identify content as TTRPG-related. This field provides the specific
    /// element categorization for filtering and faceted search.
    #[serde(default)]
    pub element_type: String,

    /// Detected game system (dnd5e, pf2e, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,

    /// Section hierarchy path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_path: Option<String>,
}

impl TTRPGSearchDocument {
    /// Create a new TTRPG document from base document and attributes
    pub fn new(base: SearchDocument, element_type: &str) -> Self {
        Self {
            base,
            damage_types: Vec::new(),
            creature_types: Vec::new(),
            conditions: Vec::new(),
            alignments: Vec::new(),
            rarities: Vec::new(),
            sizes: Vec::new(),
            spell_schools: Vec::new(),
            challenge_rating: None,
            level: None,
            element_type: element_type.to_string(),
            game_system: None,
            section_path: None,
        }
    }

    /// Create from a content chunk with TTRPG attributes
    pub fn from_chunk(
        chunk: &crate::ingestion::ContentChunk,
        attributes: &crate::ingestion::TTRPGAttributes,
        element_type: &str,
        game_system: Option<&str>,
    ) -> Self {
        let filterable = attributes.to_filterable_fields();

        let base = SearchDocument {
            id: chunk.id.clone(),
            content: chunk.content.clone(),
            source: chunk.source_id.clone(),
            // "ttrpg" identifies this as TTRPG content (vs generic documents)
            // TTRPGSearchDocument.element_type provides specific categorization
            source_type: "ttrpg".to_string(),
            page_number: chunk.page_number,
            chunk_index: Some(chunk.chunk_index as u32),
            campaign_id: None,
            session_id: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            metadata: chunk.metadata.clone(),
            // TTRPG metadata populated from game_system parameter
            game_system: game_system.map(|s| {
                // Try to get display name from game system
                crate::ingestion::ttrpg::game_detector::GameSystem::from_str(s)
                    .map(|gs| gs.display_name().to_string())
                    .unwrap_or_else(|| s.to_string())
            }),
            game_system_id: game_system.map(|s| s.to_string()),
            ..Default::default()
        };

        Self {
            base,
            damage_types: filterable.damage_types,
            creature_types: filterable.creature_types,
            conditions: filterable.conditions,
            alignments: filterable.alignments,
            rarities: filterable.rarities,
            sizes: filterable.sizes,
            spell_schools: filterable.spell_schools,
            challenge_rating: filterable.challenge_rating,
            level: None, // Level is not part of FilterableFields
            element_type: element_type.to_string(),
            game_system: game_system.map(|s| s.to_string()),
            section_path: chunk.metadata.get("section_path").cloned(),
        }
    }

    /// Extract filterable fields into a separate struct
    pub fn filterable_fields(&self) -> TTRPGFilterableFields {
        TTRPGFilterableFields {
            damage_types: self.damage_types.clone(),
            creature_types: self.creature_types.clone(),
            conditions: self.conditions.clone(),
            alignments: self.alignments.clone(),
            rarities: self.rarities.clone(),
            sizes: self.sizes.clone(),
            spell_schools: self.spell_schools.clone(),
            challenge_rating: self.challenge_rating,
            level: self.level,
        }
    }

    /// Set filterable fields from a TTRPGFilterableFields struct
    pub fn with_filterable_fields(mut self, fields: TTRPGFilterableFields) -> Self {
        self.damage_types = fields.damage_types;
        self.creature_types = fields.creature_types;
        self.conditions = fields.conditions;
        self.alignments = fields.alignments;
        self.rarities = fields.rarities;
        self.sizes = fields.sizes;
        self.spell_schools = fields.spell_schools;
        self.challenge_rating = fields.challenge_rating;
        self.level = fields.level;
        self
    }
}

/// TTRPG search result with full document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTRPGSearchResult {
    pub document: TTRPGSearchDocument,
    pub score: f32,
    pub index: String,
}

// ============================================================================
// TTRPG Index Configuration
// ============================================================================

/// Filterable attributes for TTRPG indexes
pub const TTRPG_FILTERABLE_ATTRIBUTES: &[&str] = &[
    // TTRPG-specific filters
    "damage_types",
    "creature_types",
    "conditions",
    "alignments",
    "rarities",
    "sizes",
    "spell_schools",
    "element_type",
    "challenge_rating",
    "level",
    "game_system",
    // Standard filters
    "source",
    "source_type",
    "page_number",
    "campaign_id",
    "session_id",
    "created_at",
];

/// Sortable attributes for TTRPG indexes
pub const TTRPG_SORTABLE_ATTRIBUTES: &[&str] = &["challenge_rating", "level", "created_at"];

/// Searchable attributes for TTRPG indexes
pub const TTRPG_SEARCHABLE_ATTRIBUTES: &[&str] = &["content", "source", "element_type", "metadata"];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_ttrpg_document_serialization() {
        let base = SearchDocument {
            id: "ttrpg-1".to_string(),
            content: "Goblin stat block".to_string(),
            source: "monster_manual.pdf".to_string(),
            source_type: "stat_block".to_string(),
            page_number: Some(42),
            chunk_index: Some(0),
            campaign_id: None,
            session_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            metadata: HashMap::new(),
            ..Default::default()
        };

        let mut doc = TTRPGSearchDocument::new(base, "stat_block");
        doc.damage_types = vec!["slashing".to_string()];
        doc.creature_types = vec!["humanoid".to_string()];
        doc.sizes = vec!["small".to_string()];
        doc.challenge_rating = Some(0.25);
        doc.game_system = Some("dnd5e".to_string());

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("ttrpg-1"));
        assert!(json.contains("stat_block"));
        assert!(json.contains("slashing"));
        assert!(json.contains("humanoid"));
        assert!(json.contains("dnd5e"));
    }

    #[test]
    fn test_ttrpg_document_round_trip() {
        let base = SearchDocument {
            id: "round-trip-1".to_string(),
            content: "Fire bolt cantrip".to_string(),
            source: "phb.pdf".to_string(),
            source_type: "spell".to_string(),
            page_number: Some(100),
            chunk_index: Some(5),
            campaign_id: None,
            session_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            metadata: HashMap::new(),
            ..Default::default()
        };

        let mut doc = TTRPGSearchDocument::new(base, "spell");
        doc.damage_types = vec!["fire".to_string()];
        doc.spell_schools = vec!["evocation".to_string()];
        doc.level = Some(0);

        let json = serde_json::to_string(&doc).unwrap();
        let parsed: TTRPGSearchDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.base.id, "round-trip-1");
        assert_eq!(parsed.element_type, "spell");
        assert_eq!(parsed.damage_types, vec!["fire"]);
        assert_eq!(parsed.level, Some(0));
    }

    #[test]
    fn test_filterable_fields_extraction() {
        let base = SearchDocument {
            id: "filter-test".to_string(),
            content: "Test".to_string(),
            source: "test.pdf".to_string(),
            source_type: "test".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            ..Default::default()
        };

        let mut doc = TTRPGSearchDocument::new(base, "monster");
        doc.damage_types = vec!["fire".to_string(), "cold".to_string()];
        doc.creature_types = vec!["dragon".to_string()];
        doc.challenge_rating = Some(10.0);

        let fields = doc.filterable_fields();
        assert_eq!(fields.damage_types, vec!["fire", "cold"]);
        assert_eq!(fields.creature_types, vec!["dragon"]);
        assert_eq!(fields.challenge_rating, Some(10.0));
    }

    #[test]
    fn test_ttrpg_index_constant() {
        assert_eq!(INDEX_TTRPG, "ttrpg");
    }
}
