//! Table Extraction Module
//!
//! Extracts table structure from PDF content with support for:
//! - Multi-page table continuation detection
//! - Header row detection
//! - Cell boundary inference
//!
//! Designed for TTRPG content where random tables and reference tables
//! often span multiple pages.

use regex::Regex;
use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// An extracted table with structure information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedTable {
    /// Optional title of the table
    pub title: Option<String>,
    /// Header row cells
    pub headers: Vec<String>,
    /// Data rows (each row is a vector of cell strings)
    pub rows: Vec<Vec<String>>,
    /// Page numbers where this table appears
    pub page_numbers: Vec<u32>,
    /// Whether this table is a continuation from a previous page
    pub is_continuation: bool,
    /// Table type hint (if detected)
    pub table_type: Option<TableType>,
}

impl ExtractedTable {
    /// Create a new empty table.
    pub fn new() -> Self {
        Self {
            title: None,
            headers: Vec::new(),
            rows: Vec::new(),
            page_numbers: Vec::new(),
            is_continuation: false,
            table_type: None,
        }
    }

    /// Create a table with a title.
    pub fn with_title(title: &str) -> Self {
        Self {
            title: Some(title.to_string()),
            ..Self::new()
        }
    }

    /// Add a page number to this table.
    pub fn add_page(&mut self, page: u32) {
        if !self.page_numbers.contains(&page) {
            self.page_numbers.push(page);
        }
    }

    /// Get the number of columns in this table.
    pub fn column_count(&self) -> usize {
        if !self.headers.is_empty() {
            self.headers.len()
        } else {
            self.rows.first().map(|r| r.len()).unwrap_or(0)
        }
    }

    /// Get the number of data rows (excluding headers).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Check if this table spans multiple pages.
    pub fn is_multi_page(&self) -> bool {
        self.page_numbers.len() > 1
    }
}

impl Default for ExtractedTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Type of table detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableType {
    /// Random/roll table (d4, d6, d20, etc.)
    Random,
    /// Reference/lookup table
    Reference,
    /// Price/cost table
    Price,
    /// Stat/attribute table
    Stats,
    /// Unknown/generic table
    Generic,
}

impl TableType {
    /// Get a human-readable name for this table type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Random => "random",
            Self::Reference => "reference",
            Self::Price => "price",
            Self::Stats => "stats",
            Self::Generic => "generic",
        }
    }
}

// ============================================================================
// Table Extractor
// ============================================================================

/// Extracts table structure from PDF content.
pub struct TableExtractor {
    /// Patterns that indicate table continuation
    continuation_patterns: Vec<Regex>,
    /// Patterns that indicate a table title
    title_patterns: Vec<Regex>,
    /// Patterns that indicate a random/roll table
    dice_patterns: Vec<Regex>,
}

impl Default for TableExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl TableExtractor {
    /// Create a new table extractor.
    pub fn new() -> Self {
        Self {
            continuation_patterns: vec![
                Regex::new(r"(?i)^\s*\(continued\)").unwrap(),
                Regex::new(r"(?i)table\s+\d+\s*\(cont").unwrap(),
                Regex::new(r"(?i)continued\s+from\s+page").unwrap(),
                Regex::new(r"(?i)\.{3,}\s*$").unwrap(), // Trailing ellipsis
            ],
            title_patterns: vec![
                Regex::new(r"(?i)^table\s+\d+[.:\s]").unwrap(),
                Regex::new(r"(?i)^table[.:\s]").unwrap(),
                Regex::new(r"(?i)\btable\b").unwrap(),
            ],
            dice_patterns: vec![
                Regex::new(r"(?i)\b(\d*)d(\d+)\b").unwrap(), // d6, 2d6, d20, etc.
                Regex::new(r"(?i)\bd%\b").unwrap(),          // d100 percentile
            ],
        }
    }

    /// Check if text indicates a table continuation from a previous page.
    pub fn is_table_continuation(&self, text: &str) -> bool {
        self.continuation_patterns.iter().any(|re| re.is_match(text))
    }

    /// Detect if text contains a random/roll table.
    pub fn detect_dice_notation(&self, text: &str) -> Option<String> {
        for pattern in &self.dice_patterns {
            if let Some(captures) = pattern.captures(text) {
                return captures.get(0).map(|m| m.as_str().to_string());
            }
        }
        None
    }

    /// Detect the type of table from its content.
    pub fn detect_table_type(&self, table: &ExtractedTable) -> TableType {
        let all_text = format!(
            "{} {} {}",
            table.title.as_deref().unwrap_or(""),
            table.headers.join(" "),
            table.rows.iter().flatten().cloned().collect::<Vec<_>>().join(" ")
        ).to_lowercase();

        // Check for dice notation (random table)
        if self.dice_patterns.iter().any(|p| p.is_match(&all_text)) {
            return TableType::Random;
        }

        // Check for price indicators
        if all_text.contains("gp") || all_text.contains("sp") || all_text.contains("cp")
            || all_text.contains("gold") || all_text.contains("cost") || all_text.contains("price")
        {
            return TableType::Price;
        }

        // Check for stat indicators
        if all_text.contains("str") || all_text.contains("dex") || all_text.contains("con")
            || all_text.contains("bonus") || all_text.contains("modifier")
        {
            return TableType::Stats;
        }

        TableType::Generic
    }

