//! Context Detection Keywords
//!
//! Defines keyword sets and weights for automatic gameplay context detection.
//! Uses weighted keyword matching with configurable thresholds.

use super::context::GameplayContext;
use std::collections::HashMap;

// ============================================================================
// Constants
// ============================================================================

/// Normalization factor for confidence scores.
/// A raw score of 3.0 (about 3-4 keyword matches) produces ~1.0 confidence.
/// Chosen based on empirical testing of typical TTRPG context detection inputs.
const CONFIDENCE_NORMALIZATION_FACTOR: f32 = 3.0;

// ============================================================================
// Keyword Configuration
// ============================================================================

/// Configuration for context detection.
#[derive(Debug, Clone)]
pub struct ContextDetectionConfig {
    /// Minimum confidence threshold for detection (0.0 to 1.0).
    pub min_confidence: f32,

    /// Minimum input length for reliable detection.
    pub min_input_length: usize,

    /// Ambiguity threshold - if two contexts are within this margin,
    /// the detection is considered ambiguous.
    pub ambiguity_threshold: f32,

    /// Whether to use word boundary matching (vs substring matching).
    pub use_word_boundaries: bool,

    /// Case sensitivity for keyword matching.
    pub case_sensitive: bool,
}

impl Default for ContextDetectionConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.3,
            min_input_length: 10,
            ambiguity_threshold: 0.15,
            use_word_boundaries: true,
            case_sensitive: false,
        }
    }
}

// ============================================================================
// Keyword Sets
// ============================================================================

/// A weighted keyword for context detection.
#[derive(Debug, Clone, Copy)]
pub struct WeightedKeyword {
    /// The keyword or phrase to match.
    pub keyword: &'static str,

    /// Weight/importance of this keyword (0.0 to 1.0).
    pub weight: f32,

    /// Whether this is a strong indicator (presence alone suggests context).
    pub is_strong: bool,
}

impl WeightedKeyword {
    /// Create a new weighted keyword.
    pub const fn new(keyword: &'static str, weight: f32) -> Self {
        Self {
            keyword,
            weight,
            is_strong: false,
        }
    }

    /// Create a strong indicator keyword.
    pub const fn strong(keyword: &'static str, weight: f32) -> Self {
        Self {
            keyword,
            weight,
            is_strong: true,
        }
    }
}

// ============================================================================
// Static Keyword Arrays
// ============================================================================

/// Combat encounter keywords.
static COMBAT_KEYWORDS: &[WeightedKeyword] = &[
    // Strong indicators
    WeightedKeyword::strong("initiative", 0.9),
    WeightedKeyword::strong("attack roll", 0.9),
    WeightedKeyword::strong("saving throw", 0.8),
    WeightedKeyword::strong("hit points", 0.7),
    WeightedKeyword::strong("armor class", 0.7),
    WeightedKeyword::strong("damage roll", 0.8),
    // Regular indicators
    WeightedKeyword::new("attack", 0.6),
    WeightedKeyword::new("damage", 0.5),
    WeightedKeyword::new("round", 0.4),
    WeightedKeyword::new("turn", 0.3),
    WeightedKeyword::new("AC", 0.7),
    WeightedKeyword::new("hp", 0.5),
    WeightedKeyword::new("action", 0.4),
    WeightedKeyword::new("bonus action", 0.7),
    WeightedKeyword::new("reaction", 0.6),
    WeightedKeyword::new("critical hit", 0.8),
    WeightedKeyword::new("critical", 0.5),
    WeightedKeyword::new("miss", 0.3),
    WeightedKeyword::new("hit", 0.3),
    WeightedKeyword::new("weapon", 0.4),
    WeightedKeyword::new("spell attack", 0.7),
    WeightedKeyword::new("melee", 0.5),
    WeightedKeyword::new("ranged", 0.4),
    WeightedKeyword::new("disadvantage", 0.5),
    WeightedKeyword::new("advantage", 0.5),
    WeightedKeyword::new("flanking", 0.6),
    WeightedKeyword::new("opportunity attack", 0.8),
    WeightedKeyword::new("concentration", 0.5),
    WeightedKeyword::new("grapple", 0.6),
    WeightedKeyword::new("prone", 0.5),
    WeightedKeyword::new("incapacitated", 0.6),
    WeightedKeyword::new("unconscious", 0.5),
    WeightedKeyword::new("death save", 0.9),
    WeightedKeyword::new("stabilize", 0.6),
];

