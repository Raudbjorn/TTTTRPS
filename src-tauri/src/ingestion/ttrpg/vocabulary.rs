//! Comprehensive TTRPG Vocabulary Module
//!
//! Defines game-specific vocabularies for different TTRPG systems.
//! Used by the attribute extractor to identify game-specific terms,
//! detect content categories, genres, and publishers for embedding enrichment.

use std::collections::HashSet;
use once_cell::sync::Lazy;

// ============================================================================
// GENRES
// ============================================================================

/// Primary TTRPG genres for content classification
pub static GENRES: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "fantasy", "science fiction", "horror", "cosmic horror", "modern horror",
    "cyberpunk", "post-apocalyptic", "steampunk", "urban fantasy", "dark fantasy",
    "high fantasy", "low fantasy", "grimdark", "sword and sorcery", "space opera",
    "military sci-fi", "western", "pulp", "noir", "supernatural", "historical",
    "alternate history", "supers", "superhero",
]);

// ============================================================================
// CHARACTER CLASSES BY GENRE
// ============================================================================

/// Fantasy character classes
pub static FANTASY_CLASSES: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "barbarian", "bard", "cleric", "druid", "fighter", "monk", "paladin",
    "ranger", "rogue", "sorcerer", "warlock", "wizard", "artificer",
    "knight", "thief", "assassin", "necromancer", "illusionist",
    "conjurer", "diviner", "enchanter", "evoker", "transmuter",
    "abjurer", "bladesinger", "eldritch knight", "arcane trickster",
    "champion", "battle master", "cavalier", "samurai", "kensei",
    "swashbuckler", "inquisitor", "oracle", "witch", "magus",
    "gunslinger", "alchemist", "investigator", "summoner", "kineticist",
]);

/// Sci-fi character classes/roles
pub static SCIFI_CLASSES: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "soldier", "engineer", "scientist", "pilot", "medic", "hacker",
    "operative", "diplomat", "merchant", "bounty hunter", "smuggler",
    "technician", "psion", "mechanic", "scout", "marine", "navigator",
    "gunner", "captain", "commodore", "envoy", "mystic", "solarian",
]);

/// Horror character archetypes
pub static HORROR_CLASSES: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "investigator", "antiquarian", "author", "dilettante", "doctor",
    "journalist", "lawyer", "librarian", "military officer", "nurse",
    "parapsychologist", "photographer", "police detective", "priest",
    "private eye", "professor", "psychologist", "scientist", "soldier",
    "spy", "student", "tribal member", "zealot", "drifter", "entertainer",
    "federal agent", "clergy", "alienist", "archeologist", "athlete",
]);

/// Modern/contemporary character types
pub static MODERN_CLASSES: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "street samurai", "decker", "rigger", "face", "mage", "shaman",
    "adept", "technomancer", "infiltrator", "wheelman", "fixer",
    "cleaner", "hitter", "hacker", "thief", "mastermind", "driver",
    "muscle", "grifter", "researcher", "reporter", "cop", "agent",
]);

// ============================================================================
// RACES AND SPECIES
// ============================================================================

/// Fantasy races
pub static FANTASY_RACES: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "human", "elf", "dwarf", "halfling", "gnome", "half-elf", "half-orc",
    "tiefling", "dragonborn", "aasimar", "genasi", "goliath", "tabaxi",
    "kenku", "firbolg", "triton", "yuan-ti", "bugbear", "goblin",
    "hobgoblin", "kobold", "orc", "lizardfolk", "tortle", "aarakocra",
    "warforged", "changeling", "shifter", "kalashtar", "leonin",
    "satyr", "fairy", "harengon", "owlin", "dhampir", "hexblood",
    "reborn", "autognome", "giff", "hadozee", "plasmoid", "thri-kreen",
]);

/// Sci-fi species
pub static SCIFI_RACES: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "android", "cyborg", "alien", "mutant", "clone", "uplifted",
    "synthetic", "vat-grown", "gene-modified", "transhuman", "posthuman",
    "vesk", "ysoki", "lashunta", "shirren", "kasatha",
    "skittermander", "vlaka", "asari", "turian", "salarian", "krogan",
]);

// ============================================================================
// SYSTEM-SPECIFIC TERMS
// ============================================================================

/// D&D 5e specific terms for detection
pub static DND5E_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "5th edition", "5e", "advantage", "disadvantage", "proficiency bonus",
    "ability check", "saving throw", "attack roll", "spell slot", "cantrip",
    "concentration", "bonus action", "reaction", "initiative", "hit points",
    "armor class", "difficulty class", "dc", "d20", "multiclass",
    "subclass", "background", "feat", "inspiration", "exhaustion",
    "short rest", "long rest", "attunement", "legendary action",
    "lair action", "legendary resistance", "death saving throw",
    "critical hit", "natural 20", "nat 20", "bardic inspiration",
    "sneak attack", "divine smite", "wild shape", "rage", "ki points",
    "sorcery points", "warlock patron", "eldritch invocation",
    "fighting style", "channel divinity", "action surge", "second wind",
]);

/// Pathfinder 2e specific terms
pub static PF2E_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "pathfinder 2e", "pf2e", "2nd edition", "three-action economy",
    "activity", "free action", "reaction", "condition", "trait",
    "ancestry", "heritage", "class feat", "skill feat", "general feat",
    "ancestry feat", "archetype", "dedication", "spell heightening",
    "incapacitation", "minion", "summoned", "persistent damage",
    "golarion", "inner sea", "absalom", "lost omens", "age of lost omens",
    "starstone", "pathfinder society", "critical specialization",
    "recall knowledge", "action economy", "degree of success",
    "hero point", "encounter mode", "exploration mode", "downtime mode",
]);

/// Call of Cthulhu specific terms
pub static COC_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "call of cthulhu", "coc", "7th edition", "7e", "keeper", "mythos",
    "sanity", "sanity check", "san", "san loss", "indefinite insanity",
    "temporary insanity", "bout of madness", "phobia", "mania",
    "luck", "luck roll", "pushed roll", "bonus die", "penalty die",
    "hard success", "extreme success", "fumble", "impale", "major wound",
    "hit location", "occupation", "credit rating", "cthulhu mythos",
    "mythos tome", "spell", "elder sign", "elder thing", "great old one",
    "outer god", "deep one", "shoggoth", "mi-go", "yithian", "byakhee",
    "dimensional shambler", "hound of tindalos", "nightgaunt", "ghoul",
    "lovecraft", "lovecraftian", "cosmic horror", "cultist", "cult",
]);

