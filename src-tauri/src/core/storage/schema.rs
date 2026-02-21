//! SurrealDB schema definitions for TTRPG Assistant.
//!
//! This module contains the SurrealQL schema for all tables, indexes, analyzers,
//! and graph relationships used by the application.
//!
//! ## Schema Structure
//!
//! ### Analyzers (Task 1.2.1)
//! - `ttrpg_analyzer` - English stemming for full-text search (FR-3.1)
//! - `exact_analyzer` - Lowercase matching for names/titles
//!
//! ### Core Tables (Task 1.2.2)
//! - `campaign` - Game campaigns with metadata (FR-6.1)
//! - `npc` - Non-player characters with campaign links (FR-5.2)
//! - `session` - Game sessions linked to campaigns
//! - `chat_message` - LLM chat history with optional campaign/NPC context
//!
//! ### Document Tables (Task 1.2.3)
//! - `library_item` - Document metadata (FR-2.1)
//! - `chunk` - Document chunks with BM25 + HNSW indexes (FR-2.3, FR-3.2)
//!
//! ### Graph Relations (Task 1.2.4)
//! - `npc_relation` - NPC-to-NPC relationships (FR-5.1, FR-5.2)
//! - `chunk_reference` - Cross-references between chunks
//! - `faction` - Campaign factions
//! - `location` - Campaign locations with parent hierarchy

/// Schema version 1 - Foundation schema.
///
/// Defines the core tables and indexes for:
/// - Document storage and chunking
/// - Vector embeddings for semantic search (HNSW, 768 dimensions, COSINE)
/// - Full-text search with custom analyzers (BM25 with highlights)
/// - Campaign and entity graph relationships
///
/// This schema is applied on initialization and supports incremental migrations.
/// All definitions use `IF NOT EXISTS` for idempotent application.
pub const SCHEMA_V1: &str = r#"
-- ============================================================================
-- TTRPG Assistant SurrealDB Schema v1
-- ============================================================================

-- Namespace and database (will be set by connection, but good for documentation)
-- USE NS ttrpg DB main;

-- ============================================================================
-- ANALYZERS (for full-text search) - Task 1.2.1, FR-3.1
-- ============================================================================

-- Standard TTRPG analyzer with English stemming
-- Tokenizers: class (camelCase/PascalCase), blank (whitespace), punct (punctuation)
-- Filters: lowercase, ascii (normalize unicode), snowball(english) for stemming
DEFINE ANALYZER IF NOT EXISTS ttrpg_analyzer
    TOKENIZERS class, blank, punct
    FILTERS lowercase, ascii, snowball(english);

-- Simple analyzer for exact matching (names, titles)
DEFINE ANALYZER IF NOT EXISTS exact_analyzer
    TOKENIZERS class
    FILTERS lowercase;

-- ============================================================================
-- SCHEMA VERSION TABLE (for migration tracking)
-- ============================================================================

DEFINE TABLE IF NOT EXISTS schema_version SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS version ON schema_version TYPE int;
DEFINE FIELD IF NOT EXISTS applied_at ON schema_version TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS description ON schema_version TYPE string;

-- ============================================================================
-- CAMPAIGN TABLE - Task 1.2.2, FR-6.1
-- ============================================================================

DEFINE TABLE IF NOT EXISTS campaign SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name ON campaign TYPE string;
DEFINE FIELD IF NOT EXISTS description ON campaign TYPE option<string>;
DEFINE FIELD IF NOT EXISTS game_system ON campaign TYPE option<string>;
DEFINE FIELD IF NOT EXISTS game_system_id ON campaign TYPE option<string>;
DEFINE FIELD IF NOT EXISTS status ON campaign TYPE string DEFAULT "active";
DEFINE FIELD IF NOT EXISTS created_at ON campaign TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON campaign TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS metadata ON campaign TYPE option<object>;

DEFINE INDEX IF NOT EXISTS campaign_name ON campaign FIELDS name SEARCH ANALYZER exact_analyzer BM25;
DEFINE INDEX IF NOT EXISTS campaign_status ON campaign FIELDS status;

-- ============================================================================
-- NPC TABLE (with graph relations) - Task 1.2.2, FR-5.2
-- ============================================================================

