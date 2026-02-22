//! Async File Loading Utilities
//!
//! Provides async file I/O operations for loading vocabulary banks, dialect
//! definitions, and name components from YAML files.
//!
//! CRITICAL: Uses `tokio::fs` for all file I/O to avoid blocking the async runtime.

use std::path::{Path, PathBuf};
use serde::de::DeserializeOwned;
use tokio::fs;

use super::errors::FileError;

// ============================================================================
// Type Aliases
// ============================================================================

/// Result type for file operations.
pub type FileResult<T> = std::result::Result<T, FileError>;

// ============================================================================
// YAML File Loading
// ============================================================================

/// Load and parse a YAML file asynchronously.
///
/// # Arguments
/// * `path` - Path to the YAML file
///
/// # Returns
/// * `Ok(T)` - Parsed data structure
/// * `Err(FileError)` - If file cannot be read or parsed
///
/// # Example
/// ```ignore
/// use crate::core::npc_gen::file_utils::load_yaml_file;
///
/// #[derive(serde::Deserialize)]
/// struct Config {
///     name: String,
///     values: Vec<String>,
/// }
///
/// async fn load_config() -> Result<Config, FileError> {
///     load_yaml_file("/path/to/config.yaml").await
/// }
/// ```
pub async fn load_yaml_file<T: DeserializeOwned>(path: impl AsRef<Path>) -> FileResult<T> {
    let path = path.as_ref();

    // Read file content asynchronously (handles not found via error)
    let content = fs::read_to_string(path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            FileError::not_found(path)
        } else {
            FileError::read_failed(path, e)
        }
    })?;

    // Parse YAML
    serde_yaml_ng::from_str(&content).map_err(|e| FileError::parse_failed(path, "YAML", e))
}

/// Load and parse a YAML file, returning a default value if the file doesn't exist.
///
/// # Arguments
/// * `path` - Path to the YAML file
/// * `default` - Default value to return if file doesn't exist
///
/// # Returns
/// * `Ok(T)` - Parsed data structure or default value
/// * `Err(FileError)` - If file exists but cannot be read or parsed
pub async fn load_yaml_file_or_default<T: DeserializeOwned + Default>(
    path: impl AsRef<Path>,
) -> FileResult<T> {
    let path = path.as_ref();

    match fs::try_exists(path).await {
        Ok(false) => {
            log::debug!("File not found, using default: {}", path.display());
            return Ok(T::default());
        }
        Err(e) => {
            log::debug!("Error checking file existence, using default: {} ({})", path.display(), e);
            return Ok(T::default());
        }
        Ok(true) => {}
    }

    load_yaml_file(path).await
}

/// Load and parse a YAML file with a custom default value.
///
/// # Arguments
/// * `path` - Path to the YAML file
/// * `default` - Closure that returns the default value
///
/// # Returns
/// * `Ok(T)` - Parsed data structure or result of default closure
/// * `Err(FileError)` - If file exists but cannot be read or parsed
pub async fn load_yaml_file_or_else<T, F>(path: impl AsRef<Path>, default: F) -> FileResult<T>
where
    T: DeserializeOwned,
    F: FnOnce() -> T,
{
    let path = path.as_ref();

    match fs::try_exists(path).await {
        Ok(false) => {
            log::debug!("File not found, using provided default: {}", path.display());
            return Ok(default());
        }
        Err(e) => {
            log::debug!("Error checking file existence, using default: {} ({})", path.display(), e);
            return Ok(default());
        }
        Ok(true) => {}
    }

    load_yaml_file(path).await
}

// ============================================================================
// Directory Scanning
// ============================================================================

/// Scan a directory for YAML files and return their paths.
///
/// # Arguments
/// * `dir_path` - Path to the directory to scan
/// * `recursive` - Whether to scan subdirectories
///
/// # Returns
/// * `Ok(Vec<PathBuf>)` - List of YAML file paths found
/// * `Err(FileError)` - If directory cannot be read
///
/// # Example
/// ```ignore
/// use crate::core::npc_gen::file_utils::scan_yaml_directory;
///
/// async fn find_all_configs() -> Result<Vec<PathBuf>, FileError> {
///     scan_yaml_directory("/path/to/configs", true).await
/// }
/// ```
pub async fn scan_yaml_directory(
    dir_path: impl AsRef<Path>,
    recursive: bool,
) -> FileResult<Vec<PathBuf>> {
    let dir_path = dir_path.as_ref();

    // Use async metadata check instead of blocking exists()/is_dir()
    match fs::metadata(dir_path).await {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::warn!("Directory not found: {}", dir_path.display());
            return Ok(Vec::new());
        }
        Err(e) => {
            return Err(FileError::ScanFailed {
                path: dir_path.to_path_buf(),
                source: e,
            });
        }
        Ok(meta) if !meta.is_dir() => {
            return Err(FileError::ScanFailed {
                path: dir_path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotADirectory, "Not a directory"),
            });
        }
        Ok(_) => {}
    }

    let mut yaml_files = Vec::new();
    scan_directory_inner(dir_path, recursive, &mut yaml_files).await?;

    // Sort for consistent ordering
    yaml_files.sort();

    log::debug!(
        "Found {} YAML files in {}",
        yaml_files.len(),
        dir_path.display()
    );

    Ok(yaml_files)
}

