//! Test Fixtures
//!
//! Provides shared test helpers for creating test databases, campaigns,
//! sessions, characters, NPCs, and combatants.

use tempfile::TempDir;

use crate::database::Database;
use crate::core::session_manager::{Combatant, CombatantType, SessionManager};

// =============================================================================
// Session Manager Fixtures
// =============================================================================

/// Create a test session manager (in-memory, no database required).
pub fn create_test_manager() -> SessionManager {
    SessionManager::new()
}

// =============================================================================
// Database Fixtures
// =============================================================================

/// Create a test database in a temporary directory.
/// Returns both the database and the TempDir (which must be kept alive).
pub async fn create_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db = Database::new(temp_dir.path())
        .await
        .expect("Failed to create test database");
    (db, temp_dir)
}

// =============================================================================
// Combatant Fixtures
// =============================================================================

/// Create a basic test combatant with optional HP.
pub fn create_test_combatant(name: &str, initiative: i32, hp: Option<i32>) -> Combatant {
    let mut combatant = Combatant::new(name, initiative, CombatantType::Player);
    combatant.initiative_modifier = initiative % 5;
    combatant.current_hp = hp;
    combatant.max_hp = hp;
    combatant.armor_class = Some(15);
    combatant
}

/// Create a combatant with full HP configuration.
pub fn create_combatant_with_hp(
    name: &str,
    initiative: i32,
    current_hp: i32,
    max_hp: i32,
    temp_hp: Option<i32>,
) -> Combatant {
    let mut combatant = Combatant::new(name, initiative, CombatantType::Player);
    combatant.current_hp = Some(current_hp);
    combatant.max_hp = Some(max_hp);
    combatant.temp_hp = temp_hp;
    combatant.armor_class = Some(15);
    combatant
}

/// Create a monster combatant.
pub fn create_monster(name: &str, initiative: i32, hp: i32) -> Combatant {
    let mut combatant = Combatant::new(name, initiative, CombatantType::Monster);
    combatant.initiative_modifier = 2;
    combatant.current_hp = Some(hp);
    combatant.max_hp = Some(hp);
    combatant.armor_class = Some(13);
    combatant
}

// =============================================================================
// Audio Playback Mock
// =============================================================================

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum PlaybackState {
    Idle,
    Loading,
    Playing,
    Paused,
    Stopped,
    Error(String),
}

/// Mock audio player for testing playback state transitions.
pub struct MockAudioPlayer {
    pub state: PlaybackState,
    pub current_audio_path: Option<PathBuf>,
    pub volume: f32,
    pub position_ms: u64,
    pub duration_ms: u64,
}

impl MockAudioPlayer {
    pub fn new() -> Self {
        Self {
            state: PlaybackState::Idle,
            current_audio_path: None,
            volume: 1.0,
            position_ms: 0,
            duration_ms: 0,
        }
    }

    pub fn load(&mut self, path: PathBuf) -> Result<(), String> {
        self.state = PlaybackState::Loading;
        self.current_audio_path = Some(path);
        self.duration_ms = 5000;
        self.position_ms = 0;
        self.state = PlaybackState::Stopped;
        Ok(())
    }

    pub fn play(&mut self) -> Result<(), String> {
        match self.state {
            PlaybackState::Stopped | PlaybackState::Paused => {
                self.state = PlaybackState::Playing;
                Ok(())
            }
            PlaybackState::Idle => Err("No audio loaded".to_string()),
            PlaybackState::Loading => Err("Still loading".to_string()),
            PlaybackState::Playing => Ok(()),
            PlaybackState::Error(_) => Err("Player in error state".to_string()),
        }
    }

    pub fn pause(&mut self) -> Result<(), String> {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
            Ok(())
        } else {
            Err("Not playing".to_string())
        }
    }

    pub fn stop(&mut self) -> Result<(), String> {
        match self.state {
            PlaybackState::Playing | PlaybackState::Paused => {
                self.state = PlaybackState::Stopped;
                self.position_ms = 0;
                Ok(())
            }
            _ => Err("Nothing to stop".to_string()),
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn seek(&mut self, position_ms: u64) -> Result<(), String> {
        if self.current_audio_path.is_some() {
            self.position_ms = position_ms.min(self.duration_ms);
            Ok(())
        } else {
            Err("No audio loaded".to_string())
        }
    }
}

impl Default for MockAudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Credential Manager Mock
// =============================================================================

use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq)]
pub enum CredentialError {
    NotFound(String),
    InvalidFormat,
    StorageError(String),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MockCredential {
    pub provider: String,
    pub api_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Mock credential manager for testing API key storage.
pub struct MockCredentialManager {
    credentials: HashMap<String, MockCredential>,
}

impl MockCredentialManager {
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
        }
    }

