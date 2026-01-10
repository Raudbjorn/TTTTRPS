//! Column Detection Module
//!
//! Detects multi-column layouts from text position data and reorders text
//! to logical reading order. Essential for processing two-column TTRPG
//! rulebook pages.

use serde::{Deserialize, Serialize};

// ============================================================================
// Configuration
// ============================================================================

/// Default minimum gap between columns (in points)
const DEFAULT_MIN_COLUMN_GAP: f32 = 20.0;

/// Default minimum column width to consider valid
const DEFAULT_MIN_COLUMN_WIDTH: f32 = 100.0;

// ============================================================================
// Types
// ============================================================================

/// Represents a detected column boundary in the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnBoundary {
    /// Left edge of the column (in points)
    pub left: f32,
    /// Right edge of the column (in points)
    pub right: f32,
    /// Top edge of the column (in points, 0 = top of page)
    pub top: f32,
    /// Bottom edge of the column (in points)
    pub bottom: f32,
}

impl ColumnBoundary {
    /// Create a full-height column boundary.
    pub fn new(left: f32, right: f32) -> Self {
        Self {
            left,
            right,
            top: 0.0,
            bottom: f32::MAX,
        }
    }

    /// Check if a point is within this column's horizontal bounds.
    pub fn contains_x(&self, x: f32) -> bool {
        x >= self.left && x < self.right
    }

    /// Calculate the width of this column.
    pub fn width(&self) -> f32 {
        self.right - self.left
    }
}

/// A text block with position information.
///
/// Represents a unit of text (word, line, or paragraph) with its
/// bounding box coordinates on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextBlock {
    /// The text content
    pub text: String,
    /// X position of the left edge (in points)
    pub x: f32,
    /// Y position from top of page (in points)
    pub y: f32,
    /// Width of the text block (in points)
    pub width: f32,
    /// Height of the text block (in points)
    pub height: f32,
}

impl TextBlock {
    /// Create a new text block.
    pub fn new(text: String, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { text, x, y, width, height }
    }

    /// Get the right edge of this text block.
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Get the bottom edge of this text block.
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Get the center X coordinate.
    pub fn center_x(&self) -> f32 {
        self.x + self.width / 2.0
    }
}

// ============================================================================
// Column Detector
// ============================================================================

/// Detects multi-column layouts from text position data.
///
/// Uses histogram analysis of X positions to find column gaps, then
/// reorders text blocks into logical reading order (left column first,
/// then right column, top to bottom within each column).
pub struct ColumnDetector {
    /// Minimum gap between columns to consider a column break
    min_column_gap: f32,
    /// Minimum column width to consider valid
    min_column_width: f32,
}

impl Default for ColumnDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ColumnDetector {
    /// Create a new column detector with default settings.
    pub fn new() -> Self {
        Self {
            min_column_gap: DEFAULT_MIN_COLUMN_GAP,
            min_column_width: DEFAULT_MIN_COLUMN_WIDTH,
        }
    }

    /// Create a column detector with custom gap and width thresholds.
    pub fn with_thresholds(min_gap: f32, min_width: f32) -> Self {
        Self {
            min_column_gap: min_gap,
            min_column_width: min_width,
        }
    }

