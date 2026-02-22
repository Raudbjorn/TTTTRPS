//! Dice Expression Extraction Module
//!
//! Extracts and parses dice notation from TTRPG text content.
//! Supports standard dice expressions (2d6+3), difficulty checks (DC 15),
//! and standalone modifiers (+2 to hit).
//!
//! # Example
//!
//! ```ignore
//! use crate::ingestion::ttrpg::dice_extractor::DiceExtractor;
//!
//! let extractor = DiceExtractor::new();
//! let result = extractor.extract("Roll 2d6 + 1d4 + 3 fire damage, DC 15 Wisdom save");
//!
//! assert_eq!(result.expressions.len(), 2);
//! assert_eq!(result.difficulty_checks.len(), 1);
//! ```

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Standard die types used in TTRPGs.
pub const STANDARD_DIE_SIDES: &[u32] = &[4, 6, 8, 10, 12, 20, 100];

/// A parsed dice expression (e.g., "2d6+3").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiceExpression {
    /// Number of dice to roll (e.g., 2 in "2d6")
    pub count: u32,
    /// Number of sides on each die (e.g., 6 in "2d6")
    pub sides: u32,
    /// Optional modifier to add/subtract (e.g., 3 in "2d6+3", -1 in "1d4-1")
    pub modifier: i32,
    /// The original matched text
    pub raw_text: String,
}

impl DiceExpression {
    /// Create a new dice expression.
    pub fn new(count: u32, sides: u32, modifier: i32, raw_text: String) -> Self {
        Self {
            count,
            sides,
            modifier,
            raw_text,
        }
    }

    /// Check if this uses a standard die type.
    pub fn is_standard_die(&self) -> bool {
        STANDARD_DIE_SIDES.contains(&self.sides)
    }

    /// Calculate the minimum possible roll.
    pub fn min_roll(&self) -> i32 {
        (self.count as i32) + self.modifier
    }

    /// Calculate the maximum possible roll.
    pub fn max_roll(&self) -> i32 {
        (self.count as i32 * self.sides as i32) + self.modifier
    }

    /// Calculate the average roll.
    pub fn average_roll(&self) -> f64 {
        let avg_per_die = (1.0 + self.sides as f64) / 2.0;
        (self.count as f64 * avg_per_die) + self.modifier as f64
    }

    /// Format as canonical dice notation (e.g., "2d6+3").
    pub fn to_canonical(&self) -> String {
        let base = if self.count == 1 {
            format!("d{}", self.sides)
        } else {
            format!("{}d{}", self.count, self.sides)
        };

        if self.modifier == 0 {
            base
        } else if self.modifier > 0 {
            format!("{}+{}", base, self.modifier)
        } else {
            format!("{}{}", base, self.modifier)
        }
    }
}

impl std::fmt::Display for DiceExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_canonical())
    }
}

/// A difficulty check (e.g., "DC 15 Wisdom").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifficultyCheck {
    /// The DC value (e.g., 15 in "DC 15")
    pub dc_value: u32,
    /// Optional check type (e.g., "Wisdom", "Strength")
    pub check_type: Option<String>,
    /// The original matched text
    pub raw_text: String,
}

impl DifficultyCheck {
    /// Create a new difficulty check.
    pub fn new(dc_value: u32, check_type: Option<String>, raw_text: String) -> Self {
        Self {
            dc_value,
            check_type,
            raw_text,
        }
    }

    /// Classify the DC difficulty tier (D&D 5e standard).
    pub fn difficulty_tier(&self) -> &'static str {
        match self.dc_value {
            0..=4 => "trivial",
            5..=9 => "easy",
            10..=14 => "medium",
            15..=19 => "hard",
            20..=24 => "very hard",
            25..=29 => "nearly impossible",
            _ => "legendary",
        }
    }
}

impl std::fmt::Display for DifficultyCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.check_type {
            Some(check_type) => write!(f, "DC {} {}", self.dc_value, check_type),
            None => write!(f, "DC {}", self.dc_value),
        }
    }
}

