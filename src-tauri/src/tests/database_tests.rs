//! Database Unit Tests
//!
//! Comprehensive tests for SQLite database operations.
//! Uses an in-memory database for fast, isolated testing.

use crate::database::{
    run_migrations, CampaignRecord, CampaignVersionRecord, CharacterRecord, CombatStateRecord,
    Database, EntityRelationshipRecord, EntityType, LocationRecord, NpcConversation,
    NpcRecord, PersonalityRecord, SessionEventRecord, SessionNoteRecord, SessionRecord,
    UsageRecord, VoiceProfileRecord,
};
use tempfile::TempDir;

/// Create a test database in a temporary directory
async fn create_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db = Database::new(temp_dir.path())
        .await
        .expect("Failed to create test database");
    (db, temp_dir)
}

// =============================================================================
// Campaign Tests
// =============================================================================

#[tokio::test]
async fn test_create_campaign() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-001".to_string(),
        "Dragon's Lair".to_string(),
        "D&D 5e".to_string(),
    );

    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let retrieved = db
        .get_campaign("camp-001")
        .await
        .expect("Failed to get campaign")
        .expect("Campaign not found");

    assert_eq!(retrieved.id, "camp-001");
    assert_eq!(retrieved.name, "Dragon's Lair");
    assert_eq!(retrieved.system, "D&D 5e");
}

#[tokio::test]
async fn test_update_campaign() {
    let (db, _temp) = create_test_db().await;

    let mut campaign = CampaignRecord::new(
        "camp-002".to_string(),
        "Original Name".to_string(),
        "PF2e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    campaign.name = "Updated Name".to_string();
    campaign.description = Some("A thrilling adventure".to_string());
    campaign.setting = Some("Forgotten Realms".to_string());
    campaign.updated_at = chrono::Utc::now().to_rfc3339();

    db.update_campaign(&campaign).await.expect("Failed to update campaign");

    let retrieved = db
        .get_campaign("camp-002")
        .await
        .expect("Failed to get campaign")
        .expect("Campaign not found");

    assert_eq!(retrieved.name, "Updated Name");
    assert_eq!(retrieved.description, Some("A thrilling adventure".to_string()));
    assert_eq!(retrieved.setting, Some("Forgotten Realms".to_string()));
}

#[tokio::test]
async fn test_list_campaigns() {
    let (db, _temp) = create_test_db().await;

    for i in 1..=5 {
        let campaign = CampaignRecord::new(
            format!("camp-{:03}", i),
            format!("Campaign {}", i),
            "D&D 5e".to_string(),
        );
        db.create_campaign(&campaign).await.expect("Failed to create campaign");
    }

    let campaigns = db.list_campaigns().await.expect("Failed to list campaigns");
    assert_eq!(campaigns.len(), 5);
}

#[tokio::test]
async fn test_delete_campaign() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-del".to_string(),
        "To Delete".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    db.delete_campaign("camp-del").await.expect("Failed to delete campaign");

    let retrieved = db.get_campaign("camp-del").await.expect("Query should succeed");
    assert!(retrieved.is_none(), "Campaign should be deleted");
}

// =============================================================================
// Session Tests
// =============================================================================

#[tokio::test]
async fn test_create_session() {
    let (db, _temp) = create_test_db().await;

    // Create parent campaign first
    let campaign = CampaignRecord::new(
        "camp-sess".to_string(),
        "Session Test Campaign".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-001".to_string(),
        "camp-sess".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

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
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    for i in 1..=3 {
        let session = SessionRecord::new(
            format!("sess-{:03}", i),
            "camp-multi-sess".to_string(),
            i,
        );
        db.create_session(&session).await.expect("Failed to create session");
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
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-active".to_string(),
        "camp-active".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

    let active = db
        .get_active_session("camp-active")
        .await
        .expect("Failed to get active session");

    assert!(active.is_some());
    assert_eq!(active.unwrap().id, "sess-active");
}

// =============================================================================
// Character Tests
// =============================================================================

#[tokio::test]
async fn test_save_character() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-char".to_string(),
        "Character Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let character = CharacterRecord {
        id: "char-001".to_string(),
        campaign_id: Some("camp-char".to_string()),
        name: "Thorin Ironforge".to_string(),
        system: "D&D 5e".to_string(),
        character_type: "player".to_string(),
        level: Some(5),
        data_json: r#"{"class":"Fighter","race":"Dwarf"}"#.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    db.save_character(&character).await.expect("Failed to save character");

    let retrieved = db
        .get_character("char-001")
        .await
        .expect("Failed to get character")
        .expect("Character not found");

    assert_eq!(retrieved.name, "Thorin Ironforge");
    assert_eq!(retrieved.level, Some(5));
}

#[tokio::test]
async fn test_list_characters() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-chars".to_string(),
        "Characters Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    for i in 1..=3 {
        let character = CharacterRecord {
            id: format!("char-{:03}", i),
            campaign_id: Some("camp-chars".to_string()),
            name: format!("Character {}", i),
            system: "D&D 5e".to_string(),
            character_type: "player".to_string(),
            level: Some(i),
            data_json: "{}".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        db.save_character(&character).await.expect("Failed to save character");
    }

    let characters = db
        .list_characters(Some("camp-chars"))
        .await
        .expect("Failed to list characters");
    assert_eq!(characters.len(), 3);
}

