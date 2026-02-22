//! Content Mode Classification Module
//!
//! Classifies TTRPG text content into distinct modes for chunking and search weighting.
//! This helps distinguish between mechanical rules (crunch), narrative text (fluff),
//! examples of play, optional rules, and fiction passages.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ============================================================================
// Types
// ============================================================================

/// Content mode categories for TTRPG text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentMode {
    /// Mechanical rules, stats, dice notation, prescriptive language
    Crunch,
    /// Narrative text, lore, flavor descriptions
    Fluff,
    /// Mixed content with both mechanical and narrative elements
    Mixed,
    /// Example of play, dialogue between GM and players
    Example,
    /// Variant rules, optional mechanics, house rules
    Optional,
    /// In-universe fiction, narrative vignettes, story passages
    Fiction,
}

impl ContentMode {
    /// Get a human-readable name for this mode.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Crunch => "crunch",
            Self::Fluff => "fluff",
            Self::Mixed => "mixed",
            Self::Example => "example",
            Self::Optional => "optional",
            Self::Fiction => "fiction",
        }
    }

    /// Get search weight multiplier for this mode.
    /// Crunch content typically needs higher precision matching.
    pub fn search_weight(&self) -> f32 {
        match self {
            Self::Crunch => 1.2,
            Self::Fluff => 1.0,
            Self::Mixed => 1.1,
            Self::Example => 0.9,
            Self::Optional => 0.95,
            Self::Fiction => 0.85,
        }
    }

    /// Whether this mode should prefer semantic (vector) search.
    pub fn prefers_semantic_search(&self) -> bool {
        matches!(self, Self::Fluff | Self::Fiction)
    }
}

/// Result of content mode classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentModeResult {
    /// The primary detected content mode
    pub mode: ContentMode,
    /// Score for mechanical/rules content (0.0 to 1.0)
    pub crunch_score: f32,
    /// Score for narrative/flavor content (0.0 to 1.0)
    pub fluff_score: f32,
    /// Confidence in the classification (0.0 to 1.0)
    pub confidence: f32,
}

impl ContentModeResult {
    /// Create a new result with the given mode and scores.
    pub fn new(mode: ContentMode, crunch_score: f32, fluff_score: f32, confidence: f32) -> Self {
        Self {
            mode,
            crunch_score: crunch_score.clamp(0.0, 1.0),
            fluff_score: fluff_score.clamp(0.0, 1.0),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Check if the classification is high confidence.
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.7
    }
}

// ============================================================================
// Compiled Patterns (lazy static)
// ============================================================================

/// Dice notation pattern: d6, 2d8+3, 1d20-2, etc.
static DICE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b\d*d\d+(?:\s*[+\-]\s*\d+)?\b").expect("Invalid dice pattern regex")
});

/// DC (Difficulty Class) pattern: DC 15, DC15, etc.
static DC_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bDC\s*\d+\b").expect("Invalid DC pattern regex"));

/// Modifier pattern: +2, -1, +5, etc. (standalone modifiers like "bonus +2")
/// Note: We use a simpler pattern and filter dice-related modifiers in code
static MODIFIER_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s[+\-]\d+\b").expect("Invalid modifier pattern regex"));

/// Numbered list pattern (prescriptive steps)
static NUMBERED_LIST_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*\d+[.)]\s+").expect("Invalid numbered list pattern regex")
});

/// Example dialogue pattern: "GM:", "Player:", "DM:", etc.
static DIALOGUE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^(?:GM|DM|Player|PC|NPC|Referee)\s*[:\-]")
        .expect("Invalid dialogue pattern regex")
});

/// First-person narration pattern
static FIRST_PERSON_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:I\s+(?:am|was|have|had|will|would|could|should|might)|my\s+\w+|myself)\b")
        .expect("Invalid first-person pattern regex")
});

/// Quotation pattern (dialogue or in-character speech)
static QUOTATION_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[""][^""]{10,}[""]"#).expect("Invalid quotation pattern regex"));

// ============================================================================
// Classifier
// ============================================================================