/// A standalone modifier (e.g., "+2 to hit").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StandaloneModifier {
    /// The modifier value (positive or negative)
    pub value: i32,
    /// Context describing what the modifier applies to
    pub context: String,
    /// The original matched text
    pub raw_text: String,
}

impl StandaloneModifier {
    /// Create a new standalone modifier.
    pub fn new(value: i32, context: String, raw_text: String) -> Self {
        Self {
            value,
            context,
            raw_text,
        }
    }
}

impl std::fmt::Display for StandaloneModifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.value >= 0 {
            write!(f, "+{} {}", self.value, self.context)
        } else {
            write!(f, "{} {}", self.value, self.context)
        }
    }
}

/// Result of extracting dice notation from text.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiceExtractionResult {
    /// All dice expressions found (e.g., "2d6+3", "1d20")
    pub expressions: Vec<DiceExpression>,
    /// All difficulty checks found (e.g., "DC 15 Wisdom")
    pub difficulty_checks: Vec<DifficultyCheck>,
    /// Standalone modifiers (e.g., "+2 to hit")
    pub modifiers: Vec<StandaloneModifier>,
}

impl DiceExtractionResult {
    /// Check if any dice-related content was found.
    pub fn is_empty(&self) -> bool {
        self.expressions.is_empty()
            && self.difficulty_checks.is_empty()
            && self.modifiers.is_empty()
    }

    /// Get total count of all extracted items.
    pub fn total_count(&self) -> usize {
        self.expressions.len() + self.difficulty_checks.len() + self.modifiers.len()
    }

    /// Get all unique die types found.
    pub fn unique_die_types(&self) -> Vec<u32> {
        let mut types: Vec<u32> = self.expressions.iter().map(|e| e.sides).collect();
        types.sort_unstable();
        types.dedup();
        types
    }

    /// Check if this appears to be combat-related (has attack rolls or damage).
    pub fn is_combat_related(&self) -> bool {
        // Check for d20 (attack rolls, saves)
        let has_d20 = self.expressions.iter().any(|e| e.sides == 20);

        // Check for attack/damage modifiers
        let has_attack_modifier = self.modifiers.iter().any(|m| {
            let ctx_lower = m.context.to_lowercase();
            ctx_lower.contains("attack")
                || ctx_lower.contains("hit")
                || ctx_lower.contains("damage")
        });

        // Check for saves
        let has_save = self.difficulty_checks.iter().any(|dc| {
            dc.check_type
                .as_ref()
                .map(|t| {
                    let t_lower = t.to_lowercase();
                    t_lower.contains("save")
                        || t_lower.contains("saving")
                        || ["strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma"]
                            .iter()
                            .any(|a| t_lower.contains(a))
                })
                .unwrap_or(false)
        });

        has_d20 || has_attack_modifier || has_save
    }
}

// ============================================================================
// Regex Patterns
// ============================================================================

/// Pattern for basic dice expressions: "d20", "2d6", "3d8+5", "1d4-1"
/// Captures: (count)d(sides)(+/-modifier)
static DICE_EXPR_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
        (?P<count>\d+)?                 # Optional count (defaults to 1)
        d                               # The 'd' separator
        (?P<sides>\d+|%)                # Die sides or % for d100
        (?:
            \s*                         # Optional whitespace
            (?P<mod_sign>[+\-−–])       # Modifier sign (various dash types)
            \s*                         # Optional whitespace
            (?P<modifier>\d+)           # Modifier value
        )?
        ",
    )
    .expect("Failed to compile dice expression regex")
});

/// Pattern for DC (Difficulty Class): "DC 15", "DC 18 Wisdom", "difficulty class 12"
static DC_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
        (?:DC|difficulty\s+class)       # DC or full phrase
        \s+
        (?P<dc>\d+)                     # The DC value
        (?:
            \s+
            (?P<type>
                (?:strength|dexterity|constitution|intelligence|wisdom|charisma)
                (?:\s+(?:saving\s+throw|save|check))?
                |
                (?:saving\s+throw|save|check)
            )
        )?
        ",
    )
    .expect("Failed to compile DC pattern regex")
});