/// Delta Green specific terms
pub static DELTA_GREEN_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "delta green", "dg", "handler", "agent", "cell", "program", "opera",
    "night at the opera", "the program", "the outlaws", "majestic-12",
    "mj-12", "phenomen-x", "march technologies", "saucerwatch",
    "bond", "bonds", "sanity", "breaking point", "adapted", "hardened",
    "projection", "home", "ritual", "unnatural", "willpower",
    "violence", "helplessness", "san loss", "federal agent", "green box",
    "operation", "need to know", "case officer", "friendly", "the thing",
    "karotechia", "gru sv-8", "pisces", "project rainbow", "working group",
]);

/// Blades in the Dark specific terms
pub static BITD_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "blades in the dark", "bitd", "forged in the dark", "fitd",
    "doskvol", "duskwall", "duskvol", "ghost", "spirit", "demon",
    "leviathan", "electroplasm", "ghost field", "deathseeker crow",
    "score", "crew", "claim", "turf", "heat", "wanted level", "tier",
    "hold", "rep", "coin", "stash", "stress", "trauma", "harm",
    "resistance roll", "action roll", "fortune roll", "effect",
    "position", "controlled", "risky", "desperate", "devil's bargain",
    "flashback", "load", "downtime", "entanglement", "faction",
    "assassin", "bravos", "cult", "hawkers", "shadows", "smugglers",
    "cutter", "hound", "leech", "lurk", "slide", "spider", "whisper",
]);

/// Savage Worlds specific terms
pub static SAVAGE_WORLDS_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "savage worlds", "swade", "adventure edition", "bennies", "benny",
    "wild die", "ace", "exploding dice", "raise", "wound", "shaken",
    "incapacitated", "soak roll", "edges", "hindrances", "trait",
    "attribute", "skill", "wild card", "extra", "power points",
    "arcane background", "deadlands", "rippers", "east texas university",
    "interface zero", "pinnacle", "fast furious fun", "test of wills",
    "agility trick", "smarts trick", "gang up", "support",
]);

/// FATE specific terms
pub static FATE_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "fate", "fate core", "fate accelerated", "fae", "fate condensed",
    "aspect", "invoke", "compel", "fate point", "refresh", "stunt",
    "approach", "skill", "stress", "consequence", "concession", "taken out",
    "boost", "situation aspect", "character aspect", "high concept",
    "trouble", "phase trio", "bronze rule", "fate fractal",
    "four actions", "overcome", "create advantage", "attack", "defend",
    "shift", "ladder", "fudge dice", "fate dice",
]);

/// Powered by the Apocalypse terms
pub static PBTA_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "powered by the apocalypse", "pbta", "move", "basic move", "playbook",
    "mc", "master of ceremonies", "hard move", "soft move", "hit", "miss",
    "partial success", "7-9", "10+", "6-", "hold", "forward", "ongoing",
    "apocalypse world", "dungeon world", "monster of the week", "masks",
    "urban shadows", "monsterhearts", "the sprawl", "impulse drive",
    "harm", "countdown", "front", "threat", "agenda", "principles",
]);

/// Mothership specific terms
pub static MOTHERSHIP_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "mothership", "0e", "1e", "sci-fi horror", "stress", "panic check",
    "panic table", "android", "scientist", "teamster", "marine",
    "warden", "player's survival guide", "dead planet", "gradient descent",
    "pound of flesh", "hull breach", "vacuum", "hyperspace",
    "cryosleep", "company", "corporate", "colonial marines",
]);

/// Traveller specific terms
pub static TRAVELLER_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "traveller", "mongoose traveller", "2d6", "career", "term",
    "mustering out", "characteristic", "skill check", "task chain",
    "jump drive", "jump space", "parsec", "starport", "trade goods",
    "patron", "starship", "spacecraft", "ship share", "cr", "credits",
    "imperium", "third imperium", "spinward marches", "solomani",
    "vilani", "zhodani", "vargr", "aslan", "subsector", "sector",
    "world profile", "uwp", "tech level", "law level", "government type",
]);

/// GURPS specific terms
pub static GURPS_TERMS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "gurps", "generic universal", "steve jackson games", "3d6",
    "character points", "advantages", "disadvantages", "quirks",
    "skills", "techniques", "default", "attribute", "secondary characteristic",
    "active defense", "dodge", "parry", "block", "damage resistance",
    "hit points", "fatigue points", "basic speed", "basic move",
    "reaction modifier", "tech level", "magic system", "psionic",
]);

// ============================================================================
// CONTENT CATEGORIES
// ============================================================================

/// Indicators for rulebook content
pub static RULEBOOK_INDICATORS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "rules", "mechanics", "character creation", "ability scores",
    "combat", "spellcasting", "equipment", "skills", "feats",
    "classes", "races", "leveling", "advancement", "actions",
    "conditions", "saving throws", "difficulty class", "modifier",
    "proficiency", "multiclassing", "prerequisites", "chapter",
    "core rules", "basic rules", "advanced rules", "optional rules",
    "variant rules", "house rules", "errata", "clarification",
]);

/// Indicators for adventure/scenario content
pub static ADVENTURE_INDICATORS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "adventure", "scenario", "module", "campaign", "quest", "mission",
    "hook", "encounter", "scene", "act", "chapter", "epilogue",
    "prologue", "read aloud", "boxed text", "handout", "player handout",
    "map", "dungeon", "location", "npc", "villain", "boss",
    "treasure", "reward", "loot", "experience points", "xp",
    "random encounter", "wandering monster", "investigation",
]);

/// Indicators for bestiary/monster content
pub static BESTIARY_INDICATORS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "monster", "creature", "beast", "stat block", "challenge rating",
    "cr", "hit dice", "hd", "armor class", "ac", "attack", "damage",
    "special abilities", "legendary actions", "lair actions",
    "legendary resistance", "traits", "actions", "reactions",
    "bestiary", "monster manual", "creature catalog", "enemies",
    "adversaries", "npc stats", "minion", "elite", "solo",
]);

/// Indicators for setting/worldbuilding content
pub static SETTING_INDICATORS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "setting", "world", "campaign setting", "gazetteer", "atlas",
    "geography", "history", "timeline", "culture", "society",
    "religion", "pantheon", "deity", "god", "goddess", "faction",
    "organization", "guild", "nation", "kingdom", "empire", "city",
    "town", "village", "region", "continent", "plane", "realm",
    "cosmology", "calendar", "economy", "politics", "law",
]);

/// Indicators for player options content
pub static PLAYER_OPTIONS_INDICATORS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "player option", "character option", "new class", "new subclass",
    "new race", "new ancestry", "new feat", "new spell", "new item",
    "magic item", "artifact", "equipment", "weapon", "armor",
    "background", "archetype", "prestige class", "paragon path",
    "epic destiny", "playbook", "specialization", "talent tree",
]);

