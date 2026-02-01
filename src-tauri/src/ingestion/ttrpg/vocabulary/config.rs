//! Configuration Constants
//!
//! Semantic chunking and hybrid search fusion configuration.
//! Ported from MDMAI config.py settings.

// ============================================================================
// CHUNKING CONFIGURATION
// ============================================================================

/// Semantic chunking configuration constants
/// Based on MDMAI config.py chunking settings
pub mod chunking_config {
    /// Target chunk size for RAG (characters)
    pub const TARGET_CHUNK_SIZE: usize = 1200;

    /// Minimum chunk size to avoid fragments (characters)
    pub const MIN_CHUNK_SIZE: usize = 300;

    /// Maximum chunk size hard limit (characters)
    pub const MAX_CHUNK_SIZE: usize = 2400;

    /// Overlap between chunks for context continuity (characters)
    pub const CHUNK_OVERLAP: usize = 150;

    /// Minimum pages to group together for semantic coherence
    pub const MIN_PAGES_PER_CHUNK: usize = 1;

    /// Maximum pages to group together
    pub const MAX_PAGES_PER_CHUNK: usize = 4;

    /// Token-based limits (approximate, assuming ~4 chars/token)
    pub const TARGET_TOKENS: usize = 300;
    pub const MAX_TOKENS: usize = 600;
    pub const OVERLAP_TOKENS: usize = 40;
}

// ============================================================================
// HYBRID SEARCH FUSION PARAMETERS
// ============================================================================

/// Configuration for hybrid search fusion (BM25 + vector)
/// Based on MDMAI config.py fusion settings
pub mod fusion_config {
    /// Weight for BM25 keyword search (0.0 to 1.0)
    pub const BM25_WEIGHT: f32 = 0.4;

    /// Weight for vector semantic search (0.0 to 1.0)
    pub const VECTOR_WEIGHT: f32 = 0.6;

    /// RRF (Reciprocal Rank Fusion) constant k
    /// Higher k = more weight to lower-ranked results
    pub const RRF_K: f32 = 60.0;

    /// Minimum score threshold for results
    pub const MIN_SCORE: f32 = 0.1;

    /// Maximum results to return
    pub const MAX_RESULTS: usize = 20;

    /// Boost factor for exact phrase matches
    pub const EXACT_MATCH_BOOST: f32 = 1.5;

    /// Boost factor for matches in title/section headers
    pub const HEADER_MATCH_BOOST: f32 = 1.2;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunking_config_sanity() {
        use chunking_config::*;
        assert!(MIN_CHUNK_SIZE < TARGET_CHUNK_SIZE);
        assert!(TARGET_CHUNK_SIZE < MAX_CHUNK_SIZE);
        assert!(CHUNK_OVERLAP < MIN_CHUNK_SIZE);
    }

    #[test]
    fn test_fusion_config_weights() {
        use fusion_config::*;
        // Weights should sum to 1.0
        let total = BM25_WEIGHT + VECTOR_WEIGHT;
        assert!((total - 1.0).abs() < 0.01);
    }
}
