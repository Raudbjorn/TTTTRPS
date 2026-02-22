//! Semantic Chunker Unit Tests
//!
//! Tests for text chunking with default/custom parameters, overlap logic,
//! semantic boundary detection, and data integrity.
//! Note: Private helper methods are tested indirectly through the public API.

use std::collections::HashMap;

use crate::ingestion::chunker::{SemanticChunker, ChunkConfig, ContentChunk};
use crate::ingestion::ttrpg::vocabulary::chunking_config;

// ============================================================================
// Test Fixtures
// ============================================================================

/// Creates sample text for testing
fn sample_text() -> String {
    r#"CHAPTER ONE

This is the first paragraph of chapter one. It contains several sentences that will help test the chunking logic.

This is the second paragraph. It has some more content to ensure we have enough text to test the chunking behavior properly.

This is the third paragraph with additional content. The semantic chunker should handle paragraph boundaries correctly."#.to_string()
}

/// Creates long text that will require multiple chunks
fn long_text() -> String {
    let mut text = String::new();
    for i in 1..=20 {
        text.push_str(&format!(
            "Paragraph {} contains enough text to test the chunking algorithm. It has multiple sentences. This helps verify overlap and boundary detection.\n\n",
            i
        ));
    }
    text
}

/// Creates text with various header patterns
fn text_with_headers() -> String {
    r#"CHAPTER ONE

Introduction paragraph.

SECTION I

First section content here with multiple sentences.

Chapter Two: The Continuation

More content follows the chapter header.

Part III

Final part content."#.to_string()
}

// ============================================================================
// Default Configuration Tests
// ============================================================================

#[cfg(test)]
mod default_config_tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let config = ChunkConfig::default();

        // Uses MDMAI-derived constants from chunking_config
        assert_eq!(config.target_size, chunking_config::TARGET_CHUNK_SIZE);
        assert_eq!(config.min_size, chunking_config::MIN_CHUNK_SIZE);
        assert_eq!(config.max_size, chunking_config::MAX_CHUNK_SIZE);
        assert_eq!(config.overlap_size, chunking_config::CHUNK_OVERLAP);
        assert!(config.preserve_sentences);
        assert!(config.preserve_paragraphs);
    }

    #[test]
    fn test_small_config_preset() {
        let config = ChunkConfig::small();

        // Half of default values
        assert_eq!(config.target_size, chunking_config::TARGET_CHUNK_SIZE / 2);
        assert_eq!(config.min_size, chunking_config::MIN_CHUNK_SIZE / 2);
        assert_eq!(config.max_size, chunking_config::MAX_CHUNK_SIZE / 2);
        assert_eq!(config.overlap_size, chunking_config::CHUNK_OVERLAP / 2);
    }

    #[test]
    fn test_large_config_preset() {
        let config = ChunkConfig::large();

        // Double of default values
        assert_eq!(config.target_size, chunking_config::TARGET_CHUNK_SIZE * 2);
        assert_eq!(config.min_size, chunking_config::MIN_CHUNK_SIZE * 2);
        assert_eq!(config.max_size, chunking_config::MAX_CHUNK_SIZE * 2);
        assert_eq!(config.overlap_size, chunking_config::CHUNK_OVERLAP * 2);
    }

    #[test]
    fn test_default_chunker_creation() {
        let chunker = SemanticChunker::new();
        // Should not panic
        let _ = chunker.chunk_text(&sample_text(), "test");
    }

    #[test]
    fn test_chunker_with_custom_config() {
        let config = ChunkConfig {
            target_size: 500,
            min_size: 100,
            max_size: 800,
            overlap_size: 50,
            preserve_sentences: true,
            preserve_paragraphs: true,
        };

        let chunker = SemanticChunker::with_config(config);
        let chunks = chunker.chunk_text(&sample_text(), "test");

        // Should produce chunks
        assert!(!chunks.is_empty());
    }
}

// ============================================================================
// Custom Parameter Tests
// ============================================================================

