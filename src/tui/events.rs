use crate::database::{ChatMessageRecord, NpcConversation, NpcRecord};

/// Events flowing through the Elm-architecture event loop.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Periodic tick for animations, notification TTLs, etc.
    Tick,
    /// Raw terminal input (keyboard/mouse).
    Input(crossterm::event::Event),
    /// Streaming LLM token received.
    LlmToken(String),
    /// LLM response complete.
    LlmDone,
    /// LLM streaming error.
    LlmError(String),
    /// Chat session loaded from database.
    ChatSessionLoaded {
        session_id: String,
        messages: Vec<ChatMessageRecord>,
    },
    /// Voice audio playback state change.
    AudioPlayback(crate::tui::audio::AudioEvent),
    /// Voice audio playback finished (legacy, kept for compatibility).
    AudioFinished,
    /// A resolved action to execute.
    Action(Action),
    /// Notification to display to the user.
    Notification(Notification),
    /// OAuth PKCE flow result (Claude/Gemini).
    OAuthFlowResult {
        provider_id: String,
        result: Result<String, String>,
    },
    /// Device Code flow update (Copilot).
    DeviceFlowUpdate {
        provider_id: String,
        update: DeviceFlowUpdateKind,
    },
    /// Document ingestion progress update.
    IngestionProgress {
        library_item_id: String,
        phase: IngestionProgressKind,
    },
    /// NPC conversation loaded from database.
    NpcConversationLoaded {
        npc: NpcRecord,
        conversation: NpcConversation,
    },
    /// Request to quit the application.
    Quit,
}

/// Progress phases during document ingestion.
#[derive(Debug, Clone)]
pub enum IngestionProgressKind {
    /// Extracting text from the document.
    Extracting { progress: f32, status: String },
    /// Chunking the extracted text.
    Chunking { chunk_count: usize },
    /// Storing chunks in SurrealDB.
    Storing { stored: usize, total: usize },
    /// Ingestion completed successfully.
    Complete { chunk_count: usize },
    /// Ingestion failed.
    Error(String),
}

/// Updates from the background device-code polling loop.
#[derive(Debug, Clone)]
pub enum DeviceFlowUpdateKind {
    /// Device flow started — show user_code and verification_uri.
    Started {
        user_code: String,
        verification_uri: String,
    },
    /// Background poll tick (still waiting).
    Polling,
    /// User authorized — GitHub token received, exchanging for Copilot token.
    Completing,
    /// Flow completed successfully.
    Complete,
    /// Flow failed.
    Error(String),
}

/// High-level actions dispatched by the input mapper or command palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    // Navigation
    FocusChat,
    FocusLibrary,
    FocusCampaign,
    FocusSettings,
    FocusGeneration,
    FocusPersonality,
    TabNext,
    TabPrev,

    // Modals
    OpenCommandPalette,
    CloseCommandPalette,
    ShowHelp,
    CloseHelp,

    // Chat
    NewChatSession,
    ClearChat,

    // Settings
    RefreshSettings,
    AddProvider,
    EditProvider(String),
    DeleteProvider(String),

    // Library
    RefreshLibrary,
    IngestDocument,

    // Campaign
    RefreshCampaign,
    SwitchChatSession(String),

    // Application
    Quit,
    SendMessage(String),
}

/// Which top-level view has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Focus {
    Chat,
    Library,
    Campaign,
    Settings,
    Generation,
    Personality,
}

impl Focus {
    pub const ALL: [Focus; 6] = [
        Focus::Chat,
        Focus::Library,
        Focus::Campaign,
        Focus::Settings,
        Focus::Generation,
        Focus::Personality,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Focus::Chat => "Chat",
            Focus::Library => "Library",
            Focus::Campaign => "Campaign",
            Focus::Settings => "Settings",
            Focus::Generation => "Generation",
            Focus::Personality => "Personality",
        }
    }

    pub fn next(self) -> Focus {
        let idx = Focus::ALL.iter().position(|&f| f == self).unwrap_or(0);
        Focus::ALL[(idx + 1) % Focus::ALL.len()]
    }

    pub fn prev(self) -> Focus {
        let idx = Focus::ALL.iter().position(|&f| f == self).unwrap_or(0);
        Focus::ALL[(idx + Focus::ALL.len() - 1) % Focus::ALL.len()]
    }
}

/// Notification level for the overlay system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// A timed notification shown in the overlay.
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u64,
    pub message: String,
    pub level: NotificationLevel,
    /// Ticks remaining before auto-dismiss.
    pub ttl_ticks: u32,
}
