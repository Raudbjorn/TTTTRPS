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
    /// Voice audio playback finished.
    AudioFinished,
    /// A resolved action to execute.
    Action(Action),
    /// Notification to display to the user.
    Notification(Notification),
    /// Request to quit the application.
    Quit,
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