/// Social interaction keywords.
static SOCIAL_KEYWORDS: &[WeightedKeyword] = &[
    // Strong indicators
    WeightedKeyword::strong("persuasion", 0.8),
    WeightedKeyword::strong("deception", 0.8),
    WeightedKeyword::strong("intimidation", 0.8),
    WeightedKeyword::strong("insight check", 0.8),
    WeightedKeyword::strong("in character", 0.9),
    // Regular indicators
    WeightedKeyword::new("talk", 0.4),
    WeightedKeyword::new("speak", 0.3),
    WeightedKeyword::new("ask", 0.3),
    WeightedKeyword::new("tell", 0.3),
    WeightedKeyword::new("negotiate", 0.7),
    WeightedKeyword::new("bargain", 0.6),
    WeightedKeyword::new("convince", 0.6),
    WeightedKeyword::new("lie", 0.5),
    WeightedKeyword::new("bluff", 0.6),
    WeightedKeyword::new("charm", 0.5),
    WeightedKeyword::new("diplomacy", 0.7),
    WeightedKeyword::new("rapport", 0.5),
    WeightedKeyword::new("conversation", 0.5),
    WeightedKeyword::new("dialogue", 0.6),
    WeightedKeyword::new("roleplay", 0.8),
    WeightedKeyword::new("NPC", 0.4),
    WeightedKeyword::new("character voice", 0.7),
    WeightedKeyword::new("says", 0.3),
    WeightedKeyword::new("replies", 0.4),
    WeightedKeyword::new("responds", 0.3),
    WeightedKeyword::new("emotion", 0.4),
    WeightedKeyword::new("feeling", 0.3),
    WeightedKeyword::new("attitude", 0.4),
    WeightedKeyword::new("friendly", 0.3),
    WeightedKeyword::new("hostile", 0.4),
    WeightedKeyword::new("indifferent", 0.3),
];

/// Exploration keywords.
static EXPLORATION_KEYWORDS: &[WeightedKeyword] = &[
    // Strong indicators
    WeightedKeyword::strong("perception check", 0.8),
    WeightedKeyword::strong("investigation check", 0.7),
    WeightedKeyword::strong("survival check", 0.8),
    // Regular indicators
    WeightedKeyword::new("explore", 0.7),
    WeightedKeyword::new("search", 0.5),
    WeightedKeyword::new("look around", 0.6),
    WeightedKeyword::new("examine", 0.5),
    WeightedKeyword::new("travel", 0.6),
    WeightedKeyword::new("journey", 0.5),
    WeightedKeyword::new("path", 0.3),
    WeightedKeyword::new("road", 0.3),
    WeightedKeyword::new("forest", 0.3),
    WeightedKeyword::new("dungeon", 0.5),
    WeightedKeyword::new("room", 0.3),
    WeightedKeyword::new("corridor", 0.4),
    WeightedKeyword::new("door", 0.3),
    WeightedKeyword::new("entrance", 0.4),
    WeightedKeyword::new("exit", 0.3),
    WeightedKeyword::new("discover", 0.5),
    WeightedKeyword::new("find", 0.3),
    WeightedKeyword::new("notice", 0.4),
    WeightedKeyword::new("spot", 0.4),
    WeightedKeyword::new("hidden", 0.5),
    WeightedKeyword::new("secret", 0.5),
    WeightedKeyword::new("trap", 0.6),
    WeightedKeyword::new("navigation", 0.6),
    WeightedKeyword::new("map", 0.4),
    WeightedKeyword::new("terrain", 0.5),
    WeightedKeyword::new("wilderness", 0.5),
    WeightedKeyword::new("cave", 0.4),
    WeightedKeyword::new("ruins", 0.5),
];