// =============================================================================
// NPC Tests
// =============================================================================

#[tokio::test]
async fn test_save_npc() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-npc".to_string(),
        "NPC Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let npc = NpcRecord {
        id: "npc-001".to_string(),
        campaign_id: Some("camp-npc".to_string()),
        name: "Bartender Bob".to_string(),
        role: "Tavern Owner".to_string(),
        personality_id: None,
        personality_json: r#"{"traits":["friendly","talkative"]}"#.to_string(),
        data_json: None,
        stats_json: None,
        notes: Some("Knows everyone in town".to_string()),
        location_id: None,
        voice_profile_id: None,
        quest_hooks: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    db.save_npc(&npc).await.expect("Failed to save NPC");

    let retrieved = db
        .get_npc("npc-001")
        .await
        .expect("Failed to get NPC")
        .expect("NPC not found");

    assert_eq!(retrieved.name, "Bartender Bob");
    assert_eq!(retrieved.role, "Tavern Owner");
}

// =============================================================================
// Campaign Version Tests
// =============================================================================

#[tokio::test]
async fn test_campaign_versioning() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-ver".to_string(),
        "Version Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Create multiple versions
    for i in 1..=3 {
        let version = CampaignVersionRecord::new(
            format!("ver-{:03}", i),
            "camp-ver".to_string(),
            i,
            "manual".to_string(),
            format!(r#"{{"version":{}}}"#, i),
        );
        db.save_campaign_version(&version)
            .await
            .expect("Failed to save version");
    }

    let versions = db
        .list_campaign_versions("camp-ver")
        .await
        .expect("Failed to list versions");
    assert_eq!(versions.len(), 3);

    let latest_num = db
        .get_latest_version_number("camp-ver")
        .await
        .expect("Failed to get latest version number");
    assert_eq!(latest_num, 3);
}

// =============================================================================
// Entity Relationship Tests
// =============================================================================

#[tokio::test]
async fn test_entity_relationships() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-rel".to_string(),
        "Relationship Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Create two NPCs
    for i in 1..=2 {
        let npc = NpcRecord {
            id: format!("npc-rel-{:03}", i),
            campaign_id: Some("camp-rel".to_string()),
            name: format!("NPC {}", i),
            role: "Test".to_string(),
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
        db.save_npc(&npc).await.expect("Failed to save NPC");
    }

    // Create relationship
    let relationship = EntityRelationshipRecord::new(
        "rel-001".to_string(),
        "camp-rel".to_string(),
        EntityType::Npc,
        "npc-rel-001".to_string(),
        EntityType::Npc,
        "npc-rel-002".to_string(),
        "ally".to_string(),
    );

    db.save_entity_relationship(&relationship)
        .await
        .expect("Failed to save relationship");

    // Query relationships
    let relationships = db
        .list_relationships_for_entity("npc", "npc-rel-001")
        .await
        .expect("Failed to list relationships");
    assert_eq!(relationships.len(), 1);
    assert_eq!(relationships[0].relationship_type, "ally");
}