// ============================================================================
// PUBLISHERS
// ============================================================================

/// Known TTRPG publishers
pub static PUBLISHERS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "wizards of the coast", "wotc", "paizo", "chaosium",
    "arc dream", "arc dream publishing", "evil hat", "evil hat productions",
    "pinnacle entertainment", "pinnacle entertainment group", "peg",
    "free league", "free league publishing", "modiphius", "modiphius entertainment",
    "monte cook games", "mcg", "pelgrane press", "steve jackson games", "sjg",
    "fantasy flight games", "ffg", "cubicle 7", "r. talsorian games",
    "green ronin", "green ronin publishing", "kobold press",
    "tuesday knight games", "magpie games", "onyx path", "onyx path publishing",
    "white wolf", "renegade game studios", "goodman games",
]);

// ============================================================================
// EQUIPMENT
// ============================================================================

/// Common weapon types
pub static WEAPONS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "sword", "longsword", "shortsword", "greatsword", "rapier", "scimitar",
    "dagger", "knife", "axe", "battleaxe", "greataxe", "handaxe",
    "mace", "morningstar", "flail", "warhammer", "maul", "club",
    "staff", "quarterstaff", "spear", "javelin", "trident", "halberd",
    "glaive", "pike", "lance", "whip", "bow", "longbow", "shortbow",
    "crossbow", "light crossbow", "heavy crossbow", "hand crossbow",
    "sling", "dart", "blowgun", "net", "pistol", "rifle", "shotgun",
    "submachine gun", "assault rifle", "sniper rifle", "laser",
    "plasma", "blaster", "beam weapon", "energy weapon",
]);

/// Common armor types
pub static ARMOR: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "leather armor", "studded leather", "hide armor", "chain shirt",
    "chain mail", "chainmail", "scale mail", "breastplate", "half plate",
    "plate armor", "full plate", "shield", "buckler", "helmet", "helm",
    "gauntlet", "greaves", "vambrace", "padded armor", "ring mail",
    "splint armor", "flak jacket", "kevlar", "body armor", "power armor",
    "combat armor", "vacc suit", "environmental suit", "hardsuit",
]);

// ============================================================================
// CHARACTER TRAITS
// ============================================================================

/// Common character motivations
pub static MOTIVATIONS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "revenge", "redemption", "glory", "wealth", "power", "knowledge",
    "justice", "freedom", "protection", "discovery", "survival",
    "fame", "legacy", "duty", "honor", "love", "loyalty", "faith",
    "curiosity", "ambition", "escape", "belonging", "identity",
]);

/// Common character backgrounds
pub static BACKGROUNDS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    "acolyte", "charlatan", "criminal", "entertainer", "folk hero",
    "guild artisan", "hermit", "noble", "outlander", "sage", "sailor",
    "soldier", "urchin", "spy", "pirate", "knight", "gladiator",
    "haunted one", "far traveler", "inheritor", "mercenary veteran",
    "city watch", "clan crafter", "cloistered scholar", "courtier",
    "faction agent", "marine", "shipwright", "smuggler", "urban bounty hunter",
]);

// ============================================================================
// GLOBAL VOCABULARY FUNCTIONS
// ============================================================================

/// Count how many terms from a vocabulary list appear in the text
pub fn count_vocabulary_matches(text: &str, vocabulary: &[&str]) -> usize {
    let text_lower = text.to_lowercase();
    vocabulary.iter()
        .filter(|term| text_lower.contains(&term.to_lowercase()))
        .count()
}

/// Find all matching terms from a vocabulary list in the text
pub fn find_vocabulary_matches<'a>(text: &str, vocabulary: &'a [&'a str]) -> Vec<&'a str> {
    let text_lower = text.to_lowercase();
    vocabulary.iter()
        .filter(|term| text_lower.contains(&term.to_lowercase()))
        .copied()
        .collect()
}

/// Detect the primary genre based on vocabulary matches
pub fn detect_genre_from_vocabulary(text: &str) -> Option<&'static str> {
    let text_lower = text.to_lowercase();

    let horror_score = count_vocabulary_matches(&text_lower, &COC_TERMS)
        + count_vocabulary_matches(&text_lower, &DELTA_GREEN_TERMS)
        + count_vocabulary_matches(&text_lower, &MOTHERSHIP_TERMS);

    let fantasy_score = count_vocabulary_matches(&text_lower, &DND5E_TERMS)
        + count_vocabulary_matches(&text_lower, &PF2E_TERMS)
        + count_vocabulary_matches(&text_lower, &FANTASY_CLASSES)
        + count_vocabulary_matches(&text_lower, &FANTASY_RACES);

    let scifi_score = count_vocabulary_matches(&text_lower, &TRAVELLER_TERMS)
        + count_vocabulary_matches(&text_lower, &SCIFI_CLASSES)
        + count_vocabulary_matches(&text_lower, &SCIFI_RACES);

    let noir_score = count_vocabulary_matches(&text_lower, &BITD_TERMS);

    let max_score = horror_score.max(fantasy_score).max(scifi_score).max(noir_score);

    if max_score == 0 {
        return None;
    }

    if horror_score == max_score {
        Some("horror")
    } else if fantasy_score == max_score {
        Some("fantasy")
    } else if scifi_score == max_score {
        Some("science fiction")
    } else if noir_score == max_score {
        Some("noir")
    } else {
        None
    }
}

/// Detect content category based on vocabulary matches
pub fn detect_content_category_from_vocabulary(text: &str) -> Option<&'static str> {
    let text_lower = text.to_lowercase();

    let rulebook_score = count_vocabulary_matches(&text_lower, &RULEBOOK_INDICATORS);
    let adventure_score = count_vocabulary_matches(&text_lower, &ADVENTURE_INDICATORS);
    let bestiary_score = count_vocabulary_matches(&text_lower, &BESTIARY_INDICATORS);
    let setting_score = count_vocabulary_matches(&text_lower, &SETTING_INDICATORS);
    let player_options_score = count_vocabulary_matches(&text_lower, &PLAYER_OPTIONS_INDICATORS);

    let max_score = rulebook_score
        .max(adventure_score)
        .max(bestiary_score)
        .max(setting_score)
        .max(player_options_score);

    if max_score < 3 {
        return None;
    }

    if rulebook_score == max_score {
        Some("rulebook")
    } else if adventure_score == max_score {
        Some("adventure")
    } else if bestiary_score == max_score {
        Some("bestiary")
    } else if setting_score == max_score {
        Some("setting")
    } else if player_options_score == max_score {
        Some("player options")
    } else {
        None
    }
}