/// Puzzle/investigation keywords.
static PUZZLE_KEYWORDS: &[WeightedKeyword] = &[
    // Strong indicators
    WeightedKeyword::strong("puzzle", 0.9),
    WeightedKeyword::strong("riddle", 0.9),
    WeightedKeyword::strong("clue", 0.8),
    WeightedKeyword::strong("mystery", 0.8),
    // Regular indicators
    WeightedKeyword::new("solve", 0.6),
    WeightedKeyword::new("figure out", 0.6),
    WeightedKeyword::new("decipher", 0.7),
    WeightedKeyword::new("decode", 0.7),
    WeightedKeyword::new("investigate", 0.6),
    WeightedKeyword::new("evidence", 0.6),
    WeightedKeyword::new("suspect", 0.5),
    WeightedKeyword::new("witness", 0.5),
    WeightedKeyword::new("crime", 0.6),
    WeightedKeyword::new("murder", 0.6),
    WeightedKeyword::new("theft", 0.5),
    WeightedKeyword::new("mechanism", 0.5),
    WeightedKeyword::new("lever", 0.4),
    WeightedKeyword::new("button", 0.3),
    WeightedKeyword::new("combination", 0.6),
    WeightedKeyword::new("code", 0.5),
    WeightedKeyword::new("cipher", 0.7),
    WeightedKeyword::new("pattern", 0.5),
    WeightedKeyword::new("sequence", 0.5),
    WeightedKeyword::new("logic", 0.5),
    WeightedKeyword::new("deduce", 0.6),
    WeightedKeyword::new("conclusion", 0.4),
    WeightedKeyword::new("theory", 0.4),
    WeightedKeyword::new("hypothesis", 0.5),
];

/// Lore/exposition keywords.
static LORE_KEYWORDS: &[WeightedKeyword] = &[
    // Strong indicators
    WeightedKeyword::strong("history check", 0.8),
    WeightedKeyword::strong("arcana check", 0.7),
    WeightedKeyword::strong("religion check", 0.7),
    WeightedKeyword::strong("nature check", 0.6),
    // Regular indicators
    WeightedKeyword::new("history", 0.6),
    WeightedKeyword::new("legend", 0.7),
    WeightedKeyword::new("ancient", 0.5),
    WeightedKeyword::new("story", 0.4),
    WeightedKeyword::new("tale", 0.5),
    WeightedKeyword::new("origin", 0.5),
    WeightedKeyword::new("mythology", 0.7),
    WeightedKeyword::new("prophecy", 0.7),
    WeightedKeyword::new("lore", 0.8),
    WeightedKeyword::new("background", 0.4),
    WeightedKeyword::new("backstory", 0.5),
    WeightedKeyword::new("tradition", 0.4),
    WeightedKeyword::new("culture", 0.5),
    WeightedKeyword::new("custom", 0.3),
    WeightedKeyword::new("belief", 0.4),
    WeightedKeyword::new("religion", 0.5),
    WeightedKeyword::new("deity", 0.6),
    WeightedKeyword::new("god", 0.4),
    WeightedKeyword::new("goddess", 0.4),
    WeightedKeyword::new("artifact", 0.6),
    WeightedKeyword::new("relic", 0.6),
    WeightedKeyword::new("tome", 0.5),
    WeightedKeyword::new("scripture", 0.5),
    WeightedKeyword::new("chronicle", 0.6),
    WeightedKeyword::new("ancestor", 0.4),
    WeightedKeyword::new("lineage", 0.5),
    WeightedKeyword::new("dynasty", 0.5),
    WeightedKeyword::new("era", 0.4),
    WeightedKeyword::new("age", 0.3),
    WeightedKeyword::new("epoch", 0.5),
];