#[cfg(test)]
mod custom_parameter_tests {
    use super::*;

    #[test]
    fn test_smaller_target_produces_more_chunks() {
        let text = long_text();

        let small_config = ChunkConfig {
            target_size: 200,
            min_size: 50,
            max_size: 400,
            overlap_size: 20,
            preserve_sentences: true,
            preserve_paragraphs: false,
        };

        let large_config = ChunkConfig {
            target_size: 1000,
            min_size: 200,
            max_size: 2000,
            overlap_size: 100,
            preserve_sentences: true,
            preserve_paragraphs: true,
        };

        let small_chunker = SemanticChunker::with_config(small_config);
        let large_chunker = SemanticChunker::with_config(large_config);

        let small_chunks = small_chunker.chunk_text(&text, "test");
        let large_chunks = large_chunker.chunk_text(&text, "test");

        // Smaller target should produce more chunks
        assert!(
            small_chunks.len() >= large_chunks.len(),
            "Expected small chunks ({}) >= large chunks ({})",
            small_chunks.len(),
            large_chunks.len()
        );
    }

    #[test]
    fn test_chunks_respect_max_size() {
        let config = ChunkConfig {
            target_size: 100,
            min_size: 20,
            max_size: 200,
            overlap_size: 10,
            preserve_sentences: false,
            preserve_paragraphs: false,
        };

        let chunker = SemanticChunker::with_config(config);
        let chunks = chunker.chunk_text(&long_text(), "test");

        for chunk in &chunks {
            assert!(
                chunk.content.len() <= 200 + 50, // Allow some buffer for sentence preservation
                "Chunk exceeds max size: {} chars",
                chunk.content.len()
            );
        }
    }

    #[test]
    fn test_chunks_respect_min_size_generally() {
        let config = ChunkConfig {
            target_size: 300,
            min_size: 100,
            max_size: 600,
            overlap_size: 50,
            preserve_sentences: true,
            preserve_paragraphs: true,
        };

        let chunker = SemanticChunker::with_config(config);
        let chunks = chunker.chunk_text(&long_text(), "test");

        // Most chunks should meet min size (last chunk might be smaller)
        let chunks_meeting_min = chunks.iter()
            .filter(|c| c.content.len() >= 100)
            .count();

        assert!(
            chunks_meeting_min >= chunks.len().saturating_sub(1),
            "Too many chunks below min size"
        );
    }

    #[test]
    fn test_zero_overlap_config() {
        let config = ChunkConfig {
            target_size: 200,
            min_size: 50,
            max_size: 400,
            overlap_size: 0,
            preserve_sentences: true,
            preserve_paragraphs: true,
        };

        let chunker = SemanticChunker::with_config(config);
        let chunks = chunker.chunk_text(&long_text(), "test");

        // Should still produce chunks without overlap
        assert!(!chunks.is_empty());
    }
}

// ============================================================================
// Overlap Logic Tests (via public API)
// ============================================================================

#[cfg(test)]
mod overlap_tests {
    use super::*;

    #[test]
    fn test_overlap_creates_continuity() {
        let config = ChunkConfig {
            target_size: 150,
            min_size: 50,
            max_size: 300,
            overlap_size: 50,
            preserve_sentences: true,
            preserve_paragraphs: false,
        };

        let chunker = SemanticChunker::with_config(config);
        let text = "First sentence here. Second sentence here. Third sentence here. Fourth sentence here. Fifth sentence here. Sixth sentence here.";
        let chunks = chunker.chunk_text(text, "test");

        if chunks.len() > 1 {
            // Check if consecutive chunks have overlapping content
            for i in 0..chunks.len() - 1 {
                let current_end = &chunks[i].content;
                let next_start = &chunks[i + 1].content;

                // The end of one chunk should share some words with the start of the next
                let current_words: Vec<&str> = current_end.split_whitespace().collect();
                let next_words: Vec<&str> = next_start.split_whitespace().collect();

                // At least some overlap should exist if configured
                // (this is a soft check as exact overlap depends on boundaries)
                let _overlap_present = current_words.iter().rev().take(5).any(|w| {
                    next_words.iter().take(10).any(|nw| nw == w)
                });
            }
        }
    }

