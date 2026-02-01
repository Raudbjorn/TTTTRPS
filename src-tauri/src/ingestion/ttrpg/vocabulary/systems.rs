//! Game System Vocabularies
//!
//! Defines the `GameVocabulary` trait and system-specific implementations
//! for D&D 5e, Pathfinder 2e, and other TTRPG systems.

use std::collections::HashSet;

// ============================================================================
// Trait
// ============================================================================

/// Trait for game system vocabularies.
///
/// Provides lists of game-specific terms that can be used for
/// attribute extraction and filtering.
pub trait GameVocabulary: Send + Sync {
    /// Get all damage types for this game system.
    fn damage_types(&self) -> &[&str];

    /// Get all creature types.
    fn creature_types(&self) -> &[&str];

    /// Get all conditions.
    fn conditions(&self) -> &[&str];

    /// Get all spell schools/traditions.
    fn spell_schools(&self) -> &[&str];

    /// Get all rarity levels.
    fn rarities(&self) -> &[&str];

    /// Get all size categories.
    fn sizes(&self) -> &[&str];

    /// Get ability score abbreviations and their full names.
    fn ability_abbreviations(&self) -> &[(&str, &str)];

    /// Get alignment values.
    fn alignments(&self) -> &[&str];

    /// Get all terms as a combined set (for quick lookups).
    fn all_terms(&self) -> HashSet<&str> {
        let mut terms = HashSet::new();
        terms.extend(self.damage_types());
        terms.extend(self.creature_types());
        terms.extend(self.conditions());
        terms.extend(self.spell_schools());
        terms.extend(self.rarities());
        terms.extend(self.sizes());
        terms.extend(self.alignments());
        for (abbr, full) in self.ability_abbreviations() {
            terms.insert(*abbr);
            terms.insert(*full);
        }
        terms
    }

    /// Get the canonical name for an ability score (normalize abbreviations).
    fn normalize_ability(&self, ability: &str) -> Option<&str> {
        let ability_lower = ability.to_lowercase();
        for (abbr, full) in self.ability_abbreviations() {
            if ability_lower == *abbr || ability_lower == *full {
                return Some(full);
            }
        }
        None
    }

    /// Get antonym pairs for this game system.
    fn antonym_pairs(&self) -> &[(&str, &str)] {
        &[]
    }
}

// ============================================================================
// D&D 5e Vocabulary
// ============================================================================

/// D&D 5th Edition vocabulary.
pub struct DnD5eVocabulary;

impl GameVocabulary for DnD5eVocabulary {
    fn damage_types(&self) -> &[&str] {
        &[
            "acid",
            "bludgeoning",
            "cold",
            "fire",
            "force",
            "lightning",
            "necrotic",
            "piercing",
            "poison",
            "psychic",
            "radiant",
            "slashing",
            "thunder",
        ]
    }

    fn creature_types(&self) -> &[&str] {
        &[
            "aberration",
            "beast",
            "celestial",
            "construct",
            "dragon",
            "elemental",
            "fey",
            "fiend",
            "giant",
            "humanoid",
            "monstrosity",
            "ooze",
            "plant",
            "undead",
        ]
    }

    fn conditions(&self) -> &[&str] {
        &[
            "blinded",
            "charmed",
            "deafened",
            "exhaustion",
            "frightened",
            "grappled",
            "incapacitated",
            "invisible",
            "paralyzed",
            "petrified",
            "poisoned",
            "prone",
            "restrained",
            "stunned",
            "unconscious",
        ]
    }

    fn spell_schools(&self) -> &[&str] {
        &[
            "abjuration",
            "conjuration",
            "divination",
            "enchantment",
            "evocation",
            "illusion",
            "necromancy",
            "transmutation",
        ]
    }

    fn rarities(&self) -> &[&str] {
        &[
            "common",
            "uncommon",
            "rare",
            "very rare",
            "legendary",
            "artifact",
        ]
    }

    fn sizes(&self) -> &[&str] {
        &["tiny", "small", "medium", "large", "huge", "gargantuan"]
    }

    fn ability_abbreviations(&self) -> &[(&str, &str)] {
        &[
            ("str", "strength"),
            ("dex", "dexterity"),
            ("con", "constitution"),
            ("int", "intelligence"),
            ("wis", "wisdom"),
            ("cha", "charisma"),
        ]
    }

    fn alignments(&self) -> &[&str] {
        &[
            "lawful good",
            "neutral good",
            "chaotic good",
            "lawful neutral",
            "true neutral",
            "neutral",
            "chaotic neutral",
            "lawful evil",
            "neutral evil",
            "chaotic evil",
            "unaligned",
        ]
    }

    fn antonym_pairs(&self) -> &[(&str, &str)] {
        &[
            ("fire", "cold"),
            ("radiant", "necrotic"),
            ("lawful", "chaotic"),
            ("good", "evil"),
            ("light", "darkness"),
        ]
    }
}

