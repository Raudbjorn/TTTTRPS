//! Dictionary Path Resolution
//!
//! Provides functions to locate dictionaries at runtime, handling both
//! development mode (files in source tree) and production mode (bundled resources).

use std::path::{Path, PathBuf};

/// English frequency dictionary filename (82,765 entries from SymSpell)
pub const ENGLISH_DICT_FILENAME: &str = "frequency_dictionary_en_82_765.txt";

/// TTRPG corpus dictionary filename (generated from indexed content)
pub const CORPUS_DICT_FILENAME: &str = "ttrpg_corpus.txt";

/// Bigram dictionary filename for compound word detection
pub const BIGRAM_DICT_FILENAME: &str = "ttrpg_bigrams.txt";

/// Resolves the path to the English frequency dictionary.
///
/// Checks locations in order:
/// 1. Resource directory (if provided)
/// 2. Development path (`data/`)
/// 3. User data directory fallback
///
/// Returns `None` if the dictionary cannot be found.
pub fn get_english_dictionary_path(resource_dir: Option<&Path>) -> Option<PathBuf> {
    get_dictionary_path(ENGLISH_DICT_FILENAME, resource_dir)
}

/// Resolves the path to the TTRPG corpus dictionary.
///
/// The corpus dictionary is generated from indexed content and stored
/// in the user's data directory (not bundled with the app).
pub fn get_corpus_dictionary_path() -> Option<PathBuf> {
    get_user_data_dir().map(|dir| dir.join(CORPUS_DICT_FILENAME))
}

/// Resolves the path to the bigram dictionary.
///
/// Like the corpus dictionary, bigrams are generated and stored in user data.
pub fn get_bigram_dictionary_path() -> Option<PathBuf> {
    get_user_data_dir().map(|dir| dir.join(BIGRAM_DICT_FILENAME))
}

/// Generic dictionary path resolver.
///
/// Searches for a dictionary file in multiple locations:
/// 1. Resource directory (bundled resources)
/// 2. Development source tree
/// 3. User data directory
fn get_dictionary_path(filename: &str, resource_dir: Option<&Path>) -> Option<PathBuf> {
    // 1. Try bundled resource path
    if let Some(res_dir) = resource_dir {
        let bundled_path = res_dir.join("data").join(filename);
        if bundled_path.exists() {
            return Some(bundled_path);
        }
        let flat_path = res_dir.join(filename);
        if flat_path.exists() {
            return Some(flat_path);
        }
    }

    // 2. Try development path (relative to project root)
    let dev_paths = [
        PathBuf::from("data").join(filename),
        PathBuf::from("resources").join(filename),
        // Absolute path based on environment variable if set
        std::env::var("CARGO_MANIFEST_DIR")
            .map(|dir| PathBuf::from(dir).join("data").join(filename))
            .unwrap_or_default(),
    ];

    for path in &dev_paths {
        if path.exists() {
            return Some(path.clone());
        }
    }

    // 3. Try user data directory (for dictionaries that might be installed separately)
    if let Some(data_dir) = get_user_data_dir() {
        let user_path = data_dir.join(filename);
        if user_path.exists() {
            return Some(user_path);
        }
    }

    None
}

/// Gets the user data directory for TTRPG Assistant.
///
/// This is where generated dictionaries (corpus, bigrams) are stored.
/// Returns `~/.local/share/ttrpg-assistant/` on Linux/macOS,
/// `%APPDATA%\ttrpg-assistant\` on Windows.
pub fn get_user_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("ttrpg-assistant"))
}

/// Ensures the user data directory exists.
///
/// Creates the directory if it doesn't exist.
/// Returns the path to the directory.
pub fn ensure_user_data_dir() -> std::io::Result<PathBuf> {
    let dir = get_user_data_dir().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine user data directory",
        )
    })?;

    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }

    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert!(ENGLISH_DICT_FILENAME.ends_with(".txt"));
        assert!(CORPUS_DICT_FILENAME.ends_with(".txt"));
        assert!(BIGRAM_DICT_FILENAME.ends_with(".txt"));
    }

    #[test]
    fn test_user_data_dir() {
        // Should return Some on most systems
        let dir = get_user_data_dir();
        assert!(dir.is_some());
        let dir = dir.unwrap();
        assert!(dir.to_string_lossy().contains("ttrpg-assistant"));
    }

    #[test]
    fn test_corpus_dictionary_path() {
        let path = get_corpus_dictionary_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains(CORPUS_DICT_FILENAME));
    }

    #[test]
    fn test_bigram_dictionary_path() {
        let path = get_bigram_dictionary_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains(BIGRAM_DICT_FILENAME));
    }

    #[test]
    fn test_dev_path_resolution() {
        let path = get_english_dictionary_path(None);
        // May or may not exist depending on test environment
        if let Some(p) = path {
            assert!(p.exists());
            assert!(p.to_string_lossy().contains(ENGLISH_DICT_FILENAME));
        }
    }
}
