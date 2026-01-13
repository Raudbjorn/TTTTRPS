//! Random Table Parsing Module
//!
//! Parses random/roll tables with dice notation and probability distributions.
//! Handles various formats including d4, d6, d8, d10, d12, d20, d100, and 2d6 tables.

use regex::Regex;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Parsed random table data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomTableData {
    /// Optional table title
    pub title: Option<String>,
    /// Dice notation (d6, d20, 2d6, d100, etc.)
    pub dice_notation: String,
    /// Table entries with roll ranges and results
    pub entries: Vec<TableEntry>,
    /// Total number of possible outcomes
    pub total_outcomes: u32,
}

impl RandomTableData {
    /// Create a new random table.
    pub fn new(dice_notation: String) -> Self {
        Self {
            title: None,
            dice_notation,
            entries: Vec::new(),
            total_outcomes: 0,
        }
    }

    /// Calculate probability for a specific entry.
    pub fn probability(&self, entry: &TableEntry) -> f32 {
        if self.total_outcomes == 0 {
            return 0.0;
        }
        let range_size = entry.roll_max - entry.roll_min + 1;
        range_size as f32 / self.total_outcomes as f32
    }
}

/// A single entry in a random table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableEntry {
    /// Minimum roll value for this result
    pub roll_min: u32,
    /// Maximum roll value for this result
    pub roll_max: u32,
    /// The result text
    pub result: String,
    /// Pre-calculated probability (if available)
    pub probability: Option<f32>,
}

impl TableEntry {
    /// Create a new table entry.
    pub fn new(roll_min: u32, roll_max: u32, result: String) -> Self {
        Self {
            roll_min,
            roll_max,
            result,
            probability: None,
        }
    }

    /// Check if a roll falls within this entry's range.
    pub fn matches_roll(&self, roll: u32) -> bool {
        roll >= self.roll_min && roll <= self.roll_max
    }
}

// ============================================================================
// Parser
// ============================================================================

/// Parses random table text into structured data.
pub struct RandomTableParser {
    /// Pattern for dice notation
    dice_pattern: Regex,
    /// Pattern for roll ranges
    range_pattern: Regex,
    /// Pattern for single rolls
    single_pattern: Regex,
    /// Pattern for d100 percentile ranges
    percentile_pattern: Regex,
}

impl Default for RandomTableParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RandomTableParser {
    /// Create a new random table parser.
    pub fn new() -> Self {
        Self {
            dice_pattern: Regex::new(r"(?i)(?:^|\s|\()(\d*)d(\d+|%)(?:\s|$|\)|[^a-zA-Z0-9])").unwrap(),
            range_pattern: Regex::new(r"^(\d+)[–\-−](\d+)\s*[:\|]?\s*(.+)").unwrap(),
            single_pattern: Regex::new(r"^(\d+)\s*[:\|]?\s*(.+)").unwrap(),
            percentile_pattern: Regex::new(r"^(0?\d{1,2})[–\-−](0?\d{1,2})\s*[:\|]?\s*(.+)").unwrap(),
        }
    }

    /// Parse random table text into structured data.
    ///
    /// # Arguments
    /// * `text` - The table text
    ///
    /// # Returns
    /// * `Option<RandomTableData>` - Parsed data or None if not a valid table
    pub fn parse(&self, text: &str) -> Option<RandomTableData> {
        // Find dice notation
        let dice_notation = self.detect_dice_notation(text)?;

        let mut table = RandomTableData::new(dice_notation.clone());

        // Calculate total outcomes
        table.total_outcomes = self.calculate_outcomes(&dice_notation);

        // Try to extract title
        table.title = self.extract_title(text);

        // Parse entries
        table.entries = self.parse_entries(text);

        // Calculate probabilities
        let total_outcomes = table.total_outcomes;
        for entry in &mut table.entries {
            if total_outcomes > 0 {
                let range_size = entry.roll_max - entry.roll_min + 1;
                entry.probability = Some(range_size as f32 / total_outcomes as f32);
            } else {
                entry.probability = Some(0.0);
            }
        }

        // Need at least 2 entries to be a valid table
        if table.entries.len() >= 2 {
            Some(table)
        } else {
            None
        }
    }

    /// Detect the dice notation used in the table.
    pub fn detect_dice_notation(&self, text: &str) -> Option<String> {
        self.dice_pattern
            .captures(text)
            .map(|caps| {
                let count = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let sides = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                format!("{}d{}", count, sides)
            })
    }

    /// Calculate total possible outcomes for a dice notation.
    fn calculate_outcomes(&self, notation: &str) -> u32 {
        if let Some(caps) = self.dice_pattern.captures(notation) {
            let count = caps.get(1)
                .map(|m| m.as_str())
                .filter(|s| !s.is_empty())
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(1);

            let sides = caps.get(2)
                .map(|m| m.as_str())
                .and_then(|s| {
                    if s == "%" { Some(100) }
                    else { s.parse::<u32>().ok() }
                })
                .unwrap_or(6);

            // For single dice, outcomes = sides
            // For multiple dice (e.g., 2d6), this is approximate (actual range is 2-12)
            if count == 1 {
                sides
            } else {
                // For multiple dice, return the range size (e.g., 2d6 = 11 outcomes: 2-12)
                count * sides - count + 1
            }
        } else {
            0
        }
    }

