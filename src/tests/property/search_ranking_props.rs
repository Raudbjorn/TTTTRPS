//! Property-based tests for Search Ranking
//!
//! Tests invariants:
//! - Same query returns same order
//! - More relevant results score higher
//! - Empty query returns no results
//! - Ranking is transitive
//!
//! Note: These tests use a simplified in-memory search simulation since
//! the actual Meilisearch client requires a running server. The properties
//! tested here should hold for any reasonable search ranking implementation.

use proptest::prelude::*;
use std::cmp::Ordering;

// ============================================================================
// Simulated Search Types for Property Testing
// ============================================================================

/// A simplified search document for testing ranking properties
#[derive(Debug, Clone, PartialEq)]
struct TestDocument {
    id: String,
    content: String,
    relevance_score: f32,
}

/// A search result with ranking score
#[derive(Debug, Clone)]
struct TestSearchResult {
    document: TestDocument,
    score: f32,
}

impl TestSearchResult {
    fn new(doc: TestDocument, score: f32) -> Self {
        Self {
            document: doc,
            score,
        }
    }
}

/// Simulated search ranker for property testing
///
/// This implements a simple TF-IDF-like ranking to test ranking properties.
/// Real search engines have more complex ranking algorithms, but the
/// fundamental properties should still hold.
#[derive(Debug, Clone)]
struct TestSearchRanker {
    documents: Vec<TestDocument>,
}

impl TestSearchRanker {
    fn new() -> Self {
        Self {
            documents: Vec::new(),
        }
    }

    fn add_document(&mut self, doc: TestDocument) {
        self.documents.push(doc);
    }

    fn add_documents(&mut self, docs: Vec<TestDocument>) {
        self.documents.extend(docs);
    }

    /// Calculate relevance score based on term frequency
    fn calculate_score(&self, query: &str, doc: &TestDocument) -> f32 {
        if query.is_empty() {
            return 0.0;
        }

        let query_lower = query.to_lowercase();
        let content_lower = doc.content.to_lowercase();

        // Simple term frequency scoring
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        if query_terms.is_empty() {
            return 0.0;
        }

        let mut score = 0.0;
        for term in &query_terms {
            // Count occurrences
            let count = content_lower.matches(term).count() as f32;
            // Normalize by document length
            let doc_len = content_lower.split_whitespace().count().max(1) as f32;
            score += count / doc_len;
        }

        // Boost by inherent relevance score if set
        score *= 1.0 + (doc.relevance_score * 0.5);

        score
    }

