//! Random Table Engine
//!
//! Phase 8 of the Campaign Generation Overhaul.
//!
//! Provides random table management including:
//! - Table CRUD operations
//! - Roll resolution with probability weighting
//! - Nested/cascading table support
//! - Roll history tracking
//!
//! ## Example
//!
//! ```rust,ignore
//! let engine = RandomTableEngine::new(pool);
//!
//! // Create a simple encounter table
//! let table = engine.create_table(CreateTableRequest {
//!     name: "Random Encounters".to_string(),
//!     dice_notation: "d6".to_string(),
//!     entries: vec![
//!         (1, 1, "Wolves".to_string()),
//!         (2, 3, "Bandits".to_string()),
//!         (4, 5, "Traveling Merchants".to_string()),
//!         (6, 6, "Nothing".to_string()),
//!     ],
//!     ..Default::default()
//! }).await?;
//!
//! // Roll on the table
//! let result = engine.roll_on_table(&table.id, None).await?;
//! ```

use std::sync::Arc;
use sqlx::sqlite::SqlitePool;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};

use super::dice::{DiceNotation, DiceRoller, DiceError, RollResult};
use crate::database::{
    RandomTableRecord, RandomTableEntryRecord, RollHistoryRecord,
    RandomTableType, TableResultType,
};

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur in random table operations
#[derive(Debug, Error)]
pub enum RandomTableError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Dice error: {0}")]
    Dice(#[from] DiceError),

    #[error("Table not found: {0}")]
    TableNotFound(String),

    #[error("Entry not found: {0}")]
    EntryNotFound(String),

    #[error("No matching entry for roll {roll} in table {table_id}")]
    NoMatchingEntry { roll: i32, table_id: String },

    #[error("Circular reference detected in nested tables")]
    CircularReference,

    #[error("Maximum nesting depth ({0}) exceeded")]
    MaxNestingDepth(u32),

    #[error("Invalid range: start ({start}) > end ({end})")]
    InvalidRange { start: i32, end: i32 },

    #[error("Overlapping ranges in table entries")]
    OverlappingRanges,

    #[error("Gaps in table coverage: missing range {start}-{end}")]
    GapsInCoverage { start: i32, end: i32 },

    #[error("Invalid table configuration: {0}")]
    InvalidConfiguration(String),
}

/// Result type for random table operations
pub type RandomTableResult<T> = Result<T, RandomTableError>;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request to create a new random table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTableRequest {
    pub name: String,
    pub description: Option<String>,
    pub dice_notation: String,
    pub table_type: RandomTableType,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub campaign_id: Option<String>,
    pub entries: Vec<TableEntryInput>,
    pub is_system: bool,
}

impl Default for CreateTableRequest {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: None,
            dice_notation: "d20".to_string(),
            table_type: RandomTableType::Standard,
            category: None,
            tags: Vec::new(),
            campaign_id: None,
            entries: Vec::new(),
            is_system: false,
        }
    }
}

/// Input for a table entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableEntryInput {
    pub range_start: i32,
    pub range_end: i32,
    pub result_text: String,
    pub weight: Option<f64>,
    pub nested_table_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Request to roll on a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollRequest {
    pub table_id: String,
    pub session_id: Option<String>,
    pub campaign_id: Option<String>,
    pub context: Option<String>,
    /// Override the natural roll result (for testing/GM fiat)
    pub forced_roll: Option<i32>,
    /// Maximum nesting depth for nested tables
    pub max_depth: Option<u32>,
}

/// Result of rolling on a table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRollResult {
    pub table_id: String,
    pub table_name: String,
    pub roll: RollResult,
    pub entry: TableEntry,
    pub nested_results: Vec<TableRollResult>,
    pub final_text: String,
    pub history_id: String,
}

/// Public view of a table entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableEntry {
    pub id: String,
    pub range_start: i32,
    pub range_end: i32,
    pub result_text: String,
    pub result_type: TableResultType,
    pub nested_table_id: Option<String>,
    pub weight: f64,
}

impl From<RandomTableEntryRecord> for TableEntry {
    fn from(record: RandomTableEntryRecord) -> Self {
        let result_type = record.result_type_enum().unwrap_or_default();
        Self {
            id: record.id,
            range_start: record.range_start,
            range_end: record.range_end,
            result_text: record.result_text,
            result_type,
            nested_table_id: record.nested_table_id,
            weight: record.weight,
        }
    }
}

