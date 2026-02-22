//! Data models for storage operations.
//!
//! Contains struct definitions and CRUD operations for library items and chunks.
//! Library items represent documents in the TTRPG library, while chunks are
//! searchable segments of those documents.
//!
//! ## Tasks Implemented
//!
//! - 3.2.1: Library item CRUD (FR-8.2)
//! - 3.2.2: List with pagination and chunk counts

use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

use super::error::StorageError;

// ============================================================================
// Library Item Models
// ============================================================================

/// Library item (document) record.
///
/// Represents a document in the TTRPG library, such as a rulebook, module,
/// or campaign setting. Each library item can have multiple chunks for search.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LibraryItem {
    /// Unique identifier (SurrealDB record ID without table prefix)
    #[serde(default)]
    pub id: Option<String>,
    /// URL-friendly unique identifier
    pub slug: String,
    /// Human-readable title
    pub title: String,
    /// Path to the source file on disk
    #[serde(default)]
    pub file_path: Option<String>,
    /// File type (pdf, epub, docx, etc.)
    #[serde(default)]
    pub file_type: Option<String>,
    /// File size in bytes
    #[serde(default)]
    pub file_size: Option<i64>,
    /// Number of pages (for PDFs)
    #[serde(default)]
    pub page_count: Option<i32>,
    /// Game system name (e.g., "Pathfinder 2e", "D&D 5e")
    #[serde(default)]
    pub game_system: Option<String>,
    /// Game system identifier for filtering
    #[serde(default)]
    pub game_system_id: Option<String>,
    /// Content category (rulebook, adventure, setting, etc.)
    #[serde(default)]
    pub content_category: Option<String>,
    /// Publisher name
    #[serde(default)]
    pub publisher: Option<String>,
    /// Processing status (pending, processing, ready, error)
    pub status: String,
    /// Error message if status is "error"
    #[serde(default)]
    pub error_message: Option<String>,
    /// Creation timestamp (ISO 8601)
    #[serde(default)]
    pub created_at: Option<String>,
    /// Last update timestamp (ISO 8601)
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Additional metadata as JSON
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

impl LibraryItem {
    /// Create a new library item with minimal required fields.
    pub fn new(slug: String, title: String) -> Self {
        Self {
            id: None,
            slug,
            title,
            file_path: None,
            file_type: None,
            file_size: None,
            page_count: None,
            game_system: None,
            game_system_id: None,
            content_category: None,
            publisher: None,
            status: "pending".to_string(),
            error_message: None,
            created_at: None,
            updated_at: None,
            metadata: None,
        }
    }

    /// Create a builder for constructing a library item.
    pub fn builder(slug: String, title: String) -> LibraryItemBuilder {
        LibraryItemBuilder::new(slug, title)
    }
}

/// Builder for creating library items with optional fields.
#[derive(Clone, Debug)]
pub struct LibraryItemBuilder {
    item: LibraryItem,
}

impl LibraryItemBuilder {
    /// Create a new builder with required fields.
    pub fn new(slug: String, title: String) -> Self {
        Self {
            item: LibraryItem::new(slug, title),
        }
    }

    /// Set the file path.
    pub fn file_path(mut self, path: impl Into<String>) -> Self {
        self.item.file_path = Some(path.into());
        self
    }

    /// Set the file type.
    pub fn file_type(mut self, file_type: impl Into<String>) -> Self {
        self.item.file_type = Some(file_type.into());
        self
    }

    /// Set the file size in bytes.
    pub fn file_size(mut self, size: i64) -> Self {
        self.item.file_size = Some(size);
        self
    }

    /// Set the page count.
    pub fn page_count(mut self, count: i32) -> Self {
        self.item.page_count = Some(count);
        self
    }

    /// Set the game system.
    pub fn game_system(mut self, system: impl Into<String>) -> Self {
        self.item.game_system = Some(system.into());
        self
    }

    /// Set the game system ID.
    pub fn game_system_id(mut self, id: impl Into<String>) -> Self {
        self.item.game_system_id = Some(id.into());
        self
    }

