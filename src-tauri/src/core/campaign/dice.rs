//! Dice Notation Parser
//!
//! Phase 8 of the Campaign Generation Overhaul.
//!
//! Parses standard TTRPG dice notation including:
//! - Standard dice: d4, d6, d8, d10, d12, d20, d100
//! - Compound dice: 2d6, 3d8+5, 4d6-2
//! - Percentile: d100, d%
//! - D66 tables: d66 (read as tens/ones)
//! - Fudge/Fate dice: dF (future extension)
//!
//! ## Examples
//!
//! ```rust,ignore
//! use crate::core::campaign::dice::{DiceNotation, DiceRoller};
//!
//! let notation = DiceNotation::parse("2d6+3")?;
//! assert_eq!(notation.count, 2);
//! assert_eq!(notation.sides, 6);
//! assert_eq!(notation.modifier, 3);
//!
//! let roller = DiceRoller::new();
//! let result = roller.roll(&notation);
//! assert!(result.total >= 5 && result.total <= 15);
//! ```

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during dice notation parsing or rolling
#[derive(Debug, Error)]
pub enum DiceError {
    #[error("Invalid dice notation: {0}")]
    InvalidNotation(String),

    #[error("Invalid dice count: must be between 1 and {max}, got {got}")]
    InvalidCount { max: u32, got: u32 },

    #[error("Invalid dice sides: must be greater than 0, got {0}")]
    InvalidSides(u32),

    #[error("Modifier overflow: result would exceed i32 bounds")]
    ModifierOverflow,

    #[error("Empty notation")]
    EmptyNotation,
}

/// Result type for dice operations
pub type DiceResult<T> = Result<T, DiceError>;

// ============================================================================
// Dice Notation Types
// ============================================================================

/// Standard TTRPG dice types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiceType {
    D4,
    D6,
    D8,
    D10,
    D12,
    D20,
    D100,
    /// D66 - roll d6 twice, read as tens and ones (11-66)
    D66,
    /// Custom sided die
    Custom(u32),
}

impl DiceType {
    /// Get the number of sides for this die type
    pub fn sides(&self) -> u32 {
        match self {
            DiceType::D4 => 4,
            DiceType::D6 => 6,
            DiceType::D8 => 8,
            DiceType::D10 => 10,
            DiceType::D12 => 12,
            DiceType::D20 => 20,
            DiceType::D100 => 100,
            DiceType::D66 => 66, // Special handling required
            DiceType::Custom(sides) => *sides,
        }
    }

    /// Parse from sides number
    pub fn from_sides(sides: u32) -> Self {
        match sides {
            4 => DiceType::D4,
            6 => DiceType::D6,
            8 => DiceType::D8,
            10 => DiceType::D10,
            12 => DiceType::D12,
            20 => DiceType::D20,
            100 => DiceType::D100,
            66 => DiceType::D66,
            n => DiceType::Custom(n),
        }
    }

    /// Check if this is a standard TTRPG die
    pub fn is_standard(&self) -> bool {
        matches!(
            self,
            DiceType::D4
                | DiceType::D6
                | DiceType::D8
                | DiceType::D10
                | DiceType::D12
                | DiceType::D20
                | DiceType::D100
        )
    }
}

impl fmt::Display for DiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiceType::D66 => write!(f, "d66"),
            DiceType::Custom(n) => write!(f, "d{}", n),
            other => write!(f, "d{}", other.sides()),
        }
    }
}

/// Parsed dice notation with count, sides, and modifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiceNotation {
    /// Number of dice to roll
    pub count: u32,
    /// Type of die
    pub dice_type: DiceType,
    /// Modifier to add/subtract after rolling
    pub modifier: i32,
    /// Original notation string
    pub original: String,
}

impl DiceNotation {
    /// Maximum number of dice allowed in a single roll
    pub const MAX_DICE_COUNT: u32 = 100;

    /// Create a new dice notation
    pub fn new(count: u32, dice_type: DiceType, modifier: i32) -> DiceResult<Self> {
        if count == 0 || count > Self::MAX_DICE_COUNT {
            return Err(DiceError::InvalidCount {
                max: Self::MAX_DICE_COUNT,
                got: count,
            });
        }

        let notation = if modifier == 0 {
            format!("{}{}", count, dice_type)
        } else if modifier > 0 {
            format!("{}{}+{}", count, dice_type, modifier)
        } else {
            format!("{}{}{}", count, dice_type, modifier)
        };

        Ok(Self {
            count,
            dice_type,
            modifier,
            original: notation,
        })
    }

