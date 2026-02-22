//! TTRPG-Specific Content Processing Module
//!
//! This module provides specialized processing for Tabletop Role-Playing Game
//! content, including:
//!
//! - **Element Classification**: Detecting stat blocks, random tables, read-aloud text
//! - **Stat Block Parsing**: Extracting structured creature/NPC data
//! - **Random Table Parsing**: Extracting roll tables with probability distributions
//! - **Attribute Extraction**: Identifying game-specific terms with confidence scores
//! - **Game System Detection**: Auto-detecting D&D 5e, Pathfinder, etc.
//! - **Cross-Reference Detection**: Extracting page, chapter, section, table, and figure references
//! - **Dice Expression Extraction**: Parsing dice notation (2d6+3), DCs, and modifiers
//!
//! # Example
//!
//! ```ignore
//! use crate::ingestion::ttrpg::{TTRPGClassifier, AttributeExtractor, detect_game_system};
//!
//! let classifier = TTRPGClassifier::new();
//! let element = classifier.classify(text, page_number);
//!
//! let extractor = AttributeExtractor::new();
//! let attributes = extractor.extract(text);
//!
//! let game_system = detect_game_system(text);
//! ```

pub mod classifier;
pub mod content_mode;
pub mod stat_block;
pub mod random_table;
pub mod vocabulary;
pub mod attribute_extractor;
pub mod game_detector;
pub mod boundary_scorer;
pub mod cross_reference;
pub mod dice_extractor;

pub use classifier::{TTRPGClassifier, TTRPGElementType, ClassifiedElement};
pub use content_mode::{ContentMode, ContentModeClassifier, ContentModeResult};
pub use stat_block::{StatBlockParser, StatBlockData, AbilityScores, Feature, Speed};
pub use random_table::{RandomTableParser, RandomTableData, TableEntry};
pub use attribute_extractor::{
    AttributeExtractor, TTRPGAttributes, AttributeMatch, AttributeSource,
    FilterableFields,
};
pub use vocabulary::{
    // Trait and implementations
    GameVocabulary, DnD5eVocabulary, Pf2eVocabulary,
    // Comprehensive vocabulary lists
    GENRES, FANTASY_CLASSES, SCIFI_CLASSES, HORROR_CLASSES, MODERN_CLASSES,
    FANTASY_RACES, SCIFI_RACES,
    DND5E_TERMS, PF2E_TERMS, COC_TERMS, DELTA_GREEN_TERMS, BITD_TERMS,
    SAVAGE_WORLDS_TERMS, FATE_TERMS, PBTA_TERMS, MOTHERSHIP_TERMS,
    TRAVELLER_TERMS, GURPS_TERMS,
    RULEBOOK_INDICATORS, ADVENTURE_INDICATORS, BESTIARY_INDICATORS,
    SETTING_INDICATORS, PLAYER_OPTIONS_INDICATORS,
    PUBLISHERS, WEAPONS, ARMOR, MOTIVATIONS, BACKGROUNDS,
    // Vocabulary functions
    count_vocabulary_matches, find_vocabulary_matches,
    detect_genre_from_vocabulary, detect_content_category_from_vocabulary,
    detect_publisher_from_vocabulary,
    // Query processing (ported from MDMAI)
    TTRPG_CORE_VOCABULARY, QUERY_EXPANSIONS, QUERY_SYNONYMS, MECHANIC_TYPE_KEYWORDS,
    expand_query_term, expand_query, detect_mechanic_type, extract_semantic_keywords,
    fuzzy_match, levenshtein_distance, correct_spelling, correct_query_spelling,
    // BM25 and search (ported from MDMAI)
    BM25_STOP_WORDS, is_stop_word, filter_stop_words,
    SOURCE_BOOK_PATTERNS, detect_source_book,
    HEADER_PATTERNS, detect_header_level,
    DICE_PATTERNS, TABLE_ROW_PATTERNS, contains_dice_notation, count_dice_notation,
    // Configuration modules
    chunking_config, fusion_config,
};
pub use game_detector::{detect_game_system, detect_game_system_with_confidence, GameSystem, DetectionResult};
pub use boundary_scorer::{BoundaryScorer, BoundaryType, BoundaryMatch};
pub use cross_reference::{CrossReferenceExtractor, CrossReference, ReferenceType, ReferenceSummary};
pub use dice_extractor::{
    DiceExtractor, DiceExpression, DifficultyCheck, StandaloneModifier,
    DiceExtractionResult, STANDARD_DIE_SIDES,
};