/// Detect publisher from text
pub fn detect_publisher_from_vocabulary(text: &str) -> Option<&'static str> {
    let text_lower = text.to_lowercase();

    for publisher in PUBLISHERS.iter() {
        if text_lower.contains(publisher) {
            return Some(publisher);
        }
    }

    None
}

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
            "acid", "bludgeoning", "cold", "fire", "force", "lightning",
            "necrotic", "piercing", "poison", "psychic", "radiant",
            "slashing", "thunder",
        ]
    }

    fn creature_types(&self) -> &[&str] {
        &[
            "aberration", "beast", "celestial", "construct", "dragon",
            "elemental", "fey", "fiend", "giant", "humanoid", "monstrosity",
            "ooze", "plant", "undead",
        ]
    }

    fn conditions(&self) -> &[&str] {
        &[
            "blinded", "charmed", "deafened", "exhaustion", "frightened",
            "grappled", "incapacitated", "invisible", "paralyzed", "petrified",
            "poisoned", "prone", "restrained", "stunned", "unconscious",
        ]
    }

    fn spell_schools(&self) -> &[&str] {
        &[
            "abjuration", "conjuration", "divination", "enchantment",
            "evocation", "illusion", "necromancy", "transmutation",
        ]
    }

    fn rarities(&self) -> &[&str] {
        &["common", "uncommon", "rare", "very rare", "legendary", "artifact"]
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
            "lawful good", "neutral good", "chaotic good",
            "lawful neutral", "true neutral", "neutral", "chaotic neutral",
            "lawful evil", "neutral evil", "chaotic evil",
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
            "acid", "bludgeoning", "cold", "electricity", "fire", "force",
            "mental", "negative", "piercing", "poison", "positive",
            "slashing", "sonic",
            // Precious material damage
            "cold iron", "silver", "adamantine",
            // Alignment damage
            "chaotic", "evil", "good", "lawful",
        ]
    }

    fn creature_types(&self) -> &[&str] {
        &[
            "aberration", "animal", "astral", "beast", "celestial",
            "construct", "dragon", "dream", "elemental", "ethereal",
            "fey", "fiend", "fungus", "giant", "humanoid", "monitor",
            "ooze", "petitioner", "plant", "spirit", "time", "undead",
        ]
    }

    fn conditions(&self) -> &[&str] {
        &[
            "blinded", "broken", "clumsy", "concealed", "confused",
            "controlled", "dazzled", "deafened", "doomed", "drained",
            "dying", "encumbered", "enfeebled", "fascinated", "fatigued",
            "flat-footed", "fleeing", "frightened", "grabbed", "hidden",
            "immobilized", "invisible", "observed", "paralyzed", "persistent damage",
            "petrified", "prone", "quickened", "restrained", "sickened",
            "slowed", "stunned", "stupefied", "unconscious", "undetected",
            "unfriendly", "unnoticed", "wounded",
        ]
    }

    fn spell_schools(&self) -> &[&str] {
        // PF2e remaster removed traditional schools, but legacy support
        &[
            "abjuration", "conjuration", "divination", "enchantment",
            "evocation", "illusion", "necromancy", "transmutation",
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
            "lawful good", "neutral good", "chaotic good",
            "lawful neutral", "true neutral", "neutral", "chaotic neutral",
            "lawful evil", "neutral evil", "chaotic evil",
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

// ============================================================================
// Query Processing Vocabulary (Ported from MDMAI)
// ============================================================================

/// Core TTRPG vocabulary for spell correction and fuzzy matching
/// Based on MDMAI query_processor.py ttrpg_vocabulary
pub static TTRPG_CORE_VOCABULARY: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut vocab = HashSet::new();

    // D&D/Generic TTRPG terms
    vocab.extend([
        "fireball", "magic missile", "cure wounds", "healing word",
        "armor class", "hit points", "saving throw", "ability check",
        "advantage", "disadvantage", "critical hit", "initiative",
        "dungeon master", "player character", "non-player character",
        "experience points", "challenge rating", "spell slot", "cantrip",
        "ritual", "concentration", "components", "somatic", "verbal",
        "material", "spell save", "spell attack",
    ]);

    // Stats
    vocab.extend([
        "strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma",
        "proficiency", "expertise",
    ]);

    // Conditions
    vocab.extend([
        "blinded", "charmed", "deafened", "frightened", "grappled",
        "incapacitated", "invisible", "paralyzed", "petrified",
        "poisoned", "prone", "restrained", "stunned", "unconscious",
    ]);

    // Damage types
    vocab.extend([
        "acid", "bludgeoning", "cold", "fire", "force", "lightning",
        "necrotic", "piercing", "poison", "psychic", "radiant",
        "slashing", "thunder",
    ]);

    // Creature types
    vocab.extend([
        "aberration", "beast", "celestial", "construct", "dragon",
        "elemental", "fey", "fiend", "giant", "humanoid", "monstrosity",
        "ooze", "plant", "undead",
    ]);

    // Actions
    vocab.extend([
        "action", "bonus action", "reaction", "movement", "attack",
        "cast", "dash", "disengage", "dodge", "help", "hide", "ready",
        "search", "use",
    ]);

    // Horror/Delta Green terms
    vocab.extend([
        "sanity", "willpower", "bond", "breaking point", "unnatural",
        "agent", "handler", "operation", "green box",
    ]);

    vocab
});

/// Query expansion mappings - abbreviations to full terms
/// Based on MDMAI query_processor.py expansions
pub static QUERY_EXPANSIONS: Lazy<Vec<(&'static str, &'static [&'static str])>> = Lazy::new(|| vec![
    ("ac", &["armor class", "ac"]),
    ("hp", &["hit points", "hp", "health"]),
    ("dm", &["dungeon master", "dm", "game master", "gm"]),
    ("gm", &["game master", "gm", "dungeon master", "dm"]),
    ("pc", &["player character", "pc", "character"]),
    ("npc", &["non-player character", "npc"]),
    ("xp", &["experience points", "xp", "exp"]),
    ("cr", &["challenge rating", "cr"]),
    ("str", &["strength", "str"]),
    ("dex", &["dexterity", "dex"]),
    ("con", &["constitution", "con"]),
    ("int", &["intelligence", "int"]),
    ("wis", &["wisdom", "wis"]),
    ("cha", &["charisma", "cha"]),
    ("save", &["saving throw", "save"]),
    ("dc", &["difficulty class", "dc"]),
    ("san", &["sanity", "san"]),
    ("wp", &["willpower", "wp"]),
]);