/// Pattern for standalone modifiers: "+3 bonus", "-2 penalty", "+1 to attack"
static MODIFIER_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?ix)
        (?P<sign>[+\-−–])               # Sign
        (?P<value>\d+)                  # Value
        \s+
        (?P<context>
            (?:to\s+)?
            (?:attack|hit|damage|ac|armor\s+class|initiative|save|saving\s+throw|check|skill)
            (?:\s+(?:rolls?|bonus|penalty|modifier))?
            |
            bonus|penalty|modifier
        )
        ",
    )
    .expect("Failed to compile modifier pattern regex")
});

// ============================================================================
// Extractor
// ============================================================================

/// Extracts dice notation, difficulty checks, and modifiers from text.
#[derive(Debug, Clone, Default)]
pub struct DiceExtractor;

impl DiceExtractor {
    /// Create a new dice extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract all dice-related content from text.
    pub fn extract(&self, text: &str) -> DiceExtractionResult {
        let mut result = DiceExtractionResult::default();

        result.expressions = self.extract_dice_expressions(text);
        result.difficulty_checks = self.extract_difficulty_checks(text);

        // Extract modifiers, filtering out those that overlap with dice expressions
        // to prevent double-counting (e.g., "+3" in "2d6+3" should not also be a modifier)
        let dice_ranges: Vec<(usize, usize)> = DICE_EXPR_PATTERN
            .find_iter(text)
            .map(|m| (m.start(), m.end()))
            .collect();

        result.modifiers = self.extract_modifiers_filtered(text, &dice_ranges);

        result
    }

    /// Check if text contains any dice notation.
    pub fn has_dice_notation(&self, text: &str) -> bool {
        DICE_EXPR_PATTERN.is_match(text)
    }

    /// Count the number of dice expressions in text.
    pub fn count_dice_expressions(&self, text: &str) -> usize {
        DICE_EXPR_PATTERN.find_iter(text).count()
    }

    /// Extract individual dice expressions from text.
    fn extract_dice_expressions(&self, text: &str) -> Vec<DiceExpression> {
        let mut expressions = Vec::new();

        for caps in DICE_EXPR_PATTERN.captures_iter(text) {
            let raw_text = caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default();

            // Parse count (default to 1)
            let count: u32 = caps
                .name("count")
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(1);

            // Parse sides (% means d100)
            let sides_str = caps.name("sides").map(|m| m.as_str()).unwrap_or("6");
            let sides: u32 = if sides_str == "%" {
                100
            } else {
                sides_str.parse().unwrap_or(6)
            };

            // Parse modifier
            let modifier: i32 = match (caps.name("mod_sign"), caps.name("modifier")) {
                (Some(sign), Some(value)) => {
                    let val: i32 = value.as_str().parse().unwrap_or(0);
                    let sign_char = sign.as_str().chars().next().unwrap_or('+');
                    if sign_char == '+' {
                        val
                    } else {
                        -val
                    }
                }
                _ => 0,
            };

            expressions.push(DiceExpression::new(count, sides, modifier, raw_text));
        }

        expressions
    }

    /// Extract difficulty checks from text.
    fn extract_difficulty_checks(&self, text: &str) -> Vec<DifficultyCheck> {
        let mut checks = Vec::new();

        for caps in DC_PATTERN.captures_iter(text) {
            let raw_text = caps.get(0).map(|m| m.as_str().to_string()).unwrap_or_default();

            let dc_value: u32 = caps
                .name("dc")
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(10);

            let check_type = caps
                .name("type")
                .map(|m| normalize_check_type(m.as_str()));

            checks.push(DifficultyCheck::new(dc_value, check_type, raw_text));
        }

        checks
    }