/// Classifies TTRPG text content into crunch, fluff, or specialized modes.
#[derive(Debug, Clone)]
pub struct ContentModeClassifier {
    /// Minimum text length to attempt classification
    min_text_length: usize,
}

impl Default for ContentModeClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentModeClassifier {
    /// Create a new classifier with default settings.
    pub fn new() -> Self {
        Self {
            min_text_length: 20,
        }
    }

    /// Create a classifier with custom minimum text length.
    pub fn with_min_length(min_text_length: usize) -> Self {
        Self { min_text_length }
    }

    /// Classify the content mode of the given text.
    pub fn classify(&self, text: &str) -> ContentModeResult {
        let text_trimmed = text.trim();
        let text_lower = text.to_lowercase();

        // Handle very short text
        if text_trimmed.len() < self.min_text_length {
            return ContentModeResult::new(ContentMode::Mixed, 0.5, 0.5, 0.3);
        }

        // Check for specific modes first (more specific patterns)
        if let Some(result) = self.check_example_mode(text_trimmed, &text_lower) {
            return result;
        }

        if let Some(result) = self.check_optional_mode(&text_lower) {
            return result;
        }

        if let Some(result) = self.check_fiction_mode(text_trimmed, &text_lower) {
            return result;
        }

        // Calculate crunch and fluff scores
        let crunch_score = self.calculate_crunch_score(text_trimmed, &text_lower);
        let fluff_score = self.calculate_fluff_score(text_trimmed, &text_lower);

        // Determine mode based on scores
        self.determine_mode(crunch_score, fluff_score)
    }

    /// Check if text matches Example of Play patterns.
    fn check_example_mode(&self, text: &str, text_lower: &str) -> Option<ContentModeResult> {
        let mut example_score: f32 = 0.0;

        // Direct "Example of Play" or similar headers
        if text_lower.contains("example of play")
            || text_lower.contains("sample of play")
            || text_lower.contains("play example")
        {
            example_score += 0.8;
        }

        // Dialogue format (GM:, Player:, DM:, etc.)
        let dialogue_matches = DIALOGUE_PATTERN.find_iter(text).count();
        if dialogue_matches >= 2 {
            example_score += 0.6;
        } else if dialogue_matches == 1 {
            example_score += 0.3;
        }

        // Check for example/illustration keywords
        if text_lower.contains("for example")
            || text_lower.contains("here's an example")
            || text_lower.contains("to illustrate")
            || text_lower.contains("let's say")
        {
            example_score += 0.4;
        }

        if example_score >= 0.6 {
            let confidence = example_score.min(1.0);
            return Some(ContentModeResult::new(
                ContentMode::Example,
                0.3,
                0.4,
                confidence,
            ));
        }

        None
    }

    /// Check if text matches Optional/Variant rule patterns.
    fn check_optional_mode(&self, text_lower: &str) -> Option<ContentModeResult> {
        let mut optional_score: f32 = 0.0;

        // Variant/optional keywords
        let variant_keywords = [
            "variant rule",
            "optional rule",
            "house rule",
            "alternative rule",
            "variant:",
            "optional:",
            "this variant",
            "this optional",
            "gm's option",
            "dm's option",
            "at the gm's discretion",
            "at the dm's discretion",
        ];

        for keyword in variant_keywords {
            if text_lower.contains(keyword) {
                optional_score += 0.5;
            }
        }

        // "You may" or "you can optionally" language
        if text_lower.contains("you may optionally")
            || text_lower.contains("you can optionally")
            || text_lower.contains("optionally, you")
        {
            optional_score += 0.4;
        }

        // "If you prefer" or similar
        if text_lower.contains("if you prefer")
            || text_lower.contains("if your group prefers")
            || text_lower.contains("some groups prefer")
        {
            optional_score += 0.3;
        }

        if optional_score >= 0.5 {
            let confidence = optional_score.min(1.0);
            return Some(ContentModeResult::new(
                ContentMode::Optional,
                0.5,
                0.3,
                confidence,
            ));
        }

        None
    }