    /// Set the content category.
    pub fn content_category(mut self, category: impl Into<String>) -> Self {
        self.item.content_category = Some(category.into());
        self
    }

    /// Set the publisher.
    pub fn publisher(mut self, publisher: impl Into<String>) -> Self {
        self.item.publisher = Some(publisher.into());
        self
    }

    /// Set the status.
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.item.status = status.into();
        self
    }

    /// Set additional metadata.
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.item.metadata = Some(metadata);
        self
    }

    /// Build the library item.
    pub fn build(self) -> LibraryItem {
        self.item
    }
}

/// Library item with chunk count (for list views).
///
/// Extends LibraryItem with a count of associated chunks, useful for
/// displaying ingestion progress and search coverage.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LibraryItemWithCount {
    /// The library item
    #[serde(flatten)]
    pub item: LibraryItem,
    /// Number of chunks associated with this item
    #[serde(default)]
    pub chunk_count: i64,
}

// ============================================================================
// Library Item CRUD Operations (Task 3.2.1)
// ============================================================================

/// Create a new library item.
///
/// Inserts a new library item into the database with automatic timestamp.
/// Returns the generated ID (without table prefix).
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `item` - Library item to create (id field is ignored)
///
/// # Returns
///
/// The generated record ID as a string.
///
/// # Errors
///
/// Returns `StorageError::Query` if the insert fails.
pub async fn create_library_item(
    db: &Surreal<Db>,
    item: &LibraryItem,
) -> Result<String, StorageError> {
    // Helper struct to deserialize the created record ID
    #[derive(Debug, Deserialize)]
    struct CreatedRecord {
        id: surrealdb::sql::Thing,
    }

    let query = r#"
        CREATE library_item CONTENT {
            slug: $slug,
            title: $title,
            file_path: $file_path,
            file_type: $file_type,
            file_size: $file_size,
            page_count: $page_count,
            game_system: $game_system,
            game_system_id: $game_system_id,
            content_category: $content_category,
            publisher: $publisher,
            status: $status,
            error_message: $error_message,
            metadata: $metadata
        };
    "#;

    let mut response = db
        .query(query)
        .bind(("slug", item.slug.clone()))
        .bind(("title", item.title.clone()))
        .bind(("file_path", item.file_path.clone()))
        .bind(("file_type", item.file_type.clone()))
        .bind(("file_size", item.file_size))
        .bind(("page_count", item.page_count))
        .bind(("game_system", item.game_system.clone()))
        .bind(("game_system_id", item.game_system_id.clone()))
        .bind(("content_category", item.content_category.clone()))
        .bind(("publisher", item.publisher.clone()))
        .bind(("status", item.status.clone()))
        .bind(("error_message", item.error_message.clone()))
        .bind(("metadata", item.metadata.clone()))
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

    let record: Option<CreatedRecord> = response
        .take(0)
        .map_err(|e| StorageError::Query(e.to_string()))?;

    record
        .map(|r| r.id.id.to_string())
        .ok_or_else(|| StorageError::Query("Failed to get created ID".to_string()))
}

/// Get a library item by ID.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `id` - Record ID (without table prefix)
///
/// # Returns
///
/// The library item if found, or None if not found.
pub async fn get_library_item(
    db: &Surreal<Db>,
    id: &str,
) -> Result<Option<LibraryItem>, StorageError> {
    let result: Option<LibraryItem> = db
        .query("SELECT *, meta::id(id) as id FROM type::thing('library_item', $id)")
        .bind(("id", id.to_string()))
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?
        .take(0)
        .map_err(|e| StorageError::Query(e.to_string()))?;

    Ok(result)
}

/// Get a library item by slug.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `slug` - Unique slug identifier
///
/// # Returns
///
/// The library item if found, or None if not found.
pub async fn get_library_item_by_slug(
    db: &Surreal<Db>,
    slug: &str,
) -> Result<Option<LibraryItem>, StorageError> {
    let result: Option<LibraryItem> = db
        .query("SELECT *, meta::id(id) as id FROM library_item WHERE slug = $slug")
        .bind(("slug", slug.to_string()))
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?
        .take(0)
        .map_err(|e| StorageError::Query(e.to_string()))?;

    Ok(result)
}