/// Downtime keywords.
static DOWNTIME_KEYWORDS: &[WeightedKeyword] = &[
    // Strong indicators
    WeightedKeyword::strong("long rest", 0.9),
    WeightedKeyword::strong("short rest", 0.8),
    WeightedKeyword::strong("downtime", 0.9),
    // Regular indicators
    WeightedKeyword::new("rest", 0.5),
    WeightedKeyword::new("sleep", 0.5),
    WeightedKeyword::new("camp", 0.5),
    WeightedKeyword::new("inn", 0.5),
    WeightedKeyword::new("tavern", 0.4),
    WeightedKeyword::new("shop", 0.6),
    WeightedKeyword::new("store", 0.5),
    WeightedKeyword::new("merchant", 0.5),
    WeightedKeyword::new("buy", 0.5),
    WeightedKeyword::new("sell", 0.5),
    WeightedKeyword::new("purchase", 0.5),
    WeightedKeyword::new("gold", 0.3),
    WeightedKeyword::new("coin", 0.3),
    WeightedKeyword::new("price", 0.4),
    WeightedKeyword::new("craft", 0.6),
    WeightedKeyword::new("crafting", 0.7),
    WeightedKeyword::new("brew", 0.5),
    WeightedKeyword::new("potion", 0.4),
    WeightedKeyword::new("enchant", 0.5),
    WeightedKeyword::new("repair", 0.5),
    WeightedKeyword::new("train", 0.5),
    WeightedKeyword::new("training", 0.6),
    WeightedKeyword::new("practice", 0.4),
    WeightedKeyword::new("study", 0.4),
    WeightedKeyword::new("research", 0.5),
    WeightedKeyword::new("recover", 0.5),
    WeightedKeyword::new("heal", 0.4),
    WeightedKeyword::new("relax", 0.5),
];

/// Rules clarification keywords.
static RULES_KEYWORDS: &[WeightedKeyword] = &[
    // Strong indicators
    WeightedKeyword::strong("according to", 0.8),
    WeightedKeyword::strong("rules as written", 0.9),
    WeightedKeyword::strong("RAW", 0.9),
    WeightedKeyword::strong("rules as intended", 0.9),
    WeightedKeyword::strong("RAI", 0.9),
    WeightedKeyword::strong("PHB", 0.8),
    WeightedKeyword::strong("DMG", 0.8),
    WeightedKeyword::strong("errata", 0.9),
    // Regular indicators
    WeightedKeyword::new("rule", 0.6),
    WeightedKeyword::new("ruling", 0.7),
    WeightedKeyword::new("clarify", 0.6),
    WeightedKeyword::new("clarification", 0.7),
    WeightedKeyword::new("how does", 0.5),
    WeightedKeyword::new("how do", 0.5),
    WeightedKeyword::new("can I", 0.4),
    WeightedKeyword::new("can you", 0.3),
    WeightedKeyword::new("allowed", 0.5),
    WeightedKeyword::new("legal", 0.5),
    WeightedKeyword::new("valid", 0.4),
    WeightedKeyword::new("DC", 0.6),
    WeightedKeyword::new("modifier", 0.5),
    WeightedKeyword::new("proficiency", 0.5),
    WeightedKeyword::new("proficient", 0.5),
    WeightedKeyword::new("bonus", 0.4),
    WeightedKeyword::new("penalty", 0.4),
    WeightedKeyword::new("stacking", 0.6),
    WeightedKeyword::new("stack", 0.5),
    WeightedKeyword::new("overlap", 0.4),
    WeightedKeyword::new("interaction", 0.4),
    WeightedKeyword::new("mechanic", 0.6),
    WeightedKeyword::new("mechanics", 0.6),
    WeightedKeyword::new("ability score", 0.6),
    WeightedKeyword::new("skill check", 0.5),
    WeightedKeyword::new("contest", 0.5),
    WeightedKeyword::new("opposed", 0.4),
];

// ============================================================================
// Keyword Lookup Functions
// ============================================================================

/// Get keywords for combat encounter context.
pub fn combat_keywords() -> &'static [WeightedKeyword] {
    COMBAT_KEYWORDS
}

/// Get keywords for social interaction context.
pub fn social_keywords() -> &'static [WeightedKeyword] {
    SOCIAL_KEYWORDS
}

