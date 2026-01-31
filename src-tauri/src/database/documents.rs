//! Document/source database operations
//!
//! This module provides CRUD operations for ingested source documents.

use super::models::DocumentRecord;
use super::Database;

/// Extension trait for document database operations
pub trait DocumentOps {
    fn save_document(&self, doc: &DocumentRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn list_documents(&self) -> impl std::future::Future<Output = Result<Vec<DocumentRecord>, sqlx::Error>> + Send;
    fn delete_document(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
}

impl DocumentOps for Database {
    async fn save_document(&self, doc: &DocumentRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO documents
            (id, name, source_type, file_path, page_count, chunk_count, status, ingested_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&doc.id)
        .bind(&doc.name)
        .bind(&doc.source_type)
        .bind(&doc.file_path)
        .bind(doc.page_count)
        .bind(doc.chunk_count)
        .bind(&doc.status)
        .bind(&doc.ingested_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn list_documents(&self) -> Result<Vec<DocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, DocumentRecord>(
            "SELECT * FROM documents ORDER BY ingested_at DESC"
        )
        .fetch_all(self.pool())
        .await
    }

    async fn delete_document(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM documents WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