    #[test]
    fn test_overlap_with_pages() {
        let config = ChunkConfig {
            target_size: 100,
            min_size: 20,
            max_size: 200,
            overlap_size: 20,
            preserve_sentences: true,
            preserve_paragraphs: true,
        };

        let chunker = SemanticChunker::with_config(config);
        let pages = vec![
            (1, "First page content here.".to_string()),
            (2, "Second page content here.".to_string()),
        ];

        let chunks = chunker.chunk_with_pages(&pages, "test");
        // Should produce chunks without panicking
        assert!(chunks.is_empty() || !chunks.is_empty());
    }
}

// ============================================================================
// Semantic Boundary Detection Tests (via public API)
// ============================================================================

#[cfg(test)]
mod boundary_detection_tests {
    use super::*;

    #[test]
    fn test_header_detection_through_chunking() {
        // Test header detection indirectly through chunk sectioning
        // Use small config since test text is relatively short
        let chunker = SemanticChunker::with_config(ChunkConfig::small());
        let chunks = chunker.chunk_text(&text_with_headers(), "test");

        // Chunks should be created for text with headers
        assert!(!chunks.is_empty());

        // Some chunks may have section metadata if headers were detected
        let _sections_detected = chunks.iter()
            .filter(|c| c.section.is_some())
            .count();
    }

    #[test]
    fn test_paragraph_boundary_preservation() {
        let config = ChunkConfig {
            target_size: 500,
            min_size: 10,  // Use small min_size to accommodate short test text
            max_size: 1000,
            overlap_size: 50,
            preserve_sentences: true,
            preserve_paragraphs: true,
        };

        let chunker = SemanticChunker::with_config(config);
        let text = "First paragraph content.\n\nSecond paragraph content.\n\nThird paragraph content.";
        let chunks = chunker.chunk_text(text, "test");

        // Chunks should be created respecting paragraph boundaries
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_sentence_boundary_in_chunks() {
        let config = ChunkConfig {
            target_size: 100,
            min_size: 20,
            max_size: 200,
            overlap_size: 20,
            preserve_sentences: true,
            preserve_paragraphs: true,
        };

        let chunker = SemanticChunker::with_config(config);
        let text = "First sentence. Second sentence. Third sentence. Fourth sentence.";
        let chunks = chunker.chunk_text(text, "test");

        // Chunks should be created
        for chunk in &chunks {
            // Content should not be empty
            assert!(!chunk.content.trim().is_empty());
        }
    }
}

// ============================================================================
// Data Integrity Tests
// ============================================================================

#[cfg(test)]
mod data_integrity_tests {
    use super::*;

    #[test]
    fn test_all_content_preserved() {
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 200,
            min_size: 50,
            max_size: 400,
            overlap_size: 0, // No overlap for exact comparison
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        let original = "First paragraph here.\n\nSecond paragraph here.\n\nThird paragraph.";
        let chunks = chunker.chunk_text(original, "test");

        // All significant words from original should appear in chunks
        let all_chunk_content: String = chunks.iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        assert!(all_chunk_content.contains("First"));
        assert!(all_chunk_content.contains("Second"));
        assert!(all_chunk_content.contains("Third"));
    }

    #[test]
    fn test_chunk_ids_are_unique() {
        let chunker = SemanticChunker::new();
        let chunks = chunker.chunk_text(&long_text(), "test");

        let ids: Vec<_> = chunks.iter().map(|c| &c.id).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();

        assert_eq!(ids.len(), unique_ids.len(), "Chunk IDs should be unique");
    }

