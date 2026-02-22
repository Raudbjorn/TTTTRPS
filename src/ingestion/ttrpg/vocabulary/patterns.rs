//! Pattern Detection
//!
//! Regular expression patterns for detecting source books, headers,
//! dice notation, and table structures in TTRPG documents.

use once_cell::sync::Lazy;

// ============================================================================
// SOURCE BOOK PATTERNS
// ============================================================================

/// Common TTRPG source book abbreviations and their full names
/// Format: (abbreviation, full_name, system)
/// Used for filtering by source and metadata extraction
pub static SOURCE_BOOK_PATTERNS: Lazy<Vec<(&'static str, &'static str, &'static str)>> =
    Lazy::new(|| {
        vec![
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
            (
                "motm",
                "mordenkainen presents: monsters of the multiverse",
                "dnd5e",
            ),
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
        ]
    });

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
// HEADER LEVEL DETECTION PATTERNS
// ============================================================================

/// Header level patterns with associated header level (1-6)
/// Used for TOC generation and semantic chunking
pub static HEADER_PATTERNS: Lazy<Vec<(&'static str, u8)>> = Lazy::new(|| {
    vec![
        // Level 1 - Major divisions
        (r"^chapter\s+\d+", 1),
        (r"^chapter\s+[ivxlcdm]+", 1), // Chapter I, II, III
        (r"^part\s+[ivxlcdm]+", 1),    // Roman numerals
        (r"^part\s+\d+", 1),
        (r"^book\s+\d+", 1),
        // Level 2 - Chapters and major sections
        (r"^appendix\s+[a-z]", 2),
        (r"^section\s+[a-z]", 2),      // Section A, Section B
        (r"^section\s+[ivxlcdm]+", 2), // Section I, II, III
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
    ]
});

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
        && trimmed
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(|c| c.is_uppercase())
    {
        return Some(2);
    }

    // Title case and short = possibly header
    if trimmed.len() < 60 && !trimmed.ends_with('.') && !trimmed.ends_with(',') {
        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.len() <= 8 {
            let title_case_count = words
                .iter()
                .filter(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
                .count();
            if title_case_count > words.len() / 2 {
                return Some(3);
            }
        }
    }

    None
}

// ============================================================================
// DICE TABLE DETECTION PATTERNS
// ============================================================================

/// Patterns for detecting dice notation in tables
/// Used by the RandomTableParser to identify rollable tables
pub static DICE_PATTERNS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        r"\bd\d+\b",              // d20, d6, d100, etc.
        r"\d+d\d+",               // 2d6, 3d8, etc.
        r"\bd%\b",                // Percentile dice
        r"\d+-\d+",               // Range notation (1-4, 5-8, etc.)
        r"\b\d+\s*[-–—]\s*\d+\b", // Range with various dashes
    ]
});

/// Table row patterns that indicate a random/roll table
pub static TABLE_ROW_PATTERNS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        r"^\d+[-–—]\d+\s+.+",  // "1-4 Result text"
        r"^\d+\.\s+.+",        // "1. Result text"
        r"^\d+\s+.+",          // "1 Result text" (simple numbered)
        r"^[ivxlcdm]+\.\s+.+", // Roman numeral lists
    ]
});

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
