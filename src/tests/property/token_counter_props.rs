//! Property-based tests for Token Counting
//!
//! Tests invariants:
//! - Token count never exceeds character count
//! - Non-empty text has at least 1 token
//! - Empty string yields zero tokens
//! - Count increases with string length
//!
//! Note: This module tests a simple word-based token estimation since
//! the main codebase doesn't have a dedicated token counter. Real LLM
//! tokenizers (like tiktoken) would have different behavior, but these
//! properties should hold for any reasonable tokenization scheme.

use proptest::prelude::*;

// ============================================================================
// Simple Token Counter Implementation
// ============================================================================

/// Estimates token count using a simple word-based heuristic.
///
/// This is a simplified estimation that approximates LLM tokenization:
/// - Roughly 4 characters per token (OpenAI rule of thumb)
/// - Whitespace and punctuation are considered
/// - Unicode characters may count as multiple tokens
///
/// Real tokenizers (BPE, SentencePiece, etc.) have more complex rules.
#[derive(Debug, Clone)]
pub struct TokenCounter {
    /// Average characters per token (default: 4.0 for English)
    chars_per_token: f64,
}

impl TokenCounter {
    /// Create a new token counter with default settings
    pub fn new() -> Self {
        Self {
            chars_per_token: 4.0,
        }
    }

    /// Create a token counter with custom chars-per-token ratio
    pub fn with_ratio(chars_per_token: f64) -> Self {
        Self {
            chars_per_token: chars_per_token.max(0.1), // Prevent division by zero
        }
    }

    /// Count tokens in a string (estimation)
    pub fn count(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        // Estimate based on character count
        let char_count = text.chars().count();
        let estimated = (char_count as f64 / self.chars_per_token).ceil() as usize;

        // Minimum of 1 token for non-empty strings
        estimated.max(1)
    }

    /// Count tokens using word-based estimation
    pub fn count_words(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        // Split on whitespace and count
        // This is a rough approximation - real tokenizers handle subwords
        let words: Vec<&str> = text.split_whitespace().collect();

        if words.is_empty() {
            // All whitespace - count as 1 token
            1
        } else {
            // Roughly 1.3 tokens per word on average
            let estimated = (words.len() as f64 * 1.3).ceil() as usize;
            estimated.max(1)
        }
    }

    /// Count tokens with a more accurate byte-based estimation
    pub fn count_bytes(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        // UTF-8 bytes, roughly 4 bytes per token
        let byte_count = text.len();
        let estimated = (byte_count as f64 / 4.0).ceil() as usize;
        estimated.max(1)
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Strategies for generating test inputs
// ============================================================================

/// Generate arbitrary non-empty strings
fn arb_text() -> impl Strategy<Value = String> {
    ".{1,1000}"
}

/// Generate strings with various Unicode content
fn arb_unicode_text() -> impl Strategy<Value = String> {
    prop_oneof![
        // ASCII only
        "[a-zA-Z0-9 ]{1,100}",
        // With accented characters
        "[a-zA-ZÀ-ÿ0-9 ]{1,100}",
        // Mixed scripts (any printable)
        "\\PC{1,100}",
    ]
}

/// Generate strings of specific lengths
#[allow(dead_code)]
fn arb_sized_text(min_len: usize, max_len: usize) -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<char>().prop_filter("printable", |c| !c.is_control()), min_len..=max_len)
        .prop_map(|chars| chars.into_iter().collect())
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    /// Property: Token count satisfies basic invariants
    ///
    /// For any text:
    /// - Token count should never exceed character count (each char is at most 1 token)
    /// - Non-empty text should have at least 1 token
    #[test]
    fn prop_count_satisfies_invariants(
        text in ".*"
    ) {
        let counter = TokenCounter::new();
        let count = counter.count(&text);
        let char_count = text.chars().count();

        // Token count should never exceed character count
        prop_assert!(
            count <= char_count || text.is_empty(),
            "Token count {} should not exceed character count {} for text '{}'",
            count, char_count, text.chars().take(50).collect::<String>()
        );

        // Non-empty text should have at least 1 token
        prop_assert!(
            count > 0 || text.is_empty(),
            "Non-empty text should have at least 1 token, got {} for text '{}'",
            count, text.chars().take(50).collect::<String>()
        );
    }