    /// Check if text matches Fiction/narrative vignette patterns.
    fn check_fiction_mode(&self, text: &str, text_lower: &str) -> Option<ContentModeResult> {
        let mut fiction_score: f32 = 0.0;

        // Italicized story text (often indicated by Markdown or special formatting)
        // Check for common fiction indicators
        if text.starts_with('*') && text.ends_with('*')
            || text.starts_with('_') && text.ends_with('_')
        {
            fiction_score += 0.5;
        }

        // Long quotation blocks (in-character speech/narration)
        let quote_matches = QUOTATION_PATTERN.find_iter(text).count();
        if quote_matches >= 2 {
            fiction_score += 0.5;
        }

        // First-person narration
        let first_person_count = FIRST_PERSON_PATTERN.find_iter(text).count();
        if first_person_count >= 3 {
            fiction_score += 0.4;
        }

        // Past tense narrative (common in fiction)
        let past_tense_indicators = [
            " was ",
            " were ",
            " had ",
            " came ",
            " went ",
            " said ",
            " told ",
            " saw ",
            " heard ",
            " felt ",
            " knew ",
            " thought ",
            " walked ",
            " ran ",
            " looked ",
            " turned ",
            " opened ",
            " closed ",
            " stood ",
        ];
        let past_tense_count = past_tense_indicators
            .iter()
            .filter(|ind| text_lower.contains(*ind))
            .count();

        if past_tense_count >= 4 {
            fiction_score += 0.4;
        }

        // No mechanical content (negative indicator for fiction)
        let has_dice = DICE_PATTERN.is_match(text);
        let has_dc = DC_PATTERN.is_match(text);
        if !has_dice && !has_dc {
            fiction_score += 0.2;
        }

        if fiction_score >= 0.6 {
            let confidence = fiction_score.min(1.0);
            return Some(ContentModeResult::new(
                ContentMode::Fiction,
                0.1,
                0.8,
                confidence,
            ));
        }

        None
    }

    /// Calculate crunch (mechanical content) score.
    fn calculate_crunch_score(&self, text: &str, text_lower: &str) -> f32 {
        let word_count = text.split_whitespace().count() as f32;
        if word_count < 5.0 {
            return 0.5; // Not enough data
        }

        let mut score: f32 = 0.0;

        // Dice notation (strong crunch indicator)
        let dice_count = DICE_PATTERN.find_iter(text).count() as f32;
        if dice_count > 0.0 {
            let dice_density = dice_count / word_count * 100.0;
            score += (0.3 + dice_density * 0.1).min(0.5);
        }

        // DC references
        let dc_count = DC_PATTERN.find_iter(text).count() as f32;
        if dc_count > 0.0 {
            score += (0.2 + dc_count * 0.1).min(0.4);
        }

        // Numeric modifiers (+2, -1, etc.)
        let modifier_count = MODIFIER_PATTERN.find_iter(text).count() as f32;
        if modifier_count > 0.0 {
            score += (0.1 + modifier_count * 0.03).min(0.3);
        }

        // Numbered/bullet lists (prescriptive structure)
        let list_count = NUMBERED_LIST_PATTERN.find_iter(text).count() as f32;
        if list_count > 0.0 {
            score += (0.1 + list_count * 0.05).min(0.25);
        }

        // Prescriptive language keywords
        let prescriptive_keywords = [
            "you must",
            "you cannot",
            "you can't",
            "you may not",
            "saving throw",
            "attack roll",
            "ability check",
            "skill check",
            "damage roll",
            "hit points",
            "armor class",
            "spell slot",
            "per day",
            "per long rest",
            "per short rest",
            "once per",
            "requires",
            "prerequisite",
            "action",
            "bonus action",
            "reaction",
            "movement",
            "speed",
            "check",
            "target",
            "attack",
            "damage",
        ];

        let keyword_matches = prescriptive_keywords
            .iter()
            .filter(|kw| text_lower.contains(*kw))
            .count() as f32;
        if keyword_matches > 0.0 {
            score += (0.1 + keyword_matches * 0.05).min(0.4);
        }

        // Conditional mechanics language
        let conditional_mechanics = [
            "if the target",
            "when you",
            "while you",
            "until the",
            "at the start",
            "at the end",
            "on a hit",
            "on a miss",
            "on a success",
            "on a failure",
            "make a",
            "contested by",
        ];

        let conditional_matches = conditional_mechanics
            .iter()
            .filter(|kw| text_lower.contains(*kw))
            .count() as f32;
        if conditional_matches > 0.0 {
            score += (0.1 + conditional_matches * 0.08).min(0.35);
        }

        // Clamp to [0, 1]
        score.clamp(0.0, 1.0)
    }

