//! Voice Synthesis Queue Commands
//!
//! Commands for managing the priority-based voice synthesis queue with batch
//! processing, session pre-generation, and real-time progress tracking.

use std::sync::Arc;
use serde::{Serialize, Deserialize};
use tauri::State;

use crate::core::voice::{
    VoiceProviderType, SynthesisQueue, QueueConfig,
    SynthesisJob, JobPriority, JobStatus, JobProgress,
    QueueStats as VoiceQueueStats,
};

// ============================================================================
// State and Types
// ============================================================================

/// State wrapper for the synthesis queue
pub struct SynthesisQueueState {
    pub queue: Arc<SynthesisQueue>,
}

impl Default for SynthesisQueueState {
    fn default() -> Self {
        Self {
            queue: Arc::new(SynthesisQueue::with_defaults()),
        }
    }
}

impl SynthesisQueueState {
    /// Create with custom configuration
    pub fn with_config(config: QueueConfig) -> Self {
        Self {
            queue: Arc::new(SynthesisQueue::new(config)),
        }
    }
}

/// Request type for batch job submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisJobRequest {
    pub text: String,
    pub profile_id: String,
    pub voice_id: String,
    pub provider: String,
    pub priority: Option<String>,
    pub session_id: Option<String>,
    pub npc_id: Option<String>,
    pub campaign_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper to parse provider string to VoiceProviderType for queue commands
fn parse_queue_provider(provider: &str) -> Result<VoiceProviderType, String> {
    match provider {
        "elevenlabs" => Ok(VoiceProviderType::ElevenLabs),
        "openai" => Ok(VoiceProviderType::OpenAI),
        "fish_audio" => Ok(VoiceProviderType::FishAudio),
        "piper" => Ok(VoiceProviderType::Piper),
        "ollama" => Ok(VoiceProviderType::Ollama),
        "chatterbox" => Ok(VoiceProviderType::Chatterbox),
        "gpt_sovits" => Ok(VoiceProviderType::GptSoVits),
        "xtts_v2" => Ok(VoiceProviderType::XttsV2),
        "fish_speech" => Ok(VoiceProviderType::FishSpeech),
        "dia" => Ok(VoiceProviderType::Dia),
        _ => Err(format!("Unknown provider: {}", provider)),
    }
}

/// Helper to parse priority string to JobPriority
fn parse_queue_priority(priority: Option<&str>) -> Result<JobPriority, String> {
    match priority {
        Some("immediate") => Ok(JobPriority::Immediate),
        Some("high") => Ok(JobPriority::High),
        Some("normal") | None => Ok(JobPriority::Normal),
        Some("low") => Ok(JobPriority::Low),
        Some("batch") => Ok(JobPriority::Batch),
        Some(p) => Err(format!("Unknown priority: {}", p)),
    }
}

// ============================================================================
// Synthesis Queue Commands
// ============================================================================

/// Submit a voice synthesis job to the queue
#[tauri::command]
pub async fn submit_synthesis_job(
    app_handle: tauri::AppHandle,
    text: String,
    profile_id: String,
    voice_id: String,
    provider: String,
    priority: Option<String>,
    session_id: Option<String>,
    npc_id: Option<String>,
    campaign_id: Option<String>,
    tags: Option<Vec<String>>,
    state: State<'_, SynthesisQueueState>,
) -> Result<SynthesisJob, String> {
    let provider_type = parse_queue_provider(&provider)?;
    let job_priority = parse_queue_priority(priority.as_deref())?;

    let mut job = SynthesisJob::new(&text, &profile_id, provider_type, &voice_id)
        .with_priority(job_priority);

    if let Some(sid) = session_id {
        job = job.for_session(&sid);
    }
    if let Some(nid) = npc_id {
        job = job.for_npc(&nid);
    }
    if let Some(cid) = campaign_id {
        job = job.for_campaign(&cid);
    }
    if let Some(t) = tags {
        job = job.with_tags(t);
    }

    let job_id = state.queue.submit(job, Some(&app_handle))
        .await
        .map_err(|e| e.to_string())?;

    let submitted_job = state.queue.get_job(&job_id).await
        .ok_or_else(|| "Job not found after submission".to_string())?;

    Ok(submitted_job)
}

