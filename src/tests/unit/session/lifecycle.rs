//! Session Lifecycle Tests
//!
//! Tests for session creation, pausing, resuming, and ending.
//!
//! TODO: Migrate tests from session_manager_tests.rs

use crate::core::session_manager::SessionStatus;
use crate::tests::common::fixtures::create_test_manager;

// =============================================================================
// Session Creation Tests
// =============================================================================

#[test]
fn test_start_session_creates_valid_session() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);

    assert_eq!(session.campaign_id, "campaign-001");
    assert_eq!(session.session_number, 1);
    assert_eq!(session.status, SessionStatus::Active);
    assert!(session.combat.is_none());
    assert!(session.ended_at.is_none());
    assert!(!session.id.is_empty());
}

#[test]
fn test_start_session_assigns_unique_ids() {
    let manager = create_test_manager();
    let session1 = manager.start_session("campaign-001", 1);
    let session2 = manager.start_session("campaign-001", 2);

    assert_ne!(session1.id, session2.id);
}

#[test]
fn test_get_session_returns_correct_session() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);

    let retrieved = manager.get_session(&session.id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, session.id);
}

#[test]
fn test_get_session_returns_none_for_invalid_id() {
    let manager = create_test_manager();
    let retrieved = manager.get_session("nonexistent-id");
    assert!(retrieved.is_none());
}

// =============================================================================
// Session Pause/Resume Tests
// =============================================================================

#[test]
fn test_pause_session() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);

    let result = manager.pause_session(&session.id);
    assert!(result.is_ok());

    let paused = manager.get_session(&session.id).unwrap();
    assert_eq!(paused.status, SessionStatus::Paused);
}

#[test]
fn test_pause_nonexistent_session() {
    let manager = create_test_manager();
    let result = manager.pause_session("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_resume_session() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);

    manager.pause_session(&session.id).unwrap();
    let result = manager.resume_session(&session.id);

    assert!(result.is_ok());
    let resumed = manager.get_session(&session.id).unwrap();
    assert_eq!(resumed.status, SessionStatus::Active);
}

// =============================================================================
// Session End Tests
// =============================================================================

#[test]
fn test_end_session() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);

    let result = manager.end_session(&session.id);
    assert!(result.is_ok());

    let summary = result.unwrap();
    assert_eq!(summary.status, SessionStatus::Ended);
    assert!(summary.ended_at.is_some());
}

#[test]
fn test_end_session_duration_calculated() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);

    let summary = manager.end_session(&session.id).unwrap();
    assert!(summary.duration_minutes.is_some());
}