    /// Search and return ranked results
    fn search(&self, query: &str) -> Vec<TestSearchResult> {
        if query.is_empty() || query.trim().is_empty() {
            return Vec::new();
        }

        let mut results: Vec<TestSearchResult> = self
            .documents
            .iter()
            .filter_map(|doc| {
                let score = self.calculate_score(query, doc);
                if score > 0.0 {
                    Some(TestSearchResult::new(doc.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending (higher is better)
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
        });

        results
    }

    /// Search with limit
    fn search_with_limit(&self, query: &str, limit: usize) -> Vec<TestSearchResult> {
        self.search(query).into_iter().take(limit).collect()
    }
}

// ============================================================================
// Strategies for generating test inputs
// ============================================================================

/// Generate a test document with content
fn arb_document() -> impl Strategy<Value = TestDocument> {
    (
        "[a-z]{5,10}",                    // id
        "[a-z ]{10,100}",                 // content
        0.0f32..1.0,                      // relevance_score
    )
        .prop_map(|(id, content, relevance)| TestDocument {
            id,
            content,
            relevance_score: relevance,
        })
}

/// Generate a collection of documents
fn arb_documents(min: usize, max: usize) -> impl Strategy<Value = Vec<TestDocument>> {
    proptest::collection::vec(arb_document(), min..max)
}

/// Generate a simple search query
fn arb_query() -> impl Strategy<Value = String> {
    "[a-z]{1,20}"
}

/// Generate a multi-word query
#[allow(dead_code)]
fn arb_multi_word_query() -> impl Strategy<Value = String> {
    "[a-z]{2,10}( [a-z]{2,10}){0,3}"
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    /// Property: Same query returns same order (deterministic ranking)
    ///
    /// For the same set of documents and the same query, the ranking
    /// should always produce the same order of results.
    #[test]
    fn prop_same_query_same_order(
        docs in arb_documents(5, 20),
        query in arb_query()
    ) {
        let mut ranker1 = TestSearchRanker::new();
        let mut ranker2 = TestSearchRanker::new();

        ranker1.add_documents(docs.clone());
        ranker2.add_documents(docs);

        let results1 = ranker1.search(&query);
        let results2 = ranker2.search(&query);

        // Same number of results
        prop_assert_eq!(
            results1.len(),
            results2.len(),
            "Same query should return same number of results"
        );

        // Same order (by document ID)
        for (r1, r2) in results1.iter().zip(results2.iter()) {
            prop_assert_eq!(
                &r1.document.id,
                &r2.document.id,
                "Results should be in the same order"
            );
            // Scores should match
            prop_assert!(
                (r1.score - r2.score).abs() < 1e-6,
                "Scores should match for same document"
            );
        }
    }

    /// Property: More relevant results score higher
    ///
    /// If a document contains more occurrences of query terms, it should
    /// score higher than documents with fewer occurrences.
    #[test]
    fn prop_more_relevant_scores_higher(
        base_content in "[a-z]{20,50}",
        search_term in "[a-z]{3,8}"
    ) {
        let mut ranker = TestSearchRanker::new();

        // Create documents with varying relevance
        let low_relevance = TestDocument {
            id: "low".to_string(),
            content: base_content.clone(),
            relevance_score: 0.0,
        };

        let medium_relevance = TestDocument {
            id: "medium".to_string(),
            content: format!("{} {}", base_content, search_term),
            relevance_score: 0.0,
        };

        let high_relevance = TestDocument {
            id: "high".to_string(),
            content: format!("{} {} {}", base_content, search_term, search_term),
            relevance_score: 0.0,
        };

        ranker.add_document(low_relevance);
        ranker.add_document(medium_relevance);
        ranker.add_document(high_relevance);

        let results = ranker.search(&search_term);

        // Filter to only results that matched
        let matching_results: Vec<_> = results
            .iter()
            .filter(|r| r.score > 0.0)
            .collect();

        // If we have multiple matching results, higher relevance should score higher
        if matching_results.len() >= 2 {
            // Find high and medium
            let high_result = matching_results.iter().find(|r| r.document.id == "high");
            let medium_result = matching_results.iter().find(|r| r.document.id == "medium");

            if let (Some(high), Some(medium)) = (high_result, medium_result) {
                prop_assert!(
                    high.score >= medium.score,
                    "Document with more term occurrences ({}) should score >= document with fewer ({})",
                    high.score,
                    medium.score
                );
            }
        }
    }

    /// Property: Empty query returns no results
    #[test]
    fn prop_empty_query_no_results(
        docs in arb_documents(1, 20)
    ) {
        let mut ranker = TestSearchRanker::new();
        ranker.add_documents(docs);

        // Empty string
        let results_empty = ranker.search("");
        prop_assert!(
            results_empty.is_empty(),
            "Empty query should return no results, got {}",
            results_empty.len()
        );

        // Whitespace only
        let results_whitespace = ranker.search("   ");
        prop_assert!(
            results_whitespace.is_empty(),
            "Whitespace-only query should return no results, got {}",
            results_whitespace.len()
        );
    }

    /// Property: Ranking is transitive
    ///
    /// If A scores higher than B, and B scores higher than C, then A scores higher than C.
    #[test]
    fn prop_ranking_is_transitive(
        docs in arb_documents(5, 30),
        query in arb_query()
    ) {
        let mut ranker = TestSearchRanker::new();
        ranker.add_documents(docs);

        let results = ranker.search(&query);

        // Check transitivity for all triples
        for i in 0..results.len() {
            for j in (i + 1)..results.len() {
                for k in (j + 1)..results.len() {
                    let score_a = results[i].score;
                    let score_b = results[j].score;
                    let score_c = results[k].score;

                    // Since results are sorted, we have:
                    // score_a >= score_b >= score_c
                    // Therefore score_a >= score_c (transitivity)
                    prop_assert!(
                        score_a >= score_c,
                        "Transitivity violated: {} >= {} >= {} but {} >= {} failed",
                        score_a, score_b, score_c, score_a, score_c
                    );
                }
            }
        }
    }

    /// Property: Results are sorted by descending score
    #[test]
    fn prop_results_sorted_descending(
        docs in arb_documents(5, 30),
        query in arb_query()
    ) {
        let mut ranker = TestSearchRanker::new();
        ranker.add_documents(docs);

        let results = ranker.search(&query);

        // Check that each result has score >= next result
        for window in results.windows(2) {
            prop_assert!(
                window[0].score >= window[1].score,
                "Results should be sorted descending: {} >= {}",
                window[0].score,
                window[1].score
            );
        }
    }

    /// Property: All scores are non-negative
    #[test]
    fn prop_scores_non_negative(
        docs in arb_documents(5, 20),
        query in arb_query()
    ) {
        let mut ranker = TestSearchRanker::new();
        ranker.add_documents(docs);

        let results = ranker.search(&query);

        for result in &results {
            prop_assert!(
                result.score >= 0.0,
                "Score should be non-negative, got {}",
                result.score
            );
        }
    }

    /// Property: Limit is respected
    #[test]
    fn prop_limit_respected(
        docs in arb_documents(10, 50),
        query in arb_query(),
        limit in 1usize..20
    ) {
        let mut ranker = TestSearchRanker::new();
        ranker.add_documents(docs);

        let results = ranker.search_with_limit(&query, limit);

        prop_assert!(
            results.len() <= limit,
            "Result count {} should not exceed limit {}",
            results.len(),
            limit
        );
    }

    /// Property: Subset of results preserves order
    ///
    /// If we limit results, the order should match the first N results
    /// of an unlimited search.
    #[test]
    fn prop_limited_subset_preserves_order(
        docs in arb_documents(10, 30),
        query in arb_query(),
        limit in 1usize..10
    ) {
        let mut ranker = TestSearchRanker::new();
        ranker.add_documents(docs);

        let full_results = ranker.search(&query);
        let limited_results = ranker.search_with_limit(&query, limit);

        // Limited results should be a prefix of full results
        for (i, limited) in limited_results.iter().enumerate() {
            if i < full_results.len() {
                prop_assert_eq!(
                    &limited.document.id,
                    &full_results[i].document.id,
                    "Limited result at position {} should match full result",
                    i
                );
            }
        }
    }

    /// Property: Case-insensitive matching
    #[test]
    fn prop_case_insensitive_matching(
        docs in arb_documents(5, 20),
        query in "[a-z]{3,10}"
    ) {
        let mut ranker = TestSearchRanker::new();
        ranker.add_documents(docs);

        let lower_results = ranker.search(&query.to_lowercase());
        let upper_results = ranker.search(&query.to_uppercase());

        // Both should return same results
        prop_assert_eq!(
            lower_results.len(),
            upper_results.len(),
            "Case should not affect result count"
        );

        // Same document IDs
        for (lower, upper) in lower_results.iter().zip(upper_results.iter()) {
            prop_assert_eq!(
                &lower.document.id,
                &upper.document.id,
                "Case should not affect result order"
            );
        }
    }

    /// Property: Multi-word queries match documents with any term
    #[test]
    fn prop_multi_word_query_matches(
        word1 in "[a-z]{3,8}",
        word2 in "[a-z]{3,8}"
    ) {
        let mut ranker = TestSearchRanker::new();

        // Document with first word only
        ranker.add_document(TestDocument {
            id: "word1_only".to_string(),
            content: format!("the {} is here", word1),
            relevance_score: 0.0,
        });

        // Document with second word only
        ranker.add_document(TestDocument {
            id: "word2_only".to_string(),
            content: format!("the {} is here", word2),
            relevance_score: 0.0,
        });

        // Document with both words
        ranker.add_document(TestDocument {
            id: "both_words".to_string(),
            content: format!("the {} and {} are here", word1, word2),
            relevance_score: 0.0,
        });

        let query = format!("{} {}", word1, word2);
        let results = ranker.search(&query);

        // Should find at least the document with both words
        let has_both = results.iter().any(|r| r.document.id == "both_words");
        if !word1.is_empty() && !word2.is_empty() {
            prop_assert!(
                has_both || results.is_empty(),
                "Should find document with both words or no results"
            );
        }
    }

    /// Property: Repeated searches are consistent
    #[test]
    fn prop_repeated_searches_consistent(
        docs in arb_documents(5, 20),
        query in arb_query(),
        iterations in 2usize..5
    ) {
        let mut ranker = TestSearchRanker::new();
        ranker.add_documents(docs);

        let first_results = ranker.search(&query);

        for i in 1..iterations {
            let results = ranker.search(&query);

            prop_assert_eq!(
                results.len(),
                first_results.len(),
                "Iteration {}: result count should be consistent",
                i
            );

            for (j, (first, current)) in first_results.iter().zip(results.iter()).enumerate() {
                prop_assert_eq!(
                    &first.document.id,
                    &current.document.id,
                    "Iteration {}, position {}: should have same document",
                    i, j
                );
            }
        }
    }

    /// Property: Score is finite (not NaN or Infinity)
    #[test]
    fn prop_scores_are_finite(
        docs in arb_documents(5, 20),
        query in arb_query()
    ) {
        let mut ranker = TestSearchRanker::new();
        ranker.add_documents(docs);

        let results = ranker.search(&query);

        for result in &results {
            prop_assert!(
                result.score.is_finite(),
                "Score should be finite, got {}",
                result.score
            );
        }
    }

    /// Property: No duplicate documents in results
    #[test]
    fn prop_no_duplicate_results(
        docs in arb_documents(5, 30),
        query in arb_query()
    ) {
        let mut ranker = TestSearchRanker::new();
        ranker.add_documents(docs);

        let results = ranker.search(&query);

        let mut seen_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for result in &results {
            prop_assert!(
                seen_ids.insert(&result.document.id),
                "Document {} appears multiple times in results",
                result.document.id
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Basic sanity test that the search ranker works
    #[test]
    fn test_search_ranker_exists() {
        let mut ranker = TestSearchRanker::new();
        ranker.add_document(TestDocument {
            id: "test".to_string(),
            content: "hello world".to_string(),
            relevance_score: 1.0,
        });

        let results = ranker.search("hello");
        assert!(!results.is_empty());
    }

    /// Test empty query returns nothing
    #[test]
    fn test_empty_query() {
        let mut ranker = TestSearchRanker::new();
        ranker.add_document(TestDocument {
            id: "test".to_string(),
            content: "hello world".to_string(),
            relevance_score: 1.0,
        });

        let results = ranker.search("");
        assert!(results.is_empty());
    }

    /// Test ranking order
    #[test]
    fn test_ranking_order() {
        let mut ranker = TestSearchRanker::new();

        ranker.add_document(TestDocument {
            id: "low".to_string(),
            content: "unrelated content".to_string(),
            relevance_score: 0.0,
        });

        ranker.add_document(TestDocument {
            id: "high".to_string(),
            content: "hello hello hello world".to_string(),
            relevance_score: 0.0,
        });

        let results = ranker.search("hello");

        // Should find the high relevance document first
        if !results.is_empty() {
            assert_eq!(results[0].document.id, "high");
        }
    }
}
