//! TTRPG document database operations
//!
//! This module provides CRUD operations for parsed TTRPG content (monsters, spells,
//! items, etc.) and ingestion job tracking.

use super::models::{TTRPGDocumentRecord, TTRPGDocumentAttribute, TTRPGIngestionJob};
use super::Database;
use sqlx::Row;

/// Statistics about TTRPG documents in the database
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TTRPGDocumentStats {
    pub total_documents: u64,
    pub source_documents: u32,
    pub game_systems: u32,
    pub avg_confidence: Option<f64>,
}

/// Extension trait for TTRPG document database operations
pub trait TtrpgOps {
    // TTRPG Documents
    fn save_ttrpg_document(&self, doc: &TTRPGDocumentRecord) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_ttrpg_document(&self, id: &str) -> impl std::future::Future<Output = Result<Option<TTRPGDocumentRecord>, sqlx::Error>> + Send;
    fn list_ttrpg_documents_by_source(&self, source_document_id: &str) -> impl std::future::Future<Output = Result<Vec<TTRPGDocumentRecord>, sqlx::Error>> + Send;
    fn list_ttrpg_documents_by_type(&self, element_type: &str) -> impl std::future::Future<Output = Result<Vec<TTRPGDocumentRecord>, sqlx::Error>> + Send;
    fn list_ttrpg_documents_by_system(&self, game_system: &str) -> impl std::future::Future<Output = Result<Vec<TTRPGDocumentRecord>, sqlx::Error>> + Send;
    fn search_ttrpg_documents_by_name(&self, name_pattern: &str) -> impl std::future::Future<Output = Result<Vec<TTRPGDocumentRecord>, sqlx::Error>> + Send;
    fn list_ttrpg_documents_by_cr(&self, min_cr: f64, max_cr: f64) -> impl std::future::Future<Output = Result<Vec<TTRPGDocumentRecord>, sqlx::Error>> + Send;
    fn delete_ttrpg_document(&self, id: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn delete_ttrpg_documents_by_source(&self, source_document_id: &str) -> impl std::future::Future<Output = Result<u64, sqlx::Error>> + Send;

    // TTRPG Document Attributes
    fn add_ttrpg_document_attribute(&self, document_id: &str, attribute_type: &str, attribute_value: &str) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_ttrpg_document_attributes(&self, document_id: &str) -> impl std::future::Future<Output = Result<Vec<TTRPGDocumentAttribute>, sqlx::Error>> + Send;
    fn find_ttrpg_documents_by_attribute(&self, attribute_type: &str, attribute_value: &str) -> impl std::future::Future<Output = Result<Vec<TTRPGDocumentRecord>, sqlx::Error>> + Send;

    // TTRPG Ingestion Jobs
    fn create_ttrpg_ingestion_job(&self, job: &TTRPGIngestionJob) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn get_ttrpg_ingestion_job(&self, id: &str) -> impl std::future::Future<Output = Result<Option<TTRPGIngestionJob>, sqlx::Error>> + Send;
    fn get_ttrpg_ingestion_job_by_document(&self, document_id: &str) -> impl std::future::Future<Output = Result<Option<TTRPGIngestionJob>, sqlx::Error>> + Send;
    fn update_ttrpg_ingestion_job(&self, job: &TTRPGIngestionJob) -> impl std::future::Future<Output = Result<(), sqlx::Error>> + Send;
    fn list_pending_ttrpg_ingestion_jobs(&self) -> impl std::future::Future<Output = Result<Vec<TTRPGIngestionJob>, sqlx::Error>> + Send;
    fn list_active_ttrpg_ingestion_jobs(&self) -> impl std::future::Future<Output = Result<Vec<TTRPGIngestionJob>, sqlx::Error>> + Send;

    // Statistics
    fn count_ttrpg_documents_by_type(&self) -> impl std::future::Future<Output = Result<Vec<(String, i64)>, sqlx::Error>> + Send;
    fn get_ttrpg_document_stats(&self) -> impl std::future::Future<Output = Result<TTRPGDocumentStats, sqlx::Error>> + Send;
}

impl TtrpgOps for Database {
    // =========================================================================
    // TTRPG Document Operations
    // =========================================================================

    async fn save_ttrpg_document(&self, doc: &TTRPGDocumentRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO ttrpg_documents
            (id, source_document_id, name, element_type, game_system, content,
             attributes_json, challenge_rating, level, page_number, confidence,
             meilisearch_id, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&doc.id)
        .bind(&doc.source_document_id)
        .bind(&doc.name)
        .bind(&doc.element_type)
        .bind(&doc.game_system)
        .bind(&doc.content)
        .bind(&doc.attributes_json)
        .bind(doc.challenge_rating)
        .bind(doc.level)
        .bind(doc.page_number)
        .bind(doc.confidence)
        .bind(&doc.meilisearch_id)
        .bind(&doc.created_at)
        .bind(&doc.updated_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_ttrpg_document(&self, id: &str) -> Result<Option<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn list_ttrpg_documents_by_source(&self, source_document_id: &str) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE source_document_id = ? ORDER BY page_number, name"
        )
        .bind(source_document_id)
        .fetch_all(self.pool())
        .await
    }

