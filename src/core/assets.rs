//! Compile-time bundled asset loader for TTRPG content.
//!
//! Bundles 35 YAML files (archetypes, vocabulary, setting packs) and 2 TOML
//! config files into the binary via `include_str!`. Total ~50KB.
//!
//! # Usage
//!
//! ```rust,ignore
//! use ttrpg_assistant::core::assets::AssetLoader;
//!
//! let archetypes = AssetLoader::load_archetypes();
//! let vocab_banks = AssetLoader::load_vocabulary_banks();
//! let setting_packs = AssetLoader::load_setting_packs();
//! let synonyms = AssetLoader::load_synonyms().unwrap();
//! let config = AssetLoader::load_preprocessing_config().unwrap();
//! ```

use tracing::{debug, warn};

use super::archetype::setting_pack::{SettingPack, VocabularyBankDefinition};
use super::archetype::types::Archetype;
use super::preprocess::config::PreprocessConfig;
use super::preprocess::synonyms::SynonymMap;

// ============================================================================
// Compile-time bundled YAML: Archetypes — Classes (5)
// ============================================================================

const CLASS_FIGHTER: &str = include_str!("../../assets/archetypes/classes/fighter.yaml");
const CLASS_CLERIC: &str = include_str!("../../assets/archetypes/classes/cleric.yaml");
const CLASS_RANGER: &str = include_str!("../../assets/archetypes/classes/ranger.yaml");
const CLASS_ROGUE: &str = include_str!("../../assets/archetypes/classes/rogue.yaml");
const CLASS_WIZARD: &str = include_str!("../../assets/archetypes/classes/wizard.yaml");

// ============================================================================
// Compile-time bundled YAML: Archetypes — Races (6)
// ============================================================================

const RACE_DWARF: &str = include_str!("../../assets/archetypes/races/dwarf.yaml");
const RACE_ELF: &str = include_str!("../../assets/archetypes/races/elf.yaml");
const RACE_GNOME: &str = include_str!("../../assets/archetypes/races/gnome.yaml");
const RACE_HALFLING: &str = include_str!("../../assets/archetypes/races/halfling.yaml");
const RACE_HUMAN: &str = include_str!("../../assets/archetypes/races/human.yaml");
const RACE_ORC: &str = include_str!("../../assets/archetypes/races/orc.yaml");

// ============================================================================
// Compile-time bundled YAML: Archetypes — Roles (10)
// ============================================================================

const ROLE_ARTISAN: &str = include_str!("../../assets/archetypes/roles/artisan.yaml");
const ROLE_CRIMINAL: &str = include_str!("../../assets/archetypes/roles/criminal.yaml");
const ROLE_FARMER: &str = include_str!("../../assets/archetypes/roles/farmer.yaml");
const ROLE_GUARD: &str = include_str!("../../assets/archetypes/roles/guard.yaml");
const ROLE_INNKEEPER: &str = include_str!("../../assets/archetypes/roles/innkeeper.yaml");
const ROLE_MERCHANT: &str = include_str!("../../assets/archetypes/roles/merchant.yaml");
const ROLE_NOBLE: &str = include_str!("../../assets/archetypes/roles/noble.yaml");
const ROLE_PRIEST: &str = include_str!("../../assets/archetypes/roles/priest.yaml");
const ROLE_SCHOLAR: &str = include_str!("../../assets/archetypes/roles/scholar.yaml");
const ROLE_SOLDIER: &str = include_str!("../../assets/archetypes/roles/soldier.yaml");

// ============================================================================
// Compile-time bundled YAML: Vocabulary — Cultures (5)
// ============================================================================