    /// Extract table title from text.
    fn extract_title(&self, text: &str) -> Option<String> {
        let lines: Vec<&str> = text.lines().take(3).collect();

        for line in lines {
            let line = line.trim();
            // Look for table title patterns
            if line.to_lowercase().starts_with("table")
                || (line.len() < 80 && !self.range_pattern.is_match(line) && !self.single_pattern.is_match(line))
            {
                if !line.is_empty() && line.chars().any(|c| c.is_alphabetic()) {
                    return Some(line.to_string());
                }
            }
        }
        None
    }

    /// Parse table entries from text.
    fn parse_entries(&self, text: &str) -> Vec<TableEntry> {
        let mut entries = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Try range pattern (e.g., "1-3: Goblin")
            if let Some(caps) = self.range_pattern.captures(line) {
                if let (Ok(min), Ok(max)) = (
                    caps.get(1).unwrap().as_str().parse::<u32>(),
                    caps.get(2).unwrap().as_str().parse::<u32>(),
                ) {
                    let result = caps.get(3).unwrap().as_str().trim().to_string();
                    entries.push(TableEntry::new(min, max, result));
                    continue;
                }
            }

            // Try percentile pattern (e.g., "01-65: Common")
            if let Some(caps) = self.percentile_pattern.captures(line) {
                if let (Ok(min), Ok(max)) = (
                    caps.get(1).unwrap().as_str().parse::<u32>(),
                    caps.get(2).unwrap().as_str().parse::<u32>(),
                ) {
                    let result = caps.get(3).unwrap().as_str().trim().to_string();
                    entries.push(TableEntry::new(min, max, result));
                    continue;
                }
            }

            // Try single value pattern (e.g., "1: Goblin")
            if let Some(caps) = self.single_pattern.captures(line) {
                if let Ok(val) = caps.get(1).unwrap().as_str().parse::<u32>() {
                    let result = caps.get(2).unwrap().as_str().trim().to_string();
                    entries.push(TableEntry::new(val, val, result));
                }
            }
        }

        entries
    }

    /// Check if text likely contains a random table.
    pub fn is_random_table(&self, text: &str) -> bool {
        // Must have dice notation
        if self.dice_pattern.find(text).is_none() {
            return false;
        }

        // Count potential table rows
        let row_count = text.lines()
            .filter(|l| {
                let l = l.trim();
                self.range_pattern.is_match(l) || self.single_pattern.is_match(l)
            })
            .count();

        row_count >= 2
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_dice_notation() {
        let parser = RandomTableParser::new();

        assert_eq!(parser.detect_dice_notation("Roll d6"), Some("d6".to_string()));
        assert_eq!(parser.detect_dice_notation("2d6 result"), Some("2d6".to_string()));
        assert_eq!(parser.detect_dice_notation("d20 table"), Some("d20".to_string()));
        assert_eq!(parser.detect_dice_notation("d%"), Some("d%".to_string()));
        assert!(parser.detect_dice_notation("no dice here").is_none());
    }

    #[test]
    fn test_calculate_outcomes() {
        let parser = RandomTableParser::new();

        assert_eq!(parser.calculate_outcomes("d6"), 6);
        assert_eq!(parser.calculate_outcomes("d20"), 20);
        assert_eq!(parser.calculate_outcomes("d%"), 100);
        assert_eq!(parser.calculate_outcomes("2d6"), 11); // 2-12
    }

    #[test]
    fn test_parse_d6_table() {
        let parser = RandomTableParser::new();

        let table_text = r#"
            Random Encounter (d6)
            1: Goblins
            2: Orcs
            3: Wolves
            4-5: Nothing
            6: Dragon
        "#;

        let result = parser.parse(table_text);
        assert!(result.is_some());

        let table = result.unwrap();
        assert_eq!(table.dice_notation, "d6");
        assert_eq!(table.entries.len(), 5);
        assert_eq!(table.entries[0].roll_min, 1);
        assert_eq!(table.entries[0].roll_max, 1);
        assert_eq!(table.entries[3].roll_min, 4);
        assert_eq!(table.entries[3].roll_max, 5);
    }

    #[test]
    fn test_table_entry_matches_roll() {
        let entry = TableEntry::new(4, 6, "Result".to_string());

        assert!(!entry.matches_roll(3));
        assert!(entry.matches_roll(4));
        assert!(entry.matches_roll(5));
        assert!(entry.matches_roll(6));
        assert!(!entry.matches_roll(7));
    }

    #[test]
    fn test_probability_calculation() {
        let mut table = RandomTableData::new("d6".to_string());
        table.total_outcomes = 6;

        let entry = TableEntry::new(1, 2, "Result".to_string());
        let prob = table.probability(&entry);

        // 2 out of 6 outcomes
        assert!((prob - 0.333).abs() < 0.01);
    }
}