/// Synonym groups for query expansion
/// Based on MDMAI query_processor.py synonyms
pub static QUERY_SYNONYMS: Lazy<Vec<&'static [&'static str]>> = Lazy::new(|| vec![
    &["spell", "magic", "incantation", "cantrip"],
    &["monster", "creature", "enemy", "foe", "beast"],
    &["damage", "harm", "hurt", "wound"],
    &["heal", "cure", "restore", "recover"],
    &["attack", "strike", "hit", "assault"],
    &["defend", "protect", "guard", "shield"],
    &["wizard", "mage", "sorcerer", "spellcaster"],
    &["warrior", "fighter", "soldier", "combatant"],
    &["rogue", "thief", "assassin", "scoundrel"],
    &["cleric", "priest", "healer", "divine"],
    &["sanity", "mental health", "psychological"],
    &["horror", "terror", "fear", "dread"],
]);

/// Mechanic type classification keywords
/// Used to detect mechanic_type for SearchDocument
pub static MECHANIC_TYPE_KEYWORDS: Lazy<Vec<(&'static str, &'static [&'static str])>> = Lazy::new(|| vec![
    ("skill_check", &["skill check", "ability check", "roll", "test", "contested"]),
    ("combat", &["attack", "damage", "hit", "miss", "armor class", "initiative", "weapon"]),
    ("damage", &["damage", "hit points", "hp", "wound", "injury", "lethal"]),
    ("healing", &["heal", "cure", "restore", "recovery", "rest", "medicine"]),
    ("sanity", &["sanity", "san", "mental", "madness", "insanity", "psychological"]),
    ("equipment", &["equipment", "gear", "weapon", "armor", "item", "inventory"]),
    ("character_creation", &["character creation", "ability scores", "background", "class", "race", "ancestry"]),
    ("magic", &["spell", "magic", "casting", "ritual", "arcane", "divine"]),
    ("movement", &["movement", "speed", "travel", "distance", "terrain"]),
    ("social", &["persuade", "intimidate", "deceive", "diplomacy", "social", "charisma"]),
]);

/// Expand a query term using abbreviations and synonyms
pub fn expand_query_term(term: &str) -> Vec<String> {
    let term_lower = term.to_lowercase();
    let mut expanded = vec![term_lower.clone()];

    // Check abbreviation expansions
    for (abbr, expansions) in QUERY_EXPANSIONS.iter() {
        if term_lower == *abbr {
            expanded.extend(expansions.iter().map(|s| s.to_string()));
            break;
        }
    }

    // Check synonym groups
    for group in QUERY_SYNONYMS.iter() {
        if group.contains(&term_lower.as_str()) {
            expanded.extend(group.iter().filter(|s| **s != term_lower).map(|s| s.to_string()));
            break;
        }
    }

    expanded
}

/// Expand a full query string
pub fn expand_query(query: &str) -> String {
    let words: Vec<&str> = query.split_whitespace().collect();
    let mut expanded_terms: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for word in words {
        for term in expand_query_term(word) {
            if !seen.contains(&term) {
                seen.insert(term.clone());
                expanded_terms.push(term);
            }
        }
    }

    expanded_terms.join(" ")
}

/// Detect mechanic type from content
pub fn detect_mechanic_type(text: &str) -> Option<&'static str> {
    let text_lower = text.to_lowercase();
    let mut best_match: Option<(&'static str, usize)> = None;

    for (mechanic_type, keywords) in MECHANIC_TYPE_KEYWORDS.iter() {
        let count = keywords.iter()
            .filter(|kw| text_lower.contains(*kw))
            .count();

        if count > 0 {
            match best_match {
                None => best_match = Some((mechanic_type, count)),
                Some((_, best_count)) if count > best_count => {
                    best_match = Some((mechanic_type, count));
                }
                _ => {}
            }
        }
    }

    best_match.map(|(t, _)| t)
}

/// Extract semantic keywords from content for embedding boost
pub fn extract_semantic_keywords(text: &str, max_keywords: usize) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let mut keywords = Vec::new();

    // Check for TTRPG vocabulary terms
    for term in TTRPG_CORE_VOCABULARY.iter() {
        if text_lower.contains(term) && !keywords.contains(&term.to_string()) {
            keywords.push(term.to_string());
            if keywords.len() >= max_keywords {
                break;
            }
        }
    }

    // Also check damage types, conditions, creature types from D&D vocabulary
    let dnd_vocab = DnD5eVocabulary;
    for term in dnd_vocab.damage_types() {
        if text_lower.contains(term) && !keywords.contains(&term.to_string()) {
            keywords.push(term.to_string());
            if keywords.len() >= max_keywords {
                break;
            }
        }
    }

    for term in dnd_vocab.conditions() {
        if text_lower.contains(term) && !keywords.contains(&term.to_string()) {
            keywords.push(term.to_string());
            if keywords.len() >= max_keywords {
                break;
            }
        }
    }

    keywords
}

/// Simple fuzzy match using edit distance
/// Returns similarity score (0.0 to 1.0)
pub fn fuzzy_match(a: &str, b: &str) -> f32 {
    if a == b {
        return 1.0;
    }

    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    if a_lower == b_lower {
        return 1.0;
    }

    // Simple Levenshtein-based similarity
    let distance = levenshtein_distance(&a_lower, &b_lower);
    let max_len = a_lower.len().max(b_lower.len());

    if max_len == 0 {
        return 1.0;
    }

    1.0 - (distance as f32 / max_len as f32)
}

/// Calculate Levenshtein edit distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 { return b_len; }
    if b_len == 0 { return a_len; }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len { matrix[i][0] = i; }
    for j in 0..=b_len { matrix[0][j] = j; }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i-1] == b_chars[j-1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i-1][j] + 1)
                .min(matrix[i][j-1] + 1)
                .min(matrix[i-1][j-1] + cost);
        }
    }

    matrix[a_len][b_len]
}

/// Correct spelling using fuzzy matching against TTRPG vocabulary
/// Returns corrected word if a close match is found, otherwise original
pub fn correct_spelling(word: &str, threshold: f32) -> String {
    let word_lower = word.to_lowercase();

    // Already in vocabulary
    if TTRPG_CORE_VOCABULARY.contains(word_lower.as_str()) {
        return word_lower;
    }

    // Find closest match
    let mut best_match: Option<(&str, f32)> = None;

    for term in TTRPG_CORE_VOCABULARY.iter() {
        let similarity = fuzzy_match(&word_lower, term);
        if similarity >= threshold {
            match best_match {
                None => best_match = Some((term, similarity)),
                Some((_, best_sim)) if similarity > best_sim => {
                    best_match = Some((term, similarity));
                }
                _ => {}
            }
        }
    }

    best_match
        .map(|(term, _)| term.to_string())
        .unwrap_or_else(|| word_lower)
}

