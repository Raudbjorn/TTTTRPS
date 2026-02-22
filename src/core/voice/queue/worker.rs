//! Background worker for processing the voice synthesis queue.

use std::sync::Arc;

use super::events::{channels, emit_event, QueueEventEmitter, QueueStatsEvent};
use super::SynthesisQueue;
use crate::core::voice::types::{OutputFormat, VoiceProviderType};

// ============================================================================
// Voice Synthesizer Trait
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

// ============================================================================
// Background Worker
// ============================================================================

/// Background worker that processes the synthesis queue
pub struct QueueWorker {
    queue: Arc<SynthesisQueue>,
    synthesizer: Arc<dyn VoiceSynthesizer>,
    emitter: Arc<dyn QueueEventEmitter>,
}

impl QueueWorker {
    /// Create a new queue worker
    pub fn new(
        queue: Arc<SynthesisQueue>,
        synthesizer: Arc<dyn VoiceSynthesizer>,
        emitter: Arc<dyn QueueEventEmitter>,
    ) -> Self {
        Self {
            queue,
            synthesizer,
            emitter,
        }
    }

    /// Run the worker (blocks until shutdown)
    pub async fn run(&self) {
        let mut shutdown_rx = self.queue.shutdown_receiver();
        let poll_interval = tokio::time::Duration::from_millis(self.queue.config().poll_interval_ms);

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
        let emitter: &dyn QueueEventEmitter = self.emitter.as_ref();

        // Mark as started and get cancellation token
        let cancel_rx = match self.queue.mark_started(&job_id, Some(emitter)).await {
            Ok(rx) => rx,
            Err(e) => {
                log::error!("Failed to mark job {} as started: {}", job_id, e);
                return;
            }
        };

        // Update progress: starting synthesis
        let _ = self
            .queue
            .update_progress(&job_id, 0.1, "Connecting to provider", Some(emitter))
            .await;

        // Perform synthesis with cancellation support
        let synthesizer = self.synthesizer.clone();
        let text = job.text.clone();
        let voice_id = job.voice_id.clone();
        let provider = job.provider.clone();
        let output_format = job.output_format.clone();

        let synthesis_future = async move {
            synthesizer
                .synthesize_to_file(&text, &voice_id, &provider, output_format)
                .await
        };

        tokio::select! {
            result = synthesis_future => {
                match result {
                    Ok(path) => {
                        let _ = self.queue.mark_completed(&job_id, &path, Some(emitter)).await;
                    }
                    Err(e) => {
                        let _ = self.queue.mark_failed(&job_id, &e, Some(emitter)).await;
                    }
                }
            }
            _ = cancel_rx => {
                log::info!("Synthesis job {} canceled during processing", job_id);
            }
        }

        // Emit updated stats
        let stats = self.queue.stats().await;
        emit_event(Some(emitter), channels::QUEUE_STATS, &QueueStatsEvent { stats });
    }
}