DEFINE TABLE IF NOT EXISTS npc SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name ON npc TYPE string;
DEFINE FIELD IF NOT EXISTS description ON npc TYPE option<string>;
DEFINE FIELD IF NOT EXISTS personality ON npc TYPE option<string>;
DEFINE FIELD IF NOT EXISTS appearance ON npc TYPE option<string>;
DEFINE FIELD IF NOT EXISTS backstory ON npc TYPE option<string>;
DEFINE FIELD IF NOT EXISTS campaign ON npc TYPE option<record<campaign>>;
DEFINE FIELD IF NOT EXISTS tags ON npc TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS created_at ON npc TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON npc TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS metadata ON npc TYPE option<object>;

DEFINE INDEX IF NOT EXISTS npc_name ON npc FIELDS name SEARCH ANALYZER exact_analyzer BM25;
DEFINE INDEX IF NOT EXISTS npc_campaign ON npc FIELDS campaign;

-- ============================================================================
-- NPC RELATIONSHIP EDGES (graph) - Task 1.2.4, FR-5.1, FR-5.2
-- ============================================================================

DEFINE TABLE IF NOT EXISTS npc_relation SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON npc_relation TYPE record<npc>;
DEFINE FIELD IF NOT EXISTS out ON npc_relation TYPE record<npc>;
DEFINE FIELD IF NOT EXISTS relation_type ON npc_relation TYPE string;
DEFINE FIELD IF NOT EXISTS strength ON npc_relation TYPE option<float>;
DEFINE FIELD IF NOT EXISTS notes ON npc_relation TYPE option<string>;

-- ============================================================================
-- SESSION TABLE - Task 1.2.2
-- ============================================================================

DEFINE TABLE IF NOT EXISTS session SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS campaign ON session TYPE record<campaign>;
DEFINE FIELD IF NOT EXISTS name ON session TYPE option<string>;
DEFINE FIELD IF NOT EXISTS session_number ON session TYPE option<int>;
DEFINE FIELD IF NOT EXISTS date ON session TYPE option<datetime>;
DEFINE FIELD IF NOT EXISTS summary ON session TYPE option<string>;
DEFINE FIELD IF NOT EXISTS notes ON session TYPE option<string>;
DEFINE FIELD IF NOT EXISTS status ON session TYPE string DEFAULT "planned";
DEFINE FIELD IF NOT EXISTS created_at ON session TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON session TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS session_campaign ON session FIELDS campaign;
DEFINE INDEX IF NOT EXISTS session_status ON session FIELDS status;

-- ============================================================================
-- CHAT MESSAGE TABLE - Task 1.2.2
-- ============================================================================

DEFINE TABLE IF NOT EXISTS chat_message SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS session_id ON chat_message TYPE string;
DEFINE FIELD IF NOT EXISTS role ON chat_message TYPE string;
DEFINE FIELD IF NOT EXISTS content ON chat_message TYPE string;
DEFINE FIELD IF NOT EXISTS campaign ON chat_message TYPE option<record<campaign>>;
DEFINE FIELD IF NOT EXISTS npc ON chat_message TYPE option<record<npc>>;
DEFINE FIELD IF NOT EXISTS sources ON chat_message TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS created_at ON chat_message TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS metadata ON chat_message TYPE option<object>;

DEFINE INDEX IF NOT EXISTS chat_session ON chat_message FIELDS session_id;
DEFINE INDEX IF NOT EXISTS chat_campaign ON chat_message FIELDS campaign;
DEFINE INDEX IF NOT EXISTS chat_created ON chat_message FIELDS created_at;

-- ============================================================================
-- LIBRARY ITEM TABLE (documents) - Task 1.2.3, FR-2.1
-- ============================================================================

DEFINE TABLE IF NOT EXISTS library_item SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS slug ON library_item TYPE string;
DEFINE FIELD IF NOT EXISTS title ON library_item TYPE string;
DEFINE FIELD IF NOT EXISTS file_path ON library_item TYPE option<string>;
DEFINE FIELD IF NOT EXISTS file_type ON library_item TYPE option<string>;
DEFINE FIELD IF NOT EXISTS file_size ON library_item TYPE option<int>;
DEFINE FIELD IF NOT EXISTS page_count ON library_item TYPE option<int>;
DEFINE FIELD IF NOT EXISTS game_system ON library_item TYPE option<string>;
DEFINE FIELD IF NOT EXISTS game_system_id ON library_item TYPE option<string>;
DEFINE FIELD IF NOT EXISTS content_category ON library_item TYPE option<string>;
DEFINE FIELD IF NOT EXISTS publisher ON library_item TYPE option<string>;
DEFINE FIELD IF NOT EXISTS status ON library_item TYPE string DEFAULT "pending";
DEFINE FIELD IF NOT EXISTS error_message ON library_item TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at ON library_item TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS updated_at ON library_item TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS metadata ON library_item TYPE option<object>;