/// Update a library item.
///
/// Updates all mutable fields of a library item. The slug field is not updated
/// as it serves as a stable identifier. The updated_at timestamp is set automatically.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `id` - Record ID (without table prefix)
/// * `item` - Library item with updated values
pub async fn update_library_item(
    db: &Surreal<Db>,
    id: &str,
    item: &LibraryItem,
) -> Result<(), StorageError> {
    db.query(
        r#"
        UPDATE type::thing('library_item', $id) MERGE {
            title: $title,
            file_path: $file_path,
            file_type: $file_type,
            file_size: $file_size,
            page_count: $page_count,
            game_system: $game_system,
            game_system_id: $game_system_id,
            content_category: $content_category,
            publisher: $publisher,
            status: $status,
            error_message: $error_message,
            metadata: $metadata,
            updated_at: time::now()
        };
    "#,
    )
    .bind(("id", id.to_string()))
    .bind(("title", item.title.clone()))
    .bind(("file_path", item.file_path.clone()))
    .bind(("file_type", item.file_type.clone()))
    .bind(("file_size", item.file_size))
    .bind(("page_count", item.page_count))
    .bind(("game_system", item.game_system.clone()))
    .bind(("game_system_id", item.game_system_id.clone()))
    .bind(("content_category", item.content_category.clone()))
    .bind(("publisher", item.publisher.clone()))
    .bind(("status", item.status.clone()))
    .bind(("error_message", item.error_message.clone()))
    .bind(("metadata", item.metadata.clone()))
    .await
    .map_err(|e| StorageError::Query(e.to_string()))?;

    Ok(())
}

/// Delete a library item (cascades to chunks).
///
/// Deletes the library item and all associated chunks. This is a destructive
/// operation that cannot be undone.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `id` - Record ID (without table prefix)
pub async fn delete_library_item(db: &Surreal<Db>, id: &str) -> Result<(), StorageError> {
    let id_owned = id.to_string();

    // Delete chunks first
    db.query("DELETE chunk WHERE library_item = type::thing('library_item', $id)")
        .bind(("id", id_owned.clone()))
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

    // Delete library item
    db.query("DELETE type::thing('library_item', $id)")
        .bind(("id", id_owned))
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

    Ok(())
}

// ============================================================================
// Library Item List Operations (Task 3.2.2)
// ============================================================================

/// List library items with pagination and optional status filter.
///
/// Returns library items with their associated chunk counts, ordered by
/// creation date (newest first).
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `status` - Optional status filter (pending, processing, ready, error)
/// * `limit` - Maximum number of items to return
/// * `offset` - Number of items to skip
///
/// # Returns
///
/// A vector of library items with chunk counts.
pub async fn get_library_items(
    db: &Surreal<Db>,
    status: Option<&str>,
    limit: usize,
    offset: usize,
) -> Result<Vec<LibraryItemWithCount>, StorageError> {
    let status_filter = status
        .map(|s| format!("WHERE status = '{}'", s))
        .unwrap_or_default();

    let query = format!(
        r#"
        SELECT
            *,
            meta::id(id) as id,
            (SELECT count() FROM chunk WHERE library_item = $parent.id GROUP ALL)[0].count ?? 0 as chunk_count
        FROM library_item
        {status_filter}
        ORDER BY created_at DESC
        LIMIT {limit}
        START {offset};
    "#
    );

    let results: Vec<LibraryItemWithCount> = db
        .query(&query)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?
        .take(0)
        .map_err(|e| StorageError::Query(e.to_string()))?;

    Ok(results)
}

/// Get total count of library items (for pagination).
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `status` - Optional status filter
///
/// # Returns
///
/// The total count of matching library items.
pub async fn count_library_items(
    db: &Surreal<Db>,
    status: Option<&str>,
) -> Result<usize, StorageError> {
    let status_filter = status
        .map(|s| format!("WHERE status = '{}'", s))
        .unwrap_or_default();

    let query = format!(
        "SELECT count() FROM library_item {} GROUP ALL",
        status_filter
    );

    let result: Option<i64> = db
        .query(&query)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?
        .take((0, "count"))
        .map_err(|e| StorageError::Query(e.to_string()))?;

    Ok(result.unwrap_or(0) as usize)
}