    /// Parse a dice notation string
    ///
    /// Supported formats:
    /// - "d20" -> 1d20
    /// - "2d6" -> 2d6
    /// - "3d8+5" -> 3d8+5
    /// - "d20-2" -> 1d20-2
    /// - "d%" or "d100" -> 1d100
    /// - "d66" -> d66 (special handling)
    pub fn parse(notation: &str) -> DiceResult<Self> {
        let notation = notation.trim().to_lowercase();

        if notation.is_empty() {
            return Err(DiceError::EmptyNotation);
        }

        // Handle d% as d100
        let notation = notation.replace("d%", "d100");

        // Find the 'd' separator
        let d_pos = notation
            .find('d')
            .ok_or_else(|| DiceError::InvalidNotation(notation.clone()))?;

        // Parse count (default to 1 if not specified)
        let count_str = &notation[..d_pos];
        let count: u32 = if count_str.is_empty() {
            1
        } else {
            count_str
                .parse()
                .map_err(|_| DiceError::InvalidNotation(notation.clone()))?
        };

        if count == 0 || count > Self::MAX_DICE_COUNT {
            return Err(DiceError::InvalidCount {
                max: Self::MAX_DICE_COUNT,
                got: count,
            });
        }

        // Parse sides and modifier
        let rest = &notation[d_pos + 1..];

        // Find modifier position
        let (sides_str, modifier) = if let Some(pos) = rest.find('+') {
            let sides = &rest[..pos];
            let mod_str = &rest[pos + 1..];
            let modifier: i32 = mod_str
                .parse()
                .map_err(|_| DiceError::InvalidNotation(notation.clone()))?;
            (sides, modifier)
        } else if let Some(pos) = rest.rfind('-') {
            // Use rfind to handle negative modifier correctly
            if pos == 0 {
                // No modifier, just sides
                (rest, 0)
            } else {
                let sides = &rest[..pos];
                let mod_str = &rest[pos..]; // Include the minus sign
                let modifier: i32 = mod_str
                    .parse()
                    .map_err(|_| DiceError::InvalidNotation(notation.clone()))?;
                (sides, modifier)
            }
        } else {
            (rest, 0)
        };

        let sides: u32 = sides_str
            .parse()
            .map_err(|_| DiceError::InvalidNotation(notation.clone()))?;

        if sides == 0 {
            return Err(DiceError::InvalidSides(sides));
        }

        let dice_type = DiceType::from_sides(sides);

        Ok(Self {
            count,
            dice_type,
            modifier,
            original: notation,
        })
    }

    /// Get the minimum possible result
    pub fn min_result(&self) -> i32 {
        if matches!(self.dice_type, DiceType::D66) {
            11 + self.modifier
        } else {
            (self.count as i32) + self.modifier
        }
    }

    /// Get the maximum possible result
    pub fn max_result(&self) -> i32 {
        if matches!(self.dice_type, DiceType::D66) {
            66 + self.modifier
        } else {
            (self.count as i32 * self.dice_type.sides() as i32) + self.modifier
        }
    }

    /// Get the average expected result
    pub fn average_result(&self) -> f64 {
        if matches!(self.dice_type, DiceType::D66) {
            // D66 average is (11+66)/2 = 38.5
            38.5 + self.modifier as f64
        } else {
            let die_average = (1.0 + self.dice_type.sides() as f64) / 2.0;
            (self.count as f64 * die_average) + self.modifier as f64
        }
    }

    /// Get number of sides
    pub fn sides(&self) -> u32 {
        self.dice_type.sides()
    }
}

impl fmt::Display for DiceNotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.original)
    }
}

