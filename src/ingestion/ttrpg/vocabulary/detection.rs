//! Content Detection Functions
//!
//! Functions for detecting genre, content category, and publisher
//! from document text using vocabulary matching.

use super::data::*;

// ============================================================================
// VOCABULARY MATCHING FUNCTIONS
// ============================================================================

/// Count how many terms from a vocabulary list appear in the text
pub fn count_vocabulary_matches(text: &str, vocabulary: &[&str]) -> usize {
    let text_lower = text.to_lowercase();
    vocabulary
        .iter()
        .filter(|term| text_lower.contains(&term.to_lowercase()))
        .count()
}

/// Find all matching terms from a vocabulary list in the text
pub fn find_vocabulary_matches<'a>(text: &str, vocabulary: &'a [&'a str]) -> Vec<&'a str> {
    let text_lower = text.to_lowercase();
    vocabulary
        .iter()
        .filter(|term| text_lower.contains(&term.to_lowercase()))
        .copied()
        .collect()
}

// ============================================================================
// GENRE DETECTION
// ============================================================================

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

    let max_score = horror_score
        .max(fantasy_score)
        .max(scifi_score)
        .max(noir_score);

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

// ============================================================================
// CONTENT CATEGORY DETECTION
// ============================================================================

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

// ============================================================================
// PUBLISHER DETECTION
// ============================================================================

/// Detect publisher from text
pub fn detect_publisher_from_vocabulary(text: &str) -> Option<&'static str> {
    let text_lower = text.to_lowercase();

    PUBLISHERS
        .iter()
        .find(|&publisher| text_lower.contains(publisher))
        .map(|v| v as _)
}