/// Correct spelling in a query string
pub fn correct_query_spelling(query: &str, threshold: f32) -> String {
    query
        .split_whitespace()
        .map(|word| correct_spelling(word, threshold))
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================================
// BM25 STOP WORDS (Ported from MDMAI)
// ============================================================================

/// Common English stop words filtered out of BM25 indexing
/// Based on MDMAI config.py STOP_WORDS
pub static BM25_STOP_WORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut words = HashSet::new();
    words.extend([
        // Articles
        "a", "an", "the",
        // Pronouns
        "i", "me", "my", "myself", "we", "our", "ours", "ourselves",
        "you", "your", "yours", "yourself", "yourselves",
        "he", "him", "his", "himself", "she", "her", "hers", "herself",
        "it", "its", "itself", "they", "them", "their", "theirs", "themselves",
        "what", "which", "who", "whom", "this", "that", "these", "those",
        // Verbs (common)
        "am", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "having", "do", "does", "did", "doing",
        // Prepositions
        "at", "by", "for", "with", "about", "against", "between", "into",
        "through", "during", "before", "after", "above", "below", "to",
        "from", "up", "down", "in", "out", "on", "off", "over", "under",
        // Conjunctions
        "and", "but", "if", "or", "because", "as", "until", "while",
        "of", "so", "than", "too", "very", "just", "can", "will", "should",
        // Other
        "s", "t", "now", "here", "there", "when", "where", "why", "how",
        "all", "each", "few", "more", "most", "other", "some", "such",
        "no", "nor", "not", "only", "own", "same", "then", "again",
        "further", "once",
    ]);
    words
});

/// Check if a word is a stop word
pub fn is_stop_word(word: &str) -> bool {
    BM25_STOP_WORDS.contains(word.to_lowercase().as_str())
}

/// Filter stop words from a list of tokens
pub fn filter_stop_words<'a>(tokens: &[&'a str]) -> Vec<&'a str> {
    tokens
        .iter()
        .filter(|w| !is_stop_word(w))
        .copied()
        .collect()
}

// ============================================================================
// SOURCE BOOK PATTERNS (Ported from MDMAI)
// ============================================================================

/// Common TTRPG source book abbreviations and their full names
/// Used for filtering by source and metadata extraction
pub static SOURCE_BOOK_PATTERNS: Lazy<Vec<(&'static str, &'static str, &'static str)>> = Lazy::new(|| vec![
    // D&D 5e Core
    ("phb", "player's handbook", "dnd5e"),
    ("dmg", "dungeon master's guide", "dnd5e"),
    ("mm", "monster manual", "dnd5e"),
    // D&D 5e Supplements
    ("xgte", "xanathar's guide to everything", "dnd5e"),
    ("tcoe", "tasha's cauldron of everything", "dnd5e"),
    ("vgtm", "volo's guide to monsters", "dnd5e"),
    ("mtof", "mordenkainen's tome of foes", "dnd5e"),
    ("scag", "sword coast adventurer's guide", "dnd5e"),
    ("ftod", "fizban's treasury of dragons", "dnd5e"),
    ("motm", "mordenkainen presents: monsters of the multiverse", "dnd5e"),
    ("bgdia", "baldur's gate: descent into avernus", "dnd5e"),
    ("cos", "curse of strahd", "dnd5e"),
    ("hotdq", "hoard of the dragon queen", "dnd5e"),
    ("oota", "out of the abyss", "dnd5e"),
    ("pota", "princes of the apocalypse", "dnd5e"),
    ("rot", "rise of tiamat", "dnd5e"),
    ("skt", "storm king's thunder", "dnd5e"),
    ("toa", "tomb of annihilation", "dnd5e"),
    ("wdh", "waterdeep: dragon heist", "dnd5e"),
    ("wdmm", "waterdeep: dungeon of the mad mage", "dnd5e"),
    ("gos", "ghosts of saltmarsh", "dnd5e"),
    ("idrotf", "icewind dale: rime of the frostmaiden", "dnd5e"),
    ("wbtw", "the wild beyond the witchlight", "dnd5e"),
    ("cm", "candlekeep mysteries", "dnd5e"),
    // Pathfinder 2e Core
    ("crb", "core rulebook", "pf2e"),
    ("apg", "advanced player's guide", "pf2e"),
    ("gmg", "gamemastery guide", "pf2e"),
    ("b1", "bestiary", "pf2e"),
    ("b2", "bestiary 2", "pf2e"),
    ("b3", "bestiary 3", "pf2e"),
    ("som", "secrets of magic", "pf2e"),
    ("g&g", "guns & gears", "pf2e"),
    ("da", "dark archive", "pf2e"),
    ("botd", "book of the dead", "pf2e"),
    ("loag", "lost omens: ancestry guide", "pf2e"),
    ("locg", "lost omens: character guide", "pf2e"),
    ("lowg", "lost omens: world guide", "pf2e"),
    ("logm", "lost omens: gods & magic", "pf2e"),
    ("lopsg", "lost omens: pathfinder society guide", "pf2e"),
    ("lomm", "lost omens: monsters of myth", "pf2e"),
    ("lotgb", "lost omens: the grand bazaar", "pf2e"),
    ("loil", "lost omens: impossible lands", "pf2e"),
    // Call of Cthulhu
    ("ks", "keeper's screen", "coc"),
    ("is", "investigator's handbook", "coc"),
    ("gsc", "grand grimoire of cthulhu mythos magic", "coc"),
    ("moc", "malleus monstrorum: cthulhu mythos bestiary", "coc"),
    ("pgt", "pulp cthulhu", "coc"),
    ("dg", "delta green", "dg"),
    ("dgah", "delta green: agent's handbook", "dg"),
    ("dghr", "delta green: handler's guide", "dg"),
    // Blades in the Dark
    ("bitd", "blades in the dark", "bitd"),
    ("sib", "scum and villainy", "bitd"),
    ("boh", "band of blades", "bitd"),
    // Other Systems
    ("swade", "savage worlds adventure edition", "sw"),
    ("fc", "fate core", "fate"),
    ("fae", "fate accelerated", "fate"),
    ("aw", "apocalypse world", "pbta"),
    ("dw", "dungeon world", "pbta"),
    ("motw", "monster of the week", "pbta"),
    ("mgt2", "mongoose traveller 2nd edition", "traveller"),
]);

/// Detect source book from text content
pub fn detect_source_book(text: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let text_lower = text.to_lowercase();

    for (abbr, full_name, system) in SOURCE_BOOK_PATTERNS.iter() {
        // Check for abbreviation (with word boundaries)
        let abbr_pattern = format!(r"\b{}\b", abbr);
        if regex::Regex::new(&abbr_pattern)
            .map(|r| r.is_match(&text_lower))
            .unwrap_or(false)
        {
            return Some((abbr, full_name, system));
        }

        // Check for full name
        if text_lower.contains(full_name) {
            return Some((abbr, full_name, system));
        }
    }

    None
}