    async fn list_ttrpg_documents_by_type(&self, element_type: &str) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE element_type = ? ORDER BY name"
        )
        .bind(element_type)
        .fetch_all(self.pool())
        .await
    }

    async fn list_ttrpg_documents_by_system(&self, game_system: &str) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE game_system = ? ORDER BY element_type, name"
        )
        .bind(game_system)
        .fetch_all(self.pool())
        .await
    }

    async fn search_ttrpg_documents_by_name(&self, name_pattern: &str) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        let pattern = format!("%{}%", name_pattern);
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE name LIKE ? ORDER BY name LIMIT 100"
        )
        .bind(&pattern)
        .fetch_all(self.pool())
        .await
    }

    async fn list_ttrpg_documents_by_cr(&self, min_cr: f64, max_cr: f64) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            "SELECT * FROM ttrpg_documents WHERE challenge_rating >= ? AND challenge_rating <= ? ORDER BY challenge_rating, name"
        )
        .bind(min_cr)
        .bind(max_cr)
        .fetch_all(self.pool())
        .await
    }

    async fn delete_ttrpg_document(&self, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM ttrpg_documents WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    async fn delete_ttrpg_documents_by_source(&self, source_document_id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM ttrpg_documents WHERE source_document_id = ?")
            .bind(source_document_id)
            .execute(self.pool())
            .await?;
        Ok(result.rows_affected())
    }

    // =========================================================================
    // TTRPG Document Attribute Operations
    // =========================================================================

    async fn add_ttrpg_document_attribute(
        &self,
        document_id: &str,
        attribute_type: &str,
        attribute_value: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO ttrpg_document_attributes (document_id, attribute_type, attribute_value) VALUES (?, ?, ?)"
        )
        .bind(document_id)
        .bind(attribute_type)
        .bind(attribute_value)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_ttrpg_document_attributes(&self, document_id: &str) -> Result<Vec<TTRPGDocumentAttribute>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentAttribute>(
            "SELECT * FROM ttrpg_document_attributes WHERE document_id = ?"
        )
        .bind(document_id)
        .fetch_all(self.pool())
        .await
    }

    async fn find_ttrpg_documents_by_attribute(
        &self,
        attribute_type: &str,
        attribute_value: &str,
    ) -> Result<Vec<TTRPGDocumentRecord>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGDocumentRecord>(
            r#"
            SELECT d.* FROM ttrpg_documents d
            JOIN ttrpg_document_attributes a ON d.id = a.document_id
            WHERE a.attribute_type = ? AND a.attribute_value = ?
            ORDER BY d.name
            "#
        )
        .bind(attribute_type)
        .bind(attribute_value)
        .fetch_all(self.pool())
        .await
    }

    // =========================================================================
    // TTRPG Ingestion Job Operations
    // =========================================================================

    async fn create_ttrpg_ingestion_job(&self, job: &TTRPGIngestionJob) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO ttrpg_ingestion_jobs
            (id, document_id, status, total_pages, processed_pages, elements_found,
             errors_json, started_at, completed_at, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&job.id)
        .bind(&job.document_id)
        .bind(&job.status)
        .bind(job.total_pages)
        .bind(job.processed_pages)
        .bind(job.elements_found)
        .bind(&job.errors_json)
        .bind(&job.started_at)
        .bind(&job.completed_at)
        .bind(&job.created_at)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn get_ttrpg_ingestion_job(&self, id: &str) -> Result<Option<TTRPGIngestionJob>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGIngestionJob>(
            "SELECT * FROM ttrpg_ingestion_jobs WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool())
        .await
    }

    async fn get_ttrpg_ingestion_job_by_document(&self, document_id: &str) -> Result<Option<TTRPGIngestionJob>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGIngestionJob>(
            "SELECT * FROM ttrpg_ingestion_jobs WHERE document_id = ? ORDER BY created_at DESC LIMIT 1"
        )
        .bind(document_id)
        .fetch_optional(self.pool())
        .await
    }

    async fn update_ttrpg_ingestion_job(&self, job: &TTRPGIngestionJob) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE ttrpg_ingestion_jobs
            SET status = ?, processed_pages = ?, elements_found = ?,
                errors_json = ?, started_at = ?, completed_at = ?
            WHERE id = ?
            "#
        )
        .bind(&job.status)
        .bind(job.processed_pages)
        .bind(job.elements_found)
        .bind(&job.errors_json)
        .bind(&job.started_at)
        .bind(&job.completed_at)
        .bind(&job.id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn list_pending_ttrpg_ingestion_jobs(&self) -> Result<Vec<TTRPGIngestionJob>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGIngestionJob>(
            "SELECT * FROM ttrpg_ingestion_jobs WHERE status = 'pending' ORDER BY created_at"
        )
        .fetch_all(self.pool())
        .await
    }

    async fn list_active_ttrpg_ingestion_jobs(&self) -> Result<Vec<TTRPGIngestionJob>, sqlx::Error> {
        sqlx::query_as::<_, TTRPGIngestionJob>(
            "SELECT * FROM ttrpg_ingestion_jobs WHERE status = 'processing' ORDER BY started_at"
        )
        .fetch_all(self.pool())
        .await
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    async fn count_ttrpg_documents_by_type(&self) -> Result<Vec<(String, i64)>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT element_type, COUNT(*) as count FROM ttrpg_documents GROUP BY element_type ORDER BY count DESC"
        )
        .fetch_all(self.pool())
        .await?;

        Ok(rows.into_iter()
            .map(|row| (row.get("element_type"), row.get("count")))
            .collect())
    }

    async fn get_ttrpg_document_stats(&self) -> Result<TTRPGDocumentStats, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_documents,
                COUNT(DISTINCT source_document_id) as source_documents,
                COUNT(DISTINCT game_system) as game_systems,
                AVG(confidence) as avg_confidence
            FROM ttrpg_documents
            "#
        )
        .fetch_one(self.pool())
        .await?;

        Ok(TTRPGDocumentStats {
            total_documents: row.get::<i64, _>("total_documents") as u64,
            source_documents: row.get::<i64, _>("source_documents") as u32,
            game_systems: row.get::<i64, _>("game_systems") as u32,
            avg_confidence: row.get("avg_confidence"),
        })
    }
}
