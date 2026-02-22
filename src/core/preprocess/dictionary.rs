//! Corpus Dictionary Generation
//!
//! Generates frequency dictionaries from indexed TTRPG content for use with SymSpell.
//! These dictionaries improve typo correction for domain-specific terms.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use super::error::PreprocessResult;

/// Generates frequency dictionaries from indexed TTRPG content.
pub struct DictionaryGenerator {
    /// Frequency boost for corpus terms over English (default: 10x)
    pub domain_boost: u64,
    /// Minimum word length to include (default: 2)
    pub min_word_length: usize,
    /// Minimum frequency to include (default: 2)
    pub min_frequency: u64,
}

impl Default for DictionaryGenerator {
    fn default() -> Self {
        Self {
            domain_boost: 10,
            min_word_length: 2,
            min_frequency: 2,
        }
    }
}

impl DictionaryGenerator {
    /// Create a new generator with custom settings
    pub fn new(domain_boost: u64, min_word_length: usize, min_frequency: u64) -> Self {
        Self {
            domain_boost,
            min_word_length,
            min_frequency,
        }
    }

    /// Generate word frequency dictionary from document chunks.
    ///
    /// Queries all chunks from SurrealDB, tokenizes the content,
    /// counts word frequencies, and applies a domain boost.
    #[cfg(feature = "surrealdb-dict")]
    pub async fn build_corpus_dictionary(
        &self,
        db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
        output_path: &Path,
    ) -> PreprocessResult<usize> {
        use surrealdb::sql::Value;

        // Query all chunk content
        let result: Vec<HashMap<String, Value>> = db
            .query("SELECT content FROM chunk")
            .await
            .map_err(|e| PreprocessError::Database(e.to_string()))?
            .take(0)
            .map_err(|e| PreprocessError::Database(e.to_string()))?;

        // Count word frequencies
        let mut word_counts: HashMap<String, u64> = HashMap::new();

        for row in result {
            if let Some(Value::Strand(content)) = row.get("content") {
                self.tokenize_and_count(content.as_str(), &mut word_counts);
            }
        }

        // Write dictionary with domain boost
        self.write_dictionary(&word_counts, output_path)
    }

    /// Build corpus dictionary from an iterator of text content.
    /// This is the non-async version for use without SurrealDB.
    pub fn build_corpus_dictionary_from_iter<'a, I>(
        &self,
        content_iter: I,
        output_path: &Path,
    ) -> PreprocessResult<usize>
    where
        I: Iterator<Item = &'a str>,
    {
        let mut word_counts: HashMap<String, u64> = HashMap::new();

        for content in content_iter {
            self.tokenize_and_count(content, &mut word_counts);
        }

        self.write_dictionary(&word_counts, output_path)
    }

    /// Generate bigram frequency dictionary for compound word detection.
    #[cfg(feature = "surrealdb-dict")]
    pub async fn build_bigram_dictionary(
        &self,
        db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
        output_path: &Path,
    ) -> PreprocessResult<usize> {
        use surrealdb::sql::Value;

        let result: Vec<HashMap<String, Value>> = db
            .query("SELECT content FROM chunk")
            .await
            .map_err(|e| PreprocessError::Database(e.to_string()))?
            .take(0)
            .map_err(|e| PreprocessError::Database(e.to_string()))?;

        let mut bigram_counts: HashMap<String, u64> = HashMap::new();

        for row in result {
            if let Some(Value::Strand(content)) = row.get("content") {
                self.extract_bigrams(content.as_str(), &mut bigram_counts);
            }
        }

        self.write_bigram_dictionary(&bigram_counts, output_path)
    }

    /// Build bigram dictionary from an iterator of text content.
    pub fn build_bigram_dictionary_from_iter<'a, I>(
        &self,
        content_iter: I,
        output_path: &Path,
    ) -> PreprocessResult<usize>
    where
        I: Iterator<Item = &'a str>,
    {
        let mut bigram_counts: HashMap<String, u64> = HashMap::new();

        for content in content_iter {
            self.extract_bigrams(content, &mut bigram_counts);
        }

        self.write_bigram_dictionary(&bigram_counts, output_path)
    }

    /// Tokenize text and update word counts
    fn tokenize_and_count(&self, text: &str, counts: &mut HashMap<String, u64>) {
        for word in text.split(|c: char| !c.is_alphanumeric() && c != '\'') {
            let word_lower = word.to_lowercase();

            // Skip short words and numbers-only
            if word_lower.len() < self.min_word_length {
                continue;
            }
            if word_lower.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            *counts.entry(word_lower).or_insert(0) += 1;
        }
    }

    /// Extract word bigrams from text
    fn extract_bigrams(&self, text: &str, counts: &mut HashMap<String, u64>) {
        let words: Vec<&str> = text
            .split(|c: char| !c.is_alphanumeric() && c != '\'')
            .filter(|w| w.len() >= self.min_word_length)
            .collect();

        for window in words.windows(2) {
            let bigram = format!("{} {}", window[0].to_lowercase(), window[1].to_lowercase());
            *counts.entry(bigram).or_insert(0) += 1;
        }
    }

    /// Write word frequency dictionary to file in SymSpell format
    fn write_dictionary(
        &self,
        counts: &HashMap<String, u64>,
        output_path: &Path,
    ) -> PreprocessResult<usize> {
        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(output_path)?;
        let mut written = 0;

        for (word, count) in counts {
            if *count >= self.min_frequency {
                // Apply domain boost
                let boosted_count = count * self.domain_boost;
                writeln!(file, "{} {}", word, boosted_count)?;
                written += 1;
            }
        }

        Ok(written)
    }

    /// Write bigram dictionary to file
    fn write_bigram_dictionary(
        &self,
        counts: &HashMap<String, u64>,
        output_path: &Path,
    ) -> PreprocessResult<usize> {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(output_path)?;
        let mut written = 0;

        for (bigram, count) in counts {
            if *count >= self.min_frequency {
                let boosted_count = count * self.domain_boost;
                writeln!(file, "{} {}", bigram, boosted_count)?;
                written += 1;
            }
        }

        Ok(written)
    }

    /// Rebuild all dictionaries.
    /// Call this after bulk ingestion operations.
    #[cfg(feature = "surrealdb-dict")]
    pub async fn rebuild_all(
        &self,
        db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
        data_dir: &Path,
    ) -> PreprocessResult<()> {
        let corpus_path = data_dir.join("ttrpg_corpus.txt");
        let bigram_path = data_dir.join("ttrpg_bigrams.txt");

        let corpus_count = self.build_corpus_dictionary(db, &corpus_path).await?;
        let bigram_count = self.build_bigram_dictionary(db, &bigram_path).await?;

        log::info!(
            "Dictionary rebuild complete: {} words, {} bigrams",
            corpus_count,
            bigram_count
        );

        Ok(())
    }
}

