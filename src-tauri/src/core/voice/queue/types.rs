//! Voice synthesis queue types and data models.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use uuid::Uuid;

use crate::core::voice::types::{OutputFormat, VoiceProviderType, VoiceSettings};

// ============================================================================
// Queue Types
// ============================================================================

/// Priority levels for synthesis jobs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum JobPriority {
    /// Highest priority - immediate playback requested
    Immediate = 100,
    /// High priority - user is actively waiting
    High = 75,
    /// Normal priority - standard queue processing
    #[default]
    Normal = 50,
    /// Low priority - background pre-generation
    Low = 25,
    /// Lowest priority - batch operations
    Batch = 10,
}

impl From<u8> for JobPriority {
    fn from(value: u8) -> Self {
        match value {
            v if v >= 100 => Self::Immediate,
            v if v >= 75 => Self::High,
            v if v >= 50 => Self::Normal,
            v if v >= 25 => Self::Low,
            _ => Self::Batch,
        }
    }
}

impl std::fmt::Display for JobPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Immediate => write!(f, "immediate"),
            Self::High => write!(f, "high"),
            Self::Normal => write!(f, "normal"),
            Self::Low => write!(f, "low"),
            Self::Batch => write!(f, "batch"),
        }
    }
}

/// Status of a synthesis job
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum JobStatus {
    /// Job is waiting in queue
    #[default]
    Pending,
    /// Job is currently being processed
    Processing,
    /// Job completed successfully
    Completed,
    /// Job failed with error
    Failed(String),
    /// Job was canceled
    #[serde(alias = "Cancelled")]
    Canceled,
}

impl JobStatus {
    /// Check if job is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed(_) | Self::Canceled)
    }

    /// Check if job can be canceled
    pub fn can_cancel(&self) -> bool {
        matches!(self, Self::Pending | Self::Processing)
    }
}

/// Progress information for a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    /// Progress percentage (0.0 - 1.0)
    pub progress: f32,
    /// Current stage description
    pub stage: String,
    /// Estimated time remaining in seconds
    pub eta_seconds: Option<u32>,
    /// Bytes processed so far
    pub bytes_processed: u64,
    /// Total bytes (if known)
    pub total_bytes: Option<u64>,
}

impl Default for JobProgress {
    fn default() -> Self {
        Self {
            progress: 0.0,
            stage: "Pending".to_string(),
            eta_seconds: None,
            bytes_processed: 0,
            total_bytes: None,
        }
    }
}

/// A voice synthesis job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisJob {
    /// Unique job identifier
    pub id: String,
    /// Text to synthesize
    pub text: String,
    /// Voice profile ID to use
    pub profile_id: String,
    /// Voice provider type
    pub provider: VoiceProviderType,
    /// Provider-specific voice ID
    pub voice_id: String,
    /// Voice settings
    pub settings: VoiceSettings,
    /// Output format
    pub output_format: OutputFormat,
    /// Job priority
    pub priority: JobPriority,
    /// Current status
    pub status: JobStatus,
    /// Progress information
    pub progress: JobProgress,
    /// Tags for categorization (session_id, npc_id, etc.)
    pub tags: Vec<String>,
    /// Associated campaign ID
    pub campaign_id: Option<String>,
    /// Associated session ID
    pub session_id: Option<String>,
    /// Associated NPC ID
    pub npc_id: Option<String>,
    /// When the job was created
    pub created_at: DateTime<Utc>,
    /// When the job started processing
    pub started_at: Option<DateTime<Utc>>,
    /// When the job completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Result path (if completed successfully)
    pub result_path: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Maximum retries allowed
    pub max_retries: u32,
    /// Estimated character count (for cost calculation)
    pub char_count: usize,
}