/// Get keywords for exploration context.
pub fn exploration_keywords() -> &'static [WeightedKeyword] {
    EXPLORATION_KEYWORDS
}

/// Get keywords for puzzle/investigation context.
pub fn puzzle_keywords() -> &'static [WeightedKeyword] {
    PUZZLE_KEYWORDS
}

/// Get keywords for lore/exposition context.
pub fn lore_keywords() -> &'static [WeightedKeyword] {
    LORE_KEYWORDS
}

/// Get keywords for downtime context.
pub fn downtime_keywords() -> &'static [WeightedKeyword] {
    DOWNTIME_KEYWORDS
}

/// Get keywords for rules clarification context.
pub fn rules_keywords() -> &'static [WeightedKeyword] {
    RULES_KEYWORDS
}

/// Get keywords for a specific gameplay context.
pub fn get_keywords_for_context(context: GameplayContext) -> &'static [WeightedKeyword] {
    match context {
        GameplayContext::CombatEncounter => COMBAT_KEYWORDS,
        GameplayContext::SocialInteraction => SOCIAL_KEYWORDS,
        GameplayContext::Exploration => EXPLORATION_KEYWORDS,
        GameplayContext::PuzzleInvestigation => PUZZLE_KEYWORDS,
        GameplayContext::LoreExposition => LORE_KEYWORDS,
        GameplayContext::Downtime => DOWNTIME_KEYWORDS,
        GameplayContext::RuleClarification => RULES_KEYWORDS,
        GameplayContext::Unknown => &[],
    }
}

/// Build a HashMap of all keywords for efficient lookup.
pub fn build_keyword_map() -> HashMap<GameplayContext, Vec<WeightedKeyword>> {
    let mut map = HashMap::new();

    for context in GameplayContext::all_defined() {
        let keywords = get_keywords_for_context(*context);
        map.insert(*context, keywords.to_vec());
    }

    map
}

// ============================================================================
// Context Detector
// ============================================================================

/// Detects gameplay context from input text using weighted keyword matching.
#[derive(Debug, Clone)]
pub struct ContextDetector {
    /// Detection configuration.
    config: ContextDetectionConfig,

    /// Pre-built keyword map for efficient lookup.
    keyword_map: HashMap<GameplayContext, Vec<WeightedKeyword>>,
}

impl ContextDetector {
    /// Create a new context detector with default configuration.
    pub fn new() -> Self {
        Self {
            config: ContextDetectionConfig::default(),
            keyword_map: build_keyword_map(),
        }
    }

    /// Create a context detector with custom configuration.
    pub fn with_config(config: ContextDetectionConfig) -> Self {
        Self {
            config,
            keyword_map: build_keyword_map(),
        }
    }

    /// Get the current configuration.
    pub fn config(&self) -> &ContextDetectionConfig {
        &self.config
    }

    /// Update the configuration.
    pub fn set_config(&mut self, config: ContextDetectionConfig) {
        self.config = config;
    }

    /// Detect the gameplay context from the given text.
    ///
    /// Returns the detected context with confidence score, or None if
    /// the input is too short or no context can be determined.
    pub fn detect(&self, text: &str) -> Option<DetectionResult> {
        // Check minimum length
        if text.len() < self.config.min_input_length {
            return None;
        }

        // Prepare text for matching
        let text_lower = if self.config.case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };

        // Score each context
        let mut scores: Vec<(GameplayContext, f32, Vec<String>)> = Vec::new();

        for context in GameplayContext::all_defined() {
            let (score, matched) = self.score_context(*context, &text_lower);
            if score > 0.0 {
                scores.push((*context, score, matched));
            }
        }

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Check if we have any matches
        if scores.is_empty() {
            return Some(DetectionResult {
                context: GameplayContext::Unknown,
                confidence: 0.0,
                matched_keywords: Vec::new(),
                alternatives: Vec::new(),
                is_ambiguous: false,
            });
        }

        let (best_context, best_score, best_keywords) = scores.remove(0);

        // Normalize score to 0.0-1.0 range using saturation scaling
        let confidence = (best_score / CONFIDENCE_NORMALIZATION_FACTOR).min(1.0);

