//! Database Backup and Restore
//!
//! Provides backup creation, restoration, and management functionality.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Information about a backup file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub filename: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub created_at: String,
    pub description: Option<String>,
}

/// Create a backup of the database
///
/// Returns the path to the backup file
pub fn create_backup(
    db_path: &Path,
    backup_dir: &Path,
    description: Option<String>,
) -> Result<BackupInfo, BackupError> {
    // Ensure backup directory exists
    fs::create_dir_all(backup_dir)
        .map_err(|e| BackupError::IoError(format!("Failed to create backup directory: {}", e)))?;

    // Generate backup filename with timestamp
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_filename = format!("ttrpg_backup_{}.db", timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    // Verify source database exists
    if !db_path.exists() {
        return Err(BackupError::SourceNotFound(db_path.display().to_string()));
    }

    // Copy database file
    fs::copy(db_path, &backup_path)
        .map_err(|e| BackupError::IoError(format!("Failed to copy database: {}", e)))?;

    // Also copy WAL file if it exists
    let wal_path = db_path.with_extension("db-wal");
    if wal_path.exists() {
        let backup_wal = backup_path.with_extension("db-wal");
        fs::copy(&wal_path, &backup_wal).ok();
    }

    // Copy SHM file if it exists
    let shm_path = db_path.with_extension("db-shm");
    if shm_path.exists() {
        let backup_shm = backup_path.with_extension("db-shm");
        fs::copy(&shm_path, &backup_shm).ok();
    }

    // Get file size
    let metadata = fs::metadata(&backup_path)
        .map_err(|e| BackupError::IoError(format!("Failed to get backup metadata: {}", e)))?;

    // Save description if provided
    if let Some(ref desc) = description {
        let meta_path = backup_path.with_extension("meta");
        fs::write(&meta_path, desc).ok();
    }

    let info = BackupInfo {
        filename: backup_filename,
        path: backup_path,
        size_bytes: metadata.len(),
        created_at: chrono::Utc::now().to_rfc3339(),
        description,
    };

    info!(
        backup_path = %info.path.display(),
        size_bytes = info.size_bytes,
        "Database backup created"
    );

    Ok(info)
}

/// Restore database from a backup
pub fn restore_backup(
    backup_path: &Path,
    db_path: &Path,
) -> Result<(), BackupError> {
    // Verify backup exists
    if !backup_path.exists() {
        return Err(BackupError::BackupNotFound(backup_path.display().to_string()));
    }

    // Create a safety backup of current database before restoring
    if db_path.exists() {
        let safety_backup = db_path.with_extension("db.pre-restore");
        if let Err(e) = fs::copy(db_path, &safety_backup) {
            warn!("Failed to create safety backup: {}", e);
        }
    }

    // Remove current database files
    if db_path.exists() {
        fs::remove_file(db_path)
            .map_err(|e| BackupError::IoError(format!("Failed to remove current database: {}", e)))?;
    }

    // Remove WAL and SHM files
    let wal_path = db_path.with_extension("db-wal");
    let shm_path = db_path.with_extension("db-shm");
    fs::remove_file(&wal_path).ok();
    fs::remove_file(&shm_path).ok();

    // Copy backup to database location
    fs::copy(backup_path, db_path)
        .map_err(|e| BackupError::IoError(format!("Failed to restore backup: {}", e)))?;

    // Copy backup WAL if it exists
    let backup_wal = backup_path.with_extension("db-wal");
    if backup_wal.exists() {
        fs::copy(&backup_wal, &wal_path).ok();
    }

    info!(
        backup_path = %backup_path.display(),
        db_path = %db_path.display(),
        "Database restored from backup"
    );

    Ok(())
}

/// List all available backups
pub fn list_backups(backup_dir: &Path) -> Result<Vec<BackupInfo>, BackupError> {
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();

    let entries = fs::read_dir(backup_dir)
        .map_err(|e| BackupError::IoError(format!("Failed to read backup directory: {}", e)))?;

    for entry in entries.flatten() {
        let path = entry.path();

        // Only process .db files
        if path.extension().and_then(|s| s.to_str()) != Some("db") {
            continue;
        }

        let filename = path.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        // Only include our backup files
        if !filename.starts_with("ttrpg_backup_") {
            continue;
        }

        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        // Try to read description from meta file
        let meta_path = path.with_extension("meta");
        let description = fs::read_to_string(&meta_path).ok();

        // Extract timestamp from filename
        let created_at = extract_timestamp_from_filename(&filename)
            .unwrap_or_else(|| {
                metadata.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0)
                        .unwrap_or_default()
                        .to_rfc3339())
                    .unwrap_or_else(|| "Unknown".to_string())
            });

        backups.push(BackupInfo {
            filename,
            path,
            size_bytes: metadata.len(),
            created_at,
            description,
        });
    }

    // Sort by creation time (newest first)
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(backups)
}

/// Extract timestamp from backup filename
fn extract_timestamp_from_filename(filename: &str) -> Option<String> {
    // Format: ttrpg_backup_20241227_120000.db
    let parts: Vec<&str> = filename.trim_end_matches(".db").split('_').collect();
    if parts.len() >= 4 {
        let date = parts[2];
        let time = parts[3];
        if date.len() == 8 && time.len() == 6 {
            return Some(format!(
                "{}-{}-{}T{}:{}:{}Z",
                &date[0..4], &date[4..6], &date[6..8],
                &time[0..2], &time[2..4], &time[4..6]
            ));
        }
    }
    None
}

/// Backup error types
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("Source database not found: {0}")]
    SourceNotFound(String),

    #[error("Backup not found: {0}")]
    BackupNotFound(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_timestamp() {
        let filename = "ttrpg_backup_20241227_143052.db";
        let result = extract_timestamp_from_filename(filename);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "2024-12-27T14:30:52Z");
    }

    #[test]
    fn test_backup_and_restore() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let backup_dir = temp_dir.path().join("backups");

        // Create a dummy database file
        fs::write(&db_path, b"test database content").unwrap();

        // Create backup
        let backup_info = create_backup(&db_path, &backup_dir, Some("Test backup".to_string()))
            .expect("Backup should succeed");

        assert!(backup_info.path.exists());
        assert_eq!(backup_info.description, Some("Test backup".to_string()));

        // Modify the original database
        fs::write(&db_path, b"modified content").unwrap();

        // Restore backup
        restore_backup(&backup_info.path, &db_path).expect("Restore should succeed");

        // Verify content was restored
        let content = fs::read_to_string(&db_path).unwrap();
        assert_eq!(content, "test database content");
    }
}