/// Update library item status.
///
/// Convenience function for updating just the status and optional error message.
/// This is commonly used during document processing workflows.
///
/// # Arguments
///
/// * `db` - SurrealDB connection
/// * `id` - Record ID (without table prefix)
/// * `status` - New status value
/// * `error_message` - Optional error message (typically set when status is "error")
pub async fn update_library_item_status(
    db: &Surreal<Db>,
    id: &str,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), StorageError> {
    db.query(
        r#"
        UPDATE type::thing('library_item', $id) SET
            status = $status,
            error_message = $error_message,
            updated_at = time::now();
    "#,
    )
    .bind(("id", id.to_string()))
    .bind(("status", status.to_string()))
    .bind(("error_message", error_message.map(|s| s.to_string())))
    .await
    .map_err(|e| StorageError::Query(e.to_string()))?;

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::storage::SurrealStorage;
    use tempfile::TempDir;

    /// Helper to create a test database
    async fn setup_test_db() -> (SurrealStorage, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let storage = SurrealStorage::new(temp_dir.path().to_path_buf())
            .await
            .expect("Failed to create storage");
        (storage, temp_dir)
    }

    #[tokio::test]
    async fn test_create_library_item() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        let item = LibraryItem::builder("test-rulebook".to_string(), "Test Rulebook".to_string())
            .file_type("pdf")
            .file_size(1024000)
            .page_count(100)
            .game_system("Pathfinder 2e")
            .game_system_id("pf2e")
            .content_category("rulebook")
            .publisher("Paizo")
            .build();

        let id = create_library_item(db, &item).await;
        assert!(id.is_ok(), "Failed to create library item: {:?}", id.err());

        let id = id.unwrap();
        assert!(!id.is_empty(), "ID should not be empty");
    }

    #[tokio::test]
    async fn test_get_library_item_by_id() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        // Create item
        let item = LibraryItem::new("get-by-id-test".to_string(), "Get By ID Test".to_string());
        let id = create_library_item(db, &item)
            .await
            .expect("Failed to create");

        // Get by ID
        let retrieved = get_library_item(db, &id).await;
        assert!(
            retrieved.is_ok(),
            "Failed to get library item: {:?}",
            retrieved.err()
        );

        let retrieved = retrieved.unwrap();
        assert!(retrieved.is_some(), "Library item should exist");

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.slug, "get-by-id-test");
        assert_eq!(retrieved.title, "Get By ID Test");
        assert_eq!(retrieved.status, "pending");
    }

    #[tokio::test]
    async fn test_get_library_item_by_slug() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        // Create item
        let item = LibraryItem::builder(
            "unique-slug-test".to_string(),
            "Unique Slug Test".to_string(),
        )
        .game_system("D&D 5e")
        .build();

        create_library_item(db, &item)
            .await
            .expect("Failed to create");

        // Get by slug
        let retrieved = get_library_item_by_slug(db, "unique-slug-test").await;
        assert!(
            retrieved.is_ok(),
            "Failed to get by slug: {:?}",
            retrieved.err()
        );

        let retrieved = retrieved.unwrap();
        assert!(retrieved.is_some(), "Library item should exist");

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.title, "Unique Slug Test");
        assert_eq!(retrieved.game_system, Some("D&D 5e".to_string()));
    }

    #[tokio::test]
    async fn test_get_nonexistent_library_item() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        // Get by non-existent ID
        let result = get_library_item(db, "nonexistent-id").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Get by non-existent slug
        let result = get_library_item_by_slug(db, "nonexistent-slug").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_update_library_item() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        // Create item
        let item = LibraryItem::new("update-test".to_string(), "Original Title".to_string());
        let id = create_library_item(db, &item)
            .await
            .expect("Failed to create");

        // Update item
        let mut updated_item = item.clone();
        updated_item.title = "Updated Title".to_string();
        updated_item.status = "ready".to_string();
        updated_item.page_count = Some(250);

        let result = update_library_item(db, &id, &updated_item).await;
        assert!(result.is_ok(), "Failed to update: {:?}", result.err());

        // Verify update
        let retrieved = get_library_item(db, &id)
            .await
            .expect("Failed to get")
            .expect("Item should exist");

        assert_eq!(retrieved.title, "Updated Title");
        assert_eq!(retrieved.status, "ready");
        assert_eq!(retrieved.page_count, Some(250));
        assert!(
            retrieved.updated_at.is_some(),
            "updated_at should be set after update"
        );
    }

    #[tokio::test]
    async fn test_update_library_item_status() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        // Create item
        let item = LibraryItem::new("status-test".to_string(), "Status Test".to_string());
        let id = create_library_item(db, &item)
            .await
            .expect("Failed to create");

        // Update status to processing
        update_library_item_status(db, &id, "processing", None)
            .await
            .expect("Failed to update status");

        let retrieved = get_library_item(db, &id)
            .await
            .expect("Failed to get")
            .expect("Item should exist");
        assert_eq!(retrieved.status, "processing");
        assert!(retrieved.error_message.is_none());

        // Update status to error with message
        update_library_item_status(db, &id, "error", Some("OCR failed"))
            .await
            .expect("Failed to update status");

        let retrieved = get_library_item(db, &id)
            .await
            .expect("Failed to get")
            .expect("Item should exist");
        assert_eq!(retrieved.status, "error");
        assert_eq!(retrieved.error_message, Some("OCR failed".to_string()));
    }

    #[tokio::test]
    async fn test_list_library_items_with_pagination() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        // Create multiple items
        for i in 1..=5 {
            let item = LibraryItem::new(format!("list-test-{}", i), format!("List Test {}", i));
            create_library_item(db, &item)
                .await
                .expect("Failed to create");
        }

        // Get first page
        let page1 = get_library_items(db, None, 2, 0)
            .await
            .expect("Failed to list");
        assert_eq!(page1.len(), 2, "First page should have 2 items");

        // Get second page
        let page2 = get_library_items(db, None, 2, 2)
            .await
            .expect("Failed to list");
        assert_eq!(page2.len(), 2, "Second page should have 2 items");

        // Get last page
        let page3 = get_library_items(db, None, 2, 4)
            .await
            .expect("Failed to list");
        assert_eq!(page3.len(), 1, "Third page should have 1 item");

        // Verify all items have chunk_count field
        for item in &page1 {
            assert_eq!(item.chunk_count, 0, "New items should have 0 chunks");
        }
    }

    #[tokio::test]
    async fn test_list_library_items_with_status_filter() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        // Create items with different statuses
        let pending_item =
            LibraryItem::new("filter-pending".to_string(), "Pending Item".to_string());
        create_library_item(db, &pending_item)
            .await
            .expect("Failed to create");

        let mut ready_item = LibraryItem::new("filter-ready".to_string(), "Ready Item".to_string());
        ready_item.status = "ready".to_string();
        create_library_item(db, &ready_item)
            .await
            .expect("Failed to create");

        // Filter by pending
        let pending_items = get_library_items(db, Some("pending"), 10, 0)
            .await
            .expect("Failed to list");
        assert_eq!(pending_items.len(), 1);
        assert_eq!(pending_items[0].item.status, "pending");

        // Filter by ready
        let ready_items = get_library_items(db, Some("ready"), 10, 0)
            .await
            .expect("Failed to list");
        assert_eq!(ready_items.len(), 1);
        assert_eq!(ready_items[0].item.status, "ready");
    }

    #[tokio::test]
    async fn test_count_library_items() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        // Initially empty
        let count = count_library_items(db, None)
            .await
            .expect("Failed to count");
        assert_eq!(count, 0);

        // Create items
        for i in 1..=3 {
            let item = LibraryItem::new(format!("count-test-{}", i), format!("Count Test {}", i));
            create_library_item(db, &item)
                .await
                .expect("Failed to create");
        }

        // Count all
        let count = count_library_items(db, None)
            .await
            .expect("Failed to count");
        assert_eq!(count, 3);

        // Count with status filter
        let count = count_library_items(db, Some("pending"))
            .await
            .expect("Failed to count");
        assert_eq!(count, 3);

        let count = count_library_items(db, Some("ready"))
            .await
            .expect("Failed to count");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_delete_library_item() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        // Create item
        let item = LibraryItem::new("delete-test".to_string(), "Delete Test".to_string());
        let id = create_library_item(db, &item)
            .await
            .expect("Failed to create");

        // Verify it exists
        let retrieved = get_library_item(db, &id)
            .await
            .expect("Failed to get");
        assert!(retrieved.is_some());

        // Delete it
        delete_library_item(db, &id)
            .await
            .expect("Failed to delete");

        // Verify it's gone
        let retrieved = get_library_item(db, &id)
            .await
            .expect("Failed to get");
        assert!(retrieved.is_none(), "Item should be deleted");
    }

    #[tokio::test]
    async fn test_delete_library_item_cascades_to_chunks() {
        let (storage, _temp_dir) = setup_test_db().await;
        let db = storage.db();

        // Create library item
        let item = LibraryItem::new("cascade-test".to_string(), "Cascade Test".to_string());
        let id = create_library_item(db, &item)
            .await
            .expect("Failed to create");

        // Create some chunks for this item
        let _record_id = format!("library_item:{}", id);
        for i in 1..=3 {
            // Clone id for each iteration since bind requires owned values
            let item_id_owned = id.clone();
            db.query(
                r#"
                CREATE chunk CONTENT {
                    content: $content,
                    library_item: type::thing('library_item', $item_id),
                    content_type: "text",
                    chunk_index: $index
                };
            "#,
            )
            .bind(("content", format!("Test chunk {}", i)))
            .bind(("item_id", item_id_owned))
            .bind(("index", i as i32))
            .await
            .expect("Failed to create chunk");
        }

        // Verify chunks exist
        let chunk_count: Option<i64> = db
            .query(
                "SELECT count() FROM chunk WHERE library_item = type::thing('library_item', $id) GROUP ALL",
            )
            .bind(("id", id.clone()))
            .await
            .expect("Failed to count chunks")
            .take((0, "count"))
            .expect("Failed to get count");
        assert_eq!(chunk_count, Some(3), "Should have 3 chunks before delete");

        // Delete library item (should cascade to chunks)
        delete_library_item(db, &id)
            .await
            .expect("Failed to delete");

        // Verify chunks are deleted
        let chunk_count: Option<i64> = db
            .query(
                "SELECT count() FROM chunk WHERE library_item = type::thing('library_item', $id) GROUP ALL",
            )
            .bind(("id", id.clone()))
            .await
            .expect("Failed to count chunks")
            .take((0, "count"))
            .expect("Failed to get count");
        assert!(
            chunk_count.is_none() || chunk_count == Some(0),
            "Chunks should be deleted: {:?}",
            chunk_count
        );
    }

    #[tokio::test]
    async fn test_library_item_builder() {
        let item = LibraryItem::builder("builder-test".to_string(), "Builder Test".to_string())
            .file_path("/path/to/file.pdf")
            .file_type("pdf")
            .file_size(5000000)
            .page_count(300)
            .game_system("Pathfinder 2e")
            .game_system_id("pf2e")
            .content_category("adventure")
            .publisher("Paizo")
            .status("ready")
            .metadata(serde_json::json!({"isbn": "123-456-789"}))
            .build();

        assert_eq!(item.slug, "builder-test");
        assert_eq!(item.title, "Builder Test");
        assert_eq!(item.file_path, Some("/path/to/file.pdf".to_string()));
        assert_eq!(item.file_type, Some("pdf".to_string()));
        assert_eq!(item.file_size, Some(5000000));
        assert_eq!(item.page_count, Some(300));
        assert_eq!(item.game_system, Some("Pathfinder 2e".to_string()));
        assert_eq!(item.game_system_id, Some("pf2e".to_string()));
        assert_eq!(item.content_category, Some("adventure".to_string()));
        assert_eq!(item.publisher, Some("Paizo".to_string()));
        assert_eq!(item.status, "ready");
        assert!(item.metadata.is_some());
    }
}
