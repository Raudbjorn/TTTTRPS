use std::sync::Arc;

use tokio::sync::mpsc;

use crate::config::AppConfig;
use crate::core::llm::router::LLMRouter;
use crate::core::personality::application::PersonalityApplicationManager;
use crate::core::personality_base::PersonalityStore;
use crate::core::session_manager::SessionManager;
use crate::core::storage::surrealdb::SurrealStorage;
use crate::core::voice::queue::events::QueueEventEmitter;
use crate::core::voice::queue::SynthesisQueue;
use crate::database::Database;

use super::events::AppEvent;

/// Centralized handle to all backend services.
///
/// Created once at startup, then passed (by ref or clone) to views
/// that need backend access. Clone-able fields are cloned directly;
/// non-Clone fields are wrapped in Arc.
pub struct Services {
    pub llm: LLMRouter,
    pub storage: SurrealStorage,
    pub database: Database,
    pub session: Arc<SessionManager>,
    pub personality: Arc<PersonalityApplicationManager>,
    pub voice: Arc<SynthesisQueue>,
    pub event_tx: mpsc::UnboundedSender<AppEvent>,
}

impl Services {
    /// Initialize all services from config.
    ///
    /// Failures here are fatal — the TUI cannot run without core services.
    pub async fn init(
        config: &AppConfig,
        event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = config.data_dir();
        log::info!("Initializing services with data dir: {}", data_dir.display());

        // SurrealDB (embedded RocksDB)
        let surreal_path = data_dir.join("surrealdb");
        let storage = SurrealStorage::new(surreal_path).await?;
        log::info!("SurrealDB storage initialized");

        // Legacy SQLite (still needed during migration)
        let database = Database::new(&data_dir).await?;
        log::info!("SQLite database initialized");

        // LLM router with default config
        let llm = LLMRouter::with_defaults();
        log::info!("LLM router initialized");

        // Session manager (in-memory)
        let session = Arc::new(SessionManager::new());

        // Personality system
        let personality_store = Arc::new(PersonalityStore::new());
        let personality = Arc::new(PersonalityApplicationManager::new(personality_store));

        // Voice synthesis queue
        let voice = Arc::new(SynthesisQueue::with_defaults());

        Ok(Self {
            llm,
            storage,
            database,
            session,
            personality,
            voice,
            event_tx,
        })
    }
}

/// Voice queue event emitter that forwards events into the TUI event channel.
pub struct TuiQueueEmitter {
    tx: mpsc::UnboundedSender<AppEvent>,
}

impl TuiQueueEmitter {
    pub fn new(tx: mpsc::UnboundedSender<AppEvent>) -> Self {
        Self { tx }
    }
}

impl QueueEventEmitter for TuiQueueEmitter {
    fn emit_json(&self, channel: &str, payload: serde_json::Value) {
        log::debug!("Voice queue event on {channel}: {payload}");
        // Forward as notification for now — specific voice events can be
        // added to AppEvent as the voice UI is built out.
        let msg = format!("[voice] {channel}");
        let _ = self.tx.send(AppEvent::Notification(
            super::events::Notification {
                id: 0, // Will be assigned by AppState
                message: msg,
                level: super::events::NotificationLevel::Info,
                ttl_ticks: 60,
            },
        ));
    }
}