DEFINE INDEX IF NOT EXISTS library_slug ON library_item FIELDS slug UNIQUE;
DEFINE INDEX IF NOT EXISTS library_status ON library_item FIELDS status;
DEFINE INDEX IF NOT EXISTS library_game ON library_item FIELDS game_system_id;

-- ============================================================================
-- CHUNK TABLE (document chunks with vectors) - Task 1.2.3, FR-2.1, FR-2.3, FR-3.2
-- ============================================================================

DEFINE TABLE IF NOT EXISTS chunk SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS content ON chunk TYPE string;
DEFINE FIELD IF NOT EXISTS library_item ON chunk TYPE record<library_item>;
DEFINE FIELD IF NOT EXISTS content_type ON chunk TYPE string;
DEFINE FIELD IF NOT EXISTS page_number ON chunk TYPE option<int>;
DEFINE FIELD IF NOT EXISTS page_start ON chunk TYPE option<int>;
DEFINE FIELD IF NOT EXISTS page_end ON chunk TYPE option<int>;
DEFINE FIELD IF NOT EXISTS chunk_index ON chunk TYPE option<int>;
DEFINE FIELD IF NOT EXISTS section_path ON chunk TYPE option<string>;
DEFINE FIELD IF NOT EXISTS chapter_title ON chunk TYPE option<string>;
DEFINE FIELD IF NOT EXISTS section_title ON chunk TYPE option<string>;
DEFINE FIELD IF NOT EXISTS chunk_type ON chunk TYPE option<string>;
DEFINE FIELD IF NOT EXISTS semantic_keywords ON chunk TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS embedding ON chunk TYPE option<array<float>>;
DEFINE FIELD IF NOT EXISTS embedding_model ON chunk TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at ON chunk TYPE datetime DEFAULT time::now();
DEFINE FIELD IF NOT EXISTS metadata ON chunk TYPE option<object>;

-- Full-text index on content with BM25 and highlights (FR-3.2)
DEFINE INDEX IF NOT EXISTS chunk_content ON chunk FIELDS content SEARCH ANALYZER ttrpg_analyzer BM25 HIGHLIGHTS;

-- Vector index (HNSW) - 768 dimensions for nomic-embed-text (FR-2.3)
-- Parameters: EFC 150 (search quality), M 12 (graph connectivity)
DEFINE INDEX IF NOT EXISTS chunk_embedding ON chunk FIELDS embedding HNSW DIMENSION 768 DIST COSINE EFC 150 M 12;

-- Filtering indexes
DEFINE INDEX IF NOT EXISTS chunk_library ON chunk FIELDS library_item;
DEFINE INDEX IF NOT EXISTS chunk_type ON chunk FIELDS content_type;
DEFINE INDEX IF NOT EXISTS chunk_page ON chunk FIELDS page_number;

-- ============================================================================
-- CHUNK RELATIONS (for cross-references) - Task 1.2.4, FR-5.1
-- ============================================================================

DEFINE TABLE IF NOT EXISTS chunk_reference SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS in ON chunk_reference TYPE record<chunk>;
DEFINE FIELD IF NOT EXISTS out ON chunk_reference TYPE record<chunk>;
DEFINE FIELD IF NOT EXISTS reference_type ON chunk_reference TYPE string;

-- ============================================================================
-- FACTION TABLE (for graph relations)
-- ============================================================================

DEFINE TABLE IF NOT EXISTS faction SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name ON faction TYPE string;
DEFINE FIELD IF NOT EXISTS description ON faction TYPE option<string>;
DEFINE FIELD IF NOT EXISTS campaign ON faction TYPE record<campaign>;
DEFINE FIELD IF NOT EXISTS alignment ON faction TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at ON faction TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS faction_campaign ON faction FIELDS campaign;

-- ============================================================================
-- LOCATION TABLE (for graph relations)
-- ============================================================================

DEFINE TABLE IF NOT EXISTS location SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name ON location TYPE string;
DEFINE FIELD IF NOT EXISTS description ON location TYPE option<string>;
DEFINE FIELD IF NOT EXISTS campaign ON location TYPE record<campaign>;
DEFINE FIELD IF NOT EXISTS parent_location ON location TYPE option<record<location>>;
DEFINE FIELD IF NOT EXISTS location_type ON location TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at ON location TYPE datetime DEFAULT time::now();