/// Get a synthesis job by ID
#[tauri::command]
pub async fn get_synthesis_job(
    job_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Option<SynthesisJob>, String> {
    Ok(state.queue.get_job(&job_id).await)
}

/// Get status of a synthesis job
#[tauri::command]
pub async fn get_synthesis_job_status(
    job_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Option<JobStatus>, String> {
    Ok(state.queue.get_status(&job_id).await)
}

/// Get progress of a synthesis job
#[tauri::command]
pub async fn get_synthesis_job_progress(
    job_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Option<JobProgress>, String> {
    Ok(state.queue.get_progress(&job_id).await)
}

/// Cancel a synthesis job
#[tauri::command]
pub async fn cancel_synthesis_job(
    app_handle: tauri::AppHandle,
    job_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<(), String> {
    state.queue.cancel(&job_id, Some(&app_handle))
        .await
        .map_err(|e| e.to_string())
}

/// Cancel all synthesis jobs
#[tauri::command]
pub async fn cancel_all_synthesis_jobs(
    app_handle: tauri::AppHandle,
    state: State<'_, SynthesisQueueState>,
) -> Result<usize, String> {
    state.queue.cancel_all(Some(&app_handle))
        .await
        .map_err(|e| e.to_string())
}

/// Pre-generate voice audio for a session (batch queue)
#[tauri::command]
pub async fn pregen_session_voices(
    app_handle: tauri::AppHandle,
    session_id: String,
    texts: Vec<(String, String, String)>,
    provider: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<String>, String> {
    let provider_type = parse_queue_provider(&provider)?;

    state.queue.pregen_session(&session_id, texts, provider_type, Some(&app_handle))
        .await
        .map_err(|e| e.to_string())
}

/// Submit a batch of synthesis jobs
#[tauri::command]
pub async fn submit_synthesis_batch(
    app_handle: tauri::AppHandle,
    jobs: Vec<SynthesisJobRequest>,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<String>, String> {
    let mut synthesis_jobs = Vec::with_capacity(jobs.len());

    for req in jobs {
        let provider_type = parse_queue_provider(&req.provider)?;
        let priority = parse_queue_priority(req.priority.as_deref())?;

        let mut job = SynthesisJob::new(&req.text, &req.profile_id, provider_type, &req.voice_id)
            .with_priority(priority);

        if let Some(sid) = req.session_id {
            job = job.for_session(&sid);
        }
        if let Some(nid) = req.npc_id {
            job = job.for_npc(&nid);
        }
        if let Some(cid) = req.campaign_id {
            job = job.for_campaign(&cid);
        }
        if let Some(t) = req.tags {
            job = job.with_tags(t);
        }

        synthesis_jobs.push(job);
    }

    state.queue.submit_batch(synthesis_jobs, Some(&app_handle))
        .await
        .map_err(|e| e.to_string())
}

/// Get synthesis queue statistics
#[tauri::command]
pub async fn get_synthesis_queue_stats(
    state: State<'_, SynthesisQueueState>,
) -> Result<VoiceQueueStats, String> {
    Ok(state.queue.stats().await)
}

/// Pause the synthesis queue
#[tauri::command]
pub async fn pause_synthesis_queue(
    app_handle: tauri::AppHandle,
    state: State<'_, SynthesisQueueState>,
) -> Result<(), String> {
    state.queue.pause(Some(&app_handle)).await;
    Ok(())
}

/// Resume the synthesis queue
#[tauri::command]
pub async fn resume_synthesis_queue(
    app_handle: tauri::AppHandle,
    state: State<'_, SynthesisQueueState>,
) -> Result<(), String> {
    state.queue.resume(Some(&app_handle)).await;
    Ok(())
}

/// Check if synthesis queue is paused
#[tauri::command]
pub async fn is_synthesis_queue_paused(
    state: State<'_, SynthesisQueueState>,
) -> Result<bool, String> {
    Ok(state.queue.is_paused().await)
}

/// List pending synthesis jobs
#[tauri::command]
pub async fn list_pending_synthesis_jobs(
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_pending().await)
}

/// List processing synthesis jobs
#[tauri::command]
pub async fn list_processing_synthesis_jobs(
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_processing().await)
}

/// List synthesis job history (completed/failed/cancelled)
#[tauri::command]
pub async fn list_synthesis_job_history(
    limit: Option<usize>,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_history(limit).await)
}

/// List synthesis jobs by session
#[tauri::command]
pub async fn list_synthesis_jobs_by_session(
    session_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_by_session(&session_id).await)
}

/// List synthesis jobs by NPC
#[tauri::command]
pub async fn list_synthesis_jobs_by_npc(
    npc_id: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_by_npc(&npc_id).await)
}

/// List synthesis jobs by tag
#[tauri::command]
pub async fn list_synthesis_jobs_by_tag(
    tag: String,
    state: State<'_, SynthesisQueueState>,
) -> Result<Vec<SynthesisJob>, String> {
    Ok(state.queue.list_by_tag(&tag).await)
}

/// Clear synthesis job history
#[tauri::command]
pub async fn clear_synthesis_job_history(
    state: State<'_, SynthesisQueueState>,
) -> Result<(), String> {
    state.queue.clear_history().await;
    Ok(())
}

/// Get total active jobs (pending + processing)
#[tauri::command]
pub async fn get_synthesis_queue_length(
    state: State<'_, SynthesisQueueState>,
) -> Result<usize, String> {
    Ok(state.queue.total_active().await)
}
