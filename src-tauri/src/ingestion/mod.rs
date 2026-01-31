pub mod adaptive_learning;
pub mod kreuzberg_extractor;
pub mod claude_extractor;
pub mod extraction_settings;
pub mod markdown_parser;
pub mod layout_json;
pub mod personality;
pub mod flavor;
pub mod character_gen;
pub mod rulebook_linker;
pub mod chunker;
pub mod hash;
pub mod layout;
pub mod ttrpg;

// Pipeline-specific models and utilities (extracted from core/meilisearch_pipeline.rs)
pub mod pipeline_models;
pub mod slugs;

// Document extraction via kreuzberg_extractor (local) or claude_extractor (API)

pub use adaptive_learning::AdaptiveLearningSystem;
pub use kreuzberg_extractor::{
    DocumentExtractor, ExtractedContent, ExtractionError,
    extract_text, extract_text_with_ocr, extract_document, extract_document_with_ocr,
};
pub use extraction_settings::{
    ExtractionSettings, TokenReductionLevel, OcrBackend,
    SupportedFormats, FormatInfo, TextExtractionProvider,
    MarkdownSettings, ClaudeParallelSettings,
};
pub use markdown_parser::MarkdownPageParser;
pub use layout_json::{
    LayoutDocument, LayoutPage, LayoutElement, LayoutMetadata,
    LayoutJsonError, BoundingBox, PageRegions, PageMetrics,
};
pub use claude_extractor::{
    ClaudeDocumentExtractor, ClaudeExtractorConfig, ClaudeExtractionError,
    extract_with_claude, extract_document_with_claude,
};
pub use chunker::{
    SemanticChunker, ChunkConfig, ContentChunk,
    TTRPGChunker, TTRPGChunkConfig, SectionHierarchy,
};
pub use hash::{hash_file, hash_bytes, hash_file_with_size, get_file_size};
pub use layout::{
    ColumnDetector, ColumnBoundary, TextBlock,
    RegionDetector, DetectedRegion, RegionType, RegionBounds,
    TableExtractor, ExtractedTable,
};
pub use ttrpg::{
    TTRPGClassifier, TTRPGElementType, ClassifiedElement,
    StatBlockParser, StatBlockData, AbilityScores, Feature, Speed,
    RandomTableParser, RandomTableData, TableEntry,
    AttributeExtractor, TTRPGAttributes, AttributeMatch, AttributeSource, FilterableFields,
    GameVocabulary, DnD5eVocabulary, Pf2eVocabulary,
    detect_game_system, detect_game_system_with_confidence, GameSystem, DetectionResult,
};

// Pipeline models and utilities (extracted from core/meilisearch_pipeline.rs)
pub use pipeline_models::{
    ClassificationContext, ClassificationResult,
    PageMetadata, RawDocument, ChunkedDocument,
    TTRPGMetadata,
    PipelineChunkConfig, PipelineConfig,
    ExtractionResult, ChunkingResult,
};
pub use slugs::{
    generate_source_slug, slugify,
    raw_index_name, chunks_index_name,
    MAX_SLUG_LENGTH,
};