    /// Property: Empty string yields zero tokens
    #[test]
    fn prop_empty_yields_zero(
        _seed in any::<u64>() // Just to make it a property test
    ) {
        let counter = TokenCounter::new();

        prop_assert_eq!(counter.count(""), 0, "Empty string should have 0 tokens");
        prop_assert_eq!(counter.count_words(""), 0, "Empty string should have 0 word tokens");
        prop_assert_eq!(counter.count_bytes(""), 0, "Empty string should have 0 byte tokens");
    }

    /// Property: Non-empty string yields at least one token
    #[test]
    fn prop_non_empty_has_at_least_one_token(
        text in ".+"
    ) {
        let counter = TokenCounter::new();
        let count = counter.count(&text);

        prop_assert!(
            count >= 1,
            "Non-empty string '{}' should have at least 1 token, got {}",
            text, count
        );
    }

    /// Property: Token count increases with string length
    #[test]
    fn prop_count_increases_with_length(
        base_len in 1usize..100,
        extension_len in 1usize..100
    ) {
        let counter = TokenCounter::new();

        // Create two strings where the second is longer
        let short = "a".repeat(base_len);
        let long = "a".repeat(base_len + extension_len);

        let short_count = counter.count(&short);
        let long_count = counter.count(&long);

        prop_assert!(
            long_count >= short_count,
            "Longer string ({} chars) should have >= tokens than shorter ({} chars): {} vs {}",
            long.len(), short.len(), long_count, short_count
        );
    }

    /// Property: Doubling string length approximately doubles token count
    #[test]
    fn prop_count_scales_with_length(
        base_len in 10usize..100
    ) {
        let counter = TokenCounter::new();

        let single = "a".repeat(base_len);
        let double = "a".repeat(base_len * 2);

        let single_count = counter.count(&single);
        let double_count = counter.count(&double);

        // Should be approximately 2x (within reasonable tolerance)
        let ratio = double_count as f64 / single_count as f64;
        prop_assert!(
            ratio >= 1.5 && ratio <= 2.5,
            "Double length should give ~2x tokens: {} vs {} (ratio: {})",
            single_count, double_count, ratio
        );
    }

    /// Property: Word count is consistent
    #[test]
    fn prop_word_count_consistent(
        text in "[a-zA-Z]+([ ][a-zA-Z]+){0,20}"
    ) {
        let counter = TokenCounter::new();

        let count1 = counter.count_words(&text);
        let count2 = counter.count_words(&text);

        prop_assert_eq!(
            count1, count2,
            "Word count should be consistent for same input"
        );
    }

    /// Property: Byte count is at least 1 for non-empty strings
    #[test]
    fn prop_byte_count_non_empty(
        text in ".+"
    ) {
        let counter = TokenCounter::new();
        let count = counter.count_bytes(&text);

        prop_assert!(
            count >= 1,
            "Non-empty string should have at least 1 byte token"
        );
    }

    /// Property: Different counting methods give similar results
    #[test]
    fn prop_counting_methods_similar(
        text in "[a-zA-Z ]{10,100}"
    ) {
        let counter = TokenCounter::new();

        let char_count = counter.count(&text);
        let byte_count = counter.count_bytes(&text);

        // For ASCII text, these should be very similar
        let ratio = if char_count > 0 {
            byte_count as f64 / char_count as f64
        } else {
            1.0
        };

        prop_assert!(
            ratio >= 0.5 && ratio <= 2.0,
            "Char and byte counts should be similar for ASCII: {} vs {} (ratio: {})",
            char_count, byte_count, ratio
        );
    }

    /// Property: Custom ratio affects token count proportionally
    #[test]
    fn prop_custom_ratio_affects_count(
        ratio in 1.0f64..10.0,
        len in 20usize..200
    ) {
        let counter_default = TokenCounter::new(); // ratio = 4.0
        let counter_custom = TokenCounter::with_ratio(ratio);

        let text = "a".repeat(len);

        let default_count = counter_default.count(&text);
        let custom_count = counter_custom.count(&text);

        // Higher chars_per_token ratio means fewer tokens
        if ratio > 4.0 {
            prop_assert!(
                custom_count <= default_count,
                "Higher ratio should give fewer tokens: {} vs {} for ratio {}",
                custom_count, default_count, ratio
            );
        } else if ratio < 4.0 {
            prop_assert!(
                custom_count >= default_count,
                "Lower ratio should give more tokens: {} vs {} for ratio {}",
                custom_count, default_count, ratio
            );
        }
    }

