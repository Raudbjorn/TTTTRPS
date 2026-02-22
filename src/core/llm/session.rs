//! LLM Provider Session Management
//!
//! Provides session continuity for LLM providers that support it.
//! Sessions allow conversations to be resumed, compacted, and managed.
//!
//! ## Features
//!
//! - Session ID tracking and persistence
//! - Conversation resumption via session IDs
//! - Conversation compaction for long contexts
//! - Provider-agnostic session interface
//!
//! ## Supported Providers
//!
//! - **Claude Code CLI**: Full session support via `--resume`, `--continue`, `/compact`

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Session Types
// ============================================================================

/// Unique identifier for a conversation session
pub type SessionId = String;

/// Session state and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Unique session identifier
    pub id: SessionId,
    /// Provider that created this session
    pub provider_id: String,
    /// When the session was created
    pub created_at: u64,
    /// When the session was last used
    pub last_used_at: u64,
    /// Number of messages in the session
    pub message_count: u32,
    /// Whether the session has been compacted
    pub is_compacted: bool,
    /// Optional session title/summary
    pub title: Option<String>,
    /// Working directory (for CLI-based providers)
    pub working_dir: Option<String>,
}

impl SessionInfo {
    /// Create a new session info
    pub fn new(id: SessionId, provider_id: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id,
            provider_id: provider_id.to_string(),
            created_at: now,
            last_used_at: now,
            message_count: 0,
            is_compacted: false,
            title: None,
            working_dir: None,
        }
    }

    /// Update last used timestamp
    pub fn touch(&mut self) {
        self.last_used_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// Session operation result
pub type SessionResult<T> = Result<T, SessionError>;

/// Errors that can occur during session operations
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(SessionId),

    #[error("Session expired: {0}")]
    Expired(SessionId),

    #[error("Session operation failed: {0}")]
    OperationFailed(String),

    #[error("Session storage error: {0}")]
    StorageError(String),

    #[error("Provider does not support sessions")]
    NotSupported,

    #[error("Compaction failed: {0}")]
    CompactionFailed(String),
}

// ============================================================================
// ProviderSession Trait
// ============================================================================

/// Trait for LLM providers that support conversation sessions.
///
/// Providers implementing this trait can:
/// - Create and track sessions across messages
/// - Resume sessions by ID
/// - Compact long conversations
/// - Fork sessions for branching conversations
#[async_trait]
pub trait ProviderSession: Send + Sync {
    /// Check if this provider supports sessions
    fn supports_sessions(&self) -> bool {
        true
    }

    /// Get the current active session ID, if any
    async fn current_session(&self) -> Option<SessionId>;

    /// Resume an existing session by ID
    async fn resume_session(&self, session_id: &SessionId) -> SessionResult<SessionInfo>;

    /// Continue the most recent session
    async fn continue_session(&self) -> SessionResult<SessionInfo>;

    /// Fork the current session (create a new branch)
    async fn fork_session(&self, session_id: &SessionId) -> SessionResult<SessionId>;

    /// Compact the current session to reduce token usage
    async fn compact_session(&self) -> SessionResult<()>;

    /// Get session info by ID
    async fn get_session_info(&self, session_id: &SessionId) -> SessionResult<SessionInfo>;

    /// List recent sessions
    async fn list_sessions(&self, limit: usize) -> SessionResult<Vec<SessionInfo>>;

    /// Check if a session can be resumed (exists and not expired)
    async fn can_resume(&self, session_id: &SessionId) -> bool {
        self.get_session_info(session_id).await.is_ok()
    }
}

// ============================================================================
// Session Store
// ============================================================================

/// Persistent storage for session metadata
#[derive(Debug)]
pub struct SessionStore {
    /// Sessions keyed by session ID
    sessions: RwLock<HashMap<SessionId, SessionInfo>>,
    /// File path for persistence
    storage_path: Option<PathBuf>,
    /// Maximum number of sessions to retain
    max_sessions: usize,
}