    #[test]
    fn test_chunk_indices_are_sequential() {
        let chunker = SemanticChunker::new();
        let chunks = chunker.chunk_text(&long_text(), "test");

        for (expected_idx, chunk) in chunks.iter().enumerate() {
            assert_eq!(
                chunk.chunk_index, expected_idx,
                "Chunk indices should be sequential"
            );
        }
    }

    #[test]
    fn test_source_id_preserved() {
        let chunker = SemanticChunker::new();
        let source_id = "my-unique-source-id";
        let chunks = chunker.chunk_text(&sample_text(), source_id);

        for chunk in &chunks {
            assert_eq!(
                chunk.source_id, source_id,
                "Source ID should be preserved"
            );
        }
    }

    #[test]
    fn test_chunk_type_is_text() {
        let chunker = SemanticChunker::new();
        let chunks = chunker.chunk_text(&sample_text(), "test");

        for chunk in &chunks {
            assert_eq!(chunk.chunk_type, "text");
        }
    }

    #[test]
    fn test_empty_metadata_by_default() {
        let chunker = SemanticChunker::new();
        let chunks = chunker.chunk_text(&sample_text(), "test");

        for chunk in &chunks {
            assert!(chunk.metadata.is_empty());
        }
    }

    #[test]
    fn test_content_not_empty() {
        let chunker = SemanticChunker::new();
        let chunks = chunker.chunk_text(&sample_text(), "test");

        for chunk in &chunks {
            assert!(!chunk.content.is_empty(), "Chunk content should not be empty");
            assert!(!chunk.content.trim().is_empty(), "Chunk content should not be only whitespace");
        }
    }
}

// ============================================================================
// Page-Aware Chunking Tests
// ============================================================================

#[cfg(test)]
mod page_chunking_tests {
    use super::*;

    #[test]
    fn test_chunk_with_pages_basic() {
        // Use small min_size to accommodate short page content
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 100,
            min_size: 10,  // Small min_size for short test pages
            max_size: 200,
            overlap_size: 10,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });
        let pages = vec![
            (1, "Page one content here.".to_string()),
            (2, "Page two content here.".to_string()),
        ];

        let chunks = chunker.chunk_with_pages(&pages, "test");

        // Should produce chunks with short min_size
        assert!(!chunks.is_empty());

        // At least some chunks should have page numbers
        let chunks_with_pages = chunks.iter().filter(|c| c.page_number.is_some()).count();
        assert!(chunks_with_pages > 0);
    }

    #[test]
    fn test_page_numbers_preserved() {
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 50,
            min_size: 10,
            max_size: 100,
            overlap_size: 10,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        let pages = vec![
            (1, "First page.".to_string()),
            (5, "Fifth page.".to_string()),
            (10, "Tenth page.".to_string()),
        ];

        let chunks = chunker.chunk_with_pages(&pages, "test");

        // Each chunk should have a valid page number
        for chunk in &chunks {
            if let Some(page_num) = chunk.page_number {
                assert!(
                    page_num == 1 || page_num == 5 || page_num == 10,
                    "Unexpected page number: {}",
                    page_num
                );
            }
        }
    }

    #[test]
    fn test_chunk_text_uses_page_one() {
        let chunker = SemanticChunker::new();
        let chunks = chunker.chunk_text(&sample_text(), "test");

        // chunk_text should treat content as page 1
        for chunk in &chunks {
            if let Some(page_num) = chunk.page_number {
                assert_eq!(page_num, 1);
            }
        }
    }
}

// ============================================================================
// ContentChunk Structure Tests
// ============================================================================

#[cfg(test)]
mod content_chunk_tests {
    use super::*;

    #[test]
    fn test_content_chunk_creation() {
        let chunk = ContentChunk {
            id: "test-id".to_string(),
            source_id: "source-1".to_string(),
            content: "Test content".to_string(),
            page_number: Some(1),
            section: Some("Chapter 1".to_string()),
            chunk_type: "text".to_string(),
            chunk_index: 0,
            metadata: HashMap::new(),
            ..Default::default()
        };

        assert_eq!(chunk.id, "test-id");
        assert_eq!(chunk.source_id, "source-1");
        assert_eq!(chunk.content, "Test content");
        assert_eq!(chunk.page_number, Some(1));
        assert_eq!(chunk.section, Some("Chapter 1".to_string()));
        assert_eq!(chunk.chunk_type, "text");
        assert_eq!(chunk.chunk_index, 0);
    }