/// Statistics from a dictionary rebuild operation
#[derive(Debug, Clone)]
pub struct RebuildStats {
    /// Number of unique words in corpus dictionary
    pub word_count: usize,
    /// Number of bigrams in bigram dictionary
    pub bigram_count: usize,
    /// Total documents processed
    pub documents_processed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::TempDir;

    #[test]
    fn test_tokenize_and_count() {
        let generator = DictionaryGenerator::default();
        let mut counts = HashMap::new();

        generator.tokenize_and_count("The quick brown fox jumps", &mut counts);

        assert!(counts.contains_key("quick"));
        assert!(counts.contains_key("brown"));
        assert!(counts.contains_key("jumps"));
        // "The" becomes "the"
        assert!(counts.contains_key("the"));
    }

    #[test]
    fn test_extract_bigrams() {
        let generator = DictionaryGenerator::default();
        let mut counts = HashMap::new();

        generator.extract_bigrams("magic missile fireball", &mut counts);

        assert!(counts.contains_key("magic missile"));
        assert!(counts.contains_key("missile fireball"));
    }

    #[test]
    fn test_write_dictionary() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("test_dict.txt");

        let generator = DictionaryGenerator {
            domain_boost: 10,
            min_word_length: 2,
            min_frequency: 1,
        };

        let mut counts = HashMap::new();
        counts.insert("fireball".to_string(), 5);
        counts.insert("magic".to_string(), 3);

        let written = generator.write_dictionary(&counts, &output_path).unwrap();
        assert_eq!(written, 2);

        // Read and verify content
        let mut content = String::new();
        std::fs::File::open(&output_path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        // Counts should be boosted by 10x
        assert!(content.contains("fireball 50"));
        assert!(content.contains("magic 30"));
    }

    #[test]
    fn test_build_corpus_dictionary_from_iter() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("corpus.txt");

        let generator = DictionaryGenerator {
            domain_boost: 1,
            min_word_length: 2,
            min_frequency: 1,
        };

        let documents = vec![
            "Cast fireball at the dragon",
            "The dragon breathes fire",
            "Roll for initiative against the dragon",
        ];

        let count = generator
            .build_corpus_dictionary_from_iter(documents.iter().copied(), &output_path)
            .unwrap();

        assert!(count > 0);

        let mut content = String::new();
        std::fs::File::open(&output_path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        // "dragon" appears 3 times
        assert!(content.contains("dragon 3"));
        // "the" appears multiple times
        assert!(content.contains("the"));
    }

    #[test]
    fn test_min_frequency_filter() {
        let temp_dir = TempDir::new().unwrap();
        let output_path = temp_dir.path().join("filtered.txt");

        let generator = DictionaryGenerator {
            domain_boost: 1,
            min_word_length: 2,
            min_frequency: 3, // Require at least 3 occurrences
        };

        let documents = vec![
            "dragon dragon dragon", // 3 occurrences
            "goblin goblin",        // 2 occurrences
            "orc",                  // 1 occurrence
        ];

        let count = generator
            .build_corpus_dictionary_from_iter(documents.iter().copied(), &output_path)
            .unwrap();

        let mut content = String::new();
        std::fs::File::open(&output_path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        // Only "dragon" should be included
        assert!(content.contains("dragon"));
        assert!(!content.contains("goblin"));
        assert!(!content.contains("orc"));
        assert_eq!(count, 1);
    }
}