impl SessionStore {
    /// Create a new in-memory session store
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            storage_path: None,
            max_sessions: 100,
        }
    }

    /// Create a session store with file persistence
    pub fn with_persistence(path: PathBuf) -> Self {
        let mut store = Self::new();
        store.storage_path = Some(path);
        store
    }

    /// Load sessions from storage
    pub async fn load(&self) -> SessionResult<()> {
        if let Some(ref path) = self.storage_path {
            if path.exists() {
                let content = tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| SessionError::StorageError(e.to_string()))?;

                let loaded: HashMap<SessionId, SessionInfo> = serde_json::from_str(&content)
                    .map_err(|e| SessionError::StorageError(e.to_string()))?;

                *self.sessions.write().await = loaded;
            }
        }
        Ok(())
    }

    /// Save sessions to storage
    pub async fn save(&self) -> SessionResult<()> {
        if let Some(ref path) = self.storage_path {
            let sessions = self.sessions.read().await;
            let content = serde_json::to_string_pretty(&*sessions)
                .map_err(|e| SessionError::StorageError(e.to_string()))?;

            // Ensure parent directory exists
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| SessionError::StorageError(e.to_string()))?;
            }

            tokio::fs::write(path, content)
                .await
                .map_err(|e| SessionError::StorageError(e.to_string()))?;
        }
        Ok(())
    }

    /// Store a session
    pub async fn store(&self, session: SessionInfo) -> SessionResult<()> {
        let mut sessions = self.sessions.write().await;

        // Prune old sessions if needed
        if sessions.len() >= self.max_sessions {
            // Find and remove oldest session
            if let Some(oldest_id) = sessions
                .iter()
                .min_by_key(|(_, s)| s.last_used_at)
                .map(|(id, _)| id.clone())
            {
                sessions.remove(&oldest_id);
            }
        }

        sessions.insert(session.id.clone(), session);
        drop(sessions);

        self.save().await
    }

    /// Get a session by ID
    pub async fn get(&self, session_id: &SessionId) -> Option<SessionInfo> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Remove a session
    pub async fn remove(&self, session_id: &SessionId) -> SessionResult<()> {
        self.sessions.write().await.remove(session_id);
        self.save().await
    }

    /// List sessions for a provider, sorted by last used
    pub async fn list_by_provider(&self, provider_id: &str, limit: usize) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        let mut filtered: Vec<_> = sessions
            .values()
            .filter(|s| s.provider_id == provider_id)
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        filtered.truncate(limit);
        filtered
    }

    /// Get the most recent session for a provider
    pub async fn most_recent(&self, provider_id: &str) -> Option<SessionInfo> {
        self.list_by_provider(provider_id, 1).await.into_iter().next()
    }

    /// Update session's last used time
    pub async fn touch(&self, session_id: &SessionId) -> SessionResult<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.touch();
            drop(sessions);
            self.save().await
        } else {
            Err(SessionError::NotFound(session_id.clone()))
        }
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Session-Aware Chat Request/Response Extensions
// ============================================================================

/// Extended chat request with session support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionChatRequest {
    /// The chat request
    #[serde(flatten)]
    pub request: super::router::ChatRequest,

    /// Resume a specific session by ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_session: Option<SessionId>,

    /// Continue the most recent session
    #[serde(default)]
    pub continue_session: bool,

    /// Fork the session (create new branch)
    #[serde(default)]
    pub fork_session: bool,
}

/// Extended chat response with session info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionChatResponse {
    /// The chat response
    #[serde(flatten)]
    pub response: super::router::ChatResponse,

    /// Session ID for this conversation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionId>,

    /// Whether compaction is recommended
    #[serde(default)]
    pub recommend_compaction: bool,
}

// ============================================================================
// Stream-JSON Types for Claude Code CLI
// ============================================================================

/// Event types from Claude Code CLI stream-json output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeStreamEvent {
    /// System information at stream start
    #[serde(rename = "system")]
    System {
        subtype: String,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        tools: Option<Vec<String>>,
        #[serde(default)]
        mcp_servers: Option<Vec<serde_json::Value>>,
    },

    /// Assistant message content
    #[serde(rename = "assistant")]
    Assistant {
        subtype: String,
        #[serde(default)]
        message: Option<AssistantMessage>,
    },

    /// User message acknowledgment
    #[serde(rename = "user")]
    User {
        subtype: String,
        #[serde(default)]
        message: Option<UserMessage>,
    },

    /// Result/completion event
    #[serde(rename = "result")]
    Result {
        subtype: String,
        #[serde(default)]
        result: Option<String>,
        #[serde(default)]
        session_id: Option<String>,
        #[serde(default)]
        is_error: Option<bool>,
        #[serde(default)]
        cost_usd: Option<f64>,
        #[serde(default)]
        duration_ms: Option<u64>,
        #[serde(default)]
        duration_api_ms: Option<u64>,
        #[serde(default)]
        num_turns: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub content: Option<Vec<ContentBlock>>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    #[serde(default)]
    pub content: Option<Vec<ContentBlock>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
    },
}

