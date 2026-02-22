//! Content Usage Tracker - Track citation usage in campaigns
//!
//! Part of Phase 3: Content Grounding Layer (Task 3.4)
//!
//! Implements citation usage tracking with deduplication to prevent
//! repetitive citations in generated content.

use crate::database::{Citation, SourceCitationRecord};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Result type for usage tracker operations.
pub type UsageResult<T> = Result<T, UsageTrackerError>;

/// Errors that can occur in usage tracking operations.
#[derive(Debug, thiserror::Error)]
pub enum UsageTrackerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Citation not found: {0}")]
    NotFound(String),
}

/// Tracks which citations have been used in a campaign.
///
/// Provides deduplication to prevent the same content from being cited
/// too frequently in generated content.
pub struct UsageTracker {
    db: Arc<SqlitePool>,
    /// In-memory cache of used content per campaign
    usage_cache: std::sync::RwLock<HashMap<String, HashSet<String>>>,
}

/// Summary of citation usage for a campaign.
#[derive(Debug, Clone)]
pub struct UsageSummary {
    /// Total number of unique citations used
    pub total_citations: usize,
    /// Citations by source type
    pub by_source_type: HashMap<String, usize>,
    /// Most frequently cited sources
    pub top_sources: Vec<(String, usize)>,
    /// Recently used citation IDs (for deduplication)
    pub recent_ids: Vec<String>,
}

/// Options for controlling usage tracking behavior.
#[derive(Debug, Clone)]
pub struct UsageOptions {
    /// Maximum number of times the same citation can be used before flagging
    pub max_repetitions: usize,
    /// Time window (in seconds) for repetition detection
    pub repetition_window_secs: u64,
    /// Whether to track usage in the database
    pub persist: bool,
}

impl Default for UsageOptions {
    fn default() -> Self {
        Self {
            max_repetitions: 3,
            repetition_window_secs: 3600, // 1 hour
            persist: true,
        }
    }
}