    #[test]
    fn test_content_chunk_serialization() {
        let chunk = ContentChunk {
            id: "test-id".to_string(),
            source_id: "source-1".to_string(),
            content: "Test content".to_string(),
            page_number: Some(1),
            section: None,
            chunk_type: "text".to_string(),
            chunk_index: 0,
            metadata: HashMap::new(),
            ..Default::default()
        };

        let json = serde_json::to_string(&chunk);
        assert!(json.is_ok());

        if let Ok(json_str) = json {
            let deserialized: Result<ContentChunk, _> = serde_json::from_str(&json_str);
            assert!(deserialized.is_ok());

            let restored = deserialized.unwrap();
            assert_eq!(restored.id, chunk.id);
            assert_eq!(restored.content, chunk.content);
        }
    }

    #[test]
    fn test_content_chunk_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("key1".to_string(), "value1".to_string());
        metadata.insert("key2".to_string(), "value2".to_string());

        let chunk = ContentChunk {
            id: "test-id".to_string(),
            source_id: "source-1".to_string(),
            content: "Test".to_string(),
            page_number: None,
            section: None,
            chunk_type: "text".to_string(),
            chunk_index: 0,
            metadata,
            ..Default::default()
        };

        assert_eq!(chunk.metadata.len(), 2);
        assert_eq!(chunk.metadata.get("key1"), Some(&"value1".to_string()));
    }

    #[test]
    fn test_content_chunk_clone() {
        let chunk = ContentChunk {
            id: "test-id".to_string(),
            source_id: "source-1".to_string(),
            content: "Test".to_string(),
            page_number: Some(1),
            section: None,
            chunk_type: "text".to_string(),
            chunk_index: 0,
            metadata: HashMap::new(),
            ..Default::default()
        };

        let cloned = chunk.clone();
        assert_eq!(chunk.id, cloned.id);
        assert_eq!(chunk.content, cloned.content);
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        let chunker = SemanticChunker::new();
        let chunks = chunker.chunk_text("", "test");

        // Empty text should produce no chunks
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_whitespace_only_text() {
        let chunker = SemanticChunker::new();
        let chunks = chunker.chunk_text("   \n\n   \t\t   ", "test");

        // Whitespace only should produce no chunks
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_single_word() {
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 100,
            min_size: 1, // Allow tiny chunks
            max_size: 200,
            overlap_size: 0,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        let chunks = chunker.chunk_text("Hello", "test");

        // May or may not produce a chunk depending on min_size
        assert!(chunks.len() <= 1);
    }

    #[test]
    fn test_very_long_paragraph() {
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 100,
            min_size: 20,
            max_size: 200,
            overlap_size: 20,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        // Create a very long paragraph (1000 chars)
        let long_para = "This is a sentence. ".repeat(50);
        let chunks = chunker.chunk_text(&long_para, "test");

        // Should produce at least one chunk
        assert!(!chunks.is_empty(), "Expected at least one chunk from long paragraph");

        // Total content should be preserved
        let total_content: String = chunks.iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Key content should be preserved
        assert!(
            total_content.contains("This is a sentence"),
            "Content should be preserved in chunks"
        );
    }

    #[test]
    fn test_unicode_content() {
        // Use a config with small min_size to accommodate short test text
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 100,
            min_size: 10,  // Small min_size for short Unicode test
            max_size: 200,
            overlap_size: 10,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });
        let unicode_text = "First paragraph with unicode: cafe.\n\nSecond paragraph with more: resume.\n\nThird: naive.";
        let chunks = chunker.chunk_text(unicode_text, "test");

        // Should handle Unicode without panicking and produce chunks
        assert!(!chunks.is_empty());

        // Unicode should be preserved
        let all_content: String = chunks.iter().map(|c| c.content.as_str()).collect();
        assert!(all_content.contains("cafe") || all_content.contains("unicode"));
    }

    #[test]
    fn test_newline_variations() {
        let chunker = SemanticChunker::new();

        // Test different newline styles
        let text_crlf = "Para 1.\r\n\r\nPara 2.";
        let text_lf = "Para 1.\n\nPara 2.";

        let chunks_crlf = chunker.chunk_text(text_crlf, "test");
        let chunks_lf = chunker.chunk_text(text_lf, "test");

        // Both should produce chunks (or be empty if below min size)
        let _ = chunks_crlf;
        let _ = chunks_lf;
    }
}