    /// Extract standalone modifiers from text, filtering out those that overlap
    /// with dice expression ranges to prevent double-counting.
    fn extract_modifiers_filtered(
        &self,
        text: &str,
        dice_ranges: &[(usize, usize)],
    ) -> Vec<StandaloneModifier> {
        let mut modifiers = Vec::new();

        for caps in MODIFIER_PATTERN.captures_iter(text) {
            let full_match = match caps.get(0) {
                Some(m) => m,
                None => continue,
            };

            // Skip if this modifier overlaps with any dice expression
            let overlaps_dice = dice_ranges.iter().any(|(start, end)| {
                full_match.start() < *end && full_match.end() > *start
            });

            if overlaps_dice {
                continue;
            }

            let raw_text = full_match.as_str().to_string();
            let sign = caps.name("sign").map(|m| m.as_str()).unwrap_or("+");
            let value: i32 = caps
                .name("value")
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);

            let final_value = if sign.chars().next().unwrap_or('+') == '+' {
                value
            } else {
                -value
            };

            let context = caps
                .name("context")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            modifiers.push(StandaloneModifier::new(final_value, context, raw_text));
        }

        modifiers
    }

    /// Parse a single dice expression string (e.g., "2d6+3").
    /// Returns None if the string is not a valid dice expression.
    pub fn parse_expression(&self, expr: &str) -> Option<DiceExpression> {
        let expressions = self.extract_dice_expressions(expr);
        expressions.into_iter().next()
    }

    /// Calculate the combined stats for all dice expressions.
    pub fn combined_stats(&self, expressions: &[DiceExpression]) -> (i32, i32, f64) {
        let min: i32 = expressions.iter().map(|e| e.min_roll()).sum();
        let max: i32 = expressions.iter().map(|e| e.max_roll()).sum();
        let avg: f64 = expressions.iter().map(|e| e.average_roll()).sum();
        (min, max, avg)
    }
}

