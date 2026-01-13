pub mod adaptive_learning;
pub mod kreuzberg_extractor;
pub mod extraction_settings;
pub mod personality;
pub mod flavor;
pub mod character_gen;
pub mod rulebook_linker;
pub mod chunker;
pub mod hash;
pub mod layout;
pub mod ttrpg;

// All document extraction handled by kreuzberg_extractor

pub use adaptive_learning::AdaptiveLearningSystem;
pub use kreuzberg_extractor::{
    DocumentExtractor, ExtractedContent, ExtractionError,
    extract_text, extract_text_with_ocr, extract_document, extract_document_with_ocr,
};
pub use extraction_settings::{
    ExtractionSettings, TokenReductionLevel, OcrBackend,
    SupportedFormats, FormatInfo,
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