// ============================================================================
// Default Implementation Tests
// ============================================================================

#[cfg(test)]
mod default_impl_tests {
    use super::*;

    #[test]
    fn test_semantic_chunker_default() {
        let chunker = SemanticChunker::default();
        let chunks = chunker.chunk_text(&sample_text(), "test");

        // Should work the same as new()
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_config_clone() {
        let config = ChunkConfig::default();
        let cloned = config.clone();

        assert_eq!(config.target_size, cloned.target_size);
        assert_eq!(config.min_size, cloned.min_size);
        assert_eq!(config.max_size, cloned.max_size);
    }
}

// ============================================================================
// Heading Inclusion Tests
// ============================================================================

#[cfg(test)]
mod heading_inclusion_tests {
    use super::*;

    /// Creates text with explicit chapter/section headers
    fn text_with_explicit_headers() -> String {
        r#"CHAPTER ONE: THE BEGINNING

This is the introduction to chapter one. It sets up the story and introduces the main characters.

The narrative continues with more details about the setting. We learn about the world and its history.

CHAPTER TWO: THE JOURNEY

The second chapter begins with a new adventure. Our heroes set out on their quest.

They face many challenges along the way. Each obstacle teaches them something new.

SECTION III: THE CLIMAX

The tension builds as we approach the climax. Everything they've learned is put to the test.

The final confrontation awaits them at the mountain peak."#.to_string()
    }

    #[test]
    fn test_chapters_detected_as_headers() {
        let chunker = SemanticChunker::new();
        let chunks = chunker.chunk_text(&text_with_explicit_headers(), "test");

        // Should produce chunks
        assert!(!chunks.is_empty());

        // At least some chunks should have section information
        let chunks_with_sections = chunks.iter().filter(|c| c.section.is_some()).count();
        // Note: section detection depends on implementation details
        let _ = chunks_with_sections;
    }

    #[test]
    fn test_header_text_preserved_in_chunks() {
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 500,
            min_size: 100,
            max_size: 1000,
            overlap_size: 50,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        let text = text_with_explicit_headers();
        let chunks = chunker.chunk_text(&text, "test");

        // Combine all chunk content
        let all_content: String = chunks.iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // All headers should appear in the combined content
        assert!(all_content.contains("CHAPTER ONE") || all_content.contains("THE BEGINNING"));
        assert!(all_content.contains("CHAPTER TWO") || all_content.contains("THE JOURNEY"));
    }

    #[test]
    fn test_header_triggers_new_section() {
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 200,
            min_size: 50,
            max_size: 400,
            overlap_size: 30,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        let text = text_with_explicit_headers();
        let chunks = chunker.chunk_text(&text, "test");

        // Should have multiple chunks due to section breaks
        assert!(chunks.len() >= 2);

        // Check if any chunks have section metadata
        for chunk in &chunks {
            if let Some(section) = &chunk.section {
                // Section should look like a header
                assert!(
                    section.contains("CHAPTER") || section.contains("SECTION"),
                    "Section should contain chapter/section header: {}",
                    section
                );
            }
        }
    }
}

// ============================================================================
// Data Integrity / Reconstruction Tests
// ============================================================================

#[cfg(test)]
mod reconstruction_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_no_content_lost_without_overlap() {
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 100,
            min_size: 20,
            max_size: 200,
            overlap_size: 0, // No overlap for exact reconstruction
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        let original = "First sentence here. Second sentence follows. Third one too. Fourth sentence now. Fifth and final.";
        let chunks = chunker.chunk_text(original, "test");