/// Full table with entries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RandomTable {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub dice_notation: String,
    pub table_type: RandomTableType,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub campaign_id: Option<String>,
    pub entries: Vec<TableEntry>,
    pub is_system: bool,
    pub is_nested: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl RandomTable {
    /// Get the dice notation parsed
    pub fn notation(&self) -> Result<DiceNotation, DiceError> {
        DiceNotation::parse(&self.dice_notation)
    }

    /// Check if the table covers the full range of the dice
    pub fn validate_coverage(&self) -> RandomTableResult<()> {
        let notation = self.notation()?;
        let min = notation.min_result();
        let max = notation.max_result();

        // Sort entries by range_start
        let mut entries = self.entries.clone();
        entries.sort_by_key(|e| e.range_start);

        let mut expected_start = min;
        for entry in &entries {
            if entry.range_start > expected_start {
                return Err(RandomTableError::GapsInCoverage {
                    start: expected_start,
                    end: entry.range_start - 1,
                });
            }
            if entry.range_start < expected_start {
                return Err(RandomTableError::OverlappingRanges);
            }
            if entry.range_start > entry.range_end {
                return Err(RandomTableError::InvalidRange {
                    start: entry.range_start,
                    end: entry.range_end,
                });
            }
            expected_start = entry.range_end + 1;
        }

        if expected_start <= max {
            return Err(RandomTableError::GapsInCoverage {
                start: expected_start,
                end: max,
            });
        }

        Ok(())
    }
}

// ============================================================================
// Random Table Engine
// ============================================================================

/// Core engine for random table operations
pub struct RandomTableEngine {
    pool: Arc<SqlitePool>,
    roller: DiceRoller,
}

impl RandomTableEngine {
    /// Maximum nesting depth for cascading tables
    pub const MAX_NESTING_DEPTH: u32 = 10;

