//! Search Document Models
//!
//! Data structures for search documents, results, and library metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Core Document Fields (Generic)
// ============================================================================

/// Core searchable document fields - generic across all content types
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreDocumentFields {
    /// Unique document ID
    pub id: String,
    /// Text content
    pub content: String,
    /// Source file or origin (file path)
    pub source: String,
    /// Source type for categorization (rule, fiction, chat, document)
    #[serde(default)]
    pub source_type: String,
    /// Page number if from PDF
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<u32>,
    /// Chunk index within document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<u32>,
    /// Campaign ID if associated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,
    /// Session ID if from chat
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

// ============================================================================
// TTRPG Metadata (Composable)
// ============================================================================

/// TTRPG-specific embedding metadata for semantic search
///
/// This struct groups all TTRPG-specific fields that are included in the
/// documentTemplate for semantic embedding. It can be composed into
/// SearchDocument or used independently.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TTRPGEmbeddingMetadata {
    /// Human-readable book/document title (e.g., "Delta Green: Handler's Guide")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_title: Option<String>,

    /// Game system display name (e.g., "Delta Green", "D&D 5th Edition")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,

    /// Game system machine ID (e.g., "delta_green", "dnd5e")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system_id: Option<String>,

    /// Content category: rulebook, adventure, setting, supplement, bestiary, quickstart
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_category: Option<String>,

    /// Section/chapter title (e.g., "Chapter 3: Combat", "Appendix A: Monsters")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_title: Option<String>,

    /// Genre/theme (e.g., "cosmic horror", "fantasy", "sci-fi")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,

    /// Publisher name (e.g., "Arc Dream Publishing", "Wizards of the Coast")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
}

/// Enhanced TTRPG metadata from MDMAI patterns
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TTRPGEnhancedMetadata {
    /// Chunk type for content classification (text, stat_block, table, spell, monster, rule, narrative)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_type: Option<String>,

    /// Chapter title (top-level section from TOC)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_title: Option<String>,

    /// Subsection title (nested within section)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subsection_title: Option<String>,

    /// Full section hierarchy path (e.g., "Chapter 1 > Monsters > Goblins")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_path: Option<String>,

    /// Mechanic type for rules content (skill_check, combat, damage, healing, sanity, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanic_type: Option<String>,

    /// Extracted semantic keywords for embedding boost
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_keywords: Vec<String>,
}

/// TTRPG v2 metadata for semantic chunking improvements
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TTRPGSemanticMetadata {
    /// Element type classification (stat_block, random_table, spell, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_type: Option<String>,

    /// Numeric section depth (0 = root, 1 = chapter, 2 = section, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_depth: Option<u32>,

    /// Parent section titles for breadcrumb navigation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_sections: Vec<String>,

    /// Cross-references detected in this chunk (e.g., ["page:47", "chapter:3"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_refs: Vec<String>,

    /// Content mode: crunch, fluff, mixed, example, optional, fiction
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_mode: Option<String>,

    /// Extracted dice expressions (e.g., ["2d6", "1d20+5"])
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dice_expressions: Vec<String>,

    /// Classification confidence score (0.0 to 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification_confidence: Option<f32>,

    /// Context-injected content for embeddings (section path + type prefix)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_content: Option<String>,
}

// ============================================================================
// SearchDocument (Flat structure for backward compatibility)
// ============================================================================

/// A searchable document chunk
///
/// This struct maintains backward compatibility with the existing flat structure
/// while the metadata fields are logically grouped. For new code, prefer using
/// the composed metadata structs directly.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchDocument {
    // Core fields
    /// Unique document ID
    pub id: String,
    /// Text content
    pub content: String,
    /// Source file or origin (file path)
    pub source: String,
    /// Source type for categorization (rule, fiction, chat, document)
    #[serde(default)]
    pub source_type: String,
    /// Page number if from PDF
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_number: Option<u32>,
    /// Chunk index within document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<u32>,
    /// Campaign ID if associated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign_id: Option<String>,
    /// Session ID if from chat
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,

    // TTRPG Embedding Metadata
    /// Human-readable book/document title
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_title: Option<String>,
    /// Game system display name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,
    /// Game system machine ID
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_system_id: Option<String>,
    /// Content category
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_category: Option<String>,
    /// Section/chapter title
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_title: Option<String>,
    /// Genre/theme
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    /// Publisher name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,

    // Enhanced Metadata (MDMAI patterns)
    /// Chunk type for content classification
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_type: Option<String>,
    /// Chapter title
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_title: Option<String>,
    /// Subsection title
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subsection_title: Option<String>,
    /// Full section hierarchy path
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_path: Option<String>,
    /// Mechanic type for rules content
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mechanic_type: Option<String>,
    /// Extracted semantic keywords
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_keywords: Vec<String>,

    // v2 Semantic Metadata
    /// Element type classification
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_type: Option<String>,
    /// Numeric section depth
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section_depth: Option<u32>,
    /// Parent section titles
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_sections: Vec<String>,
    /// Cross-references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cross_refs: Vec<String>,
    /// Content mode
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_mode: Option<String>,
    /// Extracted dice expressions
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dice_expressions: Vec<String>,
    /// Classification confidence score
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification_confidence: Option<f32>,
    /// Context-injected content for embeddings
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding_content: Option<String>,
}