/// Internal recursive directory scanner.
async fn scan_directory_inner(
    dir_path: &Path,
    recursive: bool,
    results: &mut Vec<PathBuf>,
) -> FileResult<()> {
    let mut entries = fs::read_dir(dir_path)
        .await
        .map_err(|e| FileError::scan_failed(dir_path, e))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| FileError::scan_failed(dir_path, e))?
    {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .await
            .map_err(|e| FileError::scan_failed(&path, e))?;

        if file_type.is_dir() && recursive {
            // Use Box::pin for recursive async call
            Box::pin(scan_directory_inner(&path, recursive, results)).await?;
        } else if file_type.is_file() && is_yaml_file(&path) {
            results.push(path);
        }
    }

    Ok(())
}

/// Check if a path has a YAML file extension.
fn is_yaml_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml"))
        .unwrap_or(false)
}

/// Load all YAML files from a directory into a collection.
///
/// # Arguments
/// * `dir_path` - Path to the directory to scan
/// * `recursive` - Whether to scan subdirectories
///
/// # Returns
/// * `Ok(Vec<(PathBuf, T)>)` - List of (path, parsed content) tuples
/// * Files that fail to parse are logged and skipped
///
/// # Example
/// ```ignore
/// use crate::core::npc_gen::file_utils::load_all_yaml_files;
///
/// #[derive(serde::Deserialize)]
/// struct VocabBank {
///     id: String,
///     phrases: Vec<String>,
/// }
///
/// async fn load_all_banks() -> Vec<(PathBuf, VocabBank)> {
///     load_all_yaml_files("/path/to/banks", true)
///         .await
///         .unwrap_or_default()
/// }
/// ```
pub async fn load_all_yaml_files<T: DeserializeOwned>(
    dir_path: impl AsRef<Path>,
    recursive: bool,
) -> FileResult<Vec<(PathBuf, T)>> {
    let paths = scan_yaml_directory(dir_path, recursive).await?;
    let mut results = Vec::with_capacity(paths.len());

    for path in paths {
        match load_yaml_file::<T>(&path).await {
            Ok(data) => {
                results.push((path, data));
            }
            Err(e) => {
                log::warn!("Failed to load YAML file {}: {}", path.display(), e);
                // Continue with other files
            }
        }
    }

    Ok(results)
}

// ============================================================================
// File Existence Checks
// ============================================================================

/// Check if a file exists asynchronously.
///
/// This is a thin wrapper around `tokio::fs::metadata` that only checks existence.
pub async fn file_exists(path: impl AsRef<Path>) -> bool {
    fs::metadata(path.as_ref()).await.is_ok()
}

/// Check if a directory exists asynchronously.
pub async fn directory_exists(path: impl AsRef<Path>) -> bool {
    fs::metadata(path.as_ref())
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false)
}

// ============================================================================
// Path Utilities
// ============================================================================

/// Resolve a path relative to a base directory.
///
/// If the path is already absolute, it is returned as-is.
/// Otherwise, it is joined with the base directory.
pub fn resolve_path(base: impl AsRef<Path>, relative: impl AsRef<Path>) -> PathBuf {
    let relative = relative.as_ref();
    if relative.is_absolute() {
        relative.to_path_buf()
    } else {
        base.as_ref().join(relative)
    }
}

/// Extract the stem (filename without extension) from a path.
pub fn extract_stem(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Get the data directory for NPC generation resources.
///
/// Returns the path to `~/.local/share/ttrpg-assistant/npc_gen/` on Linux,
/// or the equivalent on other platforms.
pub fn get_npc_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ttrpg-assistant")
        .join("npc_gen")
}

/// Get the vocabulary banks directory.
pub fn get_vocabulary_dir() -> PathBuf {
    get_npc_data_dir().join("vocabulary")
}

/// Get the dialects directory.
pub fn get_dialects_dir() -> PathBuf {
    get_npc_data_dir().join("dialects")
}

