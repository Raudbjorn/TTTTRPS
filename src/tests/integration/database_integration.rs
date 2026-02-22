//! Database Integration Tests
//!
//! Comprehensive integration tests for database operations including:
//! - Full campaign lifecycle (create -> update -> snapshot -> rollback -> delete)
//! - Session with combat flow
//! - NPC conversation persistence
//! - Concurrent campaign access
//! - Backup and restore cycle

use crate::database::{
    create_backup, list_backups, restore_backup, run_migrations, CampaignOps, CampaignRecord,
    CampaignVersionRecord, CharacterOps, CharacterRecord, CombatOps, CombatStateRecord, Database,
    EntityRelationshipRecord, EntityType, LocationOps, LocationRecord, NpcConversation, NpcOps,
    NpcRecord, RelationshipOps, SessionEventRecord, SessionNoteRecord, SessionOps, SessionRecord,
    UsageOps, UsageRecord,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Semaphore;

/// Create a test database in a temporary directory
async fn create_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db = Database::new(temp_dir.path())
        .await
        .expect("Failed to create test database");
    (db, temp_dir)
}

// =============================================================================
// Full Campaign Lifecycle Tests
// =============================================================================

#[tokio::test]
async fn test_full_campaign_lifecycle_create_update_snapshot_rollback_delete() {
    let (db, _temp) = create_test_db().await;

    // Step 1: Create a new campaign
    let mut campaign = CampaignRecord::new(
        "lifecycle-001".to_string(),
        "Epic Adventure".to_string(),
        "D&D 5e".to_string(),
    );
    campaign.description = Some("A grand adventure begins".to_string());
    campaign.setting = Some("Forgotten Realms".to_string());

    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    // Verify creation
    let retrieved = db
        .get_campaign("lifecycle-001")
        .await
        .expect("Failed to get campaign")
        .expect("Campaign not found after creation");
    assert_eq!(retrieved.name, "Epic Adventure");
    assert_eq!(retrieved.description, Some("A grand adventure begins".to_string()));

    // Step 2: Create initial snapshot (version 1)
    let version1 = CampaignVersionRecord::new(
        "ver-001".to_string(),
        "lifecycle-001".to_string(),
        1,
        "manual".to_string(),
        serde_json::json!({
            "name": campaign.name,
            "description": campaign.description,
            "setting": campaign.setting,
        }).to_string(),
    );
    db.save_campaign_version(&version1)
        .await
        .expect("Failed to save version 1");

    // Step 3: Update campaign with new state
    campaign.name = "Epic Adventure: The Return".to_string();
    campaign.description = Some("The adventure continues with new challenges".to_string());
    campaign.current_in_game_date = Some("Year 1492 DR, Day 45".to_string());
    campaign.updated_at = chrono::Utc::now().to_rfc3339();

    db.update_campaign(&campaign)
        .await
        .expect("Failed to update campaign");

    // Verify update
    let updated = db
        .get_campaign("lifecycle-001")
        .await
        .expect("Failed to get updated campaign")
        .expect("Updated campaign not found");
    assert_eq!(updated.name, "Epic Adventure: The Return");
    assert!(updated.current_in_game_date.is_some());

    // Step 4: Create second snapshot (version 2)
    let version2 = CampaignVersionRecord::new(
        "ver-002".to_string(),
        "lifecycle-001".to_string(),
        2,
        "auto_save".to_string(),
        serde_json::json!({
            "name": campaign.name,
            "description": campaign.description,
            "setting": campaign.setting,
            "current_in_game_date": campaign.current_in_game_date,
        }).to_string(),
    );
    db.save_campaign_version(&version2)
        .await
        .expect("Failed to save version 2");

    // Verify versions
    let versions = db
        .list_campaign_versions("lifecycle-001")
        .await
        .expect("Failed to list versions");
    assert_eq!(versions.len(), 2);

    let latest = db
        .get_latest_version_number("lifecycle-001")
        .await
        .expect("Failed to get latest version number");
    assert_eq!(latest, 2);

    // Step 5: Simulate rollback by loading version 1 data
    let v1_data = db
        .get_campaign_version("ver-001")
        .await
        .expect("Failed to get version 1")
        .expect("Version 1 not found");

    let v1_state: serde_json::Value = serde_json::from_str(&v1_data.data)
        .expect("Failed to parse version 1 data");

    // Apply rollback (restore original values)
    campaign.name = v1_state["name"].as_str().unwrap().to_string();
    campaign.description = v1_state["description"].as_str().map(|s| s.to_string());
    campaign.current_in_game_date = None;
    campaign.updated_at = chrono::Utc::now().to_rfc3339();

    db.update_campaign(&campaign)
        .await
        .expect("Failed to apply rollback");

    // Verify rollback
    let rolled_back = db
        .get_campaign("lifecycle-001")
        .await
        .expect("Failed to get rolled back campaign")
        .expect("Rolled back campaign not found");
    assert_eq!(rolled_back.name, "Epic Adventure");
    assert!(rolled_back.current_in_game_date.is_none());

    // Step 6: Delete campaign
    db.delete_campaign("lifecycle-001")
        .await
        .expect("Failed to delete campaign");

    // Verify deletion
    let deleted = db
        .get_campaign("lifecycle-001")
        .await
        .expect("Query should succeed");
    assert!(deleted.is_none(), "Campaign should be deleted");

    // Versions should also be cleaned up (depends on implementation)
    // Note: Current implementation may not cascade delete versions
}

