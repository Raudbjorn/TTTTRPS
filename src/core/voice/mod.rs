pub mod types;
pub mod manager;
pub mod providers;
pub mod detection;
pub mod profiles;
pub mod presets;
pub mod cache;
pub mod queue;
pub mod download;
pub mod install;

pub use types::*;
pub use manager::VoiceManager;
pub use providers::VoiceProvider;
pub use detection::detect_providers;

// Re-export profile system (TASK-004)
pub use profiles::{
    VoiceProfile, VoiceProfileManager, ProfileMetadata,
    AgeRange, Gender, ProfileError, ProfileResult, ProfileStats,
};
pub use presets::{get_dm_presets, get_presets_by_tag, get_preset_by_id};

// Re-export cache system (TASK-005)
pub use cache::{
    AudioCache, CacheEntry, CacheConfig, CacheStats,
    CacheKeyParams, CacheError, CacheResult,
};

// Re-export queue system (TASK-025)
pub use queue::{
    SynthesisQueue, SynthesisJob, JobPriority, JobStatus, JobProgress,
    QueueConfig, QueueStats, QueueError, QueueResult,
    QueueWorker, VoiceSynthesizer, JobSubmittedEvent, JobStatusEvent, QueueStatsEvent,
    channels as queue_events,
};

// Re-export download system
pub use download::{
    VoiceDownloader, AvailablePiperVoice, PiperLanguage, PiperVoiceFiles, PiperFileInfo,
    DownloadError, DownloadResult, ProgressCallback, popular_piper_voices,
};

// Re-export install system
pub use install::{
    ProviderInstaller, InstallStatus, InstallMethod, InstallError, InstallResult,
    get_recommended_piper_voices,
};
