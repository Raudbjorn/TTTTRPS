//! Voice Pre-Generation Queue (TASK-025)
//!
//! Manages a priority queue for voice synthesis jobs with progress tracking,
//! batch pre-generation, cancellation support, and Tauri event emission.

pub mod events;
mod types;
mod worker;

#[cfg(test)]
mod tests;

// Re-export public API
pub use events::{channels, JobStatusEvent, JobSubmittedEvent, QueueStatsEvent};
pub use types::{
    JobPriority, JobProgress, JobStatus, QueueCommand, QueueConfig, QueueError, QueueResult,
    QueueStats, SynthesisJob,
};
pub use worker::{QueueWorker, VoiceSynthesizer};

use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;

use chrono::Utc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, oneshot, watch, Mutex, RwLock};

use types::PrioritizedJob;

use super::types::VoiceProviderType;

// ============================================================================
// Internal Queue State
// ============================================================================

/// Internal queue state shared between queue and worker
struct QueueState {
    /// Priority queue for pending jobs
    pending: BinaryHeap<PrioritizedJob>,
    /// All jobs indexed by ID
    jobs: HashMap<String, SynthesisJob>,
    /// Jobs currently being processed
    processing: HashMap<String, SynthesisJob>,
    /// Completed/failed/canceled jobs (history)
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

// ============================================================================
// Synthesis Queue
// ============================================================================

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
    pub async fn submit(
        &self,
        mut job: SynthesisJob,
        app_handle: Option<&AppHandle>,
    ) -> QueueResult<String> {
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
        let text_preview = {
            let chars: String = job.text.chars().take(50).collect();
            if job.text.chars().count() > 50 {
                format!("{}...", chars)
            } else {
                chars
            }
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
            let _ = handle.emit(
                channels::JOB_SUBMITTED,
                JobSubmittedEvent {
                    job_id: job_id.clone(),
                    priority: job.priority.to_string(),
                    text_preview,
                    char_count: job.char_count,
                },
            );
        }

        log::info!(
            "Synthesis job {} submitted (priority: {:?}, chars: {})",
            job_id,
            job.priority,
            job.char_count
        );

        Ok(job_id)
    }

    /// Submit multiple jobs as a batch
    pub async fn submit_batch(
        &self,
        jobs: Vec<SynthesisJob>,
        app_handle: Option<&AppHandle>,
    ) -> QueueResult<Vec<String>> {
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

            let job = state
                .jobs
                .get_mut(job_id)
                .ok_or_else(|| QueueError::JobNotFound(job_id.to_string()))?;

            let was_pending = matches!(job.status, JobStatus::Pending);
            let was_processing = matches!(job.status, JobStatus::Processing);

            if !was_pending && !was_processing {
                return Err(QueueError::InvalidState(format!(
                    "Cannot cancel job in {:?} state",
                    job.status
                )));
            }

            job.mark_canceled();
            let progress = job.progress.clone();

            if was_pending {
                // Remove from pending queue
                let mut remaining: Vec<_> = state.pending.drain().collect();
                remaining.retain(|p| p.job.id != job_id);
                for pj in remaining {
                    state.pending.push(pj);
                }
                state.stats.canceled_count += 1;
                state.stats.pending_count = state.pending.len();
            } else if was_processing {
                state.processing.remove(job_id);
                state.stats.canceled_count += 1;
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
            let _ = handle.emit(
                channels::JOB_CANCELED,
                JobStatusEvent {
                    job_id: job_id.to_string(),
                    status: JobStatus::Canceled,
                    progress,
                    result_path: None,
                    error: None,
                },
            );
        }

        log::info!("Synthesis job {} canceled", job_id);

        Ok(())
    }

    /// Cancel all pending and processing jobs
    pub async fn cancel_all(&self, app_handle: Option<&AppHandle>) -> QueueResult<usize> {
        let mut state = self.state.write().await;
        let mut canceled = 0;

        // Cancel pending jobs
        while let Some(pj) = state.pending.pop() {
            if let Some(job) = state.jobs.get_mut(&pj.job.id) {
                job.mark_canceled();
                canceled += 1;
            }
        }

        // Cancel processing jobs
        let processing_ids: Vec<String> = state.processing.keys().cloned().collect();
        for job_id in &processing_ids {
            if let Some(token) = self.cancellation_tokens.write().await.remove(job_id) {
                let _ = token.send(());
            }
            if let Some(job) = state.jobs.get_mut(job_id) {
                job.mark_canceled();
                canceled += 1;
            }
        }
        state.processing.clear();

        state.stats.canceled_count += canceled as u64;
        state.stats.pending_count = 0;
        state.stats.processing_count = 0;

        // Emit stats event
        if let Some(handle) = app_handle {
            let _ = handle.emit(
                channels::QUEUE_STATS,
                QueueStatsEvent {
                    stats: state.stats.clone(),
                },
            );
        }

        log::info!("Canceled {} synthesis jobs", canceled);

        Ok(canceled)
    }

    /// Pause queue processing
    pub async fn pause(&self, app_handle: Option<&AppHandle>) {
        let mut state = self.state.write().await;
        state.is_paused = true;

        if let Some(handle) = app_handle {
            let _ = handle.emit(channels::QUEUE_PAUSED, ());
        }

        log::info!("Synthesis queue paused");
    }

