//! Session Database Tests
//!
//! Tests for session CRUD operations, notes, events, and combat states.

use crate::database::{
    CampaignOps, CampaignRecord, CombatOps, CombatStateRecord, SessionEventRecord, SessionNoteRecord,
    SessionOps, SessionRecord,
};
use crate::tests::common::create_test_db;

// =============================================================================
// Basic CRUD Tests
// =============================================================================

#[tokio::test]
async fn test_create_session() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-sess".to_string(),
        "Session Test Campaign".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session = SessionRecord::new("sess-001".to_string(), "camp-sess".to_string(), 1);
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    let retrieved = db
        .get_session("sess-001")
        .await
        .expect("Failed to get session")
        .expect("Session not found");

    assert_eq!(retrieved.id, "sess-001");
    assert_eq!(retrieved.campaign_id, "camp-sess");
    assert_eq!(retrieved.session_number, 1);
    assert_eq!(retrieved.status, "active");
}

#[tokio::test]
async fn test_list_sessions() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-multi-sess".to_string(),
        "Multi Session Campaign".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    for i in 1..=3 {
        let session =
            SessionRecord::new(format!("sess-{:03}", i), "camp-multi-sess".to_string(), i);
        db.create_session(&session)
            .await
            .expect("Failed to create session");
    }

    let sessions = db
        .list_sessions("camp-multi-sess")
        .await
        .expect("Failed to list sessions");
    assert_eq!(sessions.len(), 3);
}

#[tokio::test]
async fn test_get_active_session() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-active".to_string(),
        "Active Session Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session = SessionRecord::new("sess-active".to_string(), "camp-active".to_string(), 1);
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    let active = db
        .get_active_session("camp-active")
        .await
        .expect("Failed to get active session");

    assert!(active.is_some(), "Active session should be found");
    assert_eq!(active.expect("Active session should be present").id, "sess-active");
}

// =============================================================================
// Session Lifecycle Tests
// =============================================================================

#[tokio::test]
async fn test_session_full_lifecycle() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-sess-life".to_string(),
        "Session Lifecycle".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session = SessionRecord::new("sess-lifecycle".to_string(), "camp-sess-life".to_string(), 1);
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    // Verify initial state
    let retrieved = db
        .get_session("sess-lifecycle")
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert_eq!(retrieved.status, "active");
    assert!(retrieved.ended_at.is_none());

    // Complete the session
    let mut updated = retrieved.clone();
    updated.status = "completed".to_string();
    updated.ended_at = Some(chrono::Utc::now().to_rfc3339());
    updated.notes = Some("Great session! Defeated the dragon.".to_string());
    db.update_session(&updated)
        .await
        .expect("Failed to update");

    let after_update = db
        .get_session("sess-lifecycle")
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert_eq!(after_update.status, "completed");
    assert!(after_update.ended_at.is_some());
    assert!(after_update.notes.as_ref().map_or(false, |n| n.contains("dragon")), "Notes should contain 'dragon'");
}

#[tokio::test]
async fn test_session_multiple_per_campaign() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-multi".to_string(),
        "Multi Session".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    for i in 1..=10 {
        let session =
            SessionRecord::new(format!("sess-multi-{:02}", i), "camp-multi".to_string(), i);
        db.create_session(&session)
            .await
            .expect("Failed to create session");
    }

    let sessions = db
        .list_sessions("camp-multi")
        .await
        .expect("Failed to list");
    assert_eq!(sessions.len(), 10);

    // Verify ordering (descending by session_number)
    assert_eq!(sessions[0].session_number, 10);
    assert_eq!(sessions[9].session_number, 1);
}

#[tokio::test]
async fn test_session_no_active_when_all_completed() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-no-active".to_string(),
        "No Active".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let mut session =
        SessionRecord::new("sess-completed".to_string(), "camp-no-active".to_string(), 1);
    session.status = "completed".to_string();
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    let active = db
        .get_active_session("camp-no-active")
        .await
        .expect("Failed to get");
    assert!(active.is_none());
}

// =============================================================================
// Session Notes Tests
// =============================================================================

#[tokio::test]
async fn test_session_notes() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-notes".to_string(),
        "Notes Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session = SessionRecord::new("sess-notes".to_string(), "camp-notes".to_string(), 1);
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    let note = SessionNoteRecord::new(
        "note-001".to_string(),
        "sess-notes".to_string(),
        "camp-notes".to_string(),
        "The party found a mysterious artifact".to_string(),
    );

    db.save_session_note(&note)
        .await
        .expect("Failed to save session note");

    let notes = db
        .list_session_notes("sess-notes")
        .await
        .expect("Failed to list session notes");
    assert_eq!(notes.len(), 1);
    assert!(notes[0].content.contains("artifact"));
}

#[tokio::test]
async fn test_session_note_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-note-update".to_string(),
        "Note Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session =
        SessionRecord::new("sess-note-update".to_string(), "camp-note-update".to_string(), 1);
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    let mut note = SessionNoteRecord::new(
        "note-update".to_string(),
        "sess-note-update".to_string(),
        "camp-note-update".to_string(),
        "Initial note".to_string(),
    );
    db.save_session_note(&note)
        .await
        .expect("Failed to save");

    // Update
    note.content = "Updated note with more details".to_string();
    note.tags = Some(r#"["important","combat"]"#.to_string());
    note.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_session_note(&note)
        .await
        .expect("Failed to update");

    let retrieved = db
        .get_session_note("note-update")
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert!(retrieved.content.contains("more details"));
    assert!(retrieved.tags.as_ref().map_or(false, |t| t.contains("important")), "Tags should contain 'important'");
}

