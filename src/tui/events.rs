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
    // Navigation — legacy views
    FocusChat,
    FocusLibrary,
    FocusCampaign,
    FocusSettings,
    FocusGeneration,
    FocusPersonality,
    // Navigation — new views
    FocusCombat,
    FocusNotes,
    FocusNpcs,
    FocusLocations,
    FocusArchetypes,
    FocusVoice,
    FocusUsage,
    FocusAudit,
    // Navigation — cycling
    TabNext,
    TabPrev,
    // Sidebar
    ToggleSidebar,

    // Modals
    OpenCommandPalette,
    CloseCommandPalette,
    OpenDiceRoller,
    CloseDiceRoller,
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

    // New view refreshes
    RefreshNpcs,
    RefreshUsage,
    RefreshAudit,
    RefreshLocations,
    RefreshVoice,
    RefreshArchetypes,

    // Combat
    StartCombat,
    EndCombat,
    NextTurn,

    // Application
    Quit,
    SendMessage(String),
}

/// Which top-level view has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Focus {
    // Session group
    Chat,
    Combat,
    Notes,
    // World group
    Campaign,
    Npcs,
    Locations,
    Archetypes,
    // Tools group
    Generation,
    Voice,
    // System group
    Settings,
    Library,
    Usage,
    Audit,
    Personality,
}

/// Sidebar navigation groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SidebarGroup {
    Session,
    World,
    Tools,
    System,
}

impl SidebarGroup {
    pub const ALL: [SidebarGroup; 4] = [
        SidebarGroup::Session,
        SidebarGroup::World,
        SidebarGroup::Tools,
        SidebarGroup::System,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SidebarGroup::Session => "SESSION",
            SidebarGroup::World => "WORLD",
            SidebarGroup::Tools => "TOOLS",
            SidebarGroup::System => "SYSTEM",
        }
    }

    /// Views belonging to this group, in display order.
    pub fn views(self) -> &'static [Focus] {
        match self {
            SidebarGroup::Session => &[Focus::Chat, Focus::Combat, Focus::Notes],
            SidebarGroup::World => &[
                Focus::Campaign,
                Focus::Npcs,
                Focus::Locations,
                Focus::Archetypes,
            ],
            SidebarGroup::Tools => &[Focus::Generation, Focus::Voice],
            SidebarGroup::System => &[
                Focus::Settings,
                Focus::Library,
                Focus::Usage,
                Focus::Audit,
                Focus::Personality,
            ],
        }
    }
}

impl Focus {
    /// All focus variants in sidebar display order.
    pub const ALL: [Focus; 14] = [
        // Session
        Focus::Chat,
        Focus::Combat,
        Focus::Notes,
        // World
        Focus::Campaign,
        Focus::Npcs,
        Focus::Locations,
        Focus::Archetypes,
        // Tools
        Focus::Generation,
        Focus::Voice,
        // System
        Focus::Settings,
        Focus::Library,
        Focus::Usage,
        Focus::Audit,
        Focus::Personality,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Focus::Chat => "Chat",
            Focus::Combat => "Combat",
            Focus::Notes => "Notes",
            Focus::Campaign => "Campaign",
            Focus::Npcs => "NPCs",
            Focus::Locations => "Locations",
            Focus::Archetypes => "Archetypes",
            Focus::Generation => "Generation",
            Focus::Voice => "Voice",
            Focus::Settings => "Settings",
            Focus::Library => "Library",
            Focus::Usage => "Usage",
            Focus::Audit => "Audit",
            Focus::Personality => "Personality",
        }
    }

    /// Single-character icon for collapsed sidebar.
    pub fn icon(self) -> &'static str {
        match self {
            Focus::Chat => "💬",
            Focus::Combat => "⚔",
            Focus::Notes => "📝",
            Focus::Campaign => "🗺",
            Focus::Npcs => "👤",
            Focus::Locations => "🏰",
            Focus::Archetypes => "📖",
            Focus::Generation => "🎲",
            Focus::Voice => "🔊",
            Focus::Settings => "⚙",
            Focus::Library => "📚",
            Focus::Usage => "📊",
            Focus::Audit => "📋",
            Focus::Personality => "🎭",
        }
    }

    /// Which sidebar group this focus belongs to.
    pub fn group(self) -> SidebarGroup {
        match self {
            Focus::Chat | Focus::Combat | Focus::Notes => SidebarGroup::Session,
            Focus::Campaign | Focus::Npcs | Focus::Locations | Focus::Archetypes => {
                SidebarGroup::World
            }
            Focus::Generation | Focus::Voice => SidebarGroup::Tools,
            Focus::Settings
            | Focus::Library
            | Focus::Usage
            | Focus::Audit
            | Focus::Personality => SidebarGroup::System,
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

    /// Map Focus to its Action variant.
    pub fn to_action(self) -> Action {
        match self {
            Focus::Chat => Action::FocusChat,
            Focus::Combat => Action::FocusCombat,
            Focus::Notes => Action::FocusNotes,
            Focus::Campaign => Action::FocusCampaign,
            Focus::Npcs => Action::FocusNpcs,
            Focus::Locations => Action::FocusLocations,
            Focus::Archetypes => Action::FocusArchetypes,
            Focus::Generation => Action::FocusGeneration,
            Focus::Voice => Action::FocusVoice,
            Focus::Settings => Action::FocusSettings,
            Focus::Library => Action::FocusLibrary,
            Focus::Usage => Action::FocusUsage,
            Focus::Audit => Action::FocusAudit,
            Focus::Personality => Action::FocusPersonality,
        }
    }
}

/// Whether the sidebar or main content has input focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaFocus {
    Sidebar,
    Main,
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
