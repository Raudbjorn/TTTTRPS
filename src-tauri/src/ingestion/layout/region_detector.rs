//! Region Detection Module
//!
//! Detects boxed/shaded regions in PDF content, including:
//! - Sidebars (notes, tips, variants)
//! - Read-aloud text (boxed flavor text)
//! - Callouts and special notes
//! - Stat block regions
//!
//! Note: Full visual region detection (boxes, shading) requires PDF graphics
//! parsing. This implementation uses text-based heuristics which work well
//! for most TTRPG content.

use serde::{Deserialize, Serialize};

// ============================================================================
// Types
// ============================================================================

/// Type of detected region in the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionType {
    /// Sidebar content (notes, tips, variants)
    Sidebar,
    /// Callout or special note
    Callout,
    /// Read-aloud text (boxed flavor text for GMs)
    ReadAloud,
    /// Table content
    Table,
    /// Stat block (creature or NPC statistics)
    StatBlock,
    /// Normal body text
    Normal,
}

impl RegionType {
    /// Get a human-readable name for this region type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sidebar => "sidebar",
            Self::Callout => "callout",
            Self::ReadAloud => "read_aloud",
            Self::Table => "table",
            Self::StatBlock => "stat_block",
            Self::Normal => "normal",
        }
    }
}

/// Bounding box for a detected region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionBounds {
    /// X position of the left edge
    pub x: f32,
    /// Y position from top of page
    pub y: f32,
    /// Width of the region
    pub width: f32,
    /// Height of the region
    pub height: f32,
}

impl RegionBounds {
    /// Create new region bounds.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
}

/// A detected region in the document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedRegion {
    /// The type of region detected
    pub region_type: RegionType,
    /// The text content of the region
    pub content: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Page number where this region was found
    pub page_number: u32,
    /// Optional bounding box (if position data is available)
    pub bounds: Option<RegionBounds>,
}

impl DetectedRegion {
    /// Create a new detected region.
    pub fn new(
        region_type: RegionType,
        content: String,
        confidence: f32,
        page_number: u32,
    ) -> Self {
        Self {
            region_type,
            content,
            confidence,
            page_number,
            bounds: None,
        }
    }

    /// Set the bounds for this region.
    pub fn with_bounds(mut self, bounds: RegionBounds) -> Self {
        self.bounds = Some(bounds);
        self
    }
}

// ============================================================================
// Region Detector
// ============================================================================

/// Detects boxed/shaded regions in PDF content using text heuristics.
pub struct RegionDetector {
    /// Minimum confidence threshold for detection
    min_confidence: f32,
}

impl Default for RegionDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionDetector {
    /// Create a new region detector with default settings.
    pub fn new() -> Self {
        Self {
            min_confidence: 0.5,
        }
    }

    /// Create a region detector with custom confidence threshold.
    pub fn with_min_confidence(min_confidence: f32) -> Self {
        Self { min_confidence }
    }

    /// Detect regions from text content using heuristics.
    ///
    /// # Arguments
    /// * `text` - The text content to analyze
    /// * `page_number` - The page number for attribution
    ///
    /// # Returns
    /// A vector of detected regions, sorted by confidence
    pub fn detect_from_text(&self, text: &str, page_number: u32) -> Vec<DetectedRegion> {
        let mut regions = Vec::new();
        let text_trimmed = text.trim();
        let text_lower = text_trimmed.to_lowercase();

        // Skip very short text
        if text_trimmed.len() < 20 {
            return regions;
        }

        // Detect stat blocks (highest priority - very distinctive patterns)
        if let Some(region) = self.detect_stat_block(text_trimmed, &text_lower, page_number) {
            regions.push(region);
        }

        // Detect read-aloud text
        if let Some(region) = self.detect_read_aloud(text_trimmed, &text_lower, page_number) {
            regions.push(region);
        }

        // Detect sidebars
        if let Some(region) = self.detect_sidebar(text_trimmed, &text_lower, page_number) {
            regions.push(region);
        }

        // Detect callouts
        if let Some(region) = self.detect_callout(text_trimmed, &text_lower, page_number) {
            regions.push(region);
        }

        // Filter by confidence threshold and sort
        regions.retain(|r| r.confidence >= self.min_confidence);
        regions.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        regions
    }