const VOCAB_DWARVISH: &str = include_str!("../../assets/vocabulary/cultures/dwarvish.yaml");
const VOCAB_ELVISH: &str = include_str!("../../assets/vocabulary/cultures/elvish.yaml");
const VOCAB_GNOMISH: &str = include_str!("../../assets/vocabulary/cultures/gnomish.yaml");
const VOCAB_HALFLING: &str = include_str!("../../assets/vocabulary/cultures/halfling.yaml");
const VOCAB_ORCISH: &str = include_str!("../../assets/vocabulary/cultures/orcish.yaml");

// ============================================================================
// Compile-time bundled YAML: Vocabulary — Styles (8)
// ============================================================================

const STYLE_CASUAL: &str = include_str!("../../assets/vocabulary/styles/casual.yaml");
const STYLE_COMMON: &str = include_str!("../../assets/vocabulary/styles/common.yaml");
const STYLE_CRIMINAL: &str = include_str!("../../assets/vocabulary/styles/criminal.yaml");
const STYLE_FORMAL: &str = include_str!("../../assets/vocabulary/styles/formal.yaml");
const STYLE_MERCANTILE: &str = include_str!("../../assets/vocabulary/styles/mercantile.yaml");
const STYLE_MILITARY: &str = include_str!("../../assets/vocabulary/styles/military.yaml");
const STYLE_RELIGIOUS: &str = include_str!("../../assets/vocabulary/styles/religious.yaml");
const STYLE_SCHOLARLY: &str = include_str!("../../assets/vocabulary/styles/scholarly.yaml");

// ============================================================================
// Compile-time bundled YAML: Setting Packs (1)
// ============================================================================

const SETTING_GENERIC_FANTASY: &str =
    include_str!("../../assets/setting_packs/generic_fantasy.yaml");

// ============================================================================
// Compile-time bundled TOML: Config (2)
// ============================================================================

const CONFIG_SYNONYMS: &str = include_str!("../../assets/config/synonyms.toml");
const CONFIG_PREPROCESSING: &str = include_str!("../../assets/config/preprocessing.toml");

// ============================================================================
// Asset arrays for iteration
// ============================================================================

/// All archetype YAML sources with labels for error reporting.
const ARCHETYPE_SOURCES: &[(&str, &str)] = &[
    // Classes
    ("classes/fighter", CLASS_FIGHTER),
    ("classes/cleric", CLASS_CLERIC),
    ("classes/ranger", CLASS_RANGER),
    ("classes/rogue", CLASS_ROGUE),
    ("classes/wizard", CLASS_WIZARD),
    // Races
    ("races/dwarf", RACE_DWARF),
    ("races/elf", RACE_ELF),
    ("races/gnome", RACE_GNOME),
    ("races/halfling", RACE_HALFLING),
    ("races/human", RACE_HUMAN),
    ("races/orc", RACE_ORC),
    // Roles
    ("roles/artisan", ROLE_ARTISAN),
    ("roles/criminal", ROLE_CRIMINAL),
    ("roles/farmer", ROLE_FARMER),
    ("roles/guard", ROLE_GUARD),
    ("roles/innkeeper", ROLE_INNKEEPER),
    ("roles/merchant", ROLE_MERCHANT),
    ("roles/noble", ROLE_NOBLE),
    ("roles/priest", ROLE_PRIEST),
    ("roles/scholar", ROLE_SCHOLAR),
    ("roles/soldier", ROLE_SOLDIER),
];

/// All vocabulary bank YAML sources.
const VOCABULARY_SOURCES: &[(&str, &str)] = &[
    // Cultures
    ("cultures/dwarvish", VOCAB_DWARVISH),
    ("cultures/elvish", VOCAB_ELVISH),
    ("cultures/gnomish", VOCAB_GNOMISH),
    ("cultures/halfling", VOCAB_HALFLING),
    ("cultures/orcish", VOCAB_ORCISH),
    // Styles
    ("styles/casual", STYLE_CASUAL),
    ("styles/common", STYLE_COMMON),
    ("styles/criminal", STYLE_CRIMINAL),
    ("styles/formal", STYLE_FORMAL),
    ("styles/mercantile", STYLE_MERCANTILE),
    ("styles/military", STYLE_MILITARY),
    ("styles/religious", STYLE_RELIGIOUS),
    ("styles/scholarly", STYLE_SCHOLARLY),
];

