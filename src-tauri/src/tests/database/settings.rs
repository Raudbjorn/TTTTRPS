//! Settings Database Tests
//!
//! Tests for document and settings CRUD operations.

use crate::database::{DocumentOps, DocumentRecord};
use crate::tests::common::create_test_db;

// =============================================================================
// Document Tests
// =============================================================================

#[tokio::test]
async fn test_document_lifecycle() {
    let (db, _temp) = create_test_db().await;

    let doc = DocumentRecord {
        id: "doc-001".to_string(),
        name: "Player's Handbook".to_string(),
        source_type: "pdf".to_string(),
        file_path: Some("/path/to/phb.pdf".to_string()),
        page_count: 320,
        chunk_count: 0,
        status: "pending".to_string(),
        ingested_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_document(&doc).await.expect("Failed to save");

    let docs = db.list_documents().await.expect("Failed to list");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].name, "Player's Handbook");

    // Update status (via save)
    let mut updated = doc.clone();
    updated.status = "ready".to_string();
    updated.chunk_count = 150;
    db.save_document(&updated).await.expect("Failed to update");

    let docs_after = db.list_documents().await.expect("Failed to list");
    assert_eq!(docs_after[0].status, "ready");
    assert_eq!(docs_after[0].chunk_count, 150);

    // Delete
    db.delete_document("doc-001")
        .await
        .expect("Failed to delete");
    let docs_final = db.list_documents().await.expect("Failed to list");
    assert!(docs_final.is_empty());
}

#[tokio::test]
async fn test_document_save_and_get() {
    let (db, _temp) = create_test_db().await;

    let doc = DocumentRecord {
        id: "doc-test".to_string(),
        name: "Test Document".to_string(),
        source_type: "epub".to_string(),
        file_path: Some("/path/to/test.epub".to_string()),
        page_count: 100,
        chunk_count: 0,
        status: "pending".to_string(),
        ingested_at: chrono::Utc::now().to_rfc3339(),
    };

    db.save_document(&doc).await.expect("Failed to save");

    let docs = db.list_documents().await.expect("Failed to list");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].name, "Test Document");
    assert_eq!(docs[0].source_type, "epub");
}

#[tokio::test]
async fn test_document_update_status() {
    let (db, _temp) = create_test_db().await;

    let mut doc = DocumentRecord {
        id: "doc-status".to_string(),
        name: "Status Test".to_string(),
        source_type: "pdf".to_string(),
        file_path: None,
        page_count: 50,
        chunk_count: 0,
        status: "pending".to_string(),
        ingested_at: chrono::Utc::now().to_rfc3339(),
    };

    db.save_document(&doc).await.expect("Failed to save");

    // Update through save
    doc.status = "processing".to_string();
    db.save_document(&doc).await.expect("Failed to update");

    let docs = db.list_documents().await.expect("Failed to list");
    assert_eq!(docs[0].status, "processing");

    // Complete processing
    doc.status = "ready".to_string();
    doc.chunk_count = 25;
    db.save_document(&doc).await.expect("Failed to update");

    let docs = db.list_documents().await.expect("Failed to list");
    assert_eq!(docs[0].status, "ready");
    assert_eq!(docs[0].chunk_count, 25);
}

#[tokio::test]
async fn test_multiple_documents() {
    let (db, _temp) = create_test_db().await;

    let docs_to_create = vec![
        ("doc-1", "Player's Handbook", "pdf"),
        ("doc-2", "Monster Manual", "pdf"),
        ("doc-3", "Adventure Guide", "epub"),
    ];

    for (id, name, source_type) in docs_to_create {
        let doc = DocumentRecord {
            id: id.to_string(),
            name: name.to_string(),
            source_type: source_type.to_string(),
            file_path: Some(format!("/path/to/{}", name)),
            page_count: 100,
            chunk_count: 0,
            status: "pending".to_string(),
            ingested_at: chrono::Utc::now().to_rfc3339(),
        };
        db.save_document(&doc).await.expect("Failed to save");
    }

    let docs = db.list_documents().await.expect("Failed to list");
    assert_eq!(docs.len(), 3);
}

#[tokio::test]
async fn test_document_delete() {
    let (db, _temp) = create_test_db().await;

    let doc = DocumentRecord {
        id: "doc-delete".to_string(),
        name: "To Delete".to_string(),
        source_type: "pdf".to_string(),
        file_path: None,
        page_count: 10,
        chunk_count: 0,
        status: "ready".to_string(),
        ingested_at: chrono::Utc::now().to_rfc3339(),
    };

    db.save_document(&doc).await.expect("Failed to save");

    let docs = db.list_documents().await.expect("Failed to list");
    assert_eq!(docs.len(), 1);

    db.delete_document("doc-delete")
        .await
        .expect("Failed to delete");

    let docs = db.list_documents().await.expect("Failed to list");
    assert!(docs.is_empty());
}