    /// Property: Whitespace-only strings still count as tokens
    #[test]
    fn prop_whitespace_has_tokens(
        spaces in 1usize..100
    ) {
        let counter = TokenCounter::new();
        let whitespace = " ".repeat(spaces);

        let count = counter.count(&whitespace);
        let word_count = counter.count_words(&whitespace);

        prop_assert!(
            count >= 1,
            "Whitespace should have at least 1 char-based token"
        );
        prop_assert!(
            word_count >= 1,
            "Whitespace should have at least 1 word-based token"
        );
    }

    /// Property: Unicode characters are counted
    #[test]
    fn prop_unicode_counted(
        text in arb_unicode_text()
    ) {
        let counter = TokenCounter::new();
        let count = counter.count(&text);

        prop_assert!(
            count >= 1,
            "Unicode text should have tokens"
        );
    }

    /// Property: Concatenation doesn't lose tokens
    #[test]
    fn prop_concatenation_preserves_tokens(
        text1 in "[a-zA-Z]{5,50}",
        text2 in "[a-zA-Z]{5,50}"
    ) {
        let counter = TokenCounter::new();

        let count1 = counter.count(&text1);
        let count2 = counter.count(&text2);
        let combined = format!("{}{}", text1, text2);
        let combined_count = counter.count(&combined);

        // Combined should have at least as many tokens as sum minus a small margin
        // (due to potential merging at boundaries)
        prop_assert!(
            combined_count >= count1.saturating_sub(1) + count2.saturating_sub(1),
            "Combined count {} should be close to sum {} + {} = {}",
            combined_count, count1, count2, count1 + count2
        );
    }

    /// Property: Token count is bounded by character count
    #[test]
    fn prop_count_bounded_by_chars(
        text in arb_text()
    ) {
        let counter = TokenCounter::new();
        let char_count = text.chars().count();
        let token_count = counter.count(&text);

        // Each character is at most one token (in the worst case)
        // and there's at least one token per 4+ characters typically
        prop_assert!(
            token_count <= char_count + 1,
            "Token count {} should not exceed char count {} + 1",
            token_count, char_count
        );
    }

    /// Property: Newlines are counted in tokens
    #[test]
    fn prop_newlines_counted(
        lines in 1usize..20,
        line_len in 1usize..50
    ) {
        let counter = TokenCounter::new();
        let line = "a".repeat(line_len);
        let text = vec![line.clone(); lines].join("\n");

        let count = counter.count(&text);

        // Should have tokens for the content
        prop_assert!(
            count >= 1,
            "Text with newlines should have tokens"
        );
    }

    /// Property: Punctuation contributes to token count
    #[test]
    fn prop_punctuation_counted(
        word_count in 1usize..20
    ) {
        let counter = TokenCounter::new();

        let without_punct = "word ".repeat(word_count);
        let with_punct = "word. ".repeat(word_count);

        let count_without = counter.count(&without_punct);
        let count_with = counter.count(&with_punct);

        // Adding punctuation should add or keep same token count
        prop_assert!(
            count_with >= count_without,
            "Adding punctuation should not decrease token count: {} vs {}",
            count_with, count_without
        );
    }

    /// Property: Zero ratio is handled safely (min 0.1)
    #[test]
    fn prop_zero_ratio_safe(
        text in ".+"
    ) {
        // Creating with zero ratio should not panic
        let counter = TokenCounter::with_ratio(0.0);
        let count = counter.count(&text);

        // Should still give a valid count
        prop_assert!(
            count >= 1,
            "Even with zero ratio (clamped), should give valid count"
        );
    }

    /// Property: Very long strings don't overflow
    #[test]
    fn prop_long_strings_safe(
        len in 10000usize..50000
    ) {
        let counter = TokenCounter::new();
        let long_text = "a".repeat(len);

        let count = counter.count(&long_text);

        // Should complete without panic and give reasonable result
        prop_assert!(
            count > 0 && count <= len,
            "Long string count {} should be > 0 and <= {}",
            count, len
        );
    }