DEFINE INDEX IF NOT EXISTS location_campaign ON location FIELDS campaign;
DEFINE INDEX IF NOT EXISTS location_parent ON location FIELDS parent_location;
"#;

/// Schema version for migration tracking
pub const SCHEMA_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_v1_not_empty() {
        assert!(!SCHEMA_V1.is_empty());
        assert!(SCHEMA_V1.contains("schema_version"));
    }

    #[test]
    fn test_schema_contains_ttrpg_analyzer() {
        // Task 1.2.1: TTRPG analyzer with correct tokenizers and filters
        assert!(SCHEMA_V1.contains("DEFINE ANALYZER IF NOT EXISTS ttrpg_analyzer"));
        assert!(SCHEMA_V1.contains("TOKENIZERS class, blank, punct"));
        assert!(SCHEMA_V1.contains("FILTERS lowercase, ascii, snowball(english)"));
    }

    #[test]
    fn test_schema_contains_exact_analyzer() {
        assert!(SCHEMA_V1.contains("DEFINE ANALYZER IF NOT EXISTS exact_analyzer"));
    }

    #[test]
    fn test_schema_contains_core_tables() {
        // Task 1.2.2: Core tables
        assert!(SCHEMA_V1.contains("DEFINE TABLE IF NOT EXISTS campaign SCHEMAFULL"));
        assert!(SCHEMA_V1.contains("DEFINE TABLE IF NOT EXISTS npc SCHEMAFULL"));
        assert!(SCHEMA_V1.contains("DEFINE TABLE IF NOT EXISTS session SCHEMAFULL"));
        assert!(SCHEMA_V1.contains("DEFINE TABLE IF NOT EXISTS chat_message SCHEMAFULL"));
    }

    #[test]
    fn test_schema_contains_library_tables() {
        // Task 1.2.3: Library and chunk tables
        assert!(SCHEMA_V1.contains("DEFINE TABLE IF NOT EXISTS library_item SCHEMAFULL"));
        assert!(SCHEMA_V1.contains("DEFINE TABLE IF NOT EXISTS chunk SCHEMAFULL"));
    }

    #[test]
    fn test_schema_contains_bm25_index() {
        // Task 1.2.3: BM25 full-text index on chunk.content
        assert!(SCHEMA_V1.contains("chunk_content ON chunk FIELDS content SEARCH ANALYZER ttrpg_analyzer BM25 HIGHLIGHTS"));
    }

    #[test]
    fn test_schema_contains_hnsw_index() {
        // Task 1.2.3: HNSW vector index with correct parameters
        assert!(SCHEMA_V1.contains("chunk_embedding ON chunk FIELDS embedding HNSW DIMENSION 768 DIST COSINE EFC 150 M 12"));
    }

    #[test]
    fn test_schema_contains_graph_relations() {
        // Task 1.2.4: Graph relation tables
        assert!(SCHEMA_V1.contains("DEFINE TABLE IF NOT EXISTS npc_relation SCHEMAFULL"));
        assert!(SCHEMA_V1.contains("DEFINE TABLE IF NOT EXISTS chunk_reference SCHEMAFULL"));
    }

    #[test]
    fn test_schema_contains_record_links() {
        // FR-5.2, FR-6.1: Record link fields for relations
        assert!(SCHEMA_V1.contains("TYPE record<campaign>"));
        assert!(SCHEMA_V1.contains("TYPE record<npc>"));
        assert!(SCHEMA_V1.contains("TYPE record<library_item>"));
        assert!(SCHEMA_V1.contains("TYPE record<chunk>"));
    }

    #[test]
    fn test_schema_uses_if_not_exists() {
        // All definitions should be idempotent
        assert!(SCHEMA_V1.contains("DEFINE TABLE IF NOT EXISTS"));
        assert!(SCHEMA_V1.contains("DEFINE FIELD IF NOT EXISTS"));
        assert!(SCHEMA_V1.contains("DEFINE INDEX IF NOT EXISTS"));
        assert!(SCHEMA_V1.contains("DEFINE ANALYZER IF NOT EXISTS"));
    }

    #[test]
    fn test_schema_version_constant() {
        assert_eq!(SCHEMA_VERSION, 1);
    }
}