    pub fn store_credential(&mut self, provider: &str, api_key: &str) -> Result<(), CredentialError> {
        use crate::tests::common::validators::ApiKeyValidator;

        if ApiKeyValidator::validate(provider, api_key).is_err() {
            return Err(CredentialError::InvalidFormat);
        }

        let now = Utc::now();
        let credential = MockCredential {
            provider: provider.to_string(),
            api_key: api_key.to_string(),
            created_at: now,
            updated_at: now,
        };

        self.credentials.insert(provider.to_lowercase(), credential);
        Ok(())
    }

    pub fn get_credential(&self, provider: &str) -> Result<&MockCredential, CredentialError> {
        self.credentials
            .get(&provider.to_lowercase())
            .ok_or_else(|| CredentialError::NotFound(provider.to_string()))
    }

    pub fn delete_credential(&mut self, provider: &str) -> Result<(), CredentialError> {
        self.credentials
            .remove(&provider.to_lowercase())
            .map(|_| ())
            .ok_or_else(|| CredentialError::NotFound(provider.to_string()))
    }

    pub fn has_credential(&self, provider: &str) -> bool {
        self.credentials.contains_key(&provider.to_lowercase())
    }

    pub fn list_providers(&self) -> Vec<String> {
        self.credentials.keys().cloned().collect()
    }

    pub fn mask_api_key(&self, key: &str) -> String {
        if key.len() <= 8 {
            return "********".to_string();
        }
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }
}

impl Default for MockCredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Audit Logger Mock
// =============================================================================

use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub enum AuditEventType {
    ApiKeyAdded { provider: String },
    ApiKeyRemoved { provider: String },
    ValidationFailed { input_type: String, reason: String },
    DocumentIngested { path: String, doc_type: String },
    SessionStarted { session_id: String, campaign_id: String },
    SessionEnded { session_id: String },
    LlmRequest { provider: String, model: String, tokens: u32 },
    SecurityAlert { severity: String, message: String },
    Custom { category: String, action: String, details: String },
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum AuditSeverity {
    Info,
    Warning,
    Security,
    Critical,
}

#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub id: String,
    pub event_type: AuditEventType,
    pub severity: AuditSeverity,
    pub timestamp: DateTime<Utc>,
    pub context: Option<String>,
    pub source: Option<String>,
}

/// Mock audit logger for testing security event logging.
pub struct AuditLogger {
    pub events: VecDeque<AuditEvent>,
    max_events: usize,
}

impl AuditLogger {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: VecDeque::new(),
            max_events,
        }
    }

    pub fn log(&mut self, event_type: AuditEventType, severity: AuditSeverity) -> String {
        self.log_with_context(event_type, severity, None, None)
    }

    pub fn log_with_context(
        &mut self,
        event_type: AuditEventType,
        severity: AuditSeverity,
        context: Option<String>,
        source: Option<String>,
    ) -> String {
        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            severity,
            timestamp: Utc::now(),
            context,
            source,
        };

        let event_id = event.id.clone();
        self.events.push_back(event);

        while self.events.len() > self.max_events {
            self.events.pop_front();
        }

        event_id
    }

    pub fn get_recent(&self, count: usize) -> Vec<&AuditEvent> {
        self.events.iter().rev().take(count).collect()
    }

    pub fn get_by_severity(&self, min_severity: AuditSeverity) -> Vec<&AuditEvent> {
        self.events
            .iter()
            .filter(|e| e.severity >= min_severity)
            .collect()
    }

    pub fn count(&self) -> usize {
        self.events.len()
    }

    pub fn clear_older_than(&mut self, cutoff: DateTime<Utc>) {
        self.events.retain(|e| e.timestamp > cutoff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_audio_player_creation() {
        let player = MockAudioPlayer::new();
        assert_eq!(player.state, PlaybackState::Idle);
        assert_eq!(player.volume, 1.0);
    }

    #[test]
    fn test_mock_credential_manager_creation() {
        let manager = MockCredentialManager::new();
        assert!(!manager.has_credential("openai"));
    }

    #[test]
    fn test_audit_logger_creation() {
        let logger = AuditLogger::new(100);
        assert_eq!(logger.count(), 0);
    }
}