    /// Calculate fluff (narrative content) score.
    fn calculate_fluff_score(&self, text: &str, text_lower: &str) -> f32 {
        let word_count = text.split_whitespace().count() as f32;
        if word_count < 5.0 {
            return 0.5; // Not enough data
        }

        let mut score: f32 = 0.0;

        // Past tense (narrative indicator)
        let past_tense_words = [
            " was ", " were ", " had ", " did ", " came ", " went ", " made ", " stood ",
            " looked ", " saw ", " heard ", " said ",
        ];
        let past_tense_count = past_tense_words
            .iter()
            .filter(|w| text_lower.contains(*w))
            .count() as f32;
        if past_tense_count > 0.0 {
            score += (0.15 + past_tense_count * 0.05).min(0.35);
        }

        // Quotations (dialogue, flavor text)
        let quote_count = QUOTATION_PATTERN.find_iter(text).count() as f32;
        if quote_count > 0.0 {
            score += (0.2 + quote_count * 0.1).min(0.4);
        }

        // First-person narration
        let first_person = FIRST_PERSON_PATTERN.find_iter(text).count() as f32;
        if first_person > 0.0 {
            score += (0.1 + first_person * 0.05).min(0.3);
        }

        // Descriptive/adjective-heavy prose
        let descriptive_words = [
            "ancient",
            "mysterious",
            "dark",
            "bright",
            "towering",
            "vast",
            "terrible",
            "beautiful",
            "strange",
            "powerful",
            "legendary",
            "forgotten",
            "sacred",
            "cursed",
            "hidden",
            "majestic",
            "twisted",
            "weathered",
            "arcane",
            "ethereal",
            "famous",
            "massive",
            "grand",
            "glorious",
            "perilous",
            "beneath",
            "beyond",
            "across",
        ];
        let descriptive_count = descriptive_words
            .iter()
            .filter(|w| text_lower.contains(*w))
            .count() as f32;
        if descriptive_count > 0.0 {
            score += (0.1 + descriptive_count * 0.04).min(0.3);
        }

        // Lore/history keywords
        let lore_keywords = [
            "legend",
            "history",
            "ancient",
            "long ago",
            "centuries",
            "once upon",
            "tradition",
            "myth",
            "story tells",
            "tales of",
            "according to",
            "it is said",
            "rumor",
            "whisper",
            "gather",
            "adventurer",
            "hero",
            "empire",
            "kingdom",
            "realm",
        ];
        let lore_matches = lore_keywords
            .iter()
            .filter(|kw| text_lower.contains(*kw))
            .count() as f32;
        if lore_matches > 0.0 {
            score += (0.1 + lore_matches * 0.05).min(0.35);
        }

        // Absence of mechanical indicators (small boost)
        let has_dice = DICE_PATTERN.is_match(text);
        let has_dc = DC_PATTERN.is_match(text);
        if !has_dice && !has_dc {
            score += 0.15;
        }

        // Clamp to [0, 1]
        score.clamp(0.0, 1.0)
    }