    /// Create a new random table engine
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            pool,
            roller: DiceRoller::new(),
        }
    }

    // ========================================================================
    // Table CRUD Operations
    // ========================================================================

    /// Create a new random table
    pub async fn create_table(&self, request: CreateTableRequest) -> RandomTableResult<RandomTable> {
        // Validate dice notation
        let _notation = DiceNotation::parse(&request.dice_notation)?;

        // Check for any nested entries
        let is_nested = request.entries.iter().any(|e| e.nested_table_id.is_some());

        // Create table record
        let mut table_record = RandomTableRecord::new(request.name.clone(), request.dice_notation.clone());
        if let Some(campaign_id) = &request.campaign_id {
            table_record = table_record.with_campaign(campaign_id.clone());
        }
        if let Some(category) = &request.category {
            table_record = table_record.with_category(category.clone());
        }
        table_record = table_record
            .with_type(request.table_type)
            .with_tags(&request.tags);
        if request.is_system {
            table_record = table_record.as_system();
        }
        if is_nested {
            table_record = table_record.as_nested();
        }
        table_record.description = request.description.clone();

        // Build entries in memory first
        let mut entries = Vec::new();
        for (i, entry_input) in request.entries.iter().enumerate() {
            let mut entry_record = RandomTableEntryRecord::new(
                table_record.id.clone(),
                entry_input.range_start,
                entry_input.range_end,
                entry_input.result_text.clone(),
            )
            .with_order(i as i32);

            if let Some(weight) = entry_input.weight {
                entry_record = entry_record.with_weight(weight);
            }
            if let Some(ref nested_id) = entry_input.nested_table_id {
                entry_record = entry_record.with_nested_table(nested_id.clone());
            }
            if let Some(ref metadata) = entry_input.metadata {
                entry_record = entry_record.with_metadata(metadata.clone());
            }

            entries.push(entry_record);
        }

        // Build table struct for validation BEFORE any DB operations
        let table = RandomTable {
            id: table_record.id.clone(),
            name: table_record.name.clone(),
            description: table_record.description.clone(),
            dice_notation: table_record.dice_notation.clone(),
            table_type: request.table_type,
            category: table_record.category.clone(),
            tags: request.tags.clone(),
            campaign_id: table_record.campaign_id.clone(),
            entries: entries.iter().cloned().map(TableEntry::from).collect(),
            is_system: table_record.is_system != 0,
            is_nested,
            created_at: table_record.created_at.clone(),
            updated_at: table_record.updated_at.clone(),
        };

        // Validate coverage BEFORE inserting anything
        table.validate_coverage()?;

        // Start transaction for atomic insert
        let mut tx = self.pool.begin().await?;

        // Insert table within transaction
        sqlx::query(
            r#"
            INSERT INTO random_tables (id, campaign_id, name, description, table_type,
                dice_notation, category, tags, is_system, is_nested, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&table_record.id)
        .bind(&table_record.campaign_id)
        .bind(&table_record.name)
        .bind(&table_record.description)
        .bind(&table_record.table_type)
        .bind(&table_record.dice_notation)
        .bind(&table_record.category)
        .bind(&table_record.tags)
        .bind(table_record.is_system)
        .bind(table_record.is_nested)
        .bind(&table_record.created_at)
        .bind(&table_record.updated_at)
        .execute(&mut *tx)
        .await?;

        // Insert entries within transaction
        for entry_record in &entries {
            sqlx::query(
                r#"
                INSERT INTO random_table_entries (id, table_id, range_start, range_end,
                    weight, result_text, result_type, nested_table_id, metadata, display_order)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&entry_record.id)
            .bind(&entry_record.table_id)
            .bind(entry_record.range_start)
            .bind(entry_record.range_end)
            .bind(entry_record.weight)
            .bind(&entry_record.result_text)
            .bind(&entry_record.result_type)
            .bind(&entry_record.nested_table_id)
            .bind(&entry_record.metadata)
            .bind(entry_record.display_order)
            .execute(&mut *tx)
            .await?;
        }

        // Commit transaction
        tx.commit().await?;

        info!(table_id = %table.id, name = %table.name, "Created random table");
        Ok(table)
    }

    /// Get a table by ID
    pub async fn get_table(&self, table_id: &str) -> RandomTableResult<RandomTable> {
        let record: RandomTableRecord = sqlx::query_as(
            "SELECT * FROM random_tables WHERE id = ?"
        )
        .bind(table_id)
        .fetch_optional(self.pool.as_ref())
        .await?
        .ok_or_else(|| RandomTableError::TableNotFound(table_id.to_string()))?;

        let entries: Vec<RandomTableEntryRecord> = sqlx::query_as(
            "SELECT * FROM random_table_entries WHERE table_id = ? ORDER BY display_order"
        )
        .bind(table_id)
        .fetch_all(self.pool.as_ref())
        .await?;

        let table_type = record.table_type_enum().unwrap_or_default();
        let tags = record.tags_vec();

        Ok(RandomTable {
            id: record.id,
            name: record.name,
            description: record.description,
            dice_notation: record.dice_notation.clone(),
            table_type,
            category: record.category.clone(),
            tags,
            campaign_id: record.campaign_id,
            entries: entries.into_iter().map(TableEntry::from).collect(),
            is_system: record.is_system != 0,
            is_nested: record.is_nested != 0,
            created_at: record.created_at,
            updated_at: record.updated_at,
        })
    }

    /// List tables by campaign
    pub async fn list_tables(&self, campaign_id: Option<&str>) -> RandomTableResult<Vec<RandomTable>> {
        let records: Vec<RandomTableRecord> = if let Some(cid) = campaign_id {
            sqlx::query_as(
                "SELECT * FROM random_tables WHERE campaign_id = ? OR is_system = 1 ORDER BY category, name"
            )
            .bind(cid)
            .fetch_all(self.pool.as_ref())
            .await?
        } else {
            sqlx::query_as(
                "SELECT * FROM random_tables WHERE is_system = 1 ORDER BY category, name"
            )
            .fetch_all(self.pool.as_ref())
            .await?
        };

        let mut tables = Vec::new();
        for record in records {
            let entries: Vec<RandomTableEntryRecord> = sqlx::query_as(
                "SELECT * FROM random_table_entries WHERE table_id = ? ORDER BY display_order"
            )
            .bind(&record.id)
            .fetch_all(self.pool.as_ref())
            .await?;

            let table_type = record.table_type_enum().unwrap_or_default();
            let tags = record.tags_vec();

            tables.push(RandomTable {
                id: record.id,
                name: record.name,
                description: record.description,
                dice_notation: record.dice_notation.clone(),
                table_type,
                category: record.category.clone(),
                tags,
                campaign_id: record.campaign_id,
                entries: entries.into_iter().map(TableEntry::from).collect(),
                is_system: record.is_system != 0,
                is_nested: record.is_nested != 0,
                created_at: record.created_at,
                updated_at: record.updated_at,
            });
        }

        Ok(tables)
    }

    /// List tables by category
    pub async fn list_tables_by_category(&self, category: &str, campaign_id: Option<&str>) -> RandomTableResult<Vec<RandomTable>> {
        let records: Vec<RandomTableRecord> = if let Some(cid) = campaign_id {
            sqlx::query_as(
                "SELECT * FROM random_tables WHERE category = ? AND (campaign_id = ? OR is_system = 1) ORDER BY name"
            )
            .bind(category)
            .bind(cid)
            .fetch_all(self.pool.as_ref())
            .await?
        } else {
            sqlx::query_as(
                "SELECT * FROM random_tables WHERE category = ? AND is_system = 1 ORDER BY name"
            )
            .bind(category)
            .fetch_all(self.pool.as_ref())
            .await?
        };

        let mut tables = Vec::new();
        for record in records {
            tables.push(self.get_table(&record.id).await?);
        }

        Ok(tables)
    }

    /// Update a table
    ///
    /// Uses a transaction to ensure atomic update of table and entries.
    /// If any operation fails, all changes are rolled back.
    pub async fn update_table(&self, table_id: &str, request: CreateTableRequest) -> RandomTableResult<RandomTable> {
        // Verify table exists (outside transaction for early exit)
        let existing = self.get_table(table_id).await?;

        // Don't allow updating system tables
        if existing.is_system && !request.is_system {
            return Err(RandomTableError::InvalidConfiguration(
                "Cannot demote a system table".to_string()
            ));
        }

        // Check for nested entries
        let is_nested = request.entries.iter().any(|e| e.nested_table_id.is_some());

        // Start transaction for atomic update
        let mut tx = self.pool.begin().await?;

        // Delete existing entries within transaction
        sqlx::query("DELETE FROM random_table_entries WHERE table_id = ?")
            .bind(table_id)
            .execute(&mut *tx)
            .await?;

        // Update table record
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"
            UPDATE random_tables SET
                name = ?, description = ?, table_type = ?, dice_notation = ?,
                category = ?, tags = ?, is_nested = ?, updated_at = ?
            WHERE id = ?
            "#
        )
        .bind(&request.name)
        .bind(&request.description)
        .bind(request.table_type.as_str())
        .bind(&request.dice_notation)
        .bind(&request.category)
        .bind(serde_json::to_string(&request.tags).unwrap_or_default())
        .bind(if is_nested { 1 } else { 0 })
        .bind(&now)
        .bind(table_id)
        .execute(&mut *tx)
        .await?;

        // Insert new entries
        for (i, entry_input) in request.entries.into_iter().enumerate() {
            let mut entry_record = RandomTableEntryRecord::new(
                table_id.to_string(),
                entry_input.range_start,
                entry_input.range_end,
                entry_input.result_text.clone(),
            )
            .with_order(i as i32);

            if let Some(weight) = entry_input.weight {
                entry_record = entry_record.with_weight(weight);
            }
            if let Some(nested_id) = entry_input.nested_table_id {
                entry_record = entry_record.with_nested_table(nested_id);
            }
            if let Some(metadata) = entry_input.metadata {
                entry_record = entry_record.with_metadata(metadata);
            }

            sqlx::query(
                r#"
                INSERT INTO random_table_entries (id, table_id, range_start, range_end,
                    weight, result_text, result_type, nested_table_id, metadata, display_order)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&entry_record.id)
            .bind(&entry_record.table_id)
            .bind(entry_record.range_start)
            .bind(entry_record.range_end)
            .bind(entry_record.weight)
            .bind(&entry_record.result_text)
            .bind(&entry_record.result_type)
            .bind(&entry_record.nested_table_id)
            .bind(&entry_record.metadata)
            .bind(entry_record.display_order)
            .execute(&mut *tx)
            .await?;
        }

        // Commit transaction
        tx.commit().await?;

        // Fetch and return updated table
        self.get_table(table_id).await
    }

    /// Delete a table
    pub async fn delete_table(&self, table_id: &str) -> RandomTableResult<()> {
        let table = self.get_table(table_id).await?;

        if table.is_system {
            return Err(RandomTableError::InvalidConfiguration(
                "Cannot delete system tables".to_string()
            ));
        }

        // Entries are cascade deleted
        sqlx::query("DELETE FROM random_tables WHERE id = ?")
            .bind(table_id)
            .execute(self.pool.as_ref())
            .await?;

        info!(table_id, "Deleted random table");
        Ok(())
    }

    // ========================================================================
    // Rolling Operations
    // ========================================================================

    /// Roll on a table
    pub async fn roll_on_table(&self, request: RollRequest) -> RandomTableResult<TableRollResult> {
        self.roll_on_table_internal(
            &request.table_id,
            request.session_id.as_deref(),
            request.campaign_id.as_deref(),
            request.context.as_deref(),
            request.forced_roll,
            0,
            request.max_depth.unwrap_or(Self::MAX_NESTING_DEPTH),
            &mut Vec::new(),
        ).await
    }

    /// Internal recursive roll implementation
    async fn roll_on_table_internal(
        &self,
        table_id: &str,
        session_id: Option<&str>,
        campaign_id: Option<&str>,
        context: Option<&str>,
        forced_roll: Option<i32>,
        current_depth: u32,
        max_depth: u32,
        visited: &mut Vec<String>,
    ) -> RandomTableResult<TableRollResult> {
        // Check nesting depth
        if current_depth > max_depth {
            return Err(RandomTableError::MaxNestingDepth(max_depth));
        }

        // Check for circular references
        if visited.contains(&table_id.to_string()) {
            return Err(RandomTableError::CircularReference);
        }
        visited.push(table_id.to_string());

        // Get the table
        let table = self.get_table(table_id).await?;
        let notation = table.notation()?;

        // Roll the dice
        let roll = if let Some(forced) = forced_roll {
            // Create a fake roll result with the forced value
            super::dice::RollResult {
                notation: notation.clone(),
                rolls: vec![super::dice::SingleRoll {
                    die: notation.dice_type,
                    value: forced as u32,
                }],
                subtotal: forced,
                total: forced,
                d66_tens: if matches!(notation.dice_type, super::dice::DiceType::D66) {
                    Some((forced / 10) as u32)
                } else {
                    None
                },
                d66_ones: if matches!(notation.dice_type, super::dice::DiceType::D66) {
                    Some((forced % 10) as u32)
                } else {
                    None
                },
            }
        } else {
            self.roller.roll(&notation)
        };

        debug!(table_id, roll = roll.total, "Rolling on table");

        // Find matching entry
        let entry = table.entries.iter()
            .find(|e| e.range_start <= roll.total && roll.total <= e.range_end)
            .cloned()
            .ok_or_else(|| RandomTableError::NoMatchingEntry {
                roll: roll.total,
                table_id: table_id.to_string(),
            })?;

        // Handle nested rolls
        let mut nested_results = Vec::new();
        let mut final_text = entry.result_text.clone();

        if let Some(nested_table_id) = &entry.nested_table_id {
            let nested_result = Box::pin(self.roll_on_table_internal(
                nested_table_id,
                session_id,
                campaign_id,
                context,
                None, // Don't force nested rolls
                current_depth + 1,
                max_depth,
                visited,
            )).await?;

            // Combine text: prepend entry text if it exists
            if !entry.result_text.is_empty() {
                final_text = format!("{}: {}", entry.result_text, nested_result.final_text);
            } else {
                final_text = nested_result.final_text.clone();
            }

            nested_results.push(nested_result);
        }

        // Record roll history
        let mut history = RollHistoryRecord::new(
            notation.to_string(),
            roll.subtotal,
            notation.modifier,
        )
        .with_table_result(table_id.to_string(), entry.id.clone(), final_text.clone());

        if let Some(sid) = session_id {
            history = history.with_session(sid.to_string());
        }
        if let Some(cid) = campaign_id {
            history = history.with_campaign(cid.to_string());
        }
        if let Some(ctx) = context {
            history = history.with_context(ctx.to_string());
        }

        sqlx::query(
            r#"
            INSERT INTO roll_history (id, session_id, campaign_id, table_id, dice_notation,
                raw_roll, modifier, final_result, entry_id, result_text, context, rolled_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&history.id)
        .bind(&history.session_id)
        .bind(&history.campaign_id)
        .bind(&history.table_id)
        .bind(&history.dice_notation)
        .bind(history.raw_roll)
        .bind(history.modifier)
        .bind(history.final_result)
        .bind(&history.entry_id)
        .bind(&history.result_text)
        .bind(&history.context)
        .bind(&history.rolled_at)
        .execute(self.pool.as_ref())
        .await?;

        visited.pop();

        Ok(TableRollResult {
            table_id: table_id.to_string(),
            table_name: table.name,
            roll,
            entry,
            nested_results,
            final_text,
            history_id: history.id,
        })
    }

    /// Quick roll with just table ID
    pub async fn quick_roll(&self, table_id: &str) -> RandomTableResult<TableRollResult> {
        self.roll_on_table(RollRequest {
            table_id: table_id.to_string(),
            session_id: None,
            campaign_id: None,
            context: None,
            forced_roll: None,
            max_depth: None,
        }).await
    }

    /// Roll dice without a table (simple dice roll)
    pub async fn roll_dice(
        &self,
        notation: &str,
        session_id: Option<&str>,
        campaign_id: Option<&str>,
        context: Option<&str>,
    ) -> RandomTableResult<RollResult> {
        let parsed = DiceNotation::parse(notation)?;
        let result = self.roller.roll(&parsed);

        // Record in history
        let mut history = RollHistoryRecord::new(
            notation.to_string(),
            result.subtotal,
            parsed.modifier,
        );

        if let Some(sid) = session_id {
            history = history.with_session(sid.to_string());
        }
        if let Some(cid) = campaign_id {
            history = history.with_campaign(cid.to_string());
        }
        if let Some(ctx) = context {
            history = history.with_context(ctx.to_string());
        }

        sqlx::query(
            r#"
            INSERT INTO roll_history (id, session_id, campaign_id, table_id, dice_notation,
                raw_roll, modifier, final_result, entry_id, result_text, context, rolled_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&history.id)
        .bind(&history.session_id)
        .bind(&history.campaign_id)
        .bind(&history.table_id)
        .bind(&history.dice_notation)
        .bind(history.raw_roll)
        .bind(history.modifier)
        .bind(history.final_result)
        .bind(&history.entry_id)
        .bind(&history.result_text)
        .bind(&history.context)
        .bind(&history.rolled_at)
        .execute(self.pool.as_ref())
        .await?;

        Ok(result)
    }

    // ========================================================================
    // Roll History Operations
    // ========================================================================

    /// Get roll history for a session
    pub async fn get_session_roll_history(&self, session_id: &str, limit: u32) -> RandomTableResult<Vec<RollHistoryRecord>> {
        let records: Vec<RollHistoryRecord> = sqlx::query_as(
            "SELECT * FROM roll_history WHERE session_id = ? ORDER BY rolled_at DESC LIMIT ?"
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(records)
    }

    /// Get roll history for a campaign
    pub async fn get_campaign_roll_history(&self, campaign_id: &str, limit: u32) -> RandomTableResult<Vec<RollHistoryRecord>> {
        let records: Vec<RollHistoryRecord> = sqlx::query_as(
            "SELECT * FROM roll_history WHERE campaign_id = ? ORDER BY rolled_at DESC LIMIT ?"
        )
        .bind(campaign_id)
        .bind(limit)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(records)
    }

    /// Get roll history for a specific table
    pub async fn get_table_roll_history(&self, table_id: &str, limit: u32) -> RandomTableResult<Vec<RollHistoryRecord>> {
        let records: Vec<RollHistoryRecord> = sqlx::query_as(
            "SELECT * FROM roll_history WHERE table_id = ? ORDER BY rolled_at DESC LIMIT ?"
        )
        .bind(table_id)
        .bind(limit)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(records)
    }

    /// Clear roll history older than specified days
    pub async fn clear_old_history(&self, days: i64) -> RandomTableResult<u64> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();

        let result = sqlx::query("DELETE FROM roll_history WHERE rolled_at < ?")
            .bind(&cutoff)
            .execute(self.pool.as_ref())
            .await?;

        Ok(result.rows_affected())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_entry_input() {
        let input = TableEntryInput {
            range_start: 1,
            range_end: 5,
            result_text: "Goblin ambush".to_string(),
            weight: Some(1.0),
            nested_table_id: None,
            metadata: None,
        };

        assert_eq!(input.range_start, 1);
        assert_eq!(input.range_end, 5);
    }

    #[test]
    fn test_create_table_request_default() {
        let request = CreateTableRequest::default();
        assert_eq!(request.dice_notation, "d20");
        assert_eq!(request.table_type, RandomTableType::Standard);
        assert!(!request.is_system);
    }

    #[test]
    fn test_roll_request() {
        let request = RollRequest {
            table_id: "table-1".to_string(),
            session_id: Some("session-1".to_string()),
            campaign_id: None,
            context: Some("Combat encounter".to_string()),
            forced_roll: None,
            max_depth: Some(5),
        };

        assert_eq!(request.table_id, "table-1");
        assert!(request.session_id.is_some());
    }
}