    /// Detect stat block patterns.
    fn detect_stat_block(
        &self,
        text: &str,
        text_lower: &str,
        page_number: u32,
    ) -> Option<DetectedRegion> {
        let mut score: f32 = 0.0;
        let max_indicators: f32 = 6.0;

        // Check for stat block indicators
        let indicators = [
            ("armor class", 1.0),
            ("hit points", 1.0),
            ("speed", 0.5),
            ("str", 0.5),
            ("dex", 0.5),
            ("con", 0.5),
            ("int", 0.5),
            ("wis", 0.5),
            ("cha", 0.5),
            ("challenge", 0.5),
            ("saving throws", 0.5),
            ("damage resistances", 0.5),
            ("damage immunities", 0.5),
            ("condition immunities", 0.5),
            ("senses", 0.5),
            ("languages", 0.5),
            ("actions", 0.5),
            ("reactions", 0.5),
            ("legendary actions", 0.5),
        ];

        for (indicator, weight) in indicators {
            if text_lower.contains(indicator) {
                score += weight;
            }
        }

        // Check for ability score pattern (STR 18 (+4) or similar)
        let ability_pattern = regex::Regex::new(r"(?i)(str|dex|con|int|wis|cha)\s+\d+\s*\([+-]?\d+\)")
            .ok()?;
        if ability_pattern.is_match(text) {
            score += 1.5;
        }

        let confidence = (score / max_indicators).min(1.0_f32);

        if confidence >= 0.5 {
            Some(DetectedRegion::new(
                RegionType::StatBlock,
                text.to_string(),
                confidence,
                page_number,
            ))
        } else {
            None
        }
    }

    /// Detect read-aloud text patterns.
    fn detect_read_aloud(
        &self,
        text: &str,
        text_lower: &str,
        page_number: u32,
    ) -> Option<DetectedRegion> {
        let mut score: f32 = 0.0;

        // Direct indicators
        if text_lower.contains("read aloud") || text_lower.contains("boxed text") {
            score += 0.8;
        }

        // Text starts with quotes or italics marker (common in read-aloud)
        if text.starts_with('"') || text.starts_with('\'') || text.starts_with('*') {
            score += 0.3;
        }

        // Second-person address (common in read-aloud)
        let second_person_count = text_lower.matches(" you ").count()
            + text_lower.matches("you see").count()
            + text_lower.matches("you hear").count()
            + text_lower.matches("you notice").count()
            + text_lower.matches("before you").count();

        if second_person_count >= 2 {
            score += 0.4;
        }

        // Descriptive/atmospheric language
        let atmospheric_words = [
            "darkness", "shadow", "light", "smell", "sound", "cold", "warm",
            "ancient", "dusty", "damp", "eerie", "silence", "whisper",
        ];
        let atmospheric_count = atmospheric_words
            .iter()
            .filter(|w| text_lower.contains(*w))
            .count();
        if atmospheric_count >= 2 {
            score += 0.3;
        }

        let confidence = score.min(1.0_f32);

        if confidence >= 0.5 {
            Some(DetectedRegion::new(
                RegionType::ReadAloud,
                text.to_string(),
                confidence,
                page_number,
            ))
        } else {
            None
        }
    }

    /// Detect sidebar patterns.
    fn detect_sidebar(
        &self,
        text: &str,
        text_lower: &str,
        page_number: u32,
    ) -> Option<DetectedRegion> {
        let mut score: f32 = 0.0;

        // Common sidebar prefixes
        let prefixes = [
            "sidebar:", "note:", "tip:", "variant:", "optional rule:",
            "designer notes:", "dm tip:", "gm tip:", "roleplaying:",
            "variant rule:", "using this rule:",
        ];

        for prefix in prefixes {
            if text_lower.starts_with(prefix) || text_lower.contains(&format!("\n{}", prefix)) {
                score += 0.8;
                break;
            }
        }

        // Sidebar title patterns (often in caps or bold)
        let title_patterns = [
            "SIDEBAR", "VARIANT", "OPTIONAL", "NOTE", "TIP",
        ];
        for pattern in title_patterns {
            if text.contains(pattern) {
                score += 0.4;
                break;
            }
        }

        // Short paragraph with specific structure
        let line_count = text.lines().count();
        if line_count > 0 && line_count < 15 {
            score += 0.1;
        }

        let confidence = score.min(1.0_f32);

        if confidence >= 0.5 {
            Some(DetectedRegion::new(
                RegionType::Sidebar,
                text.to_string(),
                confidence,
                page_number,
            ))
        } else {
            None
        }
    }

