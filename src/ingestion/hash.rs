//! File Hashing Utility Module
//!
//! Provides BLAKE3 hashing for duplicate detection during document ingestion.
//! BLAKE3 is chosen for its excellent performance and security properties.

use std::path::Path;
use std::io::{self, Read};
use std::fs::File;

// ============================================================================
// Constants
// ============================================================================

/// Buffer size for streaming file hashing (64KB)
const HASH_BUFFER_SIZE: usize = 64 * 1024;

// ============================================================================
// Public Functions
// ============================================================================

/// Compute BLAKE3 hash of a file.
///
/// Uses streaming to handle large files efficiently without loading
/// the entire file into memory.
///
/// # Arguments
/// * `path` - Path to the file to hash
///
/// # Returns
/// * `io::Result<String>` - The hex-encoded BLAKE3 hash (64 characters)
///
/// # Example
/// ```ignore
/// let hash = hash_file(Path::new("document.pdf"))?;
/// println!("File hash: {}", hash);  // 64-character hex string
/// ```
pub fn hash_file(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mut reader = io::BufReader::with_capacity(HASH_BUFFER_SIZE, file);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; HASH_BUFFER_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Compute BLAKE3 hash of a byte slice.
///
/// Useful for hashing in-memory content without writing to disk.
///
/// # Arguments
/// * `bytes` - The byte slice to hash
///
/// # Returns
/// * `String` - The hex-encoded BLAKE3 hash (64 characters)
///
/// # Example
/// ```ignore
/// let content = b"Hello, world!";
/// let hash = hash_bytes(content);
/// ```
pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Get file size in bytes.
///
/// Utility function often used alongside hashing for duplicate detection.
///
/// # Arguments
/// * `path` - Path to the file
///
/// # Returns
/// * `io::Result<u64>` - The file size in bytes
pub fn get_file_size(path: &Path) -> io::Result<u64> {
    Ok(std::fs::metadata(path)?.len())
}

/// Hash file with size for quick comparison.
///
/// Returns both the hash and file size, which can be used for efficient
/// duplicate detection (compare size first, then hash if sizes match).
///
/// # Arguments
/// * `path` - Path to the file
///
/// # Returns
/// * `io::Result<(String, u64)>` - Tuple of (hash, file_size)
pub fn hash_file_with_size(path: &Path) -> io::Result<(String, u64)> {
    let size = get_file_size(path)?;
    let hash = hash_file(path)?;
    Ok((hash, size))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_bytes() {
        let content = b"Hello, World!";
        let hash = hash_bytes(content);

        // BLAKE3 produces 64-character hex strings
        assert_eq!(hash.len(), 64);

        // Same content should produce same hash
        let hash2 = hash_bytes(content);
        assert_eq!(hash, hash2);

        // Different content should produce different hash
        let hash3 = hash_bytes(b"Different content");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_hash_file() -> io::Result<()> {
        // Create a temp file with known content
        let mut file = NamedTempFile::new()?;
        let content = b"Test file content for hashing";
        file.write_all(content)?;
        file.flush()?;

        let file_hash = hash_file(file.path())?;
        let bytes_hash = hash_bytes(content);

        // File hash should match bytes hash of same content
        assert_eq!(file_hash, bytes_hash);

        Ok(())
    }

    #[test]
    fn test_hash_file_with_size() -> io::Result<()> {
        let mut file = NamedTempFile::new()?;
        let content = b"Content for size test";
        file.write_all(content)?;
        file.flush()?;

        let (hash, size) = hash_file_with_size(file.path())?;

        assert_eq!(size, content.len() as u64);
        assert_eq!(hash.len(), 64);

        Ok(())
    }

    #[test]
    fn test_empty_file() -> io::Result<()> {
        let file = NamedTempFile::new()?;
        let hash = hash_file(file.path())?;

        // Empty file should still produce a valid hash
        assert_eq!(hash.len(), 64);
        // BLAKE3 hash of empty input
        assert_eq!(hash, hash_bytes(&[]));

        Ok(())
    }

    #[test]
    fn test_hash_determinism() {
        // Ensure hashing is deterministic
        let content = b"Determinism test content";
        let hashes: Vec<_> = (0..10).map(|_| hash_bytes(content)).collect();

        // All hashes should be identical
        for hash in &hashes {
            assert_eq!(hash, &hashes[0]);
        }
    }
}
