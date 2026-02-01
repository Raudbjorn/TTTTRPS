//! Session Notes Tests
//!
//! Tests for session notes, timeline events, and snapshots.
//!
//! TODO: Migrate remaining tests from session_manager_tests.rs

use crate::core::session_manager::LogEntryType;
use crate::tests::common::fixtures::create_test_manager;

// =============================================================================
// Session Log Tests
// =============================================================================

#[test]
fn test_session_log_entry() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);

    let entry = manager.add_log_entry(
        &session.id,
        LogEntryType::Narrative,
        "The party entered the dungeon".to_string(),
        None,
    );

    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert!(entry.content.contains("dungeon"));
}

#[test]
fn test_set_active_scene() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);

    let result = manager.set_active_scene(&session.id, Some("Tavern".to_string()));
    assert!(result.is_ok());

    let updated = manager.get_session(&session.id).unwrap();
    assert_eq!(updated.active_scene, Some("Tavern".to_string()));
}

// =============================================================================
// Session Ordering Tests
// =============================================================================

#[test]
fn test_reorder_session() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);

    let result = manager.reorder_session(&session.id, 5);
    assert!(result.is_ok());

    let updated = manager.get_session(&session.id).unwrap();
    assert_eq!(updated.order_index, 5);
}

// =============================================================================
// Session List Tests
// =============================================================================

#[test]
fn test_list_sessions() {
    let manager = create_test_manager();

    manager.start_session("campaign-001", 1);
    manager.start_session("campaign-001", 2);
    manager.start_session("campaign-002", 1);

    let sessions = manager.list_sessions("campaign-001");
    assert_eq!(sessions.len(), 2);
}

#[test]
fn test_list_sessions_empty_campaign() {
    let manager = create_test_manager();
    let sessions = manager.list_sessions("nonexistent");
    assert!(sessions.is_empty());
}

#[test]
fn test_get_active_session() {
    let manager = create_test_manager();
    let session = manager.start_session("campaign-001", 1);

    let active = manager.get_active_session("campaign-001");
    assert!(active.is_some());
    assert_eq!(active.unwrap().id, session.id);
}

#[test]
fn test_get_active_session_returns_none_for_nonexistent_campaign() {
    let manager = create_test_manager();
    let active = manager.get_active_session("nonexistent-campaign");
    assert!(active.is_none());
}