impl SynthesisJob {
    /// Create a new synthesis job
    pub fn new(
        text: &str,
        profile_id: &str,
        provider: VoiceProviderType,
        voice_id: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            text: text.to_string(),
            profile_id: profile_id.to_string(),
            provider,
            voice_id: voice_id.to_string(),
            settings: VoiceSettings::default(),
            output_format: OutputFormat::Mp3,
            priority: JobPriority::Normal,
            status: JobStatus::Pending,
            progress: JobProgress::default(),
            tags: Vec::new(),
            campaign_id: None,
            session_id: None,
            npc_id: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result_path: None,
            error: None,
            retry_count: 0,
            max_retries: 0, // Default to 0; queue applies config.default_max_retries
            char_count: text.chars().count(),
        }
    }

    /// Set maximum retry attempts
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set job priority
    pub fn with_priority(mut self, priority: JobPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set voice settings
    pub fn with_settings(mut self, settings: VoiceSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Set output format
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Add multiple tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }

    /// Set campaign ID
    pub fn for_campaign(mut self, campaign_id: &str) -> Self {
        self.campaign_id = Some(campaign_id.to_string());
        self.tags.push(format!("campaign:{}", campaign_id));
        self
    }

    /// Set session ID
    pub fn for_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self.tags.push(format!("session:{}", session_id));
        self
    }

    /// Set NPC ID
    pub fn for_npc(mut self, npc_id: &str) -> Self {
        self.npc_id = Some(npc_id.to_string());
        self.tags.push(format!("npc:{}", npc_id));
        self
    }

    /// Mark job as processing
    pub fn mark_processing(&mut self) {
        self.status = JobStatus::Processing;
        self.started_at = Some(Utc::now());
        self.progress.stage = "Processing".to_string();
    }

    /// Mark job as completed
    pub fn mark_completed(&mut self, result_path: &str) {
        self.status = JobStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.result_path = Some(result_path.to_string());
        self.progress.progress = 1.0;
        self.progress.stage = "Completed".to_string();
    }

    /// Mark job as failed
    pub fn mark_failed(&mut self, error: &str) {
        self.status = JobStatus::Failed(error.to_string());
        self.completed_at = Some(Utc::now());
        self.error = Some(error.to_string());
        self.progress.stage = "Failed".to_string();
    }

    /// Mark job as canceled
    pub fn mark_canceled(&mut self) {
        self.status = JobStatus::Canceled;
        self.completed_at = Some(Utc::now());
        self.progress.stage = "Canceled".to_string();
    }

    /// Update progress
    pub fn update_progress(&mut self, progress: f32, stage: &str) {
        self.progress.progress = progress.clamp(0.0, 1.0);
        self.progress.stage = stage.to_string();
    }

    /// Check if job can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Increment retry count and reset status
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.status = JobStatus::Pending;
        self.error = None;
        self.progress = JobProgress::default();
        self.started_at = None;
    }

    /// Get job age in seconds
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.created_at).num_seconds()
    }

    /// Get processing duration in seconds
    pub fn processing_seconds(&self) -> Option<i64> {
        self.started_at.map(|start| {
            let end = self.completed_at.unwrap_or_else(Utc::now);
            (end - start).num_seconds()
        })
    }
}

/// Wrapper for priority queue ordering
#[derive(Debug, Clone)]
pub(crate) struct PrioritizedJob {
    pub job: SynthesisJob,
}

impl PartialEq for PrioritizedJob {
    fn eq(&self, other: &Self) -> bool {
        self.job.id == other.job.id
    }
}

impl Eq for PrioritizedJob {}

impl PartialOrd for PrioritizedJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedJob {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first (reverse comparison since BinaryHeap is max-heap)
        let priority_cmp = (self.job.priority as u8).cmp(&(other.job.priority as u8));
        if priority_cmp != Ordering::Equal {
            return priority_cmp;
        }

        // Older jobs first (FIFO within same priority)
        other.job.created_at.cmp(&self.job.created_at)
    }
}

// ============================================================================
// Queue Configuration
// ============================================================================

/// Configuration for the synthesis queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Maximum number of concurrent jobs
    pub max_concurrent: usize,
    /// Maximum queue size (0 = unlimited)
    pub max_queue_size: usize,
    /// Enable automatic retries
    pub auto_retry: bool,
    /// Default max retries per job
    pub default_max_retries: u32,
    /// Timeout for synthesis in seconds
    pub synthesis_timeout_secs: u64,
    /// Enable job persistence
    pub persist_jobs: bool,
    /// Worker poll interval in milliseconds
    pub poll_interval_ms: u64,
    /// Maximum job history to keep
    pub max_history: usize,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 2,
            max_queue_size: 100,
            auto_retry: true,
            default_max_retries: 2,
            synthesis_timeout_secs: 60,
            persist_jobs: false,
            poll_interval_ms: 100,
            max_history: 100,
        }
    }
}

// ============================================================================
// Queue Statistics
// ============================================================================

/// Statistics about the synthesis queue
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueueStats {
    /// Total jobs submitted
    pub total_submitted: u64,
    /// Jobs currently pending
    pub pending_count: usize,
    /// Jobs currently processing
    pub processing_count: usize,
    /// Total jobs completed
    pub completed_count: u64,
    /// Total jobs failed
    pub failed_count: u64,
    /// Total jobs canceled
    pub canceled_count: u64,
    /// Average processing time in milliseconds
    pub avg_processing_ms: f64,
    /// Queue utilization (0.0 - 1.0)
    pub utilization: f64,
    /// Total characters processed
    pub total_chars_processed: u64,
    /// Jobs by priority
    pub by_priority: HashMap<String, usize>,
}

// ============================================================================
// Queue Command & Error
// ============================================================================

/// Command for queue control
#[derive(Debug)]
pub enum QueueCommand {
    /// Pause queue processing
    Pause,
    /// Resume queue processing
    Resume,
    /// Cancel a specific job
    Cancel(String),
    /// Cancel all jobs
    CancelAll,
    /// Clear job history
    ClearHistory,
    /// Shutdown the queue
    Shutdown,
}

/// Error type for queue operations
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue is full")]
    QueueFull,

    #[error("Job not found: {0}")]
    JobNotFound(String),

    #[error("Queue is paused")]
    Paused,

    #[error("Queue is shutdown")]
    Shutdown,

    #[error("Invalid job state: {0}")]
    InvalidState(String),

    #[error("Synthesis failed: {0}")]
    SynthesisFailed(String),

    #[error("Timeout")]
    Timeout,

    #[error("Worker error: {0}")]
    WorkerError(String),
}

pub type QueueResult<T> = Result<T, QueueError>;