impl std::str::FromStr for DiceNotation {
    type Err = DiceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

// ============================================================================
// Roll Result Types
// ============================================================================

/// Result of a single die roll
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleRoll {
    /// The die type that was rolled
    pub die: DiceType,
    /// The result of the roll
    pub value: u32,
}

/// Complete result of a dice roll
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollResult {
    /// The notation that was rolled
    pub notation: DiceNotation,
    /// Individual die results
    pub rolls: Vec<SingleRoll>,
    /// Sum of all dice (before modifier)
    pub subtotal: i32,
    /// Final total (after modifier)
    pub total: i32,
    /// For d66: the tens digit
    pub d66_tens: Option<u32>,
    /// For d66: the ones digit
    pub d66_ones: Option<u32>,
}

impl RollResult {
    /// Check if this roll is a natural maximum (all dice showing max)
    pub fn is_natural_max(&self) -> bool {
        self.rolls
            .iter()
            .all(|r| r.value == r.die.sides())
    }

    /// Check if this roll is a natural minimum (all dice showing 1)
    pub fn is_natural_min(&self) -> bool {
        self.rolls.iter().all(|r| r.value == 1)
    }

    /// Check if this is a critical (for d20 rolls)
    pub fn is_critical(&self) -> bool {
        self.notation.count == 1
            && matches!(self.notation.dice_type, DiceType::D20)
            && self.subtotal == 20
    }

    /// Check if this is a critical failure (for d20 rolls)
    pub fn is_critical_fail(&self) -> bool {
        self.notation.count == 1
            && matches!(self.notation.dice_type, DiceType::D20)
            && self.subtotal == 1
    }
}

impl fmt::Display for RollResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let (Some(tens), Some(ones)) = (self.d66_tens, self.d66_ones) {
            write!(f, "d66: {}{} ({})", tens, ones, self.total)
        } else {
            let rolls_str: Vec<String> = self.rolls.iter().map(|r| r.value.to_string()).collect();
            if self.notation.modifier == 0 {
                write!(f, "{}: [{}] = {}", self.notation, rolls_str.join(", "), self.total)
            } else {
                write!(
                    f,
                    "{}: [{}] ({}) = {}",
                    self.notation,
                    rolls_str.join(", "),
                    self.subtotal,
                    self.total
                )
            }
        }
    }
}

// ============================================================================
// Dice Roller
// ============================================================================

/// Thread-safe dice roller with optional seeded RNG
pub struct DiceRoller {
    // Could be extended to support seeded RNG for reproducible tests
}

impl DiceRoller {
    /// Create a new dice roller
    pub fn new() -> Self {
        Self {}
    }

    /// Roll dice according to the notation
    pub fn roll(&self, notation: &DiceNotation) -> RollResult {
        let mut rng = rand::thread_rng();
        self.roll_with_rng(notation, &mut rng)
    }

    /// Roll dice with a specific RNG (useful for testing)
    pub fn roll_with_rng<R: Rng>(&self, notation: &DiceNotation, rng: &mut R) -> RollResult {
        if matches!(notation.dice_type, DiceType::D66) {
            self.roll_d66_with_rng(notation, rng)
        } else {
            self.roll_standard_with_rng(notation, rng)
        }
    }

    /// Roll standard dice
    fn roll_standard_with_rng<R: Rng>(&self, notation: &DiceNotation, rng: &mut R) -> RollResult {
        let sides = notation.dice_type.sides();
        let mut rolls = Vec::with_capacity(notation.count as usize);
        let mut subtotal: i32 = 0;

        for _ in 0..notation.count {
            let value = rng.gen_range(1..=sides);
            subtotal += value as i32;
            rolls.push(SingleRoll {
                die: notation.dice_type,
                value,
            });
        }

        let total = subtotal + notation.modifier;

        RollResult {
            notation: notation.clone(),
            rolls,
            subtotal,
            total,
            d66_tens: None,
            d66_ones: None,
        }
    }

    /// Roll d66 (two d6, read as tens/ones)
    fn roll_d66_with_rng<R: Rng>(&self, notation: &DiceNotation, rng: &mut R) -> RollResult {
        let tens = rng.gen_range(1..=6);
        let ones = rng.gen_range(1..=6);
        let subtotal = (tens * 10 + ones) as i32;
        let total = subtotal + notation.modifier;

        RollResult {
            notation: notation.clone(),
            rolls: vec![
                SingleRoll {
                    die: DiceType::D6,
                    value: tens,
                },
                SingleRoll {
                    die: DiceType::D6,
                    value: ones,
                },
            ],
            subtotal,
            total,
            d66_tens: Some(tens),
            d66_ones: Some(ones),
        }
    }