// ============================================================================
// Session Manager
// ============================================================================

/// Manages sessions across all providers
pub struct SessionManager {
    store: Arc<SessionStore>,
    /// Active session per provider
    active_sessions: RwLock<HashMap<String, SessionId>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self {
            store,
            active_sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Get the session store
    pub fn store(&self) -> Arc<SessionStore> {
        self.store.clone()
    }

    /// Set active session for a provider
    pub async fn set_active(&self, provider_id: &str, session_id: SessionId) {
        self.active_sessions
            .write()
            .await
            .insert(provider_id.to_string(), session_id);
    }

    /// Get active session for a provider
    pub async fn get_active(&self, provider_id: &str) -> Option<SessionId> {
        self.active_sessions.read().await.get(provider_id).cloned()
    }

    /// Clear active session for a provider
    pub async fn clear_active(&self, provider_id: &str) {
        self.active_sessions.write().await.remove(provider_id);
    }

    /// Create session store path for the app
    pub fn default_store_path() -> Option<PathBuf> {
        dirs::data_local_dir().map(|d| d.join("ttrpg-assistant").join("sessions.json"))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_info_creation() {
        let info = SessionInfo::new("test-123".to_string(), "claude-code");
        assert_eq!(info.id, "test-123");
        assert_eq!(info.provider_id, "claude-code");
        assert_eq!(info.message_count, 0);
        assert!(!info.is_compacted);
    }

    #[test]
    fn test_session_info_touch() {
        let mut info = SessionInfo::new("test-123".to_string(), "claude-code");
        let original_time = info.last_used_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        info.touch();

        assert!(info.last_used_at >= original_time);
    }

    #[tokio::test]
    async fn test_session_store_basic() {
        let store = SessionStore::new();

        let session = SessionInfo::new("sess-1".to_string(), "claude-code");
        store.store(session.clone()).await.unwrap();

        let retrieved = store.get(&"sess-1".to_string()).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "sess-1");
    }

    #[tokio::test]
    async fn test_session_store_list_by_provider() {
        let store = SessionStore::new();

        // Add sessions for different providers
        store
            .store(SessionInfo::new("sess-1".to_string(), "claude-code"))
            .await
            .unwrap();
        store
            .store(SessionInfo::new("sess-2".to_string(), "claude-code"))
            .await
            .unwrap();
        store
            .store(SessionInfo::new("sess-3".to_string(), "other"))
            .await
            .unwrap();

        let claude_sessions = store.list_by_provider("claude-code", 10).await;
        assert_eq!(claude_sessions.len(), 2);

        let other_sessions = store.list_by_provider("other", 10).await;
        assert_eq!(other_sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_session_store_most_recent() {
        let store = SessionStore::new();

        let mut older = SessionInfo::new("old".to_string(), "claude-code");
        older.last_used_at = 1000;
        store.store(older).await.unwrap();

        let mut newer = SessionInfo::new("new".to_string(), "claude-code");
        newer.last_used_at = 2000;
        store.store(newer).await.unwrap();

        let recent = store.most_recent("claude-code").await;
        assert!(recent.is_some());
        assert_eq!(recent.unwrap().id, "new");
    }

    #[test]
    fn test_claude_stream_event_parsing() {
        let json = r#"{"type":"system","subtype":"init","session_id":"abc123"}"#;
        let event: ClaudeStreamEvent = serde_json::from_str(json).unwrap();

        match event {
            ClaudeStreamEvent::System { subtype, session_id, .. } => {
                assert_eq!(subtype, "init");
                assert_eq!(session_id, Some("abc123".to_string()));
            }
            _ => panic!("Expected System event"),
        }
    }

    #[test]
    fn test_content_block_parsing() {
        let text_json = r#"{"type":"text","text":"Hello"}"#;
        let block: ContentBlock = serde_json::from_str(text_json).unwrap();

        match block {
            ContentBlock::Text { text } => assert_eq!(text, "Hello"),
            _ => panic!("Expected Text block"),
        }
    }
}