// =============================================================================
// Session with Combat Flow Tests
// =============================================================================

#[tokio::test]
async fn test_session_with_combat_flow() {
    let (db, _temp) = create_test_db().await;

    // Setup: Create campaign
    let campaign = CampaignRecord::new(
        "combat-camp".to_string(),
        "Combat Test Campaign".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    // Step 1: Create session
    let session = SessionRecord::new(
        "combat-sess".to_string(),
        "combat-camp".to_string(),
        1,
    );
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    // Step 2: Record session start event
    let start_event = SessionEventRecord::new(
        "event-start".to_string(),
        "combat-sess".to_string(),
        "session_start".to_string(),
    );
    db.save_session_event(&start_event)
        .await
        .expect("Failed to save session start event");

    // Step 3: Start combat encounter
    let combat_start_event = SessionEventRecord::new(
        "event-combat-start".to_string(),
        "combat-sess".to_string(),
        "combat_start".to_string(),
    );
    db.save_session_event(&combat_start_event)
        .await
        .expect("Failed to save combat start event");

    // Step 4: Create combat state with combatants
    let combatants = serde_json::json!([
        {"id": "fighter-1", "name": "Sir Brave", "initiative": 18, "hp": 45, "max_hp": 45},
        {"id": "wizard-1", "name": "Mystara", "initiative": 12, "hp": 28, "max_hp": 28},
        {"id": "goblin-1", "name": "Goblin Archer", "initiative": 15, "hp": 12, "max_hp": 12},
        {"id": "goblin-2", "name": "Goblin Warrior", "initiative": 10, "hp": 15, "max_hp": 15},
    ]);

    let combat = CombatStateRecord::new(
        "combat-001".to_string(),
        "combat-sess".to_string(),
        combatants.to_string(),
    );
    db.save_combat_state(&combat)
        .await
        .expect("Failed to save combat state");

    // Verify active combat
    let active_combat = db
        .get_active_combat("combat-sess")
        .await
        .expect("Failed to get active combat")
        .expect("No active combat found");
    assert!(active_combat.is_active);
    assert_eq!(active_combat.round, 1);

    // Step 5: Simulate combat rounds - update combat state
    let mut updated_combat = active_combat.clone();
    updated_combat.round = 3;
    updated_combat.current_turn = 2;

    // Update combatants (goblin-1 defeated)
    let updated_combatants = serde_json::json!([
        {"id": "fighter-1", "name": "Sir Brave", "initiative": 18, "hp": 32, "max_hp": 45},
        {"id": "wizard-1", "name": "Mystara", "initiative": 12, "hp": 20, "max_hp": 28},
        {"id": "goblin-1", "name": "Goblin Archer", "initiative": 15, "hp": 0, "max_hp": 12, "defeated": true},
        {"id": "goblin-2", "name": "Goblin Warrior", "initiative": 10, "hp": 5, "max_hp": 15},
    ]);
    updated_combat.combatants = updated_combatants.to_string();
    updated_combat.updated_at = chrono::Utc::now().to_rfc3339();

    db.save_combat_state(&updated_combat)
        .await
        .expect("Failed to update combat state");

    // Step 6: End combat
    db.end_combat("combat-001")
        .await
        .expect("Failed to end combat");

    // Record combat end event
    let combat_end_event = SessionEventRecord::new(
        "event-combat-end".to_string(),
        "combat-sess".to_string(),
        "combat_end".to_string(),
    );
    db.save_session_event(&combat_end_event)
        .await
        .expect("Failed to save combat end event");

    // Verify combat ended
    let ended_combat = db
        .get_active_combat("combat-sess")
        .await
        .expect("Failed to query combat");
    assert!(ended_combat.is_none(), "Combat should be ended");

    // Verify combat history
    let all_combats = db
        .list_session_combats("combat-sess")
        .await
        .expect("Failed to list combats");
    assert_eq!(all_combats.len(), 1);
    assert!(!all_combats[0].is_active);

    // Verify session events timeline
    let events = db
        .list_session_events("combat-sess")
        .await
        .expect("Failed to list events");
    assert_eq!(events.len(), 3);

    let combat_events = db
        .list_session_events_by_type("combat-sess", "combat_start")
        .await
        .expect("Failed to list combat events");
    assert_eq!(combat_events.len(), 1);
}

// =============================================================================
// NPC Conversation Persistence Tests
// =============================================================================

#[tokio::test]
async fn test_npc_conversation_persistence() {
    let (db, _temp) = create_test_db().await;

    // Setup: Create campaign and NPC
    let campaign = CampaignRecord::new(
        "npc-conv-camp".to_string(),
        "NPC Conversation Campaign".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let npc = NpcRecord {
        id: "blacksmith-001".to_string(),
        campaign_id: Some("npc-conv-camp".to_string()),
        name: "Tormund the Blacksmith".to_string(),
        role: "Village Blacksmith".to_string(),
        personality_id: None,
        personality_json: serde_json::json!({
            "traits": ["gruff", "honest", "hardworking"],
            "quirks": ["speaks in short sentences", "always polishing tools"],
            "voice": {"pitch": "low", "pace": "slow"}
        }).to_string(),
        data_json: None,
        stats_json: None,
        notes: Some("Knows secret about the old mine".to_string()),
        location_id: None,
        voice_profile_id: None,
        quest_hooks: Some(serde_json::json!([
            {"id": "quest-1", "hook": "Needs rare ore from the mine"}
        ]).to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_npc(&npc).await.expect("Failed to save NPC");

    // Step 1: Create initial conversation
    let mut conversation = NpcConversation::new(
        "conv-001".to_string(),
        "blacksmith-001".to_string(),
        "npc-conv-camp".to_string(),
    );

    // Add messages to conversation
    let messages = vec![
        serde_json::json!({
            "id": "msg-001",
            "role": "user",
            "content": "Hello, are you the blacksmith?",
            "created_at": "2024-01-01T10:00:00Z"
        }),
        serde_json::json!({
            "id": "msg-002",
            "role": "npc",
            "content": "*looks up from anvil* Aye. What you need?",
            "created_at": "2024-01-01T10:00:05Z"
        }),
        serde_json::json!({
            "id": "msg-003",
            "role": "user",
            "content": "I need a sword repaired. Can you help?",
            "created_at": "2024-01-01T10:00:15Z"
        }),
        serde_json::json!({
            "id": "msg-004",
            "role": "npc",
            "content": "*examines the blade* Good steel. This'll take a day. Twenty gold.",
            "created_at": "2024-01-01T10:00:25Z"
        }),
    ];

    conversation.messages_json = serde_json::to_string(&messages).unwrap();
    conversation.last_message_at = "2024-01-01T10:00:25Z".to_string();

    db.save_npc_conversation(&conversation)
        .await
        .expect("Failed to save conversation");

    // Step 2: Retrieve and verify conversation
    let retrieved = db
        .get_npc_conversation("blacksmith-001")
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");

    let retrieved_messages: Vec<serde_json::Value> =
        serde_json::from_str(&retrieved.messages_json).unwrap();
    assert_eq!(retrieved_messages.len(), 4);
    assert_eq!(retrieved_messages[0]["role"], "user");
    assert_eq!(retrieved_messages[3]["role"], "npc");

    // Step 3: Add more messages (simulating continuation)
    let mut new_messages = retrieved_messages.clone();
    new_messages.push(serde_json::json!({
        "id": "msg-005",
        "role": "user",
        "content": "Twenty gold? That seems fair. Deal!",
        "created_at": "2024-01-01T10:00:35Z"
    }));
    new_messages.push(serde_json::json!({
        "id": "msg-006",
        "role": "npc",
        "content": "*nods* Come back tomorrow. Name's Tormund.",
        "created_at": "2024-01-01T10:00:45Z"
    }));

    let mut updated_conv = retrieved.clone();
    updated_conv.messages_json = serde_json::to_string(&new_messages).unwrap();
    updated_conv.last_message_at = "2024-01-01T10:00:45Z".to_string();
    updated_conv.updated_at = chrono::Utc::now().to_rfc3339();

    db.save_npc_conversation(&updated_conv)
        .await
        .expect("Failed to update conversation");

    // Step 4: Verify updated conversation
    let final_conv = db
        .get_npc_conversation("blacksmith-001")
        .await
        .expect("Failed to get final conversation")
        .expect("Final conversation not found");

    let final_messages: Vec<serde_json::Value> =
        serde_json::from_str(&final_conv.messages_json).unwrap();
    assert_eq!(final_messages.len(), 6);

    // Step 5: List all conversations for campaign
    let all_convs = db
        .list_npc_conversations("npc-conv-camp")
        .await
        .expect("Failed to list conversations");
    assert_eq!(all_convs.len(), 1);
    assert_eq!(all_convs[0].npc_id, "blacksmith-001");
}

// =============================================================================
// Concurrent Campaign Access Tests
// =============================================================================

#[tokio::test]
async fn test_concurrent_campaign_access() {
    let (db, _temp) = create_test_db().await;
    let db = Arc::new(db);

    // Create initial campaign
    let campaign = CampaignRecord::new(
        "concurrent-camp".to_string(),
        "Concurrent Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    // Spawn multiple concurrent tasks
    let num_tasks = 10;
    let mut handles = Vec::new();

    for i in 0..num_tasks {
        let db_clone = Arc::clone(&db);
        let handle = tokio::spawn(async move {
            // Each task performs multiple operations

            // 1. Read campaign
            let camp = db_clone
                .get_campaign("concurrent-camp")
                .await
                .expect("Failed to get campaign")
                .expect("Campaign not found");

            // 2. Create a session
            let session = SessionRecord::new(
                format!("concurrent-sess-{}", i),
                "concurrent-camp".to_string(),
                i as i32 + 1,
            );
            db_clone
                .create_session(&session)
                .await
                .expect("Failed to create session");

            // 3. Create a note
            let note = SessionNoteRecord::new(
                format!("concurrent-note-{}", i),
                format!("concurrent-sess-{}", i),
                "concurrent-camp".to_string(),
                format!("Note from task {}", i),
            );
            db_clone
                .save_session_note(&note)
                .await
                .expect("Failed to save note");

            // 4. Read sessions
            let sessions = db_clone
                .list_sessions("concurrent-camp")
                .await
                .expect("Failed to list sessions");

            (camp.name.clone(), sessions.len())
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let results: Vec<_> = futures_util::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("Task panicked"))
        .collect();

    // Verify all tasks succeeded
    assert_eq!(results.len(), num_tasks);
    for (name, _) in &results {
        assert_eq!(name, "Concurrent Test");
    }

    // Verify final state
    let final_sessions = db
        .list_sessions("concurrent-camp")
        .await
        .expect("Failed to list final sessions");
    assert_eq!(final_sessions.len(), num_tasks);

    let final_notes = db
        .list_campaign_notes("concurrent-camp")
        .await
        .expect("Failed to list final notes");
    assert_eq!(final_notes.len(), num_tasks);
}

#[tokio::test]
async fn test_concurrent_write_same_entity() {
    let (db, _temp) = create_test_db().await;
    let db = Arc::new(db);

    // Create initial campaign
    let campaign = CampaignRecord::new(
        "write-test-camp".to_string(),
        "Write Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    // Create initial NPC
    let npc = NpcRecord {
        id: "shared-npc".to_string(),
        campaign_id: Some("write-test-camp".to_string()),
        name: "Shared NPC".to_string(),
        role: "Test Role".to_string(),
        personality_id: None,
        personality_json: "{}".to_string(),
        data_json: None,
        stats_json: None,
        notes: Some("Initial notes".to_string()),
        location_id: None,
        voice_profile_id: None,
        quest_hooks: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_npc(&npc).await.expect("Failed to save NPC");

    // Semaphore to coordinate access
    let semaphore = Arc::new(Semaphore::new(5));

    // Multiple tasks try to update the same NPC
    let num_tasks = 20;
    let mut handles = Vec::new();

    for i in 0..num_tasks {
        let db_clone = Arc::clone(&db);
        let sem_clone = Arc::clone(&semaphore);

        let handle = tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.unwrap();

            // Read current state
            let mut current = db_clone
                .get_npc("shared-npc")
                .await
                .expect("Failed to get NPC")
                .expect("NPC not found");

            // Modify and save
            current.notes = Some(format!("Updated by task {}", i));

            db_clone.save_npc(&current).await.expect("Failed to save NPC");

            i
        });
        handles.push(handle);
    }

    // Wait for all tasks
    let results: Vec<_> = futures_util::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("Task panicked"))
        .collect();

    assert_eq!(results.len(), num_tasks);

    // Verify NPC still exists and has valid state
    let final_npc = db
        .get_npc("shared-npc")
        .await
        .expect("Failed to get final NPC")
        .expect("Final NPC not found");

    assert!(final_npc.notes.is_some());
    assert!(final_npc.notes.unwrap().starts_with("Updated by task"));
}

// =============================================================================
// Backup and Restore Cycle Tests
// =============================================================================

#[tokio::test]
async fn test_backup_and_restore_cycle() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let backup_dir = temp_dir.path().join("backups");
    std::fs::create_dir_all(&backup_dir).expect("Failed to create backup directory");

    // Step 1: Create database and populate with data
    let db = Database::new(temp_dir.path())
        .await
        .expect("Failed to create database");

    // Create campaign
    let campaign = CampaignRecord::new(
        "backup-camp".to_string(),
        "Backup Test Campaign".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    // Create session
    let session = SessionRecord::new(
        "backup-sess".to_string(),
        "backup-camp".to_string(),
        1,
    );
    db.create_session(&session)
        .await
        .expect("Failed to create session");

    // Create character
    let character = CharacterRecord {
        id: "backup-char".to_string(),
        campaign_id: Some("backup-camp".to_string()),
        name: "Backup Hero".to_string(),
        system: "D&D 5e".to_string(),
        character_type: "player".to_string(),
        level: Some(5),
        data_json: r#"{"class":"Fighter"}"#.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_character(&character)
        .await
        .expect("Failed to save character");

    // Record some usage
    let usage = UsageRecord::new("claude".to_string(), "claude-3-sonnet".to_string(), 1000, 500);
    db.record_usage(&usage).await.expect("Failed to record usage");

    // Step 2: Create backup
    let backup_info = create_backup(
        db.path(),
        &backup_dir,
        Some("Pre-disaster backup".to_string()),
    )
    .expect("Failed to create backup");

    assert!(backup_info.path.exists());
    assert!(backup_info.size_bytes > 0);

    // Step 3: Modify data (simulating changes after backup)
    let mut modified_campaign = campaign.clone();
    modified_campaign.name = "MODIFIED - Should Be Lost".to_string();
    modified_campaign.updated_at = chrono::Utc::now().to_rfc3339();
    db.update_campaign(&modified_campaign)
        .await
        .expect("Failed to update campaign");

    // Delete the character
    db.delete_character("backup-char")
        .await
        .expect("Failed to delete character");

    // Verify modifications
    let modified = db
        .get_campaign("backup-camp")
        .await
        .expect("Failed to get campaign")
        .expect("Campaign not found");
    assert_eq!(modified.name, "MODIFIED - Should Be Lost");

    let deleted_char = db
        .get_character("backup-char")
        .await
        .expect("Query should succeed");
    assert!(deleted_char.is_none());

    // Drop database connection before restore
    drop(db);

    // Step 4: Restore from backup
    let db_path = temp_dir.path().join("ttrpg_assistant.db");
    restore_backup(&backup_info.path, &db_path).expect("Failed to restore backup");

    // Step 5: Verify restored data
    let restored_db = Database::new(temp_dir.path())
        .await
        .expect("Failed to open restored database");

    // Campaign should have original name
    let restored_campaign = restored_db
        .get_campaign("backup-camp")
        .await
        .expect("Failed to get restored campaign")
        .expect("Restored campaign not found");
    assert_eq!(restored_campaign.name, "Backup Test Campaign");

    // Character should be back
    let restored_char = restored_db
        .get_character("backup-char")
        .await
        .expect("Failed to get restored character")
        .expect("Restored character not found");
    assert_eq!(restored_char.name, "Backup Hero");

    // Session should exist
    let restored_session = restored_db
        .get_session("backup-sess")
        .await
        .expect("Failed to get restored session")
        .expect("Restored session not found");
    assert_eq!(restored_session.campaign_id, "backup-camp");

    // Usage stats should be preserved
    let usage_stats = restored_db
        .get_total_usage()
        .await
        .expect("Failed to get usage stats");
    assert_eq!(usage_stats.total_input_tokens, 1000);
    assert_eq!(usage_stats.total_output_tokens, 500);
}

#[tokio::test]
async fn test_list_and_manage_backups() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let backup_dir = temp_dir.path().join("backups");
    std::fs::create_dir_all(&backup_dir).expect("Failed to create backup directory");

    // Create database
    let db = Database::new(temp_dir.path())
        .await
        .expect("Failed to create database");

    // Create multiple backups with 1-second delays to ensure unique timestamps
    // (backup filenames use second-precision timestamps)
    let mut backup_paths = Vec::new();
    for i in 1..=3 {
        let backup_info = create_backup(
            db.path(),
            &backup_dir,
            Some(format!("Backup {}", i)),
        )
        .expect("Failed to create backup");
        backup_paths.push(backup_info.path.clone());

        // Wait 1 second to ensure next backup has different timestamp
        if i < 3 {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    // List all backups
    let backups = list_backups(&backup_dir).expect("Failed to list backups");
    assert_eq!(backups.len(), 3);

    // Verify backups are sorted by creation time (newest first)
    assert!(backups[0].created_at >= backups[1].created_at);
    assert!(backups[1].created_at >= backups[2].created_at);

    // Verify each backup has description
    for backup in backups.iter() {
        assert!(backup.description.is_some());
        let desc = backup.description.as_ref().unwrap();
        // Note: Order is reversed (newest first)
        assert!(desc.contains("Backup"));
    }
}

// =============================================================================
// Complex Relationship Tests
// =============================================================================

#[tokio::test]
async fn test_complex_entity_relationships() {
    let (db, _temp) = create_test_db().await;

    // Create campaign
    let campaign = CampaignRecord::new(
        "rel-camp".to_string(),
        "Relationship Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    // Create locations
    let city = LocationRecord::new(
        "loc-city".to_string(),
        "rel-camp".to_string(),
        "Waterdeep".to_string(),
        "city".to_string(),
    );
    db.save_location(&city).await.expect("Failed to save city");

    let mut tavern = LocationRecord::new(
        "loc-tavern".to_string(),
        "rel-camp".to_string(),
        "Yawning Portal".to_string(),
        "tavern".to_string(),
    );
    tavern.parent_id = Some("loc-city".to_string());
    db.save_location(&tavern).await.expect("Failed to save tavern");

    // Create NPCs
    let innkeeper = NpcRecord {
        id: "npc-innkeeper".to_string(),
        campaign_id: Some("rel-camp".to_string()),
        name: "Durnan".to_string(),
        role: "Innkeeper".to_string(),
        personality_id: None,
        personality_json: "{}".to_string(),
        data_json: None,
        stats_json: None,
        notes: None,
        location_id: Some("loc-tavern".to_string()),
        voice_profile_id: None,
        quest_hooks: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_npc(&innkeeper).await.expect("Failed to save innkeeper");

    let rival = NpcRecord {
        id: "npc-rival".to_string(),
        campaign_id: Some("rel-camp".to_string()),
        name: "Xanathar".to_string(),
        role: "Crime Lord".to_string(),
        personality_id: None,
        personality_json: "{}".to_string(),
        data_json: None,
        stats_json: None,
        notes: None,
        location_id: None,
        voice_profile_id: None,
        quest_hooks: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_npc(&rival).await.expect("Failed to save rival");

    // Create relationships
    // Durnan is enemy of Xanathar (bidirectional)
    let mut enemy_rel = EntityRelationshipRecord::new(
        "rel-enemy".to_string(),
        "rel-camp".to_string(),
        EntityType::Npc,
        "npc-innkeeper".to_string(),
        EntityType::Npc,
        "npc-rival".to_string(),
        "enemy".to_string(),
    );
    enemy_rel.bidirectional = true;
    enemy_rel.description = Some("Long-standing rivalry over Waterdeep's underworld".to_string());
    enemy_rel.strength = 0.9;
    db.save_entity_relationship(&enemy_rel)
        .await
        .expect("Failed to save enemy relationship");

    // Durnan is located_at Yawning Portal
    let located_rel = EntityRelationshipRecord::new(
        "rel-located".to_string(),
        "rel-camp".to_string(),
        EntityType::Npc,
        "npc-innkeeper".to_string(),
        EntityType::Location,
        "loc-tavern".to_string(),
        "located_at".to_string(),
    );
    db.save_entity_relationship(&located_rel)
        .await
        .expect("Failed to save located relationship");

    // Query relationships for Durnan
    let durnan_rels = db
        .list_relationships_for_entity("npc", "npc-innkeeper")
        .await
        .expect("Failed to list Durnan's relationships");
    assert_eq!(durnan_rels.len(), 2);

    // Query relationships for Xanathar (should find bidirectional enemy)
    let xanathar_rels = db
        .list_relationships_for_entity("npc", "npc-rival")
        .await
        .expect("Failed to list Xanathar's relationships");
    assert_eq!(xanathar_rels.len(), 1);
    assert_eq!(xanathar_rels[0].relationship_type, "enemy");

    // Query all enemy relationships in campaign
    let enemies = db
        .list_relationships_by_type("rel-camp", "enemy")
        .await
        .expect("Failed to list enemies");
    assert_eq!(enemies.len(), 1);

    // Delete relationships for Durnan
    let deleted = db
        .delete_relationships_for_entity("npc", "npc-innkeeper")
        .await
        .expect("Failed to delete relationships");
    assert_eq!(deleted, 2);

    // Verify deletion
    let remaining = db
        .list_campaign_relationships("rel-camp")
        .await
        .expect("Failed to list remaining relationships");
    assert_eq!(remaining.len(), 0);
}

// =============================================================================
// Data Integrity Tests
// =============================================================================

#[tokio::test]
async fn test_usage_tracking_accumulation() {
    let (db, _temp) = create_test_db().await;

    // Record multiple usage entries
    let usage_entries = vec![
        UsageRecord::new("claude".to_string(), "claude-3-sonnet".to_string(), 1000, 500),
        UsageRecord::new("claude".to_string(), "claude-3-opus".to_string(), 500, 200),
        UsageRecord::new("openai".to_string(), "gpt-4".to_string(), 800, 400),
        UsageRecord::new("ollama".to_string(), "llama3".to_string(), 2000, 1000),
    ];

    for usage in &usage_entries {
        db.record_usage(usage).await.expect("Failed to record usage");
    }

    // Verify total usage
    let total = db.get_total_usage().await.expect("Failed to get total usage");
    assert_eq!(total.total_input_tokens, 4300); // 1000 + 500 + 800 + 2000
    assert_eq!(total.total_output_tokens, 2100); // 500 + 200 + 400 + 1000
    assert_eq!(total.total_requests, 4);

    // Verify per-provider usage
    let by_provider = db
        .get_usage_by_provider()
        .await
        .expect("Failed to get usage by provider");
    assert_eq!(by_provider.len(), 3);

    let claude_usage = by_provider.iter().find(|u| u.provider == "claude").unwrap();
    assert_eq!(claude_usage.input_tokens, 1500);
    assert_eq!(claude_usage.output_tokens, 700);
    assert_eq!(claude_usage.requests, 2);

    // Reset and verify
    db.reset_usage_stats().await.expect("Failed to reset usage");

    let reset_total = db.get_total_usage().await.expect("Failed to get reset total");
    assert_eq!(reset_total.total_requests, 0);
}

#[tokio::test]
async fn test_migrations_are_idempotent() {
    let (db, _temp) = create_test_db().await;

    // Run migrations again - should be idempotent
    run_migrations(db.pool())
        .await
        .expect("Re-running migrations should not fail");

    // Run again
    run_migrations(db.pool())
        .await
        .expect("Third migration run should also succeed");

    // Database should still work
    let campaign = CampaignRecord::new(
        "idempotent-camp".to_string(),
        "Idempotent Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Database should work after multiple migration runs");

    let retrieved = db
        .get_campaign("idempotent-camp")
        .await
        .expect("Query should work")
        .expect("Campaign should exist");
    assert_eq!(retrieved.name, "Idempotent Test");
}