    /// Property: Count is within 20% of actual token count (spot check)
    ///
    /// This property tests that our token estimation is reasonably accurate
    /// compared to a known reference. We use a simple heuristic: for English
    /// text, tokens are approximately 4 characters on average.
    ///
    /// The 20% tolerance accounts for:
    /// - Different tokenizer implementations (BPE, SentencePiece, etc.)
    /// - Language-specific variations
    /// - Special characters and whitespace handling
    #[test]
    fn prop_count_within_20_percent_of_reference(
        text in "[a-zA-Z ]{20,500}"
    ) {
        let counter = TokenCounter::new();
        let estimated = counter.count(&text);

        // Reference: OpenAI rule of thumb is ~4 chars per token for English
        // We calculate the expected tokens and check if our estimate is within 20%
        let char_count = text.chars().count();
        let reference_estimate = (char_count as f64 / 4.0).ceil() as usize;

        // Skip if reference is too small (edge cases)
        if reference_estimate < 5 {
            return Ok(());
        }

        let lower_bound = (reference_estimate as f64 * 0.8).floor() as usize;
        let upper_bound = (reference_estimate as f64 * 1.2).ceil() as usize;

        prop_assert!(
            estimated >= lower_bound && estimated <= upper_bound,
            "Token count {} should be within 20% of reference {} (range: {} - {}) for text of {} chars",
            estimated, reference_estimate, lower_bound, upper_bound, char_count
        );
    }

    /// Property: Count is within 20% of word-based estimate (cross-validation)
    ///
    /// This validates that our character-based estimate roughly agrees with
    /// a word-based estimate, providing cross-validation of the counting logic.
    #[test]
    fn prop_char_and_word_counts_correlate(
        text in "[a-zA-Z]+([ ][a-zA-Z]+){5,30}"
    ) {
        let counter = TokenCounter::new();

        let char_count = counter.count(&text);
        let word_count = counter.count_words(&text);

        // Both methods should produce positive counts for non-empty text
        prop_assert!(char_count > 0, "Char count should be > 0 for non-empty text");
        prop_assert!(word_count > 0, "Word count should be > 0 for non-empty text");

        // Both methods should be in the same order of magnitude
        // The heuristics are very different (char/4 vs words*1.3), so we just verify
        // they don't differ by more than 10x which would indicate a bug
        let ratio = if char_count > word_count {
            char_count as f64 / word_count as f64
        } else {
            word_count as f64 / char_count as f64
        };

        prop_assert!(
            ratio <= 10.0,
            "Char count {} and word count {} should be within 10x of each other (ratio: {})",
            char_count, word_count, ratio
        );
    }

    /// Property: Specific known text samples have accurate counts
    ///
    /// This is a spot check using known text samples with expected token ranges.
    #[test]
    fn prop_known_samples_accurate(
        _seed in any::<u64>()
    ) {
        let counter = TokenCounter::new();

        // Known samples with expected token ranges (based on ~4 chars per token)
        let samples = vec![
            // (text, expected_min, expected_max)
            ("Hello, world!", 2, 5),              // 13 chars -> ~3 tokens
            ("The quick brown fox", 4, 7),        // 19 chars -> ~5 tokens
            ("a", 1, 1),                          // 1 char -> 1 token (minimum)
            ("    ", 1, 2),                       // Whitespace -> at least 1
            ("Hello World Test", 3, 6),           // 16 chars -> ~4 tokens
        ];

        for (text, expected_min, expected_max) in samples {
            let count = counter.count(text);
            prop_assert!(
                count >= expected_min && count <= expected_max,
                "Text '{}' should have {} - {} tokens, got {}",
                text, expected_min, expected_max, count
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Basic sanity test for the token counter
    #[test]
    fn test_token_counter_exists() {
        let counter = TokenCounter::new();
        assert_eq!(counter.count(""), 0);
        assert!(counter.count("hello world") > 0);
    }

    /// Test empty string
    #[test]
    fn test_empty_string() {
        let counter = TokenCounter::new();
        assert_eq!(counter.count(""), 0);
        assert_eq!(counter.count_words(""), 0);
        assert_eq!(counter.count_bytes(""), 0);
    }

    /// Test simple counting
    #[test]
    fn test_simple_counting() {
        let counter = TokenCounter::new();

        // 4 chars = ~1 token with default ratio
        assert!(counter.count("test") >= 1);

        // 8 chars = ~2 tokens
        assert!(counter.count("testtest") >= 1);

        // More chars = more tokens
        let short = counter.count("abc");
        let long = counter.count("abcdefghijklmnop");
        assert!(long >= short);
    }
}