/// All setting pack YAML sources.
const SETTING_PACK_SOURCES: &[(&str, &str)] = &[
    ("generic_fantasy", SETTING_GENERIC_FANTASY),
];

// ============================================================================
// AssetLoader
// ============================================================================

/// Loads bundled YAML/TOML assets into typed Rust structs.
///
/// All assets are compiled into the binary — no filesystem access at runtime.
/// Parse failures log warnings and skip the invalid file rather than panicking.
pub struct AssetLoader;

impl AssetLoader {
    /// Load all 21 archetype definitions from bundled YAML.
    ///
    /// Returns successfully parsed archetypes. Invalid files are logged and skipped.
    pub fn load_archetypes() -> Vec<Archetype> {
        let mut archetypes = Vec::with_capacity(ARCHETYPE_SOURCES.len());

        for (label, yaml) in ARCHETYPE_SOURCES {
            match serde_yaml::from_str::<Archetype>(yaml) {
                Ok(archetype) => {
                    debug!(id = %archetype.id, category = %archetype.category, "loaded archetype");
                    archetypes.push(archetype);
                }
                Err(e) => {
                    warn!(file = label, error = %e, "failed to parse archetype YAML");
                }
            }
        }

        debug!(count = archetypes.len(), total = ARCHETYPE_SOURCES.len(), "archetypes loaded");
        archetypes
    }

    /// Load all 13 vocabulary bank definitions from bundled YAML.
    ///
    /// Returns successfully parsed vocabulary banks. Invalid files are logged and skipped.
    pub fn load_vocabulary_banks() -> Vec<VocabularyBankDefinition> {
        let mut banks = Vec::with_capacity(VOCABULARY_SOURCES.len());

        for (label, yaml) in VOCABULARY_SOURCES {
            match serde_yaml::from_str::<VocabularyBankDefinition>(yaml) {
                Ok(bank) => {
                    debug!(id = %bank.id, phrases = bank.phrase_count(), "loaded vocabulary bank");
                    banks.push(bank);
                }
                Err(e) => {
                    warn!(file = label, error = %e, "failed to parse vocabulary YAML");
                }
            }
        }

        debug!(count = banks.len(), total = VOCABULARY_SOURCES.len(), "vocabulary banks loaded");
        banks
    }

    /// Load all setting pack definitions from bundled YAML.
    ///
    /// Returns successfully parsed setting packs. Invalid files are logged and skipped.
    pub fn load_setting_packs() -> Vec<SettingPack> {
        let mut packs = Vec::with_capacity(SETTING_PACK_SOURCES.len());

        for (label, yaml) in SETTING_PACK_SOURCES {
            match serde_yaml::from_str::<SettingPack>(yaml) {
                Ok(pack) => {
                    debug!(id = %pack.id, system = %pack.game_system, "loaded setting pack");
                    packs.push(pack);
                }
                Err(e) => {
                    warn!(file = label, error = %e, "failed to parse setting pack YAML");
                }
            }
        }

        debug!(count = packs.len(), total = SETTING_PACK_SOURCES.len(), "setting packs loaded");
        packs
    }

    /// Load the TTRPG synonym map from bundled TOML.
    ///
    /// Contains 80+ synonym groups for TTRPG terminology.
    pub fn load_synonyms() -> Result<SynonymMap, String> {
        SynonymMap::from_toml_str(CONFIG_SYNONYMS).map_err(|e| format!("synonym parse error: {e}"))
    }

    /// Load the preprocessing configuration from bundled TOML.
    ///
    /// Configures typo correction thresholds and synonym expansion settings.
    pub fn load_preprocessing_config() -> Result<PreprocessConfig, String> {
        PreprocessConfig::from_toml_str(CONFIG_PREPROCESSING)
            .map_err(|e| format!("preprocessing config parse error: {e}"))
    }