    /// Determine the final mode based on crunch and fluff scores.
    fn determine_mode(&self, crunch_score: f32, fluff_score: f32) -> ContentModeResult {
        // High crunch, low fluff
        if crunch_score > 0.7 && fluff_score < 0.3 {
            let confidence = crunch_score - fluff_score;
            return ContentModeResult::new(
                ContentMode::Crunch,
                crunch_score,
                fluff_score,
                confidence,
            );
        }

        // High fluff, low crunch
        if fluff_score > 0.7 && crunch_score < 0.3 {
            let confidence = fluff_score - crunch_score;
            return ContentModeResult::new(
                ContentMode::Fluff,
                crunch_score,
                fluff_score,
                confidence,
            );
        }

        // Strongly crunch-dominant
        if crunch_score > fluff_score + 0.3 {
            let confidence = (crunch_score - fluff_score) * 0.8;
            return ContentModeResult::new(
                ContentMode::Crunch,
                crunch_score,
                fluff_score,
                confidence,
            );
        }

        // Strongly fluff-dominant
        if fluff_score > crunch_score + 0.3 {
            let confidence = (fluff_score - crunch_score) * 0.8;
            return ContentModeResult::new(
                ContentMode::Fluff,
                crunch_score,
                fluff_score,
                confidence,
            );
        }

        // Mixed content - both scores are significant or close
        let confidence = 1.0 - (crunch_score - fluff_score).abs();
        ContentModeResult::new(ContentMode::Mixed, crunch_score, fluff_score, confidence)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn classifier() -> ContentModeClassifier {
        ContentModeClassifier::new()
    }

    // ========================================================================
    // Crunch Detection Tests
    // ========================================================================

    #[test]
    fn test_crunch_dice_heavy() {
        let text = r#"
            Fireball
            3rd-level evocation
            Casting Time: 1 action
            Range: 150 feet
            A bright streak flashes from your pointing finger to a point you choose
            within range and then blossoms with a low roar into an explosion of flame.
            Each creature in a 20-foot-radius sphere centered on that point must make
            a Dexterity saving throw. A target takes 8d6 fire damage on a failed save,
            or half as much damage on a successful one.
            At Higher Levels: When you cast this spell using a spell slot of 4th level
            or higher, the damage increases by 1d6 for each slot level above 3rd.
        "#;

        let result = classifier().classify(text);
        assert!(
            result.mode == ContentMode::Crunch || result.mode == ContentMode::Mixed,
            "Expected Crunch or Mixed for spell description, got {:?}",
            result.mode
        );
        assert!(result.crunch_score > result.fluff_score);
    }

    #[test]
    fn test_crunch_stat_block() {
        let text = r#"
            Armor Class 15 (natural armor)
            Hit Points 52 (7d10 + 14)
            Speed 30 ft.
            STR 18 (+4) DEX 12 (+1) CON 14 (+2) INT 6 (-2) WIS 10 (+0) CHA 7 (-2)
            Saving Throws Str +6, Con +4
            Skills Athletics +6, Perception +2
            Challenge 3 (700 XP)
        "#;

        let result = classifier().classify(text);
        assert!(
            result.mode == ContentMode::Crunch || result.mode == ContentMode::Mixed,
            "Expected Crunch or Mixed for stat block, got {:?} (crunch: {}, fluff: {})",
            result.mode,
            result.crunch_score,
            result.fluff_score
        );
        assert!(
            result.crunch_score > result.fluff_score,
            "Expected crunch_score ({}) > fluff_score ({})",
            result.crunch_score,
            result.fluff_score
        );
    }

    #[test]
    fn test_crunch_mechanics_rules() {
        let text = r#"
            When you take the Attack action on your turn, you can forgo one of your
            attacks to make a grapple attempt. You must have at least one free hand
            to attempt a grapple. The target must be no more than one size larger
            than you. Make a Strength (Athletics) check contested by the target's
            Strength (Athletics) or Dexterity (Acrobatics) check.
        "#;

        let result = classifier().classify(text);
        assert!(
            result.mode == ContentMode::Crunch || result.mode == ContentMode::Mixed,
            "Expected Crunch or Mixed for rules text, got {:?} (crunch: {}, fluff: {})",
            result.mode,
            result.crunch_score,
            result.fluff_score
        );
        // Rules text should have higher crunch than fluff
        assert!(
            result.crunch_score >= result.fluff_score,
            "Expected crunch_score ({}) >= fluff_score ({}) for rules text",
            result.crunch_score,
            result.fluff_score
        );
    }

    // ========================================================================
    // Fluff Detection Tests
    // ========================================================================

    #[test]
    fn test_fluff_lore_description() {
        let text = r#"
            The ancient empire of Netheril was once the most powerful magical
            civilization in all of Faerûn. For centuries, their floating cities
            soared above the clouds, powered by mythallars that drew upon the
            very fabric of the Weave itself. The legends say that their archwizards
            could reshape reality with a mere thought, bending the laws of nature
            to their will.
        "#;

        let result = classifier().classify(text);
        assert!(
            result.mode == ContentMode::Fluff || result.mode == ContentMode::Fiction,
            "Expected Fluff or Fiction for lore text, got {:?}",
            result.mode
        );
        assert!(result.fluff_score > result.crunch_score);
    }

    #[test]
    fn test_fluff_location_description() {
        let text = r#"
            The Yawning Portal is a famous inn and tavern located in the Castle
            Ward of Waterdeep. It is built around a massive well that descends
            into the Undermountain, a vast dungeon complex beneath the city.
            Adventurers from across the Sword Coast gather here to swap tales
            of glory and peril before descending into the depths below.
        "#;

        let result = classifier().classify(text);
        // Location description should have more fluff than crunch
        assert!(
            result.fluff_score >= result.crunch_score,
            "Expected fluff_score ({}) >= crunch_score ({}) for location description",
            result.fluff_score,
            result.crunch_score
        );
    }

    // ========================================================================
    // Mixed Content Tests
    // ========================================================================

    #[test]
    fn test_mixed_content() {
        let text = r#"
            The orc chieftain stands atop the ridge, his scarred face twisted in
            a snarl of hatred. Behind him, his war band waits with weapons ready.
            The chieftain has Armor Class 16 and 93 hit points (11d8 + 44). He
            wields a greataxe that deals 1d12 + 5 slashing damage on a hit.
        "#;

        let result = classifier().classify(text);
        // This text has both narrative and mechanical elements
        // Both scores should be non-trivial (neither should be near zero)
        assert!(
            result.crunch_score > 0.1,
            "Expected crunch_score ({}) > 0.1 for mixed content",
            result.crunch_score
        );
        assert!(
            result.fluff_score > 0.1,
            "Expected fluff_score ({}) > 0.1 for mixed content",
            result.fluff_score
        );
    }

    // ========================================================================
    // Example of Play Tests
    // ========================================================================

    #[test]
    fn test_example_of_play_dialogue() {
        let text = r#"
            Example of Play

            GM: As you enter the tavern, you notice a hooded figure in the corner.
            Player: I want to approach them cautiously.
            GM: Roll a Stealth check.
            Player: I got a 15.
            GM: The figure doesn't seem to notice your approach.
        "#;

        let result = classifier().classify(text);
        assert_eq!(result.mode, ContentMode::Example);
        assert!(result.confidence >= 0.6);
    }

    #[test]
    fn test_example_dm_player_format() {
        let text = r#"
            DM: The dragon rears back, preparing to breathe fire.
            Player: I want to use my reaction to cast Shield.
            DM: Go ahead and roll for it.
            Player: That's a 22 total on my AC now.
            DM: The flames wash over you, but the magical barrier holds.
        "#;

        let result = classifier().classify(text);
        // DM/Player dialogue format should be recognized as Example
        // or at minimum have characteristics of dialogue
        assert!(
            result.mode == ContentMode::Example || result.mode == ContentMode::Mixed,
            "Expected Example or Mixed for DM/Player dialogue, got {:?} (crunch: {}, fluff: {})",
            result.mode,
            result.crunch_score,
            result.fluff_score
        );
    }

    // ========================================================================
    // Optional/Variant Rule Tests
    // ========================================================================

    #[test]
    fn test_optional_variant_rule() {
        let text = r#"
            Variant Rule: Gritty Realism

            This variant uses a short rest of 8 hours and a long rest of 7 days.
            This puts the brakes on the game, requiring the players to carefully
            judge the risks of each encounter. If your group prefers a grittier
            campaign with more emphasis on resource management, this optional
            rule can provide that experience.
        "#;

        let result = classifier().classify(text);
        assert_eq!(result.mode, ContentMode::Optional);
        assert!(result.confidence >= 0.5);
    }

    #[test]
    fn test_optional_house_rule() {
        let text = r#"
            House Rule: Critical Fumbles

            At the GM's discretion, rolling a natural 1 on an attack roll may
            result in an additional consequence beyond simply missing. You may
            optionally have the attacker drop their weapon or grant advantage
            to the next attack against them. Some groups prefer this variant
            for added dramatic tension.
        "#;

        let result = classifier().classify(text);
        assert_eq!(result.mode, ContentMode::Optional);
    }

    // ========================================================================
    // Fiction Tests
    // ========================================================================

    #[test]
    fn test_fiction_narrative_vignette() {
        let text = r#"
            *The old wizard stood at the window, watching the storm clouds gather
            over the distant mountains. "It begins," he whispered to himself,
            his weathered hands trembling. He had seen this before, long ago,
            when he was young and foolish enough to believe he could stop it.
            Now he knew better. Now he knew that some darkness could only be
            endured, never defeated.*
        "#;

        let result = classifier().classify(text);
        assert_eq!(result.mode, ContentMode::Fiction);
    }

    #[test]
    fn test_fiction_in_character_narrative() {
        let text = r#"
            "I remember the day the dragons came," the old soldier said, staring
            into his mug of ale. "We thought we were ready. We had trained for
            years, honed our skills, built our defenses. But nothing could have
            prepared us for the fury of their flames. Half my company was gone
            before we could draw our swords."
        "#;

        let result = classifier().classify(text);
        assert!(
            result.mode == ContentMode::Fiction || result.mode == ContentMode::Fluff,
            "Expected Fiction or Fluff for narrative, got {:?}",
            result.mode
        );
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_short_text_returns_mixed() {
        let text = "Short text.";
        let result = classifier().classify(text);
        assert_eq!(result.mode, ContentMode::Mixed);
        assert!(result.confidence < 0.5);
    }

    #[test]
    fn test_empty_text() {
        let text = "";
        let result = classifier().classify(text);
        assert_eq!(result.mode, ContentMode::Mixed);
    }

    #[test]
    fn test_whitespace_only() {
        let text = "   \n\t   ";
        let result = classifier().classify(text);
        assert_eq!(result.mode, ContentMode::Mixed);
    }

    // ========================================================================
    // ContentMode Methods Tests
    // ========================================================================

    #[test]
    fn test_content_mode_as_str() {
        assert_eq!(ContentMode::Crunch.as_str(), "crunch");
        assert_eq!(ContentMode::Fluff.as_str(), "fluff");
        assert_eq!(ContentMode::Mixed.as_str(), "mixed");
        assert_eq!(ContentMode::Example.as_str(), "example");
        assert_eq!(ContentMode::Optional.as_str(), "optional");
        assert_eq!(ContentMode::Fiction.as_str(), "fiction");
    }

    #[test]
    fn test_content_mode_search_weight() {
        assert!(ContentMode::Crunch.search_weight() > 1.0);
        assert_eq!(ContentMode::Fluff.search_weight(), 1.0);
        assert!(ContentMode::Fiction.search_weight() < 1.0);
    }

    #[test]
    fn test_content_mode_prefers_semantic() {
        assert!(ContentMode::Fluff.prefers_semantic_search());
        assert!(ContentMode::Fiction.prefers_semantic_search());
        assert!(!ContentMode::Crunch.prefers_semantic_search());
        assert!(!ContentMode::Example.prefers_semantic_search());
    }

    #[test]
    fn test_result_high_confidence() {
        let high_conf = ContentModeResult::new(ContentMode::Crunch, 0.9, 0.1, 0.85);
        assert!(high_conf.is_high_confidence());

        let low_conf = ContentModeResult::new(ContentMode::Mixed, 0.5, 0.5, 0.4);
        assert!(!low_conf.is_high_confidence());
    }

    #[test]
    fn test_result_score_clamping() {
        let result = ContentModeResult::new(ContentMode::Crunch, 1.5, -0.5, 2.0);
        assert_eq!(result.crunch_score, 1.0);
        assert_eq!(result.fluff_score, 0.0);
        assert_eq!(result.confidence, 1.0);
    }
}