/// Normalize check type strings to a canonical form.
fn normalize_check_type(check_type: &str) -> String {
    let normalized = check_type.trim().to_lowercase();

    // Extract ability if present
    let abilities = [
        "strength",
        "dexterity",
        "constitution",
        "intelligence",
        "wisdom",
        "charisma",
    ];

    for ability in abilities {
        if normalized.contains(ability) {
            // Capitalize first letter
            let mut chars = ability.chars();
            let capitalized = chars
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default()
                + chars.as_str();

            return if normalized.contains("save") || normalized.contains("saving") {
                format!("{} save", capitalized)
            } else if normalized.contains("check") {
                format!("{} check", capitalized)
            } else {
                capitalized
            };
        }
    }

    // Just return capitalized version
    let mut chars = normalized.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default()
        + chars.as_str()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // DiceExpression Tests
    // ========================================================================

    #[test]
    fn test_dice_expression_basics() {
        let expr = DiceExpression::new(2, 6, 3, "2d6+3".to_string());

        assert_eq!(expr.count, 2);
        assert_eq!(expr.sides, 6);
        assert_eq!(expr.modifier, 3);
        assert!(expr.is_standard_die());
    }

    #[test]
    fn test_dice_expression_min_max_avg() {
        let expr = DiceExpression::new(2, 6, 3, "2d6+3".to_string());

        assert_eq!(expr.min_roll(), 5);  // 2 + 3
        assert_eq!(expr.max_roll(), 15); // 12 + 3
        assert!((expr.average_roll() - 10.0).abs() < 0.01); // 7 + 3
    }

    #[test]
    fn test_dice_expression_canonical() {
        assert_eq!(
            DiceExpression::new(1, 20, 0, "d20".to_string()).to_canonical(),
            "d20"
        );
        assert_eq!(
            DiceExpression::new(2, 6, 0, "2d6".to_string()).to_canonical(),
            "2d6"
        );
        assert_eq!(
            DiceExpression::new(3, 8, 5, "3d8+5".to_string()).to_canonical(),
            "3d8+5"
        );
        assert_eq!(
            DiceExpression::new(1, 4, -1, "1d4-1".to_string()).to_canonical(),
            "d4-1"
        );
    }

    #[test]
    fn test_non_standard_die() {
        let expr = DiceExpression::new(1, 7, 0, "d7".to_string());
        assert!(!expr.is_standard_die());
    }

    // ========================================================================
    // DifficultyCheck Tests
    // ========================================================================

    #[test]
    fn test_difficulty_check_basics() {
        let dc = DifficultyCheck::new(15, Some("Wisdom".to_string()), "DC 15 Wisdom".to_string());

        assert_eq!(dc.dc_value, 15);
        assert_eq!(dc.check_type, Some("Wisdom".to_string()));
    }

    #[test]
    fn test_difficulty_tiers() {
        assert_eq!(DifficultyCheck::new(5, None, "".into()).difficulty_tier(), "easy");
        assert_eq!(DifficultyCheck::new(10, None, "".into()).difficulty_tier(), "medium");
        assert_eq!(DifficultyCheck::new(15, None, "".into()).difficulty_tier(), "hard");
        assert_eq!(DifficultyCheck::new(20, None, "".into()).difficulty_tier(), "very hard");
        assert_eq!(DifficultyCheck::new(25, None, "".into()).difficulty_tier(), "nearly impossible");
        assert_eq!(DifficultyCheck::new(30, None, "".into()).difficulty_tier(), "legendary");
    }

    #[test]
    fn test_difficulty_check_display() {
        let dc1 = DifficultyCheck::new(15, None, "".into());
        assert_eq!(dc1.to_string(), "DC 15");

        let dc2 = DifficultyCheck::new(18, Some("Wisdom".to_string()), "".into());
        assert_eq!(dc2.to_string(), "DC 18 Wisdom");
    }

    // ========================================================================
    // DiceExtractor - Basic Dice Tests
    // ========================================================================

    #[test]
    fn test_extract_simple_dice() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("Roll d20 to hit");
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].count, 1);
        assert_eq!(result.expressions[0].sides, 20);
    }

    #[test]
    fn test_extract_dice_with_count() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("Deal 2d6 damage");
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].count, 2);
        assert_eq!(result.expressions[0].sides, 6);
    }

    #[test]
    fn test_extract_dice_with_modifier() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("Attack bonus: 3d8+5");
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].count, 3);
        assert_eq!(result.expressions[0].sides, 8);
        assert_eq!(result.expressions[0].modifier, 5);
    }

    #[test]
    fn test_extract_dice_negative_modifier() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("1d4-1 damage");
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].modifier, -1);
    }

    #[test]
    fn test_extract_percentile_dice() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("Roll d% for wild magic");
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].sides, 100);
    }

    #[test]
    fn test_extract_d100() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("Roll d100 on the table");
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].sides, 100);
    }

    #[test]
    fn test_extract_multiple_dice() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("Roll 2d6 + 1d8 + 4 damage");
        assert_eq!(result.expressions.len(), 2);
        assert_eq!(result.expressions[0].sides, 6);
        assert_eq!(result.expressions[1].sides, 8);
    }

    #[test]
    fn test_extract_all_standard_dice() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("d4, d6, d8, d10, d12, d20, d100");
        assert_eq!(result.expressions.len(), 7);

        let sides: Vec<u32> = result.expressions.iter().map(|e| e.sides).collect();
        assert!(sides.contains(&4));
        assert!(sides.contains(&6));
        assert!(sides.contains(&8));
        assert!(sides.contains(&10));
        assert!(sides.contains(&12));
        assert!(sides.contains(&20));
        assert!(sides.contains(&100));
    }

    // ========================================================================
    // DiceExtractor - DC Tests
    // ========================================================================

    #[test]
    fn test_extract_simple_dc() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("DC 15 to resist");
        assert_eq!(result.difficulty_checks.len(), 1);
        assert_eq!(result.difficulty_checks[0].dc_value, 15);
        assert!(result.difficulty_checks[0].check_type.is_none());
    }

    #[test]
    fn test_extract_dc_with_ability() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("DC 18 Wisdom saving throw");
        assert_eq!(result.difficulty_checks.len(), 1);
        assert_eq!(result.difficulty_checks[0].dc_value, 18);
        assert_eq!(
            result.difficulty_checks[0].check_type,
            Some("Wisdom save".to_string())
        );
    }

    #[test]
    fn test_extract_difficulty_class_full() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("Make a difficulty class 12 check");
        assert_eq!(result.difficulty_checks.len(), 1);
        assert_eq!(result.difficulty_checks[0].dc_value, 12);
    }

    #[test]
    fn test_extract_multiple_dcs() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("DC 15 Strength or DC 12 Dexterity");
        assert_eq!(result.difficulty_checks.len(), 2);
    }

    #[test]
    fn test_extract_dc_with_check_type() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("DC 14 Constitution check");
        assert_eq!(result.difficulty_checks.len(), 1);
        assert_eq!(
            result.difficulty_checks[0].check_type,
            Some("Constitution check".to_string())
        );
    }

    // ========================================================================
    // DiceExtractor - Modifier Tests
    // ========================================================================

    #[test]
    fn test_extract_attack_modifier() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("+5 to hit");
        assert_eq!(result.modifiers.len(), 1);
        assert_eq!(result.modifiers[0].value, 5);
    }

    #[test]
    fn test_extract_negative_modifier() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("-2 penalty to attack");
        assert_eq!(result.modifiers.len(), 1);
        assert_eq!(result.modifiers[0].value, -2);
    }

    #[test]
    fn test_extract_damage_modifier() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("+3 to damage");
        assert_eq!(result.modifiers.len(), 1);
        assert_eq!(result.modifiers[0].value, 3);
        assert!(result.modifiers[0].context.contains("damage"));
    }

    #[test]
    fn test_extract_bonus_modifier() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("+2 bonus");
        assert_eq!(result.modifiers.len(), 1);
        assert_eq!(result.modifiers[0].value, 2);
    }

    // ========================================================================
    // DiceExtractor - Complex Expression Tests
    // ========================================================================

    #[test]
    fn test_complex_expression_multiple_dice() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("2d6 + 1d8 + 4 fire damage");
        assert_eq!(result.expressions.len(), 2);
    }

    #[test]
    fn test_real_world_stat_block() {
        let extractor = DiceExtractor::new();

        let text = r#"
            Melee Weapon Attack: +5 to hit, reach 5 ft., one target.
            Hit: 10 (2d6 + 3) slashing damage.
            The target must succeed on a DC 14 Constitution saving throw
            or take an additional 2d6 poison damage.
        "#;

        let result = extractor.extract(text);

        // Should find: 2d6, 2d6
        assert!(result.expressions.len() >= 2);

        // Should find: DC 14
        assert_eq!(result.difficulty_checks.len(), 1);
        assert_eq!(result.difficulty_checks[0].dc_value, 14);

        // Should find: +5 to hit
        assert!(!result.modifiers.is_empty());
    }

    #[test]
    fn test_damage_with_types() {
        let extractor = DiceExtractor::new();

        let text = "2d6 fire damage plus 1d8 cold damage";
        let result = extractor.extract(text);

        assert_eq!(result.expressions.len(), 2);
    }

    // ========================================================================
    // DiceExtractor - Helper Method Tests
    // ========================================================================

    #[test]
    fn test_has_dice_notation() {
        let extractor = DiceExtractor::new();

        assert!(extractor.has_dice_notation("Roll 2d6 for damage"));
        assert!(extractor.has_dice_notation("On a d20 roll"));
        assert!(!extractor.has_dice_notation("No dice here"));
    }

    #[test]
    fn test_count_dice_expressions() {
        let extractor = DiceExtractor::new();

        assert_eq!(extractor.count_dice_expressions("Roll 2d6 + 1d4"), 2);
        assert_eq!(extractor.count_dice_expressions("No dice"), 0);
        assert_eq!(extractor.count_dice_expressions("d20, d6, d8"), 3);
    }

    #[test]
    fn test_parse_expression() {
        let extractor = DiceExtractor::new();

        let expr = extractor.parse_expression("2d6+3").unwrap();
        assert_eq!(expr.count, 2);
        assert_eq!(expr.sides, 6);
        assert_eq!(expr.modifier, 3);
    }

    #[test]
    fn test_parse_expression_invalid() {
        let extractor = DiceExtractor::new();

        assert!(extractor.parse_expression("no dice").is_none());
    }

    #[test]
    fn test_combined_stats() {
        let extractor = DiceExtractor::new();

        let expressions = vec![
            DiceExpression::new(2, 6, 0, "2d6".to_string()),
            DiceExpression::new(1, 4, 2, "1d4+2".to_string()),
        ];

        let (min, max, avg) = extractor.combined_stats(&expressions);

        // 2d6: min=2, max=12, avg=7
        // 1d4+2: min=3, max=6, avg=4.5
        assert_eq!(min, 5);
        assert_eq!(max, 18);
        assert!((avg - 11.5).abs() < 0.01);
    }

    // ========================================================================
    // DiceExtractionResult Tests
    // ========================================================================

    #[test]
    fn test_extraction_result_is_empty() {
        let result = DiceExtractionResult::default();
        assert!(result.is_empty());
    }

    #[test]
    fn test_extraction_result_total_count() {
        let mut result = DiceExtractionResult::default();
        result.expressions.push(DiceExpression::new(1, 20, 0, "d20".into()));
        result.difficulty_checks.push(DifficultyCheck::new(15, None, "DC 15".into()));

        assert_eq!(result.total_count(), 2);
    }

    #[test]
    fn test_unique_die_types() {
        let mut result = DiceExtractionResult::default();
        result.expressions.push(DiceExpression::new(2, 6, 0, "2d6".into()));
        result.expressions.push(DiceExpression::new(1, 6, 0, "1d6".into()));
        result.expressions.push(DiceExpression::new(1, 8, 0, "1d8".into()));

        let types = result.unique_die_types();
        assert_eq!(types, vec![6, 8]);
    }

    #[test]
    fn test_is_combat_related() {
        let extractor = DiceExtractor::new();

        // Combat text with d20 and saves
        let combat_text = "Roll d20 to hit, DC 15 Dexterity save";
        let result = extractor.extract(combat_text);
        assert!(result.is_combat_related());

        // Non-combat text
        let non_combat = "Roll 2d6 for random encounter";
        let result2 = extractor.extract(non_combat);
        assert!(!result2.is_combat_related());
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_dice_with_spaces() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("2d6 + 3");
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].modifier, 3);
    }

    #[test]
    fn test_unicode_dashes() {
        let extractor = DiceExtractor::new();

        // Test various dash types: regular minus, en-dash, em-dash
        let result1 = extractor.extract("1d4-1");  // Regular minus
        assert_eq!(result1.expressions[0].modifier, -1);

        let result2 = extractor.extract("1d4–1");  // En-dash
        assert_eq!(result2.expressions[0].modifier, -1);

        let result3 = extractor.extract("1d4−1");  // Unicode minus
        assert_eq!(result3.expressions[0].modifier, -1);
    }

    #[test]
    fn test_case_insensitivity() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("DC 15 WISDOM SAVING THROW");
        assert_eq!(result.difficulty_checks.len(), 1);
        assert!(result.difficulty_checks[0]
            .check_type
            .as_ref()
            .unwrap()
            .to_lowercase()
            .contains("wisdom"));
    }

    #[test]
    fn test_empty_input() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_no_dice() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("The dragon breathes fire at you!");
        assert!(result.expressions.is_empty());
        assert!(result.difficulty_checks.is_empty());
    }

    #[test]
    fn test_large_dice_values() {
        let extractor = DiceExtractor::new();

        let result = extractor.extract("10d10+100 damage");
        assert_eq!(result.expressions.len(), 1);
        assert_eq!(result.expressions[0].count, 10);
        assert_eq!(result.expressions[0].sides, 10);
        assert_eq!(result.expressions[0].modifier, 100);
    }
}
