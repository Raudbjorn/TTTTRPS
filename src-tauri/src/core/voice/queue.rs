//! Voice Pre-Generation Queue (TASK-025)
//!
//! Manages a priority queue for voice synthesis jobs with progress tracking,
//! batch pre-generation, cancellation support, and Tauri event emission.

use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use tokio::sync::{RwLock, mpsc, watch, Mutex, oneshot};
use uuid::Uuid;
use tauri::{AppHandle, Emitter};

use super::types::{VoiceProviderType, VoiceSettings, OutputFormat};

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
    /// Job was cancelled
    Cancelled,
}


impl JobStatus {
    /// Check if job is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed(_) | Self::Cancelled)
    }

    /// Check if job can be cancelled
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
            max_retries: 2,
            char_count: text.chars().count(),
        }
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

    /// Mark job as cancelled
    pub fn mark_cancelled(&mut self) {
        self.status = JobStatus::Cancelled;
        self.completed_at = Some(Utc::now());
        self.progress.stage = "Cancelled".to_string();
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
struct PrioritizedJob {
    job: SynthesisJob,
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
    /// Total jobs cancelled
    pub cancelled_count: u64,
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
// Queue Events (Tauri)
// ============================================================================

/// Event emitted when a job is submitted
#[derive(Debug, Clone, Serialize)]
pub struct JobSubmittedEvent {
    pub job_id: String,
    pub priority: String,
    pub text_preview: String,
    pub char_count: usize,
}

/// Event emitted when job status changes
#[derive(Debug, Clone, Serialize)]
pub struct JobStatusEvent {
    pub job_id: String,
    pub status: JobStatus,
    pub progress: JobProgress,
    pub result_path: Option<String>,
    pub error: Option<String>,
}

/// Event emitted for queue statistics updates
#[derive(Debug, Clone, Serialize)]
pub struct QueueStatsEvent {
    pub stats: QueueStats,
}

/// Event channel names
pub mod events {
    pub const JOB_SUBMITTED: &str = "synthesis:job-submitted";
    pub const JOB_STARTED: &str = "synthesis:job-started";
    pub const JOB_PROGRESS: &str = "synthesis:job-progress";
    pub const JOB_COMPLETED: &str = "synthesis:job-completed";
    pub const JOB_FAILED: &str = "synthesis:job-failed";
    pub const JOB_CANCELLED: &str = "synthesis:job-cancelled";
    pub const QUEUE_STATS: &str = "synthesis:queue-stats";
    pub const QUEUE_PAUSED: &str = "synthesis:queue-paused";
    pub const QUEUE_RESUMED: &str = "synthesis:queue-resumed";
}

// ============================================================================
// Synthesis Queue
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

/// Internal queue state shared between queue and worker
struct QueueState {
    /// Priority queue for pending jobs
    pending: BinaryHeap<PrioritizedJob>,
    /// All jobs indexed by ID
    jobs: HashMap<String, SynthesisJob>,
    /// Jobs currently being processed
    processing: HashMap<String, SynthesisJob>,
    /// Completed/failed/cancelled jobs (history)
    history: Vec<SynthesisJob>,
    /// Statistics
    stats: QueueStats,
    /// Pause state
    is_paused: bool,
    /// Processing time samples (for averaging)
    processing_times: Vec<u64>,
}

impl QueueState {
    fn new() -> Self {
        Self {
            pending: BinaryHeap::new(),
            jobs: HashMap::new(),
            processing: HashMap::new(),
            history: Vec::new(),
            stats: QueueStats::default(),
            is_paused: false,
            processing_times: Vec::new(),
        }
    }

    fn update_avg_processing_time(&mut self, duration_ms: u64) {
        self.processing_times.push(duration_ms);
        if self.processing_times.len() > 100 {
            self.processing_times.remove(0);
        }
        self.stats.avg_processing_ms =
            self.processing_times.iter().sum::<u64>() as f64 / self.processing_times.len() as f64;
    }

    fn update_priority_counts(&mut self) {
        let mut counts = HashMap::new();
        for job in self.pending.iter() {
            *counts.entry(job.job.priority.to_string()).or_insert(0) += 1;
        }
        self.stats.by_priority = counts;
    }
}

/// Voice synthesis queue with priority handling, progress tracking, and Tauri event emission
pub struct SynthesisQueue {
    /// Queue configuration
    config: QueueConfig,
    /// Shared queue state
    state: Arc<RwLock<QueueState>>,
    /// Command channel sender
    command_tx: mpsc::Sender<QueueCommand>,
    /// Command channel receiver (for worker)
    #[allow(dead_code)]
    command_rx: Arc<Mutex<mpsc::Receiver<QueueCommand>>>,
    /// Shutdown signal
    shutdown_tx: watch::Sender<bool>,
    /// Shutdown receiver
    shutdown_rx: watch::Receiver<bool>,
    /// Cancellation tokens by job ID
    cancellation_tokens: Arc<RwLock<HashMap<String, oneshot::Sender<()>>>>,
}

impl SynthesisQueue {
    /// Create a new synthesis queue
    pub fn new(config: QueueConfig) -> Self {
        let (command_tx, command_rx) = mpsc::channel(32);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            config,
            state: Arc::new(RwLock::new(QueueState::new())),
            command_tx,
            command_rx: Arc::new(Mutex::new(command_rx)),
            shutdown_tx,
            shutdown_rx,
            cancellation_tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(QueueConfig::default())
    }

    /// Get the queue configuration
    pub fn config(&self) -> &QueueConfig {
        &self.config
    }

    /// Submit a new job to the queue
    pub async fn submit(&self, mut job: SynthesisJob, app_handle: Option<&AppHandle>) -> QueueResult<String> {
        let mut state = self.state.write().await;

        // Check queue size
        if self.config.max_queue_size > 0 && state.pending.len() >= self.config.max_queue_size {
            return Err(QueueError::QueueFull);
        }

        // Set default max retries if not set
        if job.max_retries == 0 && self.config.auto_retry {
            job.max_retries = self.config.default_max_retries;
        }

        let job_id = job.id.clone();
        let text_preview = if job.text.len() > 50 {
            format!("{}...", &job.text[..50])
        } else {
            job.text.clone()
        };

        // Add to jobs map
        state.jobs.insert(job_id.clone(), job.clone());

        // Add to priority queue
        state.pending.push(PrioritizedJob { job: job.clone() });

        // Update stats
        state.stats.total_submitted += 1;
        state.stats.pending_count = state.pending.len();
        state.update_priority_counts();

        // Emit event
        if let Some(handle) = app_handle {
            let _ = handle.emit(events::JOB_SUBMITTED, JobSubmittedEvent {
                job_id: job_id.clone(),
                priority: job.priority.to_string(),
                text_preview,
                char_count: job.char_count,
            });
        }

        log::info!("Synthesis job {} submitted (priority: {:?}, chars: {})",
            job_id, job.priority, job.char_count);

        Ok(job_id)
    }

    /// Submit multiple jobs as a batch
    pub async fn submit_batch(&self, jobs: Vec<SynthesisJob>, app_handle: Option<&AppHandle>) -> QueueResult<Vec<String>> {
        let mut ids = Vec::with_capacity(jobs.len());

        for job in jobs {
            let id = self.submit(job, app_handle).await?;
            ids.push(id);
        }

        Ok(ids)
    }

    /// Get a job by ID
    pub async fn get_job(&self, job_id: &str) -> Option<SynthesisJob> {
        let state = self.state.read().await;
        state.jobs.get(job_id).cloned()
    }

    /// Get job status
    pub async fn get_status(&self, job_id: &str) -> Option<JobStatus> {
        let state = self.state.read().await;
        state.jobs.get(job_id).map(|j| j.status.clone())
    }

    /// Get job progress
    pub async fn get_progress(&self, job_id: &str) -> Option<JobProgress> {
        let state = self.state.read().await;
        state.jobs.get(job_id).map(|j| j.progress.clone())
    }

    /// Cancel a job
    pub async fn cancel(&self, job_id: &str, app_handle: Option<&AppHandle>) -> QueueResult<()> {
        // First, check the job status and determine what action to take
        let (_was_pending, was_processing, progress) = {
            let mut state = self.state.write().await;

            let job = state.jobs.get_mut(job_id)
                .ok_or_else(|| QueueError::JobNotFound(job_id.to_string()))?;

            let was_pending = matches!(job.status, JobStatus::Pending);
            let was_processing = matches!(job.status, JobStatus::Processing);

            if !was_pending && !was_processing {
                return Err(QueueError::InvalidState(format!(
                    "Cannot cancel job in {:?} state",
                    job.status
                )));
            }

            job.mark_cancelled();
            let progress = job.progress.clone();

            if was_pending {
                // Remove from pending queue
                let mut remaining: Vec<_> = state.pending.drain().collect();
                remaining.retain(|p| p.job.id != job_id);
                for pj in remaining {
                    state.pending.push(pj);
                }
                state.stats.cancelled_count += 1;
                state.stats.pending_count = state.pending.len();
            } else if was_processing {
                state.processing.remove(job_id);
                state.stats.cancelled_count += 1;
                state.stats.processing_count = state.processing.len();
            }

            (was_pending, was_processing, progress)
        };

        // Signal cancellation to worker (outside the state lock)
        if was_processing {
            if let Some(token) = self.cancellation_tokens.write().await.remove(job_id) {
                let _ = token.send(());
            }
        }

        // Emit event
        if let Some(handle) = app_handle {
            let _ = handle.emit(events::JOB_CANCELLED, JobStatusEvent {
                job_id: job_id.to_string(),
                status: JobStatus::Cancelled,
                progress,
                result_path: None,
                error: None,
            });
        }

        log::info!("Synthesis job {} cancelled", job_id);

        Ok(())
    }

    /// Cancel all pending and processing jobs
    pub async fn cancel_all(&self, app_handle: Option<&AppHandle>) -> QueueResult<usize> {
        let mut state = self.state.write().await;
        let mut cancelled = 0;

        // Cancel pending jobs
        while let Some(pj) = state.pending.pop() {
            if let Some(job) = state.jobs.get_mut(&pj.job.id) {
                job.mark_cancelled();
                cancelled += 1;
            }
        }

        // Cancel processing jobs
        let processing_ids: Vec<String> = state.processing.keys().cloned().collect();
        for job_id in &processing_ids {
            if let Some(token) = self.cancellation_tokens.write().await.remove(job_id) {
                let _ = token.send(());
            }
            if let Some(job) = state.jobs.get_mut(job_id) {
                job.mark_cancelled();
                cancelled += 1;
            }
        }
        state.processing.clear();

        state.stats.cancelled_count += cancelled as u64;
        state.stats.pending_count = 0;
        state.stats.processing_count = 0;

        // Emit stats event
        if let Some(handle) = app_handle {
            let _ = handle.emit(events::QUEUE_STATS, QueueStatsEvent {
                stats: state.stats.clone(),
            });
        }

        log::info!("Cancelled {} synthesis jobs", cancelled);

        Ok(cancelled)
    }

    /// Pause queue processing
    pub async fn pause(&self, app_handle: Option<&AppHandle>) {
        let mut state = self.state.write().await;
        state.is_paused = true;

        if let Some(handle) = app_handle {
            let _ = handle.emit(events::QUEUE_PAUSED, ());
        }

        log::info!("Synthesis queue paused");
    }

    /// Resume queue processing
    pub async fn resume(&self, app_handle: Option<&AppHandle>) {
        let mut state = self.state.write().await;
        state.is_paused = false;

        if let Some(handle) = app_handle {
            let _ = handle.emit(events::QUEUE_RESUMED, ());
        }

        log::info!("Synthesis queue resumed");
    }

    /// Check if queue is paused
    pub async fn is_paused(&self) -> bool {
        self.state.read().await.is_paused
    }

    /// Get the next pending job (for worker)
    pub async fn next_job(&self) -> Option<SynthesisJob> {
        let state = self.state.read().await;

        if state.is_paused {
            return None;
        }

        if state.processing.len() >= self.config.max_concurrent {
            return None;
        }

        drop(state);

        let mut state = self.state.write().await;
        state.pending.pop().map(|pj| pj.job)
    }

    /// Mark a job as started (called by worker)
    pub async fn mark_started(&self, job_id: &str, app_handle: Option<&AppHandle>) -> QueueResult<oneshot::Receiver<()>> {
        let mut state = self.state.write().await;

        let job = state.jobs.get_mut(job_id)
            .ok_or_else(|| QueueError::JobNotFound(job_id.to_string()))?;

        job.mark_processing();
        let job_clone = job.clone();

        state.processing.insert(job_id.to_string(), job_clone.clone());
        state.stats.processing_count = state.processing.len();
        state.stats.pending_count = state.pending.len();

        // Create cancellation token
        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.cancellation_tokens.write().await.insert(job_id.to_string(), cancel_tx);

        // Emit event
        if let Some(handle) = app_handle {
            let _ = handle.emit(events::JOB_STARTED, JobStatusEvent {
                job_id: job_id.to_string(),
                status: JobStatus::Processing,
                progress: job_clone.progress,
                result_path: None,
                error: None,
            });
        }

        log::debug!("Synthesis job {} started", job_id);

        Ok(cancel_rx)
    }

    /// Update job progress (called by worker)
    pub async fn update_progress(&self, job_id: &str, progress: f32, stage: &str, app_handle: Option<&AppHandle>) -> QueueResult<()> {
        let mut state = self.state.write().await;

        let job = state.jobs.get_mut(job_id)
            .ok_or_else(|| QueueError::JobNotFound(job_id.to_string()))?;

        job.update_progress(progress, stage);

        // Emit event
        if let Some(handle) = app_handle {
            let _ = handle.emit(events::JOB_PROGRESS, JobStatusEvent {
                job_id: job_id.to_string(),
                status: job.status.clone(),
                progress: job.progress.clone(),
                result_path: None,
                error: None,
            });
        }

        Ok(())
    }

    /// Mark a job as completed (called by worker)
    pub async fn mark_completed(&self, job_id: &str, result_path: &str, app_handle: Option<&AppHandle>) -> QueueResult<()> {
        let mut state = self.state.write().await;

        let job = state.jobs.get_mut(job_id)
            .ok_or_else(|| QueueError::JobNotFound(job_id.to_string()))?;

        let duration_ms = job.started_at
            .map(|s| (Utc::now() - s).num_milliseconds() as u64)
            .unwrap_or(0);

        job.mark_completed(result_path);
        let job_clone = job.clone();

        state.processing.remove(job_id);
        self.cancellation_tokens.write().await.remove(job_id);

        // Update stats
        state.stats.completed_count += 1;
        state.stats.processing_count = state.processing.len();
        state.stats.total_chars_processed += job_clone.char_count as u64;
        state.update_avg_processing_time(duration_ms);

        // Add to history
        if state.history.len() >= self.config.max_history {
            state.history.remove(0);
        }
        state.history.push(job_clone.clone());

        // Emit event
        if let Some(handle) = app_handle {
            let _ = handle.emit(events::JOB_COMPLETED, JobStatusEvent {
                job_id: job_id.to_string(),
                status: JobStatus::Completed,
                progress: job_clone.progress,
                result_path: Some(result_path.to_string()),
                error: None,
            });
        }

        log::info!("Synthesis job {} completed in {}ms", job_id, duration_ms);

        Ok(())
    }

    /// Mark a job as failed (called by worker)
    pub async fn mark_failed(&self, job_id: &str, error: &str, app_handle: Option<&AppHandle>) -> QueueResult<()> {
        // Remove cancellation token first (outside state lock)
        self.cancellation_tokens.write().await.remove(job_id);

        let (should_retry, retry_count, job_for_event) = {
            let mut state = self.state.write().await;

            // Check if job exists
            if !state.jobs.contains_key(job_id) {
                return Err(QueueError::JobNotFound(job_id.to_string()));
            }

            // Remove from processing
            state.processing.remove(job_id);

            // Get the job and check for retry
            let job = state.jobs.get_mut(job_id).unwrap();
            let should_retry = self.config.auto_retry && job.can_retry();

            if should_retry {
                job.increment_retry();
                let job_clone = job.clone();
                let retry_count = job.retry_count;

                // Re-queue
                state.pending.push(PrioritizedJob { job: job_clone });
                state.stats.processing_count = state.processing.len();
                state.stats.pending_count = state.pending.len();

                (true, retry_count, None)
            } else {
                job.mark_failed(error);
                let job_clone = job.clone();

                state.stats.failed_count += 1;
                state.stats.processing_count = state.processing.len();

                // Add to history
                if state.history.len() >= self.config.max_history {
                    state.history.remove(0);
                }
                state.history.push(job_clone.clone());

                (false, 0, Some(job_clone))
            }
        };

        if should_retry {
            log::info!("Synthesis job {} failed, retrying (attempt {})", job_id, retry_count);
        } else if let Some(job_clone) = job_for_event {
            // Emit event
            if let Some(handle) = app_handle {
                let _ = handle.emit(events::JOB_FAILED, JobStatusEvent {
                    job_id: job_id.to_string(),
                    status: JobStatus::Failed(error.to_string()),
                    progress: job_clone.progress,
                    result_path: None,
                    error: Some(error.to_string()),
                });
            }

            log::error!("Synthesis job {} failed: {}", job_id, error);
        }

        Ok(())
    }

    /// Get queue statistics
    pub async fn stats(&self) -> QueueStats {
        let state = self.state.read().await;
        let mut stats = state.stats.clone();

        // Calculate utilization
        if self.config.max_concurrent > 0 {
            stats.utilization = state.processing.len() as f64 / self.config.max_concurrent as f64;
        }

        stats
    }

    /// List all pending jobs
    pub async fn list_pending(&self) -> Vec<SynthesisJob> {
        let state = self.state.read().await;
        state.pending.iter().map(|pj| pj.job.clone()).collect()
    }

    /// List all processing jobs
    pub async fn list_processing(&self) -> Vec<SynthesisJob> {
        let state = self.state.read().await;
        state.processing.values().cloned().collect()
    }

    /// List job history (completed/failed/cancelled)
    pub async fn list_history(&self, limit: Option<usize>) -> Vec<SynthesisJob> {
        let state = self.state.read().await;
        let limit = limit.unwrap_or(state.history.len());
        state.history.iter().rev().take(limit).cloned().collect()
    }

    /// List jobs by tag
    pub async fn list_by_tag(&self, tag: &str) -> Vec<SynthesisJob> {
        let state = self.state.read().await;
        state.jobs
            .values()
            .filter(|j| j.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// List jobs by session
    pub async fn list_by_session(&self, session_id: &str) -> Vec<SynthesisJob> {
        let state = self.state.read().await;
        state.jobs
            .values()
            .filter(|j| j.session_id.as_deref() == Some(session_id))
            .cloned()
            .collect()
    }

    /// List jobs by NPC
    pub async fn list_by_npc(&self, npc_id: &str) -> Vec<SynthesisJob> {
        let state = self.state.read().await;
        state.jobs
            .values()
            .filter(|j| j.npc_id.as_deref() == Some(npc_id))
            .cloned()
            .collect()
    }

    /// Create session batch pre-generation
    ///
    /// Pre-generates voice audio for a list of texts associated with a session.
    pub async fn pregen_session(
        &self,
        session_id: &str,
        texts: Vec<(String, String, String)>, // (text, profile_id, voice_id)
        provider: VoiceProviderType,
        app_handle: Option<&AppHandle>,
    ) -> QueueResult<Vec<String>> {
        let mut job_ids = Vec::new();

        for (text, profile_id, voice_id) in texts {
            let job = SynthesisJob::new(&text, &profile_id, provider.clone(), &voice_id)
                .with_priority(JobPriority::Batch)
                .for_session(session_id);

            let id = self.submit(job, app_handle).await?;
            job_ids.push(id);
        }

        log::info!("Pre-generation batch queued for session {} ({} jobs)",
            session_id, job_ids.len());

        Ok(job_ids)
    }

    /// Clear job history
    pub async fn clear_history(&self) {
        let mut state = self.state.write().await;

        // Remove terminal jobs from jobs map
        state.jobs.retain(|_, job| !job.status.is_terminal());

        // Clear history
        state.history.clear();

        log::info!("Synthesis queue history cleared");
    }

    /// Shutdown the queue
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        let _ = self.command_tx.send(QueueCommand::Shutdown).await;
        log::info!("Synthesis queue shutdown initiated");
    }

    /// Get shutdown receiver (for worker)
    pub fn shutdown_receiver(&self) -> watch::Receiver<bool> {
        self.shutdown_rx.clone()
    }

    /// Get queue length
    pub async fn len(&self) -> usize {
        self.state.read().await.pending.len()
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        self.state.read().await.pending.is_empty()
    }

    /// Get total items (pending + processing)
    pub async fn total_active(&self) -> usize {
        let state = self.state.read().await;
        state.pending.len() + state.processing.len()
    }
}

impl Default for SynthesisQueue {
    fn default() -> Self {
        Self::with_defaults()
    }
}

// ============================================================================
// Background Worker
// ============================================================================

/// Trait for voice synthesis backends (implemented by VoiceManager)
#[async_trait::async_trait]
pub trait VoiceSynthesizer: Send + Sync {
    /// Synthesize text to audio file, returning the path
    async fn synthesize_to_file(
        &self,
        text: &str,
        voice_id: &str,
        provider: &VoiceProviderType,
        output_format: OutputFormat,
    ) -> Result<String, String>;
}

/// Background worker that processes the synthesis queue
pub struct QueueWorker {
    queue: Arc<SynthesisQueue>,
    synthesizer: Arc<dyn VoiceSynthesizer>,
    app_handle: AppHandle,
}

impl QueueWorker {
    /// Create a new queue worker
    pub fn new(
        queue: Arc<SynthesisQueue>,
        synthesizer: Arc<dyn VoiceSynthesizer>,
        app_handle: AppHandle,
    ) -> Self {
        Self {
            queue,
            synthesizer,
            app_handle,
        }
    }

    /// Run the worker (blocks until shutdown)
    pub async fn run(&self) {
        let mut shutdown_rx = self.queue.shutdown_receiver();
        let poll_interval = tokio::time::Duration::from_millis(self.queue.config.poll_interval_ms);

        log::info!("Synthesis queue worker started");

        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        log::info!("Synthesis queue worker shutting down");
                        break;
                    }
                }
                _ = tokio::time::sleep(poll_interval) => {
                    self.process_next().await;
                }
            }
        }
    }

    /// Process the next job in the queue
    async fn process_next(&self) {
        // Get next job
        let job = match self.queue.next_job().await {
            Some(j) => j,
            None => return,
        };

        let job_id = job.id.clone();

        // Mark as started and get cancellation token
        let cancel_rx = match self.queue.mark_started(&job_id, Some(&self.app_handle)).await {
            Ok(rx) => rx,
            Err(e) => {
                log::error!("Failed to mark job {} as started: {}", job_id, e);
                return;
            }
        };

        // Update progress: starting synthesis
        let _ = self.queue.update_progress(&job_id, 0.1, "Connecting to provider", Some(&self.app_handle)).await;

        // Perform synthesis with cancellation support
        let synthesizer = self.synthesizer.clone();
        let text = job.text.clone();
        let voice_id = job.voice_id.clone();
        let provider = job.provider.clone();
        let output_format = job.output_format.clone();

        let synthesis_future = async move {
            synthesizer.synthesize_to_file(&text, &voice_id, &provider, output_format).await
        };

        tokio::select! {
            result = synthesis_future => {
                match result {
                    Ok(path) => {
                        let _ = self.queue.mark_completed(&job_id, &path, Some(&self.app_handle)).await;
                    }
                    Err(e) => {
                        let _ = self.queue.mark_failed(&job_id, &e, Some(&self.app_handle)).await;
                    }
                }
            }
            _ = cancel_rx => {
                log::info!("Synthesis job {} cancelled during processing", job_id);
                // Job already marked as cancelled by cancel()
            }
        }

        // Emit updated stats
        let stats = self.queue.stats().await;
        let _ = self.app_handle.emit(events::QUEUE_STATS, QueueStatsEvent { stats });
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_job(text: &str) -> SynthesisJob {
        SynthesisJob::new(text, "profile-1", VoiceProviderType::OpenAI, "alloy")
    }

    #[tokio::test]
    async fn test_submit_job() {
        let queue = SynthesisQueue::with_defaults();

        let job = create_test_job("Hello world");
        let job_id = queue.submit(job, None).await.unwrap();

        assert!(!job_id.is_empty());

        let retrieved = queue.get_job(&job_id).await.unwrap();
        assert_eq!(retrieved.text, "Hello world");
        assert_eq!(retrieved.status, JobStatus::Pending);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let queue = SynthesisQueue::with_defaults();

        // Submit low priority first
        let low = create_test_job("Low priority").with_priority(JobPriority::Low);
        let low_id = queue.submit(low, None).await.unwrap();

        // Submit high priority second
        let high = create_test_job("High priority").with_priority(JobPriority::High);
        let high_id = queue.submit(high, None).await.unwrap();

        // Submit immediate priority third
        let immediate = create_test_job("Immediate priority").with_priority(JobPriority::Immediate);
        let immediate_id = queue.submit(immediate, None).await.unwrap();

        // Next job should be immediate priority
        let next = queue.next_job().await.unwrap();
        assert_eq!(next.id, immediate_id);

        // Then high priority
        let next = queue.next_job().await.unwrap();
        assert_eq!(next.id, high_id);

        // Then low priority
        let next = queue.next_job().await.unwrap();
        assert_eq!(next.id, low_id);
    }

    #[tokio::test]
    async fn test_cancel_job() {
        let queue = SynthesisQueue::with_defaults();

        let job = create_test_job("To be cancelled");
        let job_id = queue.submit(job, None).await.unwrap();

        queue.cancel(&job_id, None).await.unwrap();

        let status = queue.get_status(&job_id).await.unwrap();
        assert!(matches!(status, JobStatus::Cancelled));
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let queue = SynthesisQueue::with_defaults();

        let job = create_test_job("Test");
        queue.submit(job, None).await.unwrap();

        // Pause queue
        queue.pause(None).await;
        assert!(queue.is_paused().await);

        // Should not return jobs when paused
        let next = queue.next_job().await;
        assert!(next.is_none());

        // Resume queue
        queue.resume(None).await;
        assert!(!queue.is_paused().await);

        // Should return job now
        let next = queue.next_job().await;
        assert!(next.is_some());
    }

    #[tokio::test]
    async fn test_queue_stats() {
        let queue = SynthesisQueue::with_defaults();

        for i in 0..5 {
            let job = create_test_job(&format!("Job {}", i));
            queue.submit(job, None).await.unwrap();
        }

        let stats = queue.stats().await;
        assert_eq!(stats.total_submitted, 5);
        assert_eq!(stats.pending_count, 5);
    }

    #[tokio::test]
    async fn test_session_batch() {
        let queue = SynthesisQueue::with_defaults();

        let texts = vec![
            ("Hello".to_string(), "profile-1".to_string(), "alloy".to_string()),
            ("World".to_string(), "profile-2".to_string(), "echo".to_string()),
        ];

        let job_ids = queue
            .pregen_session("session-123", texts, VoiceProviderType::OpenAI, None)
            .await
            .unwrap();

        assert_eq!(job_ids.len(), 2);

        // Check session filter
        let session_jobs = queue.list_by_session("session-123").await;
        assert_eq!(session_jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_job_lifecycle() {
        let queue = SynthesisQueue::with_defaults();

        let job = create_test_job("Lifecycle test");
        let job_id = queue.submit(job, None).await.unwrap();

        // Get next job (simulating worker)
        let _job = queue.next_job().await.unwrap();

        // Start processing
        queue.mark_started(&job_id, None).await.unwrap();
        let status = queue.get_status(&job_id).await.unwrap();
        assert!(matches!(status, JobStatus::Processing));

        // Update progress
        queue.update_progress(&job_id, 0.5, "Synthesizing", None).await.unwrap();
        let progress = queue.get_progress(&job_id).await.unwrap();
        assert_eq!(progress.progress, 0.5);
        assert_eq!(progress.stage, "Synthesizing");

        // Complete
        queue.mark_completed(&job_id, "/path/to/audio.mp3", None).await.unwrap();
        let status = queue.get_status(&job_id).await.unwrap();
        assert!(matches!(status, JobStatus::Completed));

        let job = queue.get_job(&job_id).await.unwrap();
        assert_eq!(job.result_path, Some("/path/to/audio.mp3".to_string()));
    }

    #[tokio::test]
    async fn test_job_with_tags() {
        let queue = SynthesisQueue::with_defaults();

        let job = create_test_job("Tagged job")
            .for_campaign("campaign-1")
            .for_session("session-1")
            .for_npc("npc-1")
            .with_tag("custom-tag");

        let job_id = queue.submit(job, None).await.unwrap();

        let retrieved = queue.get_job(&job_id).await.unwrap();
        assert!(retrieved.tags.contains(&"campaign:campaign-1".to_string()));
        assert!(retrieved.tags.contains(&"session:session-1".to_string()));
        assert!(retrieved.tags.contains(&"npc:npc-1".to_string()));
        assert!(retrieved.tags.contains(&"custom-tag".to_string()));
    }

    #[tokio::test]
    async fn test_max_queue_size() {
        let config = QueueConfig {
            max_queue_size: 2,
            ..Default::default()
        };
        let queue = SynthesisQueue::new(config);

        // Fill queue
        queue.submit(create_test_job("Job 1"), None).await.unwrap();
        queue.submit(create_test_job("Job 2"), None).await.unwrap();

        // Third should fail
        let result = queue.submit(create_test_job("Job 3"), None).await;
        assert!(matches!(result, Err(QueueError::QueueFull)));
    }

    #[tokio::test]
    async fn test_clear_history() {
        let queue = SynthesisQueue::with_defaults();

        let job = create_test_job("Test");
        let job_id = queue.submit(job, None).await.unwrap();

        // Simulate completion
        let _job = queue.next_job().await.unwrap();
        queue.mark_started(&job_id, None).await.unwrap();
        queue.mark_completed(&job_id, "/path/to/audio.mp3", None).await.unwrap();

        // History should have the job
        let history = queue.list_history(None).await;
        assert_eq!(history.len(), 1);

        // Clear history
        queue.clear_history().await;

        let history = queue.list_history(None).await;
        assert_eq!(history.len(), 0);
    }
}