// ============================================================================
// HEADER LEVEL DETECTION PATTERNS (Ported from MDMAI)
// ============================================================================

/// Header level patterns with associated header level (1-6)
/// Used for TOC generation and semantic chunking
pub static HEADER_PATTERNS: Lazy<Vec<(&'static str, u8)>> = Lazy::new(|| vec![
    // Level 1 - Major divisions
    (r"^chapter\s+\d+", 1),
    (r"^chapter\s+[ivxlcdm]+", 1),    // Chapter I, II, III
    (r"^part\s+[ivxlcdm]+", 1),       // Roman numerals
    (r"^part\s+\d+", 1),
    (r"^book\s+\d+", 1),
    // Level 2 - Chapters and major sections
    (r"^appendix\s+[a-z]", 2),
    (r"^section\s+[a-z]", 2),         // Section A, Section B
    (r"^section\s+[ivxlcdm]+", 2),    // Section I, II, III
    (r"^introduction$", 2),
    (r"^preface$", 2),
    (r"^prologue$", 2),
    (r"^epilogue$", 2),
    (r"^index$", 2),
    (r"^glossary$", 2),
    // Level 3 - Numbered sections
    (r"^section\s+\d+", 3),
    // Level 4 - Subsections (often used for class features, spells, etc.)
    (r"^at \d+(st|nd|rd|th) level", 4),
    (r"^starting at \d+(st|nd|rd|th) level", 4),
]);

/// Detect header level from text
/// Returns None if not recognized as a header
pub fn detect_header_level(text: &str) -> Option<u8> {
    let text_lower = text.trim().to_lowercase();

    for (pattern, level) in HEADER_PATTERNS.iter() {
        if regex::Regex::new(pattern)
            .map(|r| r.is_match(&text_lower))
            .unwrap_or(false)
        {
            return Some(*level);
        }
    }

    // Additional heuristics for unlabeled headers
    let trimmed = text.trim();

    // All caps and short = likely header
    if trimmed.len() < 50
        && !trimmed.is_empty()
        && trimmed.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())
    {
        return Some(2);
    }

    // Title case and short = possibly header
    if trimmed.len() < 60 && !trimmed.ends_with('.') && !trimmed.ends_with(',') {
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() <= 8 {
            let title_case_count = words
                .iter()
                .filter(|w| {
                    w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                })
                .count();
            if title_case_count > words.len() / 2 {
                return Some(3);
            }
        }
    }

    None
}

// ============================================================================
// DICE TABLE DETECTION PATTERNS (Ported from MDMAI)
// ============================================================================

/// Patterns for detecting dice notation in tables
/// Used by the RandomTableParser to identify rollable tables
pub static DICE_PATTERNS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    r"\bd\d+\b",              // d20, d6, d100, etc.
    r"\d+d\d+",               // 2d6, 3d8, etc.
    r"\bd%\b",                // Percentile dice
    r"\d+-\d+",               // Range notation (1-4, 5-8, etc.)
    r"\b\d+\s*[-–—]\s*\d+\b", // Range with various dashes
]);

/// Table row patterns that indicate a random/roll table
pub static TABLE_ROW_PATTERNS: Lazy<Vec<&'static str>> = Lazy::new(|| vec![
    r"^\d+[-–—]\d+\s+.+",       // "1-4 Result text"
    r"^\d+\.\s+.+",             // "1. Result text"
    r"^\d+\s+.+",               // "1 Result text" (simple numbered)
    r"^[ivxlcdm]+\.\s+.+",      // Roman numeral lists
]);

/// Check if text contains dice notation
pub fn contains_dice_notation(text: &str) -> bool {
    let text_lower = text.to_lowercase();

    for pattern in DICE_PATTERNS.iter() {
        if regex::Regex::new(pattern)
            .map(|r| r.is_match(&text_lower))
            .unwrap_or(false)
        {
            return true;
        }
    }

    false
}

/// Count dice notation occurrences in text
pub fn count_dice_notation(text: &str) -> usize {
    let text_lower = text.to_lowercase();
    let mut count = 0;

    for pattern in DICE_PATTERNS.iter() {
        if let Ok(re) = regex::Regex::new(pattern) {
            count += re.find_iter(&text_lower).count();
        }
    }

    count
}

// ============================================================================
// CHUNKING CONFIGURATION (Ported from MDMAI)
// ============================================================================

/// Semantic chunking configuration constants
/// Based on MDMAI config.py chunking settings
pub mod chunking_config {
    /// Target chunk size for RAG (characters)
    pub const TARGET_CHUNK_SIZE: usize = 1200;

    /// Minimum chunk size to avoid fragments (characters)
    pub const MIN_CHUNK_SIZE: usize = 300;

    /// Maximum chunk size hard limit (characters)
    pub const MAX_CHUNK_SIZE: usize = 2400;

    /// Overlap between chunks for context continuity (characters)
    pub const CHUNK_OVERLAP: usize = 150;

    /// Minimum pages to group together for semantic coherence
    pub const MIN_PAGES_PER_CHUNK: usize = 1;

    /// Maximum pages to group together
    pub const MAX_PAGES_PER_CHUNK: usize = 4;

    /// Token-based limits (approximate, assuming ~4 chars/token)
    pub const TARGET_TOKENS: usize = 300;
    pub const MAX_TOKENS: usize = 600;
    pub const OVERLAP_TOKENS: usize = 40;
}

// ============================================================================
// HYBRID SEARCH FUSION PARAMETERS (Ported from MDMAI)
// ============================================================================

/// Configuration for hybrid search fusion (BM25 + vector)
/// Based on MDMAI config.py fusion settings
pub mod fusion_config {
    /// Weight for BM25 keyword search (0.0 to 1.0)
    pub const BM25_WEIGHT: f32 = 0.4;

    /// Weight for vector semantic search (0.0 to 1.0)
    pub const VECTOR_WEIGHT: f32 = 0.6;

    /// RRF (Reciprocal Rank Fusion) constant k
    /// Higher k = more weight to lower-ranked results
    pub const RRF_K: f32 = 60.0;

    /// Minimum score threshold for results
    pub const MIN_SCORE: f32 = 0.1;

    /// Maximum results to return
    pub const MAX_RESULTS: usize = 20;

    /// Boost factor for exact phrase matches
    pub const EXACT_MATCH_BOOST: f32 = 1.5;

    /// Boost factor for matches in title/section headers
    pub const HEADER_MATCH_BOOST: f32 = 1.2;
}