    /// Detect callout patterns.
    fn detect_callout(
        &self,
        text: &str,
        text_lower: &str,
        page_number: u32,
    ) -> Option<DetectedRegion> {
        let mut score: f32 = 0.0;

        // Warning/caution patterns
        let warning_patterns = [
            "warning:", "caution:", "important:", "remember:",
            "don't forget:", "note that:", "be aware:",
        ];

        for pattern in warning_patterns {
            if text_lower.starts_with(pattern) || text_lower.contains(&format!("\n{}", pattern)) {
                score += 0.7;
                break;
            }
        }

        // Exclamation marks often indicate callouts
        if text.contains('!') && text.len() < 500 {
            score += 0.2;
        }

        let confidence = score.min(1.0_f32);

        if confidence >= 0.5 {
            Some(DetectedRegion::new(
                RegionType::Callout,
                text.to_string(),
                confidence,
                page_number,
            ))
        } else {
            None
        }
    }

    /// Classify a single piece of text into the most likely region type.
    ///
    /// Returns `RegionType::Normal` if no specific region type is detected.
    pub fn classify(&self, text: &str, page_number: u32) -> RegionType {
        let regions = self.detect_from_text(text, page_number);
        regions.first()
            .map(|r| r.region_type)
            .unwrap_or(RegionType::Normal)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_block_detection() {
        let detector = RegionDetector::new();
        let stat_block = r#"
            Goblin
            Small humanoid, neutral evil
            Armor Class 15 (leather armor, shield)
            Hit Points 7 (2d6)
            Speed 30 ft.
            STR 8 (-1) DEX 14 (+2) CON 10 (+0) INT 10 (+0) WIS 8 (-1) CHA 8 (-1)
            Skills Stealth +6
            Senses darkvision 60 ft.
            Languages Common, Goblin
            Challenge 1/4
        "#;

        let regions = detector.detect_from_text(stat_block, 1);
        assert!(!regions.is_empty());
        assert_eq!(regions[0].region_type, RegionType::StatBlock);
        assert!(regions[0].confidence >= 0.5);
    }

    #[test]
    fn test_read_aloud_detection() {
        let detector = RegionDetector::new();
        let read_aloud = r#"
            You enter a dimly lit chamber. Before you, ancient stone pillars
            rise into the darkness above. You hear the distant drip of water
            and smell the musty odor of decay. Shadows dance at the edges of
            your torchlight.
        "#;

        let regions = detector.detect_from_text(read_aloud, 1);
        assert!(!regions.is_empty());
        assert_eq!(regions[0].region_type, RegionType::ReadAloud);
    }

    #[test]
    fn test_sidebar_detection() {
        let detector = RegionDetector::new();
        let sidebar = r#"
            Variant: Flanking
            If you regularly use miniatures, flanking gives combatants a
            simple way to gain advantage on attack rolls against a common enemy.
        "#;

        let regions = detector.detect_from_text(sidebar, 1);
        assert!(!regions.is_empty());
        assert_eq!(regions[0].region_type, RegionType::Sidebar);
    }

    #[test]
    fn test_normal_text() {
        let detector = RegionDetector::new();
        let normal = "This is just regular body text without any special indicators.";

        let region_type = detector.classify(normal, 1);
        assert_eq!(region_type, RegionType::Normal);
    }

    #[test]
    fn test_region_type_as_str() {
        assert_eq!(RegionType::StatBlock.as_str(), "stat_block");
        assert_eq!(RegionType::ReadAloud.as_str(), "read_aloud");
        assert_eq!(RegionType::Sidebar.as_str(), "sidebar");
    }
}