// ============================================================================
// Pathfinder 2e Vocabulary
// ============================================================================

/// Pathfinder 2nd Edition vocabulary.
pub struct Pf2eVocabulary;

impl GameVocabulary for Pf2eVocabulary {
    fn damage_types(&self) -> &[&str] {
        &[
            "acid",
            "bludgeoning",
            "cold",
            "electricity",
            "fire",
            "force",
            "mental",
            "negative",
            "piercing",
            "poison",
            "positive",
            "slashing",
            "sonic",
            // Precious material damage
            "cold iron",
            "silver",
            "adamantine",
            // Alignment damage
            "chaotic",
            "evil",
            "good",
            "lawful",
        ]
    }

    fn creature_types(&self) -> &[&str] {
        &[
            "aberration",
            "animal",
            "astral",
            "beast",
            "celestial",
            "construct",
            "dragon",
            "dream",
            "elemental",
            "ethereal",
            "fey",
            "fiend",
            "fungus",
            "giant",
            "humanoid",
            "monitor",
            "ooze",
            "petitioner",
            "plant",
            "spirit",
            "time",
            "undead",
        ]
    }

    fn conditions(&self) -> &[&str] {
        &[
            "blinded",
            "broken",
            "clumsy",
            "concealed",
            "confused",
            "controlled",
            "dazzled",
            "deafened",
            "doomed",
            "drained",
            "dying",
            "encumbered",
            "enfeebled",
            "fascinated",
            "fatigued",
            "flat-footed",
            "fleeing",
            "frightened",
            "grabbed",
            "hidden",
            "immobilized",
            "invisible",
            "observed",
            "paralyzed",
            "persistent damage",
            "petrified",
            "prone",
            "quickened",
            "restrained",
            "sickened",
            "slowed",
            "stunned",
            "stupefied",
            "unconscious",
            "undetected",
            "unfriendly",
            "unnoticed",
            "wounded",
        ]
    }

    fn spell_schools(&self) -> &[&str] {
        // PF2e remaster removed traditional schools, but legacy support
        &[
            "abjuration",
            "conjuration",
            "divination",
            "enchantment",
            "evocation",
            "illusion",
            "necromancy",
            "transmutation",
        ]
    }

    fn rarities(&self) -> &[&str] {
        &["common", "uncommon", "rare", "unique"]
    }

    fn sizes(&self) -> &[&str] {
        &["tiny", "small", "medium", "large", "huge", "gargantuan"]
    }

    fn ability_abbreviations(&self) -> &[(&str, &str)] {
        &[
            ("str", "strength"),
            ("dex", "dexterity"),
            ("con", "constitution"),
            ("int", "intelligence"),
            ("wis", "wisdom"),
            ("cha", "charisma"),
        ]
    }

    fn alignments(&self) -> &[&str] {
        // PF2e remaster uses edicts/anathema instead of alignment
        // but legacy support for older content
        &[
            "lawful good",
            "neutral good",
            "chaotic good",
            "lawful neutral",
            "true neutral",
            "neutral",
            "chaotic neutral",
            "lawful evil",
            "neutral evil",
            "chaotic evil",
            "no alignment",
        ]
    }

    fn antonym_pairs(&self) -> &[(&str, &str)] {
        &[
            ("fire", "cold"),
            ("positive", "negative"),
            ("lawful", "chaotic"),
            ("good", "evil"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dnd5e_damage_types() {
        let vocab = DnD5eVocabulary;
        assert!(vocab.damage_types().contains(&"fire"));
        assert!(vocab.damage_types().contains(&"cold"));
        assert!(!vocab.damage_types().contains(&"electricity")); // PF2e term
    }

    #[test]
    fn test_dnd5e_normalize_ability() {
        let vocab = DnD5eVocabulary;
        assert_eq!(vocab.normalize_ability("str"), Some("strength"));
        assert_eq!(vocab.normalize_ability("STR"), Some("strength"));
        assert_eq!(vocab.normalize_ability("strength"), Some("strength"));
        assert_eq!(vocab.normalize_ability("invalid"), None);
    }

    #[test]
    fn test_pf2e_creature_types() {
        let vocab = Pf2eVocabulary;
        assert!(vocab.creature_types().contains(&"aberration"));
        assert!(vocab.creature_types().contains(&"monitor")); // PF2e-specific
        assert!(!vocab.creature_types().contains(&"monstrosity")); // D&D term
    }

    #[test]
    fn test_all_terms() {
        let vocab = DnD5eVocabulary;
        let terms = vocab.all_terms();

        assert!(terms.contains("fire"));
        assert!(terms.contains("humanoid"));
        assert!(terms.contains("str"));
        assert!(terms.contains("strength"));
    }

    #[test]
    fn test_antonym_pairs() {
        let vocab = DnD5eVocabulary;
        let pairs = vocab.antonym_pairs();

        assert!(pairs.contains(&("fire", "cold")));
        assert!(pairs.contains(&("radiant", "necrotic")));
    }
}