#[tokio::test]
async fn test_session_note_delete() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-note-del".to_string(),
        "Note Delete".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session = SessionRecord::new("sess-note-del".to_string(), "camp-note-del".to_string(), 1);
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    let note = SessionNoteRecord::new(
        "note-delete".to_string(),
        "sess-note-del".to_string(),
        "camp-note-del".to_string(),
        "To be deleted".to_string(),
    );
    db.save_session_note(&note)
        .await
        .expect("Failed to save");

    db.delete_session_note("note-delete")
        .await
        .expect("Failed to delete");

    let retrieved = db
        .get_session_note("note-delete")
        .await
        .expect("Query should succeed");
    assert!(retrieved.is_none());
}

// =============================================================================
// Session Events Tests
// =============================================================================

#[tokio::test]
async fn test_session_events() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-events".to_string(),
        "Events Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session = SessionRecord::new("sess-events".to_string(), "camp-events".to_string(), 1);
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    let event = SessionEventRecord::new(
        "event-001".to_string(),
        "sess-events".to_string(),
        "combat_start".to_string(),
    );

    db.save_session_event(&event)
        .await
        .expect("Failed to save session event");

    let events = db
        .list_session_events("sess-events")
        .await
        .expect("Failed to list session events");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "combat_start");

    let combat_events = db
        .list_session_events_by_type("sess-events", "combat_start")
        .await
        .expect("Failed to list events by type");
    assert_eq!(combat_events.len(), 1);
}

#[tokio::test]
async fn test_session_events_multiple_types() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-evt-types".to_string(),
        "Event Types".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session =
        SessionRecord::new("sess-evt-types".to_string(), "camp-evt-types".to_string(), 1);
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    let event_types = [
        "combat_start",
        "combat_end",
        "npc_interaction",
        "location_change",
    ];
    for (i, event_type) in event_types.iter().enumerate() {
        let event = SessionEventRecord::new(
            format!("evt-type-{}", i),
            "sess-evt-types".to_string(),
            event_type.to_string(),
        );
        db.save_session_event(&event)
            .await
            .expect("Failed to save");
    }

    let all_events = db
        .list_session_events("sess-evt-types")
        .await
        .expect("Failed to list");
    assert_eq!(all_events.len(), 4);

    let combat_starts = db
        .list_session_events_by_type("sess-evt-types", "combat_start")
        .await
        .expect("Failed to list");
    assert_eq!(combat_starts.len(), 1);
}

// =============================================================================
// Combat State Tests
// =============================================================================

#[tokio::test]
async fn test_combat_states() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-combat".to_string(),
        "Combat Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session = SessionRecord::new("sess-combat".to_string(), "camp-combat".to_string(), 1);
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    let combat = CombatStateRecord::new(
        "combat-001".to_string(),
        "sess-combat".to_string(),
        r#"[{"name":"Goblin","initiative":15},{"name":"Fighter","initiative":18}]"#.to_string(),
    );

    db.save_combat_state(&combat)
        .await
        .expect("Failed to save combat state");

    let active = db
        .get_active_combat("sess-combat")
        .await
        .expect("Failed to get active combat");
    let active_combat = active.expect("Active combat should be present");
    assert!(active_combat.is_active, "Combat should be active");

    db.end_combat("combat-001")
        .await
        .expect("Failed to end combat");

    let after_end = db
        .get_active_combat("sess-combat")
        .await
        .expect("Failed to get active combat");
    assert!(after_end.is_none(), "Combat should be ended");
}

#[tokio::test]
async fn test_combat_state_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-combat-update".to_string(),
        "Combat Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-combat-update".to_string(),
        "camp-combat-update".to_string(),
        1,
    );
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    let mut combat = CombatStateRecord::new(
        "combat-update".to_string(),
        "sess-combat-update".to_string(),
        r#"[{"name":"Goblin","hp":10}]"#.to_string(),
    );
    db.save_combat_state(&combat)
        .await
        .expect("Failed to save");

    // Update combat state
    combat.round = 5;
    combat.current_turn = 2;
    combat.combatants =
        r#"[{"name":"Goblin","hp":3},{"name":"Fighter","hp":20}]"#.to_string();
    combat.notes = Some("Intense battle!".to_string());
    combat.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_combat_state(&combat)
        .await
        .expect("Failed to update");

    let retrieved = db
        .get_combat_state("combat-update")
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert_eq!(retrieved.round, 5);
    assert_eq!(retrieved.current_turn, 2);
    assert!(retrieved.notes.as_ref().map_or(false, |n| n.contains("Intense")), "Notes should contain 'Intense'");
}

// =============================================================================
// Cascade Delete Tests
// =============================================================================

#[tokio::test]
async fn test_campaign_delete_cascades_sessions() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-cascade".to_string(),
        "Cascade Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let session = SessionRecord::new("sess-cascade".to_string(), "camp-cascade".to_string(), 1);
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    db.delete_campaign("camp-cascade")
        .await
        .expect("Failed to delete campaign");

    let sessions = db
        .list_sessions("camp-cascade")
        .await
        .expect("Query should succeed");
    assert_eq!(sessions.len(), 0, "Sessions should be deleted with campaign");
}
