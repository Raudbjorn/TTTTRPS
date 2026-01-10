//! TTRPG Search Enhancement Module
//!
//! Provides intelligent search capabilities for TTRPG content including:
//! - Query parsing with negation support
//! - Query expansion with synonyms and abbreviations
//! - Antonym-based penalty scoring
//! - Reciprocal Rank Fusion (RRF) result ranking
//! - Background indexing queue with retry logic
//! - Meilisearch filter string building

pub mod query_parser;
pub mod query_expansion;
pub mod antonym_mapper;
pub mod result_ranker;
pub mod index_queue;
pub mod attribute_filter;
pub mod ttrpg_constants;

pub use query_parser::{QueryParser, QueryConstraints, RequiredAttribute};
pub use query_expansion::QueryExpander;
pub use antonym_mapper::AntonymMapper;
pub use result_ranker::{ResultRanker, RankingConfig, ScoreBreakdown, RankedResult, SearchCandidate};
pub use index_queue::{IndexQueue, PendingDocument};
pub use attribute_filter::AttributeFilter;
pub use ttrpg_constants::{
    TTRPGGenre, CharacterClass, CharacterRace, CharacterTrait, TraitCategory,
    CharacterBackground, CharacterMotivation, NPCRole, WeaponType, ItemType,
    EquipmentQuality, NamePools, EquipmentPools,
};