    /// Resume queue processing
    pub async fn resume(&self, app_handle: Option<&AppHandle>) {
        let mut state = self.state.write().await;
        state.is_paused = false;

        if let Some(handle) = app_handle {
            let _ = handle.emit(channels::QUEUE_RESUMED, ());
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
    pub async fn mark_started(
        &self,
        job_id: &str,
        app_handle: Option<&AppHandle>,
    ) -> QueueResult<oneshot::Receiver<()>> {
        let mut state = self.state.write().await;

        let job = state
            .jobs
            .get_mut(job_id)
            .ok_or_else(|| QueueError::JobNotFound(job_id.to_string()))?;

        job.mark_processing();
        let job_clone = job.clone();

        state
            .processing
            .insert(job_id.to_string(), job_clone.clone());
        state.stats.processing_count = state.processing.len();
        state.stats.pending_count = state.pending.len();

        // Create cancellation token
        let (cancel_tx, cancel_rx) = oneshot::channel();
        self.cancellation_tokens
            .write()
            .await
            .insert(job_id.to_string(), cancel_tx);

        // Emit event
        if let Some(handle) = app_handle {
            let _ = handle.emit(
                channels::JOB_STARTED,
                JobStatusEvent {
                    job_id: job_id.to_string(),
                    status: JobStatus::Processing,
                    progress: job_clone.progress,
                    result_path: None,
                    error: None,
                },
            );
        }

        log::debug!("Synthesis job {} started", job_id);

        Ok(cancel_rx)
    }

    /// Update job progress (called by worker)
    pub async fn update_progress(
        &self,
        job_id: &str,
        progress: f32,
        stage: &str,
        app_handle: Option<&AppHandle>,
    ) -> QueueResult<()> {
        let mut state = self.state.write().await;

        let job = state
            .jobs
            .get_mut(job_id)
            .ok_or_else(|| QueueError::JobNotFound(job_id.to_string()))?;

        job.update_progress(progress, stage);

        // Emit event
        if let Some(handle) = app_handle {
            let _ = handle.emit(
                channels::JOB_PROGRESS,
                JobStatusEvent {
                    job_id: job_id.to_string(),
                    status: job.status.clone(),
                    progress: job.progress.clone(),
                    result_path: None,
                    error: None,
                },
            );
        }

        Ok(())
    }

    /// Mark a job as completed (called by worker)
    pub async fn mark_completed(
        &self,
        job_id: &str,
        result_path: &str,
        app_handle: Option<&AppHandle>,
    ) -> QueueResult<()> {
        let mut state = self.state.write().await;

        let job = state
            .jobs
            .get_mut(job_id)
            .ok_or_else(|| QueueError::JobNotFound(job_id.to_string()))?;

        let duration_ms = job
            .started_at
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
            let _ = handle.emit(
                channels::JOB_COMPLETED,
                JobStatusEvent {
                    job_id: job_id.to_string(),
                    status: JobStatus::Completed,
                    progress: job_clone.progress,
                    result_path: Some(result_path.to_string()),
                    error: None,
                },
            );
        }

        log::info!("Synthesis job {} completed in {}ms", job_id, duration_ms);

        Ok(())
    }

    /// Mark a job as failed (called by worker)
    pub async fn mark_failed(
        &self,
        job_id: &str,
        error: &str,
        app_handle: Option<&AppHandle>,
    ) -> QueueResult<()> {
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
            log::info!(
                "Synthesis job {} failed, retrying (attempt {})",
                job_id,
                retry_count
            );
        } else if let Some(job_clone) = job_for_event {
            // Emit event
            if let Some(handle) = app_handle {
                let _ = handle.emit(
                    channels::JOB_FAILED,
                    JobStatusEvent {
                        job_id: job_id.to_string(),
                        status: JobStatus::Failed(error.to_string()),
                        progress: job_clone.progress,
                        result_path: None,
                        error: Some(error.to_string()),
                    },
                );
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

    /// List job history (completed/failed/canceled)
    pub async fn list_history(&self, limit: Option<usize>) -> Vec<SynthesisJob> {
        let state = self.state.read().await;
        let limit = limit.unwrap_or(state.history.len());
        state.history.iter().rev().take(limit).cloned().collect()
    }

    /// List jobs by tag
    pub async fn list_by_tag(&self, tag: &str) -> Vec<SynthesisJob> {
        let state = self.state.read().await;
        state
            .jobs
            .values()
            .filter(|j| j.tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// List jobs by session
    pub async fn list_by_session(&self, session_id: &str) -> Vec<SynthesisJob> {
        let state = self.state.read().await;
        state
            .jobs
            .values()
            .filter(|j| j.session_id.as_deref() == Some(session_id))
            .cloned()
            .collect()
    }

    /// List jobs by NPC
    pub async fn list_by_npc(&self, npc_id: &str) -> Vec<SynthesisJob> {
        let state = self.state.read().await;
        state
            .jobs
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

        log::info!(
            "Pre-generation batch queued for session {} ({} jobs)",
            session_id,
            job_ids.len()
        );

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