    /// Quick roll with notation string
    pub fn quick_roll(&self, notation: &str) -> DiceResult<RollResult> {
        let parsed = DiceNotation::parse(notation)?;
        Ok(self.roll(&parsed))
    }

    /// Roll with advantage (roll twice, take higher)
    pub fn roll_advantage(&self, notation: &DiceNotation) -> (RollResult, RollResult, RollResult) {
        let roll1 = self.roll(notation);
        let roll2 = self.roll(notation);
        let best = if roll1.total >= roll2.total {
            roll1.clone()
        } else {
            roll2.clone()
        };
        (roll1, roll2, best)
    }

    /// Roll with disadvantage (roll twice, take lower)
    pub fn roll_disadvantage(&self, notation: &DiceNotation) -> (RollResult, RollResult, RollResult) {
        let roll1 = self.roll(notation);
        let roll2 = self.roll(notation);
        let worst = if roll1.total <= roll2.total {
            roll1.clone()
        } else {
            roll2.clone()
        };
        (roll1, roll2, worst)
    }

    /// Generate a random value in range (for random tables)
    pub fn random_in_range(&self, min: i32, max: i32) -> i32 {
        let mut rng = rand::thread_rng();
        // Normalize range to prevent panic when min > max
        let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
        rng.gen_range(lo..=hi)
    }
}

impl Default for DiceRoller {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_d20() {
        let notation = DiceNotation::parse("d20").unwrap();
        assert_eq!(notation.count, 1);
        assert_eq!(notation.dice_type, DiceType::D20);
        assert_eq!(notation.modifier, 0);
    }

    #[test]
    fn test_parse_2d6() {
        let notation = DiceNotation::parse("2d6").unwrap();
        assert_eq!(notation.count, 2);
        assert_eq!(notation.dice_type, DiceType::D6);
        assert_eq!(notation.modifier, 0);
    }

    #[test]
    fn test_parse_3d8_plus_5() {
        let notation = DiceNotation::parse("3d8+5").unwrap();
        assert_eq!(notation.count, 3);
        assert_eq!(notation.dice_type, DiceType::D8);
        assert_eq!(notation.modifier, 5);
    }

    #[test]
    fn test_parse_d20_minus_2() {
        let notation = DiceNotation::parse("d20-2").unwrap();
        assert_eq!(notation.count, 1);
        assert_eq!(notation.dice_type, DiceType::D20);
        assert_eq!(notation.modifier, -2);
    }

    #[test]
    fn test_parse_d100() {
        let notation = DiceNotation::parse("d100").unwrap();
        assert_eq!(notation.count, 1);
        assert_eq!(notation.dice_type, DiceType::D100);
        assert_eq!(notation.sides(), 100);
    }

    #[test]
    fn test_parse_d_percent() {
        let notation = DiceNotation::parse("d%").unwrap();
        assert_eq!(notation.dice_type, DiceType::D100);
    }

    #[test]
    fn test_parse_d66() {
        let notation = DiceNotation::parse("d66").unwrap();
        assert_eq!(notation.dice_type, DiceType::D66);
        assert_eq!(notation.min_result(), 11);
        assert_eq!(notation.max_result(), 66);
    }

    #[test]
    fn test_parse_uppercase() {
        let notation = DiceNotation::parse("2D6+3").unwrap();
        assert_eq!(notation.count, 2);
        assert_eq!(notation.dice_type, DiceType::D6);
        assert_eq!(notation.modifier, 3);
    }

    #[test]
    fn test_parse_with_whitespace() {
        // Leading/trailing whitespace is trimmed
        let notation = DiceNotation::parse("  2d6  ").unwrap();
        assert_eq!(notation.count, 2);
        assert_eq!(notation.dice_type, DiceType::D6);
        // Whitespace in middle of notation (e.g., "2d6 + 3") is not supported
        // This would fail to parse
        assert!(DiceNotation::parse("2d6 + 3").is_err());
    }

    #[test]
    fn test_invalid_notation_no_d() {
        assert!(DiceNotation::parse("20").is_err());
    }

    #[test]
    fn test_invalid_notation_empty() {
        assert!(DiceNotation::parse("").is_err());
    }