    /// Detect columns from text blocks and reorder to logical reading order.
    ///
    /// For a two-column layout, this returns all text from the left column
    /// (top to bottom), followed by all text from the right column.
    ///
    /// # Arguments
    /// * `text_blocks` - Text blocks with position information
    /// * `page_width` - Width of the page in points
    ///
    /// # Returns
    /// Text blocks reordered in logical reading order
    pub fn reorder_text_by_columns(
        &self,
        text_blocks: &[TextBlock],
        page_width: f32,
    ) -> Vec<TextBlock> {
        if text_blocks.is_empty() {
            return Vec::new();
        }

        let columns = self.detect_column_boundaries(text_blocks, page_width);

        if columns.len() <= 1 {
            // Single column - return sorted by Y position (top to bottom)
            let mut sorted = text_blocks.to_vec();
            sorted.sort_by(|a, b| {
                a.y.partial_cmp(&b.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            return sorted;
        }

        // Multi-column: sort by column, then by Y within each column
        let mut result = Vec::with_capacity(text_blocks.len());

        for col in &columns {
            let mut col_blocks: Vec<_> = text_blocks
                .iter()
                .filter(|b| col.contains_x(b.x) || col.contains_x(b.center_x()))
                .cloned()
                .collect();

            col_blocks.sort_by(|a, b| {
                a.y.partial_cmp(&b.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            result.extend(col_blocks);
        }

        result
    }

    /// Detect column boundaries from text block positions.
    ///
    /// Uses histogram analysis to find gaps in X positions that indicate
    /// column breaks.
    pub fn detect_column_boundaries(
        &self,
        blocks: &[TextBlock],
        page_width: f32,
    ) -> Vec<ColumnBoundary> {
        if blocks.is_empty() {
            return vec![ColumnBoundary::new(0.0, page_width)];
        }

        // Collect all X positions (left edges of text blocks)
        let mut x_positions: Vec<f32> = blocks.iter().map(|b| b.x).collect();
        x_positions.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        x_positions.dedup_by(|a, b| (*a - *b).abs() < 1.0); // Remove near-duplicates

        if x_positions.is_empty() {
            return vec![ColumnBoundary::new(0.0, page_width)];
        }

        // Find gaps larger than min_column_gap between consecutive x positions
        // gaps[i] = (left_x_position, right_x_position) where the gap is between them
        let mut gap_indices: Vec<usize> = Vec::new();
        for (i, window) in x_positions.windows(2).enumerate() {
            let gap = window[1] - window[0];
            if gap > self.min_column_gap {
                gap_indices.push(i);
            }
        }

        // If no significant gaps found, return single column
        if gap_indices.is_empty() {
            return vec![ColumnBoundary::new(0.0, page_width)];
        }

        // Split x_positions into column groups based on gaps
        let mut columns = Vec::new();
        let mut start_idx = 0;

        for &gap_idx in &gap_indices {
            // Column contains x_positions from start_idx to gap_idx (inclusive)
            let col_x_positions = &x_positions[start_idx..=gap_idx];

            if !col_x_positions.is_empty() {
                let col_min_x = col_x_positions[0];
                // Find max right edge of blocks in this column
                let col_max_x = blocks.iter()
                    .filter(|b| col_x_positions.iter().any(|&x| (b.x - x).abs() < 1.0))
                    .map(|b| b.right())
                    .fold(col_min_x, f32::max);

                // Extend column boundary to midpoint of gap
                let gap_midpoint = (x_positions[gap_idx] + x_positions[gap_idx + 1]) / 2.0;
                columns.push(ColumnBoundary::new(col_min_x, gap_midpoint));
            }

            start_idx = gap_idx + 1;
        }

        // Add final column (after last gap)
        if start_idx < x_positions.len() {
            let col_x_positions = &x_positions[start_idx..];
            let col_min_x = col_x_positions[0];
            let col_max_x = blocks.iter()
                .filter(|b| col_x_positions.iter().any(|&x| (b.x - x).abs() < 1.0))
                .map(|b| b.right())
                .fold(page_width, f32::max);

            // Use midpoint of last gap as left boundary
            let last_gap_idx = gap_indices[gap_indices.len() - 1];
            let gap_midpoint = (x_positions[last_gap_idx] + x_positions[last_gap_idx + 1]) / 2.0;
            columns.push(ColumnBoundary::new(gap_midpoint, page_width));
        }

        // If no valid columns were found, return a single full-width column
        if columns.is_empty() {
            columns.push(ColumnBoundary::new(0.0, page_width));
        }

        columns
    }

    /// Detect the number of columns in a page.
    pub fn count_columns(&self, blocks: &[TextBlock], page_width: f32) -> usize {
        self.detect_column_boundaries(blocks, page_width).len()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block(text: &str, x: f32, y: f32) -> TextBlock {
        TextBlock::new(text.to_string(), x, y, 50.0, 12.0)
    }

    #[test]
    fn test_single_column_detection() {
        let detector = ColumnDetector::new();
        let blocks = vec![
            make_block("Line 1", 50.0, 10.0),
            make_block("Line 2", 50.0, 24.0),
            make_block("Line 3", 50.0, 38.0),
        ];

        let columns = detector.detect_column_boundaries(&blocks, 600.0);
        assert_eq!(columns.len(), 1);
    }

    #[test]
    fn test_two_column_detection() {
        let detector = ColumnDetector::new();

        // Left column at x=50, right column at x=320
        let blocks = vec![
            make_block("Left 1", 50.0, 10.0),
            make_block("Left 2", 50.0, 24.0),
            make_block("Right 1", 320.0, 10.0),
            make_block("Right 2", 320.0, 24.0),
        ];

        let columns = detector.detect_column_boundaries(&blocks, 600.0);
        assert_eq!(columns.len(), 2);
    }

    #[test]
    fn test_reorder_two_columns() {
        let detector = ColumnDetector::new();

        // Interleaved left/right column blocks
        let blocks = vec![
            make_block("Left 1", 50.0, 10.0),
            make_block("Right 1", 320.0, 10.0),
            make_block("Left 2", 50.0, 24.0),
            make_block("Right 2", 320.0, 24.0),
        ];

        let reordered = detector.reorder_text_by_columns(&blocks, 600.0);

        // Should be: Left 1, Left 2, Right 1, Right 2
        assert_eq!(reordered.len(), 4);
        assert!(reordered[0].text.starts_with("Left"));
        assert!(reordered[1].text.starts_with("Left"));
        assert!(reordered[2].text.starts_with("Right"));
        assert!(reordered[3].text.starts_with("Right"));
    }

    #[test]
    fn test_column_boundary_contains() {
        let col = ColumnBoundary::new(100.0, 300.0);
        assert!(col.contains_x(150.0));
        assert!(col.contains_x(100.0)); // Inclusive left
        assert!(!col.contains_x(300.0)); // Exclusive right
        assert!(!col.contains_x(50.0));
    }

    #[test]
    fn test_empty_blocks() {
        let detector = ColumnDetector::new();
        let reordered = detector.reorder_text_by_columns(&[], 600.0);
        assert!(reordered.is_empty());
    }
}