impl SearchDocument {
    /// Extract core fields into a CoreDocumentFields struct
    pub fn core_fields(&self) -> CoreDocumentFields {
        CoreDocumentFields {
            id: self.id.clone(),
            content: self.content.clone(),
            source: self.source.clone(),
            source_type: self.source_type.clone(),
            page_number: self.page_number,
            chunk_index: self.chunk_index,
            campaign_id: self.campaign_id.clone(),
            session_id: self.session_id.clone(),
            created_at: self.created_at.clone(),
            metadata: self.metadata.clone(),
        }
    }

    /// Extract TTRPG embedding metadata
    pub fn ttrpg_embedding_metadata(&self) -> TTRPGEmbeddingMetadata {
        TTRPGEmbeddingMetadata {
            book_title: self.book_title.clone(),
            game_system: self.game_system.clone(),
            game_system_id: self.game_system_id.clone(),
            content_category: self.content_category.clone(),
            section_title: self.section_title.clone(),
            genre: self.genre.clone(),
            publisher: self.publisher.clone(),
        }
    }

    /// Extract enhanced TTRPG metadata
    pub fn ttrpg_enhanced_metadata(&self) -> TTRPGEnhancedMetadata {
        TTRPGEnhancedMetadata {
            chunk_type: self.chunk_type.clone(),
            chapter_title: self.chapter_title.clone(),
            subsection_title: self.subsection_title.clone(),
            section_path: self.section_path.clone(),
            mechanic_type: self.mechanic_type.clone(),
            semantic_keywords: self.semantic_keywords.clone(),
        }
    }

    /// Extract v2 semantic metadata
    pub fn ttrpg_semantic_metadata(&self) -> TTRPGSemanticMetadata {
        TTRPGSemanticMetadata {
            element_type: self.element_type.clone(),
            section_depth: self.section_depth,
            parent_sections: self.parent_sections.clone(),
            cross_refs: self.cross_refs.clone(),
            content_mode: self.content_mode.clone(),
            dice_expressions: self.dice_expressions.clone(),
            classification_confidence: self.classification_confidence,
            embedding_content: self.embedding_content.clone(),
        }
    }
}

// ============================================================================
// Search Results
// ============================================================================

/// A search result with score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: SearchDocument,
    pub score: f32,
    pub index: String,
}

/// Federated search results from multiple indexes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedResults {
    pub results: Vec<SearchResult>,
    pub total_hits: usize,
    pub processing_time_ms: u64,
}

// ============================================================================
// Library Document Metadata
// ============================================================================

/// Library document metadata - stored in Meilisearch for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryDocumentMetadata {
    /// Unique document ID
    pub id: String,
    /// Document name (file name without path)
    pub name: String,
    /// File format (pdf, epub, mobi, docx, txt)
    pub source_type: String,
    /// Original file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Number of pages in the document
    pub page_count: u32,
    /// Number of chunks indexed
    pub chunk_count: u32,
    /// Total characters extracted
    pub character_count: u64,
    /// Index where content chunks are stored (rules, fiction, documents)
    pub content_index: String,
    /// Processing status (pending, processing, ready, error)
    pub status: String,
    /// Error message if status is error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    /// Timestamp when ingested
    pub ingested_at: String,

    // TTRPG Metadata (Phase 1)
    /// Game system (e.g., "D&D 5e", "Pathfinder 2e", "Call of Cthulhu 7e")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_system: Option<String>,
    /// Campaign setting (e.g., "Forgotten Realms", "Eberron", "Golarion")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<String>,
    /// Content type (e.g., "core_rulebook", "supplement", "adventure", "bestiary")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Publisher (e.g., "Wizards of the Coast", "Paizo", "Chaosium")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_document_serialization() {
        let doc = SearchDocument {
            id: "test-1".to_string(),
            content: "Test content".to_string(),
            source: "test.pdf".to_string(),
            source_type: "document".to_string(),
            page_number: Some(1),
            chunk_index: Some(0),
            campaign_id: None,
            session_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            metadata: HashMap::new(),
            ..Default::default()
        };

        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("test-1"));
        assert!(json.contains("Test content"));
    }

    #[test]
    fn test_core_fields_extraction() {
        let doc = SearchDocument {
            id: "extract-test".to_string(),
            content: "Some content".to_string(),
            source: "source.pdf".to_string(),
            source_type: "rules".to_string(),
            page_number: Some(42),
            chunk_index: Some(5),
            campaign_id: Some("camp-1".to_string()),
            session_id: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            metadata: HashMap::new(),
            book_title: Some("Player's Handbook".to_string()),
            ..Default::default()
        };

        let core = doc.core_fields();
        assert_eq!(core.id, "extract-test");
        assert_eq!(core.page_number, Some(42));
        assert_eq!(core.campaign_id, Some("camp-1".to_string()));
    }

    #[test]
    fn test_ttrpg_metadata_extraction() {
        let doc = SearchDocument {
            id: "ttrpg-test".to_string(),
            content: "Combat rules".to_string(),
            source: "phb.pdf".to_string(),
            source_type: "rules".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            book_title: Some("Player's Handbook".to_string()),
            game_system: Some("D&D 5e".to_string()),
            game_system_id: Some("dnd5e".to_string()),
            content_category: Some("rulebook".to_string()),
            genre: Some("fantasy".to_string()),
            ..Default::default()
        };

        let meta = doc.ttrpg_embedding_metadata();
        assert_eq!(meta.book_title, Some("Player's Handbook".to_string()));
        assert_eq!(meta.game_system, Some("D&D 5e".to_string()));
        assert_eq!(meta.genre, Some("fantasy".to_string()));
    }
}