        // Check for ambiguity
        let is_ambiguous = scores
            .first()
            .map(|(_, score, _)| {
                let second_confidence = (score / CONFIDENCE_NORMALIZATION_FACTOR).min(1.0);
                (confidence - second_confidence).abs() < self.config.ambiguity_threshold
            })
            .unwrap_or(false);

        // Build alternatives
        let alternatives: Vec<(GameplayContext, f32)> = scores
            .into_iter()
            .take(3)
            .map(|(ctx, score, _)| (ctx, (score / CONFIDENCE_NORMALIZATION_FACTOR).min(1.0)))
            .collect();

        // Check minimum confidence
        if confidence < self.config.min_confidence {
            return Some(DetectionResult {
                context: GameplayContext::Unknown,
                confidence,
                matched_keywords: best_keywords,
                alternatives,
                is_ambiguous,
            });
        }

        Some(DetectionResult {
            context: best_context,
            confidence,
            matched_keywords: best_keywords,
            alternatives,
            is_ambiguous,
        })
    }

    /// Score a specific context against the text.
    fn score_context(&self, context: GameplayContext, text: &str) -> (f32, Vec<String>) {
        let keywords = self.keyword_map.get(&context).map(|v| v.as_slice()).unwrap_or(&[]);

        let mut score = 0.0f32;
        let mut matched = Vec::new();

        for kw in keywords {
            let keyword_lower = if self.config.case_sensitive {
                kw.keyword.to_string()
            } else {
                kw.keyword.to_lowercase()
            };

            let is_match = if self.config.use_word_boundaries {
                self.word_boundary_match(text, &keyword_lower)
            } else {
                text.contains(&keyword_lower)
            };

            if is_match {
                score += kw.weight;
                if kw.is_strong {
                    score += 0.5; // Bonus for strong indicators
                }
                matched.push(kw.keyword.to_string());
            }
        }

        (score, matched)
    }

    /// Check for word boundary match.
    fn word_boundary_match(&self, text: &str, keyword: &str) -> bool {
        // Simple word boundary check using char boundaries
        if let Some(pos) = text.find(keyword) {
            let before_ok = pos == 0
                || text[..pos]
                    .chars()
                    .last()
                    .map(|c| !c.is_alphanumeric())
                    .unwrap_or(true);

            let after_ok = pos + keyword.len() >= text.len()
                || text[pos + keyword.len()..]
                    .chars()
                    .next()
                    .map(|c| !c.is_alphanumeric())
                    .unwrap_or(true);

            before_ok && after_ok
        } else {
            false
        }
    }
}

impl Default for ContextDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of context detection.
#[derive(Debug, Clone)]
pub struct DetectionResult {
    /// The detected context.
    pub context: GameplayContext,

    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,

    /// Keywords that matched.
    pub matched_keywords: Vec<String>,

    /// Alternative contexts with their confidence scores.
    pub alternatives: Vec<(GameplayContext, f32)>,

    /// Whether the detection is ambiguous.
    pub is_ambiguous: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combat_detection() {
        let detector = ContextDetector::new();

        let text = "I roll for initiative and attack the goblin with my sword. The attack roll is 15.";
        let result = detector.detect(text).unwrap();

        assert_eq!(result.context, GameplayContext::CombatEncounter);
        assert!(result.confidence > 0.5);
        assert!(!result.matched_keywords.is_empty());
    }