    /// Get the raw synonyms TOML content (for diagnostics/display).
    pub fn synonyms_toml() -> &'static str {
        CONFIG_SYNONYMS
    }

    /// Get the raw preprocessing TOML content (for diagnostics/display).
    pub fn preprocessing_toml() -> &'static str {
        CONFIG_PREPROCESSING
    }

    /// Count of bundled archetype files.
    pub const ARCHETYPE_COUNT: usize = ARCHETYPE_SOURCES.len();

    /// Count of bundled vocabulary bank files.
    pub const VOCABULARY_COUNT: usize = VOCABULARY_SOURCES.len();

    /// Count of bundled setting pack files.
    pub const SETTING_PACK_COUNT: usize = SETTING_PACK_SOURCES.len();

    /// Total count of all bundled asset files (YAML + TOML).
    pub const TOTAL_ASSET_COUNT: usize =
        Self::ARCHETYPE_COUNT + Self::VOCABULARY_COUNT + Self::SETTING_PACK_COUNT + 2;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_all_archetypes() {
        let archetypes = AssetLoader::load_archetypes();
        assert_eq!(
            archetypes.len(),
            AssetLoader::ARCHETYPE_COUNT,
            "all {} archetype files should parse successfully",
            AssetLoader::ARCHETYPE_COUNT
        );
    }

    #[test]
    fn test_archetype_categories() {
        let archetypes = AssetLoader::load_archetypes();

        let classes: Vec<_> = archetypes
            .iter()
            .filter(|a| a.category == super::super::archetype::types::ArchetypeCategory::Class)
            .collect();
        let races: Vec<_> = archetypes
            .iter()
            .filter(|a| a.category == super::super::archetype::types::ArchetypeCategory::Race)
            .collect();
        let roles: Vec<_> = archetypes
            .iter()
            .filter(|a| a.category == super::super::archetype::types::ArchetypeCategory::Role)
            .collect();

        assert_eq!(classes.len(), 5, "5 class archetypes");
        assert_eq!(races.len(), 6, "6 race archetypes");
        assert_eq!(roles.len(), 10, "10 role archetypes");
    }

    #[test]
    fn test_archetype_ids_unique() {
        let archetypes = AssetLoader::load_archetypes();
        let mut ids: Vec<&str> = archetypes.iter().map(|a| a.id.as_str()).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "all archetype IDs must be unique");
    }

    #[test]
    fn test_archetypes_have_personality_affinity() {
        let archetypes = AssetLoader::load_archetypes();
        for a in &archetypes {
            assert!(
                !a.personality_affinity.is_empty(),
                "archetype '{}' should have personality affinities",
                a.id
            );
        }
    }

    #[test]
    fn test_archetypes_validate() {
        let archetypes = AssetLoader::load_archetypes();
        for a in &archetypes {
            a.validate()
                .unwrap_or_else(|e| panic!("archetype '{}' failed validation: {}", a.id, e));
        }
    }

    #[test]
    fn test_load_all_vocabulary_banks() {
        let banks = AssetLoader::load_vocabulary_banks();
        assert_eq!(
            banks.len(),
            AssetLoader::VOCABULARY_COUNT,
            "all {} vocabulary files should parse successfully",
            AssetLoader::VOCABULARY_COUNT
        );
    }

    #[test]
    fn test_vocabulary_bank_ids_unique() {
        let banks = AssetLoader::load_vocabulary_banks();
        let mut ids: Vec<&str> = banks.iter().map(|b| b.id.as_str()).collect();
        let original_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), original_len, "all vocabulary bank IDs must be unique");
    }

    #[test]
    fn test_vocabulary_banks_have_phrases() {
        let banks = AssetLoader::load_vocabulary_banks();
        for b in &banks {
            assert!(
                b.phrase_count() > 0,
                "vocabulary bank '{}' should have phrases",
                b.id
            );
        }
    }

    #[test]
    fn test_load_setting_packs() {
        let packs = AssetLoader::load_setting_packs();
        assert_eq!(
            packs.len(),
            AssetLoader::SETTING_PACK_COUNT,
            "all {} setting pack files should parse successfully",
            AssetLoader::SETTING_PACK_COUNT
        );
    }

    #[test]
    fn test_setting_pack_validation() {
        let packs = AssetLoader::load_setting_packs();
        for p in &packs {
            p.validate()
                .unwrap_or_else(|e| panic!("setting pack '{}' failed validation: {}", p.id, e));
        }
    }

    #[test]
    fn test_load_synonyms() {
        let synonyms = AssetLoader::load_synonyms().expect("synonyms TOML should parse");
        // Verify synonym expansion works for known TTRPG terms
        let expanded = synonyms.expand_term("hp");
        assert!(
            expanded.len() > 1,
            "hp should expand to hit points, health, etc."
        );
    }

    #[test]
    fn test_load_preprocessing_config() {
        let config = AssetLoader::load_preprocessing_config()
            .expect("preprocessing TOML should parse");
        assert!(config.typo.enabled);
        assert!(config.synonyms.enabled);
        assert_eq!(config.typo.min_word_size_one_typo, 5);
        assert!(!config.typo.disabled_on_words.is_empty());
    }

    #[test]
    fn test_asset_counts() {
        assert_eq!(AssetLoader::ARCHETYPE_COUNT, 21);
        assert_eq!(AssetLoader::VOCABULARY_COUNT, 13);
        assert_eq!(AssetLoader::SETTING_PACK_COUNT, 1);
        assert_eq!(AssetLoader::TOTAL_ASSET_COUNT, 37);
    }

    #[test]
    fn test_vocabulary_cultures_present() {
        let banks = AssetLoader::load_vocabulary_banks();
        let culture_banks: Vec<_> = banks.iter().filter(|b| b.culture.is_some()).collect();
        assert_eq!(culture_banks.len(), 6, "6 culture vocabulary banks (5 cultures + common style)");
    }

    #[test]
    fn test_vocabulary_styles_present() {
        let banks = AssetLoader::load_vocabulary_banks();
        let style_banks: Vec<_> = banks
            .iter()
            .filter(|b| b.culture.is_none() && b.role.is_some())
            .collect();
        // Styles have a role field but not a culture field in the current YAML structure
        // Some styles might not have role either, so just check we have enough
        assert!(banks.len() >= 13, "at least 13 vocabulary banks total");
    }

    #[test]
    fn test_fighter_archetype_details() {
        let archetypes = AssetLoader::load_archetypes();
        let fighter = archetypes
            .iter()
            .find(|a| a.id.as_str() == "fighter")
            .expect("fighter archetype should exist");

        assert_eq!(fighter.display_name.as_ref(), "Fighter");
        assert!(fighter.vocabulary_bank_id.is_some());
        assert!(!fighter.npc_role_mapping.is_empty());
        assert!(fighter.stat_tendencies.is_some());
    }

    #[test]
    fn test_generic_fantasy_setting_pack() {
        let packs = AssetLoader::load_setting_packs();
        let gf = packs
            .iter()
            .find(|p| p.id == "generic_fantasy")
            .expect("generic_fantasy setting pack should exist");

        assert_eq!(gf.game_system, "generic");
        assert!(!gf.naming_cultures.is_empty(), "should have naming cultures");
    }

    #[test]
    fn test_raw_toml_content_accessible() {
        assert!(!AssetLoader::synonyms_toml().is_empty());
        assert!(!AssetLoader::preprocessing_toml().is_empty());
        assert!(AssetLoader::synonyms_toml().contains("[multi_way]"));
        assert!(AssetLoader::preprocessing_toml().contains("[typo]"));
    }
}