#[tokio::test]
async fn test_bidirectional_relationships() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-bidir".to_string(),
        "Bidirectional Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let mut relationship = EntityRelationshipRecord::new(
        "rel-bidir".to_string(),
        "camp-bidir".to_string(),
        EntityType::Npc,
        "npc-a".to_string(),
        EntityType::Npc,
        "npc-b".to_string(),
        "friend".to_string(),
    );
    relationship.bidirectional = true;

    db.save_entity_relationship(&relationship)
        .await
        .expect("Failed to save relationship");

    // Query from target entity side (should find due to bidirectional flag)
    let from_target = db
        .list_relationships_for_entity("npc", "npc-b")
        .await
        .expect("Failed to list relationships");
    assert_eq!(from_target.len(), 1);
}

// =============================================================================
// Voice Profile Tests
// =============================================================================

#[tokio::test]
async fn test_voice_profiles() {
    let (db, _temp) = create_test_db().await;

    let profile = VoiceProfileRecord::new(
        "voice-001".to_string(),
        "Gruff Warrior".to_string(),
        "elevenlabs".to_string(),
        "abc123".to_string(),
    );

    db.save_voice_profile(&profile)
        .await
        .expect("Failed to save voice profile");

    let retrieved = db
        .get_voice_profile("voice-001")
        .await
        .expect("Failed to get voice profile")
        .expect("Voice profile not found");

    assert_eq!(retrieved.name, "Gruff Warrior");
    assert_eq!(retrieved.provider, "elevenlabs");

    let profiles = db
        .list_voice_profiles()
        .await
        .expect("Failed to list voice profiles");
    assert_eq!(profiles.len(), 1);
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
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-notes".to_string(),
        "camp-notes".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

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
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-events".to_string(),
        "camp-events".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

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

    // Test filtering by event type
    let combat_events = db
        .list_session_events_by_type("sess-events", "combat_start")
        .await
        .expect("Failed to list events by type");
    assert_eq!(combat_events.len(), 1);
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
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-combat".to_string(),
        "camp-combat".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

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
    assert!(active.is_some());
    assert!(active.unwrap().is_active);

    // End the combat
    db.end_combat("combat-001")
        .await
        .expect("Failed to end combat");

    let after_end = db
        .get_active_combat("sess-combat")
        .await
        .expect("Failed to get active combat");
    assert!(after_end.is_none(), "Combat should be ended");
}

// =============================================================================
// Location Tests
// =============================================================================

#[tokio::test]
async fn test_locations() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-loc".to_string(),
        "Location Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let location = LocationRecord::new(
        "loc-001".to_string(),
        "camp-loc".to_string(),
        "Dragon's Den Tavern".to_string(),
        "tavern".to_string(),
    );

    db.save_location(&location)
        .await
        .expect("Failed to save location");

    let retrieved = db
        .get_location("loc-001")
        .await
        .expect("Failed to get location")
        .expect("Location not found");

    assert_eq!(retrieved.name, "Dragon's Den Tavern");
    assert_eq!(retrieved.location_type, "tavern");

    let locations = db
        .list_locations("camp-loc")
        .await
        .expect("Failed to list locations");
    assert_eq!(locations.len(), 1);
}

#[tokio::test]
async fn test_location_hierarchy() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-hier".to_string(),
        "Hierarchy Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Parent location
    let city = LocationRecord::new(
        "loc-city".to_string(),
        "camp-hier".to_string(),
        "Waterdeep".to_string(),
        "city".to_string(),
    );
    db.save_location(&city).await.expect("Failed to save city");

    // Child location
    let mut tavern = LocationRecord::new(
        "loc-tavern".to_string(),
        "camp-hier".to_string(),
        "Yawning Portal".to_string(),
        "tavern".to_string(),
    );
    tavern.parent_id = Some("loc-city".to_string());
    db.save_location(&tavern).await.expect("Failed to save tavern");

    let children = db
        .list_child_locations("loc-city")
        .await
        .expect("Failed to list children");
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "Yawning Portal");

    let roots = db
        .list_root_locations("camp-hier")
        .await
        .expect("Failed to list roots");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].name, "Waterdeep");
}

// =============================================================================
// Usage Tracking Tests
// =============================================================================

#[tokio::test]
async fn test_usage_tracking() {
    let (db, _temp) = create_test_db().await;

    let usage = UsageRecord::new(
        "claude".to_string(),
        "claude-3-sonnet".to_string(),
        1000,
        500,
    );

    db.record_usage(&usage)
        .await
        .expect("Failed to record usage");

    let stats = db.get_total_usage().await.expect("Failed to get total usage");
    assert_eq!(stats.total_input_tokens, 1000);
    assert_eq!(stats.total_output_tokens, 500);
    assert_eq!(stats.total_requests, 1);

    let by_provider = db
        .get_usage_by_provider()
        .await
        .expect("Failed to get usage by provider");
    assert_eq!(by_provider.len(), 1);
    assert_eq!(by_provider[0].provider, "claude");
}