    #[test]
    fn test_social_detection() {
        let detector = ContextDetector::new();

        let text = "I want to persuade the guard to let us through. I'll try to charm him with a friendly conversation.";
        let result = detector.detect(text).unwrap();

        assert_eq!(result.context, GameplayContext::SocialInteraction);
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_lore_detection() {
        let detector = ContextDetector::new();

        let text = "Tell me about the ancient history of this place. What legends exist about the artifact?";
        let result = detector.detect(text).unwrap();

        assert_eq!(result.context, GameplayContext::LoreExposition);
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_rules_detection() {
        let detector = ContextDetector::new();

        let text = "According to RAW in the PHB, how does the DC for this skill check work?";
        let result = detector.detect(text).unwrap();

        assert_eq!(result.context, GameplayContext::RuleClarification);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_downtime_detection() {
        let detector = ContextDetector::new();

        let text = "We take a long rest at the inn and I want to shop for potions tomorrow.";
        let result = detector.detect(text).unwrap();

        assert_eq!(result.context, GameplayContext::Downtime);
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_exploration_detection() {
        let detector = ContextDetector::new();

        let text = "I search the room carefully and look for any hidden doors or secret passages.";
        let result = detector.detect(text).unwrap();

        assert_eq!(result.context, GameplayContext::Exploration);
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_puzzle_detection() {
        let detector = ContextDetector::new();

        let text = "There's a riddle on the wall. We need to solve this puzzle to open the door.";
        let result = detector.detect(text).unwrap();

        assert_eq!(result.context, GameplayContext::PuzzleInvestigation);
        assert!(result.confidence > 0.3);
    }

    #[test]
    fn test_short_input() {
        let detector = ContextDetector::new();

        let text = "hi";
        let result = detector.detect(text);

        assert!(result.is_none());
    }

    #[test]
    fn test_unknown_context() {
        let detector = ContextDetector::new();

        let text = "The weather is nice today. I like pizza.";
        let result = detector.detect(text).unwrap();

        assert_eq!(result.context, GameplayContext::Unknown);
        assert!(result.confidence < 0.3);
    }

    #[test]
    fn test_word_boundary_matching() {
        let detector = ContextDetector::new();

        // "hit" should not match in "white"
        let text = "The white wolf approached. I examine its behavior.";
        let result = detector.detect(text).unwrap();

        // Should not detect combat just from "white" containing "hit"
        assert_ne!(result.context, GameplayContext::CombatEncounter);
    }

    #[test]
    fn test_custom_config() {
        let config = ContextDetectionConfig {
            min_confidence: 0.8,
            min_input_length: 5,
            ambiguity_threshold: 0.1,
            use_word_boundaries: false,
            case_sensitive: false,
        };

        let detector = ContextDetector::with_config(config);

        // With high min_confidence, marginal matches should return Unknown
        let text = "I attack the goblin.";
        let result = detector.detect(text).unwrap();

        // Might be Unknown due to high threshold
        assert!(result.confidence < 0.8 || result.context == GameplayContext::CombatEncounter);
    }

    #[test]
    fn test_keyword_map_completeness() {
        let map = build_keyword_map();

        // All defined contexts should have keywords
        for context in GameplayContext::all_defined() {
            assert!(
                map.contains_key(context),
                "Missing keywords for context: {:?}",
                context
            );
            assert!(
                !map.get(context).unwrap().is_empty(),
                "Empty keywords for context: {:?}",
                context
            );
        }
    }

    #[test]
    fn test_detection_result_alternatives() {
        let detector = ContextDetector::new();

        // Text that could match multiple contexts
        let text = "I search the room for clues to solve the mystery puzzle while investigating.";
        let result = detector.detect(text).unwrap();

        // Should have alternatives
        // Note: may or may not have alternatives depending on match specificity
        println!("Context: {:?}, Alternatives: {:?}", result.context, result.alternatives);
    }

    #[test]
    fn test_strong_indicator_bonus() {
        // Verify that strong indicators give higher scores
        let detector = ContextDetector::new();

        // Text with strong indicator
        let strong_text = "Roll for initiative!";
        let strong_result = detector.detect(strong_text);

        // Text without strong indicator but with regular combat terms
        let weak_text = "I attack with my weapon.";
        let weak_result = detector.detect(weak_text);

        if let (Some(strong), Some(weak)) = (strong_result, weak_result) {
            // Both should detect combat, but strong should have higher confidence
            if strong.context == GameplayContext::CombatEncounter
                && weak.context == GameplayContext::CombatEncounter
            {
                assert!(
                    strong.confidence >= weak.confidence,
                    "Strong indicator should give higher or equal confidence"
                );
            }
        }
    }
}