    /// Merge continuation tables from multiple pages.
    ///
    /// Takes a vector of tables (potentially with continuations) and merges
    /// tables that span multiple pages into single logical tables.
    pub fn merge_continuation_tables(&self, tables: Vec<ExtractedTable>) -> Vec<ExtractedTable> {
        let mut result: Vec<ExtractedTable> = Vec::new();
        let mut pending_merge: Option<ExtractedTable> = None;

        for table in tables {
            if table.is_continuation {
                if let Some(ref mut base) = pending_merge {
                    // Append rows to existing table
                    base.rows.extend(table.rows);
                    for page in table.page_numbers {
                        base.add_page(page);
                    }
                } else {
                    // Orphan continuation - keep as-is
                    result.push(table);
                }
            } else {
                // Flush pending merge if any
                if let Some(merged) = pending_merge.take() {
                    result.push(merged);
                }
                pending_merge = Some(table);
            }
        }

        // Flush final pending table
        if let Some(merged) = pending_merge {
            result.push(merged);
        }

        result
    }

    /// Parse a simple text table (tab or pipe delimited).
    ///
    /// # Arguments
    /// * `text` - The table text content
    /// * `page_number` - The page number for attribution
    /// * `delimiter` - The column delimiter (None = auto-detect)
    pub fn parse_simple_table(
        &self,
        text: &str,
        page_number: u32,
        delimiter: Option<&str>,
    ) -> Option<ExtractedTable> {
        let lines: Vec<&str> = text.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.is_empty() {
            return None;
        }

        // Auto-detect delimiter if not specified
        // Look through lines to find one that contains delimiters (skip title-only lines)
        let delim = delimiter.unwrap_or_else(|| {
            for line in &lines {
                if line.contains('|') {
                    return "|";
                }
                if line.contains('\t') {
                    return "\t";
                }
            }
            "  " // Double-space as fallback
        });

        let mut table = ExtractedTable::new();
        table.add_page(page_number);
        table.is_continuation = self.is_table_continuation(text);

        // Check for title in first line
        let mut skip_first_line = false;
        if let Some(title_match) = self.title_patterns.iter()
            .find_map(|p| p.find(lines[0]))
        {
            table.title = Some(title_match.as_str().trim().to_string());
            // Skip title line if it doesn't look like a data row (no delimiter)
            if !lines[0].contains(delim) {
                skip_first_line = true;
            }
        }

        // Parse rows
        let mut is_first_data_row = true;
        for (i, line) in lines.iter().enumerate() {
            // Skip title line if detected
            if i == 0 && skip_first_line {
                continue;
            }
            let line = *line;
            // Skip separator lines (------)
            if line.chars().all(|c| c == '-' || c == '=' || c == '+' || c == '|') {
                continue;
            }

            let cells: Vec<String> = line
                .split(delim)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if cells.is_empty() {
                continue;
            }

            if is_first_data_row && table.headers.is_empty() {
                // First row is likely headers
                table.headers = cells;
                is_first_data_row = false;
            } else {
                table.rows.push(cells);
            }
        }

        // Detect dice notation for random tables
        if let Some(dice) = self.detect_dice_notation(text) {
            table.title = Some(format!(
                "{} ({})",
                table.title.as_deref().unwrap_or("Random Table"),
                dice
            ));
            table.table_type = Some(TableType::Random);
        } else {
            table.table_type = Some(self.detect_table_type(&table));
        }

        if table.headers.is_empty() && table.rows.is_empty() {
            None
        } else {
            Some(table)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_continuation_detection() {
        let extractor = TableExtractor::new();

        assert!(extractor.is_table_continuation("(Continued)"));
        assert!(extractor.is_table_continuation("Table 1 (cont.)"));
        assert!(extractor.is_table_continuation("Continued from page 42"));
        assert!(!extractor.is_table_continuation("Normal table content"));
    }

    #[test]
    fn test_dice_notation_detection() {
        let extractor = TableExtractor::new();

        assert_eq!(extractor.detect_dice_notation("Roll d6"), Some("d6".to_string()));
        assert_eq!(extractor.detect_dice_notation("Roll 2d6"), Some("2d6".to_string()));
        assert_eq!(extractor.detect_dice_notation("d20 result"), Some("d20".to_string()));
        assert!(extractor.detect_dice_notation("No dice here").is_none());
    }

    #[test]
    fn test_parse_simple_table() {
        let extractor = TableExtractor::new();

        let table_text = r#"
            Table 1: Random Encounters
            d6 | Encounter
            1  | Goblins
            2  | Orcs
            3  | Wolves
            4  | Bandits
            5  | Nothing
            6  | Dragon!
        "#;

        let table = extractor.parse_simple_table(table_text, 1, None);
        assert!(table.is_some());

        let table = table.unwrap();
        assert_eq!(table.headers.len(), 2);
        assert!(table.rows.len() >= 5);
        assert_eq!(table.table_type, Some(TableType::Random));
    }

    #[test]
    fn test_merge_continuation_tables() {
        let extractor = TableExtractor::new();

        let mut table1 = ExtractedTable::with_title("Test Table");
        table1.add_page(1);
        table1.headers = vec!["Col1".to_string(), "Col2".to_string()];
        table1.rows = vec![vec!["A".to_string(), "B".to_string()]];

        let mut table2 = ExtractedTable::new();
        table2.is_continuation = true;
        table2.add_page(2);
        table2.rows = vec![vec!["C".to_string(), "D".to_string()]];

        let merged = extractor.merge_continuation_tables(vec![table1, table2]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].rows.len(), 2);
        assert_eq!(merged[0].page_numbers.len(), 2);
    }

    #[test]
    fn test_table_type_detection() {
        let extractor = TableExtractor::new();

        let mut random_table = ExtractedTable::new();
        random_table.headers = vec!["d6".to_string(), "Result".to_string()];
        assert_eq!(extractor.detect_table_type(&random_table), TableType::Random);

        let mut price_table = ExtractedTable::new();
        price_table.headers = vec!["Item".to_string(), "Cost (gp)".to_string()];
        assert_eq!(extractor.detect_table_type(&price_table), TableType::Price);
    }
}