// =============================================================================
// Settings Tests
// =============================================================================

#[tokio::test]
async fn test_settings() {
    let (db, _temp) = create_test_db().await;

    db.set_setting("theme", "dark")
        .await
        .expect("Failed to set setting");

    let value = db
        .get_setting("theme")
        .await
        .expect("Failed to get setting")
        .expect("Setting not found");

    assert_eq!(value, "dark");

    db.delete_setting("theme")
        .await
        .expect("Failed to delete setting");

    let deleted = db.get_setting("theme").await.expect("Query should succeed");
    assert!(deleted.is_none(), "Setting should be deleted");
}

// =============================================================================
// NPC Conversation Tests
// =============================================================================

#[tokio::test]
async fn test_npc_conversations() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-conv".to_string(),
        "Conversation Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let npc = NpcRecord {
        id: "npc-conv".to_string(),
        campaign_id: Some("camp-conv".to_string()),
        name: "Wise Wizard".to_string(),
        role: "Quest Giver".to_string(),
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
    db.save_npc(&npc).await.expect("Failed to save NPC");

    let conversation = NpcConversation::new(
        "conv-001".to_string(),
        "npc-conv".to_string(),
        "camp-conv".to_string(),
    );

    db.save_npc_conversation(&conversation)
        .await
        .expect("Failed to save conversation");

    let retrieved = db
        .get_npc_conversation("npc-conv")
        .await
        .expect("Failed to get conversation")
        .expect("Conversation not found");

    assert_eq!(retrieved.npc_id, "npc-conv");

    let conversations = db
        .list_npc_conversations("camp-conv")
        .await
        .expect("Failed to list conversations");
    assert_eq!(conversations.len(), 1);
}

// =============================================================================
// Personality Tests
// =============================================================================

#[tokio::test]
async fn test_personalities() {
    let (db, _temp) = create_test_db().await;

    let personality = PersonalityRecord {
        id: "pers-001".to_string(),
        name: "Grumpy Old Man".to_string(),
        source: Some("custom".to_string()),
        data_json: r#"{"traits":["grumpy","wise","impatient"]}"#.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    db.save_personality(&personality)
        .await
        .expect("Failed to save personality");

    let retrieved = db
        .get_personality("pers-001")
        .await
        .expect("Failed to get personality")
        .expect("Personality not found");

    assert_eq!(retrieved.name, "Grumpy Old Man");

    let personalities = db
        .list_personalities()
        .await
        .expect("Failed to list personalities");
    assert_eq!(personalities.len(), 1);
}

// =============================================================================
// Migration Tests
// =============================================================================

#[tokio::test]
async fn test_migrations_idempotent() {
    let (db, _temp) = create_test_db().await;

    // Run migrations again - should be idempotent
    run_migrations(db.pool())
        .await
        .expect("Migrations should be idempotent");

    // Database should still work
    let campaigns = db.list_campaigns().await.expect("Database should work after re-running migrations");
    assert_eq!(campaigns.len(), 0);
}

// =============================================================================
// Cascade Delete Tests
// =============================================================================

#[tokio::test]
async fn test_campaign_delete_cascades() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-cascade".to_string(),
        "Cascade Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-cascade".to_string(),
        "camp-cascade".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

    // Delete campaign
    db.delete_campaign("camp-cascade")
        .await
        .expect("Failed to delete campaign");

    // Session should also be deleted
    let sessions = db
        .list_sessions("camp-cascade")
        .await
        .expect("Query should succeed");
    assert_eq!(sessions.len(), 0, "Sessions should be deleted with campaign");
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[tokio::test]
async fn test_get_nonexistent_campaign() {
    let (db, _temp) = create_test_db().await;

    let result = db
        .get_campaign("nonexistent")
        .await
        .expect("Query should not fail");
    assert!(result.is_none(), "Should return None for nonexistent campaign");
}

#[tokio::test]
async fn test_empty_list() {
    let (db, _temp) = create_test_db().await;

    let campaigns = db.list_campaigns().await.expect("List should succeed");
    assert!(campaigns.is_empty(), "Should return empty list");
}