/// Get the name components directory.
pub fn get_names_dir() -> PathBuf {
    get_npc_data_dir().join("names")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use tempfile::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncWriteExt;

    #[derive(Debug, Deserialize, PartialEq, Default)]
    struct TestConfig {
        name: String,
        #[serde(default)]
        values: Vec<String>,
    }

    async fn create_temp_yaml(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = File::create(&path).await.unwrap();
        file.write_all(content.as_bytes()).await.unwrap();
        path
    }

    #[tokio::test]
    async fn test_load_yaml_file() {
        let temp = TempDir::new().unwrap();
        let yaml_content = r#"
name: "test"
values:
  - "one"
  - "two"
"#;
        let path = create_temp_yaml(&temp, "config.yaml", yaml_content).await;

        let config: TestConfig = load_yaml_file(&path).await.unwrap();
        assert_eq!(config.name, "test");
        assert_eq!(config.values, vec!["one", "two"]);
    }

    #[tokio::test]
    async fn test_load_yaml_file_not_found() {
        let result: FileResult<TestConfig> = load_yaml_file("/nonexistent/path.yaml").await;
        assert!(matches!(result, Err(FileError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_load_yaml_file_parse_error() {
        let temp = TempDir::new().unwrap();
        let path = create_temp_yaml(&temp, "invalid.yaml", "{ invalid yaml").await;

        let result: FileResult<TestConfig> = load_yaml_file(&path).await;
        assert!(matches!(result, Err(FileError::ParseFailed { .. })));
    }

    #[tokio::test]
    async fn test_load_yaml_file_or_default() {
        let result: TestConfig = load_yaml_file_or_default("/nonexistent/path.yaml")
            .await
            .unwrap();
        assert_eq!(result, TestConfig::default());
    }

    #[tokio::test]
    async fn test_scan_yaml_directory() {
        let temp = TempDir::new().unwrap();

        // Create some YAML files
        create_temp_yaml(&temp, "one.yaml", "name: one").await;
        create_temp_yaml(&temp, "two.yml", "name: two").await;
        create_temp_yaml(&temp, "three.txt", "not yaml").await;

        let paths = scan_yaml_directory(temp.path(), false).await.unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|p| p.ends_with("one.yaml")));
        assert!(paths.iter().any(|p| p.ends_with("two.yml")));
    }

    #[tokio::test]
    async fn test_scan_yaml_directory_recursive() {
        let temp = TempDir::new().unwrap();

        // Create nested structure
        let subdir = temp.path().join("subdir");
        fs::create_dir(&subdir).await.unwrap();

        create_temp_yaml(&temp, "root.yaml", "name: root").await;
        let mut file = File::create(subdir.join("nested.yaml")).await.unwrap();
        file.write_all(b"name: nested").await.unwrap();

        let paths = scan_yaml_directory(temp.path(), true).await.unwrap();
        assert_eq!(paths.len(), 2);
    }

    #[tokio::test]
    async fn test_scan_yaml_directory_nonexistent() {
        let paths = scan_yaml_directory("/nonexistent/path", false)
            .await
            .unwrap();
        assert!(paths.is_empty());
    }

    #[tokio::test]
    async fn test_load_all_yaml_files() {
        let temp = TempDir::new().unwrap();

        create_temp_yaml(&temp, "one.yaml", "name: one\nvalues: [a]").await;
        create_temp_yaml(&temp, "two.yaml", "name: two\nvalues: [b]").await;
        create_temp_yaml(&temp, "invalid.yaml", "{ broken").await;

        let results: Vec<(PathBuf, TestConfig)> =
            load_all_yaml_files(temp.path(), false).await.unwrap();

        // Should have 2 valid files (invalid is skipped with warning)
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_is_yaml_file() {
        assert!(is_yaml_file(Path::new("test.yaml")));
        assert!(is_yaml_file(Path::new("test.yml")));
        assert!(is_yaml_file(Path::new("test.YAML")));
        assert!(is_yaml_file(Path::new("test.YML")));
        assert!(!is_yaml_file(Path::new("test.json")));
        assert!(!is_yaml_file(Path::new("test.txt")));
        assert!(!is_yaml_file(Path::new("test")));
    }

    #[test]
    fn test_resolve_path() {
        let base = PathBuf::from("/home/user/data");

        // Relative path should be joined
        let result = resolve_path(&base, "subdir/file.yaml");
        assert_eq!(result, PathBuf::from("/home/user/data/subdir/file.yaml"));

        // Absolute path should be returned as-is
        let result = resolve_path(&base, "/other/path/file.yaml");
        assert_eq!(result, PathBuf::from("/other/path/file.yaml"));
    }

    #[test]
    fn test_extract_stem() {
        assert_eq!(extract_stem("path/to/file.yaml"), Some("file".to_string()));
        assert_eq!(
            extract_stem("complex.name.yaml"),
            Some("complex.name".to_string())
        );
        assert_eq!(extract_stem("noextension"), Some("noextension".to_string()));
    }

    #[tokio::test]
    async fn test_file_exists() {
        let temp = TempDir::new().unwrap();
        let path = create_temp_yaml(&temp, "exists.yaml", "test: true").await;

        assert!(file_exists(&path).await);
        assert!(!file_exists("/nonexistent/file.yaml").await);
    }

    #[tokio::test]
    async fn test_directory_exists() {
        let temp = TempDir::new().unwrap();

        assert!(directory_exists(temp.path()).await);
        assert!(!directory_exists("/nonexistent/directory").await);
    }
}