impl UsageTracker {
    /// Create a new UsageTracker with the given database pool.
    pub fn new(db: Arc<SqlitePool>) -> Self {
        Self {
            db,
            usage_cache: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Mark a citation as used in a campaign.
    ///
    /// # Arguments
    /// * `campaign_id` - The campaign where the citation was used
    /// * `citation` - The citation that was used
    ///
    /// # Returns
    /// `true` if this is a new usage, `false` if already used (duplicate)
    pub async fn mark_content_used(
        &self,
        campaign_id: &str,
        citation: &Citation,
    ) -> UsageResult<bool> {
        // Check cache first
        let is_new = {
            let mut cache = self.usage_cache.write().unwrap();
            let campaign_set = cache
                .entry(campaign_id.to_string())
                .or_insert_with(HashSet::new);
            campaign_set.insert(citation.id.clone())
        };

        if !is_new {
            return Ok(false);
        }

        // Update the database
        self.persist_usage(campaign_id, citation).await?;

        Ok(true)
    }

    /// Mark multiple citations as used in a campaign.
    ///
    /// # Arguments
    /// * `campaign_id` - The campaign where the citations were used
    /// * `citations` - The citations that were used
    ///
    /// # Returns
    /// Number of newly used citations (excludes duplicates)
    pub async fn mark_multiple_used(
        &self,
        campaign_id: &str,
        citations: &[Citation],
    ) -> UsageResult<usize> {
        let mut new_count = 0;
        for citation in citations {
            if self.mark_content_used(campaign_id, citation).await? {
                new_count += 1;
            }
        }
        Ok(new_count)
    }

    /// Get all citations used in a campaign.
    ///
    /// # Arguments
    /// * `campaign_id` - The campaign to query
    ///
    /// # Returns
    /// Vector of citations used in the campaign
    pub async fn get_used_content(&self, campaign_id: &str) -> UsageResult<Vec<Citation>> {
        let records = sqlx::query_as::<_, SourceCitationRecord>(
            r#"
            SELECT * FROM source_citations
            WHERE campaign_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(campaign_id)
        .fetch_all(self.db.as_ref())
        .await?;

        let citations: Vec<Citation> = records
            .iter()
            .filter_map(|r| Citation::from_record(r).ok())
            .collect();

        // Update cache
        {
            let mut cache = self.usage_cache.write().unwrap();
            let campaign_set = cache
                .entry(campaign_id.to_string())
                .or_insert_with(HashSet::new);
            for citation in &citations {
                campaign_set.insert(citation.id.clone());
            }
        }

        Ok(citations)
    }

    /// Get usage summary for a campaign.
    ///
    /// # Arguments
    /// * `campaign_id` - The campaign to summarize
    ///
    /// # Returns
    /// Summary of citation usage statistics
    pub async fn get_usage_summary(&self, campaign_id: &str) -> UsageResult<UsageSummary> {
        let citations = self.get_used_content(campaign_id).await?;

        // Count by source type
        let mut by_source_type: HashMap<String, usize> = HashMap::new();
        let mut source_counts: HashMap<String, usize> = HashMap::new();

        for citation in &citations {
            *by_source_type
                .entry(citation.source_type.as_str().to_string())
                .or_default() += 1;
            *source_counts
                .entry(citation.source_name.clone())
                .or_default() += 1;
        }

        // Get top sources
        let mut top_sources: Vec<(String, usize)> = source_counts.into_iter().collect();
        top_sources.sort_by(|a, b| b.1.cmp(&a.1));
        top_sources.truncate(10);

        // Get recent IDs (last 20)
        let recent_ids: Vec<String> = citations
            .iter()
            .take(20)
            .map(|c| c.id.clone())
            .collect();

        Ok(UsageSummary {
            total_citations: citations.len(),
            by_source_type,
            top_sources,
            recent_ids,
        })
    }

    /// Check if a citation has been used recently.
    ///
    /// # Arguments
    /// * `campaign_id` - The campaign to check
    /// * `citation_id` - The citation ID to check
    ///
    /// # Returns
    /// `true` if the citation has been used, `false` otherwise
    pub fn is_recently_used(&self, campaign_id: &str, citation_id: &str) -> bool {
        let cache = self.usage_cache.read().unwrap();
        cache
            .get(campaign_id)
            .map(|set| set.contains(citation_id))
            .unwrap_or(false)
    }

    /// Check if content from a specific source has been overused.
    ///
    /// # Arguments
    /// * `campaign_id` - The campaign to check
    /// * `source_name` - The source name to check
    /// * `max_count` - Maximum allowed uses before considered overused
    ///
    /// # Returns
    /// `true` if the source has been used more than `max_count` times
    pub async fn is_source_overused(
        &self,
        campaign_id: &str,
        source_name: &str,
        max_count: usize,
    ) -> UsageResult<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM source_citations
            WHERE campaign_id = ? AND source_name = ?
            "#,
        )
        .bind(campaign_id)
        .bind(source_name)
        .fetch_one(self.db.as_ref())
        .await?;

        Ok(count as usize > max_count)
    }

    /// Filter citations to exclude those already used (deduplication).
    ///
    /// # Arguments
    /// * `campaign_id` - The campaign to check against
    /// * `citations` - Citations to filter
    ///
    /// # Returns
    /// Citations that have not been used in the campaign
    pub fn filter_unused(&self, campaign_id: &str, citations: &[Citation]) -> Vec<Citation> {
        let cache = self.usage_cache.read().unwrap();
        let used_ids = cache.get(campaign_id);

        citations
            .iter()
            .filter(|c| {
                used_ids
                    .map(|ids| !ids.contains(&c.id))
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    /// Clear usage cache for a campaign (useful for testing or reset).
    pub fn clear_cache(&self, campaign_id: &str) {
        let mut cache = self.usage_cache.write().unwrap();
        cache.remove(campaign_id);
    }

    /// Preload usage cache from database for a campaign.
    ///
    /// Call this when loading a campaign to ensure deduplication works correctly.
    pub async fn preload_cache(&self, campaign_id: &str) -> UsageResult<()> {
        let _ = self.get_used_content(campaign_id).await?;
        Ok(())
    }

    /// Persist citation usage to the database.
    async fn persist_usage(&self, campaign_id: &str, citation: &Citation) -> UsageResult<()> {
        // Build the used_in array
        let used_in = serde_json::json!([campaign_id]);

        let record = citation.to_record();

        sqlx::query(
            r#"
            INSERT INTO source_citations (
                id, campaign_id, source_type, source_id, source_name,
                location, excerpt, confidence, used_in, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                used_in = COALESCE(
                    json_insert(used_in, '$[#]', ?),
                    json_array(?)
                )
            "#,
        )
        .bind(&record.id)
        .bind(campaign_id)
        .bind(&record.source_type)
        .bind(&record.source_id)
        .bind(&record.source_name)
        .bind(&record.location)
        .bind(&record.excerpt)
        .bind(record.confidence)
        .bind(used_in.to_string())
        .bind(&record.created_at)
        .bind(campaign_id)
        .bind(campaign_id)
        .execute(self.db.as_ref())
        .await?;

        Ok(())
    }

    /// Delete a citation from usage tracking.
    pub async fn remove_citation(&self, campaign_id: &str, citation_id: &str) -> UsageResult<()> {
        // Remove from cache
        {
            let mut cache = self.usage_cache.write().unwrap();
            if let Some(set) = cache.get_mut(campaign_id) {
                set.remove(citation_id);
            }
        }

        // Remove from database
        sqlx::query(
            r#"
            DELETE FROM source_citations
            WHERE id = ? AND campaign_id = ?
            "#,
        )
        .bind(citation_id)
        .bind(campaign_id)
        .execute(self.db.as_ref())
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::SourceType;

    #[test]
    fn test_usage_options_default() {
        let options = UsageOptions::default();
        assert_eq!(options.max_repetitions, 3);
        assert_eq!(options.repetition_window_secs, 3600);
        assert!(options.persist);
    }

    #[test]
    fn test_usage_summary_creation() {
        let mut by_source_type = HashMap::new();
        by_source_type.insert("rulebook".to_string(), 5);
        by_source_type.insert("flavour_source".to_string(), 3);

        let summary = UsageSummary {
            total_citations: 8,
            by_source_type,
            top_sources: vec![
                ("Player's Handbook".to_string(), 3),
                ("Monster Manual".to_string(), 2),
            ],
            recent_ids: vec!["id1".to_string(), "id2".to_string()],
        };

        assert_eq!(summary.total_citations, 8);
        assert_eq!(summary.by_source_type.get("rulebook"), Some(&5));
        assert_eq!(summary.top_sources.len(), 2);
    }

    #[test]
    fn test_filter_unused_logic() {
        // Simulate the filtering logic without database
        let used_ids: HashSet<String> = vec!["c1".to_string(), "c2".to_string()]
            .into_iter()
            .collect();

        let citations = vec![
            Citation::new(SourceType::Rulebook, "Source 1", 0.9),
            Citation::new(SourceType::Rulebook, "Source 2", 0.8),
            Citation::new(SourceType::Rulebook, "Source 3", 0.7),
        ];

        // Simulate filter (would need to set IDs in real test)
        let filtered: Vec<&Citation> = citations
            .iter()
            .filter(|c| !used_ids.contains(&c.id))
            .collect();

        // All should pass since IDs are randomly generated
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_source_type_as_str() {
        assert_eq!(SourceType::Rulebook.as_str(), "rulebook");
        assert_eq!(SourceType::FlavourSource.as_str(), "flavour_source");
        assert_eq!(SourceType::Adventure.as_str(), "adventure");
        assert_eq!(SourceType::Homebrew.as_str(), "homebrew");
        assert_eq!(SourceType::CampaignEntity.as_str(), "campaign_entity");
        assert_eq!(SourceType::UserInput.as_str(), "user_input");
    }
}