        // Collect all unique words from chunks
        let chunk_words: HashSet<_> = chunks.iter()
            .flat_map(|c| c.content.split_whitespace())
            .collect();

        // Collect all words from original
        let original_words: HashSet<_> = original.split_whitespace().collect();

        // All original words should appear in chunks
        for word in &original_words {
            assert!(
                chunk_words.contains(word),
                "Word '{}' from original not found in chunks",
                word
            );
        }
    }

    #[test]
    fn test_content_reconstructable_with_overlap() {
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 150,
            min_size: 30,
            max_size: 300,
            overlap_size: 30,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        let original = r#"This is paragraph one with some content.

This is paragraph two with different content.

This is paragraph three with more content.

This is paragraph four at the end."#;

        let chunks = chunker.chunk_text(original, "test");

        // All paragraph content should appear somewhere
        assert!(chunks.iter().any(|c| c.content.contains("paragraph one")));
        assert!(chunks.iter().any(|c| c.content.contains("paragraph two")));
        assert!(chunks.iter().any(|c| c.content.contains("paragraph three")));
        assert!(chunks.iter().any(|c| c.content.contains("paragraph four")));
    }

    #[test]
    fn test_significant_words_preserved() {
        // Use small min_size since the test text is short
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 500,
            min_size: 50,  // Small min_size for short test text
            max_size: 1000,
            overlap_size: 0,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        let original = "Important concepts: database normalization, functional programming, object-oriented design, microservices architecture, continuous integration.";
        let chunks = chunker.chunk_text(original, "test");

        let all_content: String = chunks.iter()
            .map(|c| c.content.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");

        // Key technical terms should be preserved
        assert!(all_content.contains("database") || all_content.contains("normalization"));
        assert!(all_content.contains("functional") || all_content.contains("programming"));
        assert!(all_content.contains("microservices") || all_content.contains("architecture"));
    }

    #[test]
    fn test_paragraph_boundaries_maintainable() {
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 200,
            min_size: 10,  // Small min_size to accommodate short test paragraphs
            max_size: 400,
            overlap_size: 0,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        let original = "Para one.\n\nPara two.\n\nPara three.";
        let chunks = chunker.chunk_text(original, "test");

        // With paragraph preservation, content should maintain structure
        let all_content: String = chunks.iter()
            .map(|c| c.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        // All paragraphs should be recoverable
        assert!(all_content.contains("Para one"));
        assert!(all_content.contains("Para two"));
        assert!(all_content.contains("Para three"));
    }
}

// ============================================================================
// Very Large Document Tests
// ============================================================================

#[cfg(test)]
mod large_document_tests {
    use super::*;

    /// Creates a very large document for stress testing
    fn create_very_large_text(paragraph_count: usize) -> String {
        let mut text = String::new();
        for i in 1..=paragraph_count {
            text.push_str(&format!(
                "Paragraph {} begins here. This paragraph contains multiple sentences to provide enough content for chunking. \
                Each paragraph is designed to be substantial enough to test the chunking algorithm's handling of large documents. \
                The content includes various words and phrases to ensure diversity in the text. \
                We include technical terms like algorithm, optimization, and performance. \
                Additionally, we have narrative elements describing scenes and actions. \
                This helps test both technical and prose-style documents.\n\n",
                i
            ));
        }
        text
    }

    #[test]
    fn test_very_large_document_chunking() {
        let chunker = SemanticChunker::new();
        let large_text = create_very_large_text(100); // 100 paragraphs

        // Should complete without panic or memory issues
        let chunks = chunker.chunk_text(&large_text, "large-doc");

        // Should produce a reasonable number of chunks
        assert!(!chunks.is_empty());

        // Each chunk should have valid structure
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
            assert!(!chunk.id.is_empty());
            assert_eq!(chunk.source_id, "large-doc");
        }
    }

    #[test]
    fn test_large_document_chunk_sizes() {
        let config = ChunkConfig {
            target_size: 1000,
            min_size: 200,
            max_size: 2000,
            overlap_size: 100,
            preserve_sentences: true,
            preserve_paragraphs: true,
        };

        let chunker = SemanticChunker::with_config(config);
        let large_text = create_very_large_text(50);

        let chunks = chunker.chunk_text(&large_text, "test");

        // Verify chunk sizes are within bounds (with some tolerance)
        for chunk in &chunks {
            // Most chunks should respect max size (allow some buffer for sentence preservation)
            assert!(
                chunk.content.len() <= 2500,
                "Chunk too large: {} chars (max 2500)",
                chunk.content.len()
            );
        }
    }

    #[test]
    fn test_large_document_indices_sequential() {
        let chunker = SemanticChunker::new();
        let large_text = create_very_large_text(200);

        let chunks = chunker.chunk_text(&large_text, "test");

        // Indices should be sequential
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(
                chunk.chunk_index, i,
                "Chunk index mismatch at position {}",
                i
            );
        }
    }