    #[test]
    fn test_invalid_notation_zero_count() {
        assert!(DiceNotation::parse("0d6").is_err());
    }

    #[test]
    fn test_invalid_notation_too_many_dice() {
        assert!(DiceNotation::parse("1000d6").is_err());
    }

    #[test]
    fn test_min_max_results() {
        let notation = DiceNotation::parse("2d6+3").unwrap();
        assert_eq!(notation.min_result(), 5);  // 2 + 3
        assert_eq!(notation.max_result(), 15); // 12 + 3
    }

    #[test]
    fn test_average_result() {
        let notation = DiceNotation::parse("2d6").unwrap();
        assert!((notation.average_result() - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_roll_in_range() {
        let roller = DiceRoller::new();
        let notation = DiceNotation::parse("2d6+3").unwrap();

        for _ in 0..100 {
            let result = roller.roll(&notation);
            assert!(result.total >= notation.min_result());
            assert!(result.total <= notation.max_result());
        }
    }

    #[test]
    fn test_roll_d66_in_range() {
        let roller = DiceRoller::new();
        let notation = DiceNotation::parse("d66").unwrap();

        for _ in 0..100 {
            let result = roller.roll(&notation);
            assert!(result.total >= 11);
            assert!(result.total <= 66);
            assert!(result.d66_tens.is_some());
            assert!(result.d66_ones.is_some());
        }
    }

    #[test]
    fn test_quick_roll() {
        let roller = DiceRoller::new();
        let result = roller.quick_roll("d20+5").unwrap();
        assert!(result.total >= 6);
        assert!(result.total <= 25);
    }

    #[test]
    fn test_roll_result_display() {
        let roller = DiceRoller::new();
        let notation = DiceNotation::parse("2d6+3").unwrap();
        let result = roller.roll(&notation);
        let display = format!("{}", result);
        assert!(display.contains("2d6+3"));
    }

    #[test]
    fn test_critical_detection() {
        let notation = DiceNotation::new(1, DiceType::D20, 0).unwrap();
        let result = RollResult {
            notation: notation.clone(),
            rolls: vec![SingleRoll {
                die: DiceType::D20,
                value: 20,
            }],
            subtotal: 20,
            total: 20,
            d66_tens: None,
            d66_ones: None,
        };
        assert!(result.is_critical());
        assert!(!result.is_critical_fail());
    }

    #[test]
    fn test_dice_type_display() {
        assert_eq!(format!("{}", DiceType::D20), "d20");
        assert_eq!(format!("{}", DiceType::D66), "d66");
        assert_eq!(format!("{}", DiceType::Custom(30)), "d30");
    }

    #[test]
    fn test_notation_new() {
        let notation = DiceNotation::new(2, DiceType::D6, 3).unwrap();
        assert_eq!(notation.count, 2);
        assert_eq!(notation.dice_type, DiceType::D6);
        assert_eq!(notation.modifier, 3);
        assert_eq!(notation.original, "2d6+3");
    }

    #[test]
    fn test_notation_new_negative_modifier() {
        let notation = DiceNotation::new(1, DiceType::D20, -2).unwrap();
        assert_eq!(notation.original, "1d20-2");
    }

    #[test]
    fn test_from_str() {
        let notation: DiceNotation = "3d8+2".parse().unwrap();
        assert_eq!(notation.count, 3);
        assert_eq!(notation.dice_type, DiceType::D8);
        assert_eq!(notation.modifier, 2);
    }

    #[test]
    fn test_advantage_disadvantage() {
        let roller = DiceRoller::new();
        let notation = DiceNotation::parse("d20").unwrap();

        let (r1, r2, best) = roller.roll_advantage(&notation);
        assert!(best.total >= r1.total || best.total >= r2.total);
        assert!(best.total == r1.total.max(r2.total));

        let (r1, r2, worst) = roller.roll_disadvantage(&notation);
        assert!(worst.total <= r1.total || worst.total <= r2.total);
        assert!(worst.total == r1.total.min(r2.total));
    }

    #[test]
    fn test_random_in_range() {
        let roller = DiceRoller::new();
        for _ in 0..100 {
            let value = roller.random_in_range(1, 10);
            assert!(value >= 1 && value <= 10);
        }
    }
}