// ============================================================================
// Tests
// ============================================================================

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

    // ========================================================================
    // Query Processing Tests
    // ========================================================================

    #[test]
    fn test_expand_query_term_abbreviation() {
        let expanded = expand_query_term("ac");
        assert!(expanded.contains(&"armor class".to_string()));
        assert!(expanded.contains(&"ac".to_string()));
    }

    #[test]
    fn test_expand_query_term_synonym() {
        let expanded = expand_query_term("monster");
        assert!(expanded.contains(&"monster".to_string()));
        assert!(expanded.contains(&"creature".to_string()));
    }

    #[test]
    fn test_expand_query() {
        let expanded = expand_query("ac damage");
        assert!(expanded.contains("armor class"));
        assert!(expanded.contains("damage"));
    }

    #[test]
    fn test_detect_mechanic_type_combat() {
        let text = "Make an attack roll against the target's armor class. On a hit, deal damage.";
        assert_eq!(detect_mechanic_type(text), Some("combat"));
    }

    #[test]
    fn test_detect_mechanic_type_sanity() {
        let text = "Roll a sanity check. On failure, lose 1d6 sanity points.";
        assert_eq!(detect_mechanic_type(text), Some("sanity"));
    }

    #[test]
    fn test_extract_semantic_keywords() {
        let text = "The wizard casts fireball dealing fire damage to all creatures in the area.";
        let keywords = extract_semantic_keywords(text, 5);
        assert!(!keywords.is_empty());
        assert!(keywords.contains(&"fireball".to_string()) || keywords.contains(&"fire".to_string()));
    }

    #[test]
    fn test_fuzzy_match_exact() {
        assert_eq!(fuzzy_match("fireball", "fireball"), 1.0);
        assert_eq!(fuzzy_match("FIREBALL", "fireball"), 1.0);
    }

    #[test]
    fn test_fuzzy_match_similar() {
        let similarity = fuzzy_match("firebll", "fireball");
        assert!(similarity > 0.7); // Should be close match
    }

    #[test]
    fn test_fuzzy_match_different() {
        let similarity = fuzzy_match("hello", "world");
        assert!(similarity < 0.5); // Should be distant
    }

    #[test]
    fn test_correct_spelling() {
        // Typo should correct to vocabulary term
        let corrected = correct_spelling("firebll", 0.7);
        assert_eq!(corrected, "fireball");
    }

    #[test]
    fn test_correct_spelling_unchanged() {
        // Word not in vocabulary should remain
        let corrected = correct_spelling("xyzabc", 0.7);
        assert_eq!(corrected, "xyzabc");
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "abd"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    // ========================================================================
    // BM25 Stop Words Tests
    // ========================================================================

    #[test]
    fn test_is_stop_word() {
        assert!(is_stop_word("the"));
        assert!(is_stop_word("THE"));
        assert!(is_stop_word("and"));
        assert!(!is_stop_word("fireball"));
        assert!(!is_stop_word("dragon"));
    }

    #[test]
    fn test_filter_stop_words() {
        let tokens = vec!["the", "dragon", "is", "a", "creature"];
        let filtered = filter_stop_words(&tokens);
        assert_eq!(filtered, vec!["dragon", "creature"]);
    }

    // ========================================================================
    // Source Book Pattern Tests
    // ========================================================================

    #[test]
    fn test_detect_source_book_abbreviation() {
        let text = "See PHB page 123 for more details.";
        let result = detect_source_book(text);
        assert!(result.is_some());
        let (abbr, _full, system) = result.unwrap();
        assert_eq!(abbr, "phb");
        assert_eq!(system, "dnd5e");
    }

    #[test]
    fn test_detect_source_book_full_name() {
        let text = "As described in the Player's Handbook";
        let result = detect_source_book(text);
        assert!(result.is_some());
        let (abbr, _, system) = result.unwrap();
        assert_eq!(abbr, "phb");
        assert_eq!(system, "dnd5e");
    }

    #[test]
    fn test_detect_source_book_pathfinder() {
        let text = "This feat is from the Advanced Player's Guide";
        let result = detect_source_book(text);
        assert!(result.is_some());
        let (abbr, _, system) = result.unwrap();
        assert_eq!(abbr, "apg");
        assert_eq!(system, "pf2e");
    }

    // ========================================================================
    // Header Level Detection Tests
    // ========================================================================

    #[test]
    fn test_detect_header_level_chapter() {
        assert_eq!(detect_header_level("Chapter 1"), Some(1));
        assert_eq!(detect_header_level("Chapter 12: Combat"), Some(1));
    }

    #[test]
    fn test_detect_header_level_part() {
        assert_eq!(detect_header_level("Part I"), Some(1));
        assert_eq!(detect_header_level("Part III"), Some(1));
        assert_eq!(detect_header_level("Part 2"), Some(1));
    }

    #[test]
    fn test_detect_header_level_appendix() {
        assert_eq!(detect_header_level("Appendix A"), Some(2));
        assert_eq!(detect_header_level("Appendix B: Monsters"), Some(2));
    }

    #[test]
    fn test_detect_header_level_all_caps() {
        assert_eq!(detect_header_level("COMBAT RULES"), Some(2));
        assert_eq!(detect_header_level("SPELLCASTING"), Some(2));
    }

    #[test]
    fn test_detect_header_level_none() {
        // Regular sentences shouldn't be detected as headers
        assert_eq!(detect_header_level("This is a regular sentence."), None);
        assert_eq!(detect_header_level("The dragon attacks the party,"), None);
    }

    // ========================================================================
    // Dice Notation Tests
    // ========================================================================

    #[test]
    fn test_contains_dice_notation() {
        assert!(contains_dice_notation("Roll 2d6 for damage"));
        assert!(contains_dice_notation("On a d20 roll of 15+"));
        assert!(contains_dice_notation("1-4: Minor effect"));
        assert!(!contains_dice_notation("This is regular text"));
    }

    #[test]
    fn test_count_dice_notation() {
        assert_eq!(count_dice_notation("Roll 2d6 + 1d4 damage"), 2);
        assert_eq!(count_dice_notation("No dice here"), 0);
        assert!(count_dice_notation("d20, d6, d8, d12") >= 4);
    }

    // ========================================================================
    // Config Module Tests
    // ========================================================================

    #[test]
    fn test_chunking_config_sanity() {
        use chunking_config::*;
        assert!(MIN_CHUNK_SIZE < TARGET_CHUNK_SIZE);
        assert!(TARGET_CHUNK_SIZE < MAX_CHUNK_SIZE);
        assert!(CHUNK_OVERLAP < MIN_CHUNK_SIZE);
    }

    #[test]
    fn test_fusion_config_weights() {
        use fusion_config::*;
        // Weights should sum to 1.0
        let total = BM25_WEIGHT + VECTOR_WEIGHT;
        assert!((total - 1.0).abs() < 0.01);
    }
}