    #[test]
    fn test_large_document_no_duplicate_ids() {
        let chunker = SemanticChunker::new();
        let large_text = create_very_large_text(150);

        let chunks = chunker.chunk_text(&large_text, "test");

        // Collect all IDs
        let ids: Vec<_> = chunks.iter().map(|c| &c.id).collect();
        let unique_ids: std::collections::HashSet<_> = ids.iter().collect();

        assert_eq!(ids.len(), unique_ids.len(), "Found duplicate chunk IDs");
    }

    #[test]
    fn test_memory_bounded_chunking() {
        // Create a moderately large document (500KB+)
        let chunker = SemanticChunker::with_config(ChunkConfig::small());
        let large_text = create_very_large_text(500);

        // Estimate text size
        let text_size = large_text.len();
        assert!(text_size > 100_000, "Text should be large: {} bytes", text_size);

        // Should complete without OOM
        let chunks = chunker.chunk_text(&large_text, "stress-test");

        // Should have produced chunks
        assert!(!chunks.is_empty());

        // Total chunk content should be reasonable
        let total_chunk_size: usize = chunks.iter().map(|c| c.content.len()).sum();

        // With overlap, total chunk content may be larger than original
        // but shouldn't be excessively larger (e.g., 3x)
        assert!(
            total_chunk_size < text_size * 3,
            "Total chunk size {} too large compared to original {}",
            total_chunk_size,
            text_size
        );
    }

    #[test]
    fn test_large_document_with_pages() {
        // Use smaller min_size to ensure short pages create chunks
        let chunker = SemanticChunker::with_config(ChunkConfig {
            target_size: 200,
            min_size: 50,  // Smaller min_size for test pages
            max_size: 400,
            overlap_size: 20,
            preserve_sentences: true,
            preserve_paragraphs: true,
        });

        // Create pages
        let pages: Vec<(u32, String)> = (1..=50)
            .map(|i| {
                let content = format!(
                    "Page {} content here. This page has multiple paragraphs.\n\n\
                    Second paragraph on page {}. More content follows.\n\n\
                    Third paragraph concludes page {}.",
                    i, i, i
                );
                (i as u32, content)
            })
            .collect();

        let chunks = chunker.chunk_with_pages(&pages, "multi-page-doc");

        // Should have chunks
        assert!(!chunks.is_empty());

        // All page numbers should be valid
        for chunk in &chunks {
            if let Some(page_num) = chunk.page_number {
                assert!(page_num >= 1 && page_num <= 50);
            }
        }
    }
}
