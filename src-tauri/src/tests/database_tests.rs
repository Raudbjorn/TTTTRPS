//! Database Unit Tests
//!
//! Comprehensive tests for SQLite database operations.
//! Uses an in-memory database for fast, isolated testing.

use crate::database::{
    run_migrations, CampaignOps, CampaignRecord, CampaignVersionRecord, CharacterOps,
    CharacterRecord, CombatOps, CombatStateRecord, Database, DocumentOps, DocumentRecord,
    EntityRelationshipRecord, EntityType, LocationOps, LocationRecord, NpcConversation,
    NpcOps, NpcRecord, PersonalityRecord, RelationshipOps, SearchAnalyticsOps,
    SearchAnalyticsRecord, SessionEventRecord, SessionNoteRecord, SessionOps, SessionRecord,
    SettingsOps, UsageOps, UsageRecord, VoiceProfileOps, VoiceProfileRecord,
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

// =============================================================================
// Extended CRUD Tests - Campaigns
// =============================================================================

#[tokio::test]
async fn test_campaign_full_lifecycle() {
    let (db, _temp) = create_test_db().await;

    // Create
    let mut campaign = CampaignRecord::new(
        "camp-lifecycle".to_string(),
        "Lifecycle Test".to_string(),
        "D&D 5e".to_string(),
    );
    campaign.description = Some("A test campaign".to_string());
    campaign.setting = Some("Forgotten Realms".to_string());
    campaign.house_rules = Some(r#"{"max_hp": true}"#.to_string());
    campaign.world_state = Some(r#"{"season": "winter"}"#.to_string());

    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Read
    let retrieved = db.get_campaign("camp-lifecycle").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.description, Some("A test campaign".to_string()));
    assert_eq!(retrieved.setting, Some("Forgotten Realms".to_string()));

    // Update multiple fields
    let mut updated = retrieved.clone();
    updated.name = "Updated Lifecycle".to_string();
    updated.description = Some("Updated description".to_string());
    updated.current_in_game_date = Some("Year 1492".to_string());
    updated.updated_at = chrono::Utc::now().to_rfc3339();

    db.update_campaign(&updated).await.expect("Failed to update");

    let after_update = db.get_campaign("camp-lifecycle").await.expect("Failed to get").expect("Not found");
    assert_eq!(after_update.name, "Updated Lifecycle");
    assert_eq!(after_update.current_in_game_date, Some("Year 1492".to_string()));

    // Archive (soft delete pattern)
    let mut archived = after_update.clone();
    archived.archived_at = Some(chrono::Utc::now().to_rfc3339());
    archived.updated_at = chrono::Utc::now().to_rfc3339();
    db.update_campaign(&archived).await.expect("Failed to archive");

    let after_archive = db.get_campaign("camp-lifecycle").await.expect("Failed to get").expect("Not found");
    assert!(after_archive.archived_at.is_some());

    // Delete
    db.delete_campaign("camp-lifecycle").await.expect("Failed to delete");
    let after_delete = db.get_campaign("camp-lifecycle").await.expect("Query should succeed");
    assert!(after_delete.is_none());
}

#[tokio::test]
async fn test_campaign_with_special_characters() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-special".to_string(),
        "Dragon's \"Lair\" & More".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create");

    let retrieved = db.get_campaign("camp-special").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.name, "Dragon's \"Lair\" & More");
}

#[tokio::test]
async fn test_campaign_with_unicode() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-unicode".to_string(),
        "冒険の世界".to_string(),
        "システム".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create");

    let retrieved = db.get_campaign("camp-unicode").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.name, "冒険の世界");
    assert_eq!(retrieved.system, "システム");
}

#[tokio::test]
async fn test_campaign_with_large_json() {
    let (db, _temp) = create_test_db().await;

    // Create large JSON data
    let large_world_state: String = (0..1000)
        .map(|i| format!(r#""key_{i}": "value_{i}""#))
        .collect::<Vec<_>>()
        .join(", ");
    let large_json = format!("{{{}}}", large_world_state);

    let mut campaign = CampaignRecord::new(
        "camp-large".to_string(),
        "Large Data Test".to_string(),
        "D&D 5e".to_string(),
    );
    campaign.world_state = Some(large_json.clone());

    db.create_campaign(&campaign).await.expect("Failed to create");

    let retrieved = db.get_campaign("camp-large").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.world_state, Some(large_json));
}

// =============================================================================
// Extended CRUD Tests - Sessions
// =============================================================================

#[tokio::test]
async fn test_session_full_lifecycle() {
    let (db, _temp) = create_test_db().await;

    // Create parent campaign
    let campaign = CampaignRecord::new(
        "camp-sess-life".to_string(),
        "Session Lifecycle".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Create session
    let session = SessionRecord::new(
        "sess-lifecycle".to_string(),
        "camp-sess-life".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

    // Verify initial state
    let retrieved = db.get_session("sess-lifecycle").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.status, "active");
    assert!(retrieved.ended_at.is_none());

    // Update - complete the session
    let mut updated = retrieved.clone();
    updated.status = "completed".to_string();
    updated.ended_at = Some(chrono::Utc::now().to_rfc3339());
    updated.notes = Some("Great session! Defeated the dragon.".to_string());
    db.update_session(&updated).await.expect("Failed to update");

    let after_update = db.get_session("sess-lifecycle").await.expect("Failed to get").expect("Not found");
    assert_eq!(after_update.status, "completed");
    assert!(after_update.ended_at.is_some());
    assert!(after_update.notes.unwrap().contains("dragon"));
}

#[tokio::test]
async fn test_session_multiple_per_campaign() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-multi".to_string(),
        "Multi Session".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Create 10 sessions
    for i in 1..=10 {
        let session = SessionRecord::new(
            format!("sess-multi-{:02}", i),
            "camp-multi".to_string(),
            i,
        );
        db.create_session(&session).await.expect("Failed to create session");
    }

    let sessions = db.list_sessions("camp-multi").await.expect("Failed to list");
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
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Create completed session
    let mut session = SessionRecord::new(
        "sess-completed".to_string(),
        "camp-no-active".to_string(),
        1,
    );
    session.status = "completed".to_string();
    db.create_session(&session).await.expect("Failed to create session");

    let active = db.get_active_session("camp-no-active").await.expect("Failed to get");
    assert!(active.is_none());
}

// =============================================================================
// Extended CRUD Tests - Characters
// =============================================================================

#[tokio::test]
async fn test_character_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-char-update".to_string(),
        "Character Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let character = CharacterRecord {
        id: "char-update".to_string(),
        campaign_id: Some("camp-char-update".to_string()),
        name: "Gandalf".to_string(),
        system: "D&D 5e".to_string(),
        character_type: "player".to_string(),
        level: Some(1),
        data_json: r#"{"class":"Wizard"}"#.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_character(&character).await.expect("Failed to save");

    // Level up
    let mut updated = character.clone();
    updated.level = Some(20);
    updated.data_json = r#"{"class":"Wizard","subclass":"Divination"}"#.to_string();
    updated.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_character(&updated).await.expect("Failed to update");

    let retrieved = db.get_character("char-update").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.level, Some(20));
    assert!(retrieved.data_json.contains("Divination"));
}

#[tokio::test]
async fn test_character_delete() {
    let (db, _temp) = create_test_db().await;

    let character = CharacterRecord {
        id: "char-delete".to_string(),
        campaign_id: None,
        name: "Deletable".to_string(),
        system: "D&D 5e".to_string(),
        character_type: "player".to_string(),
        level: None,
        data_json: "{}".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_character(&character).await.expect("Failed to save");

    db.delete_character("char-delete").await.expect("Failed to delete");

    let retrieved = db.get_character("char-delete").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_character_list_all() {
    let (db, _temp) = create_test_db().await;

    // Create characters without campaign
    for i in 1..=3 {
        let character = CharacterRecord {
            id: format!("char-global-{}", i),
            campaign_id: None,
            name: format!("Global Character {}", i),
            system: "D&D 5e".to_string(),
            character_type: "player".to_string(),
            level: None,
            data_json: "{}".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        db.save_character(&character).await.expect("Failed to save");
    }

    // List all characters (no campaign filter)
    let all_chars = db.list_characters(None).await.expect("Failed to list");
    assert_eq!(all_chars.len(), 3);
}

// =============================================================================
// Extended CRUD Tests - NPCs
// =============================================================================

#[tokio::test]
async fn test_npc_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-npc-update".to_string(),
        "NPC Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let npc = NpcRecord {
        id: "npc-update".to_string(),
        campaign_id: Some("camp-npc-update".to_string()),
        name: "Bob the Builder".to_string(),
        role: "Carpenter".to_string(),
        personality_id: None,
        personality_json: r#"{"traits":["hardworking"]}"#.to_string(),
        data_json: None,
        stats_json: None,
        notes: None,
        location_id: None,
        voice_profile_id: None,
        quest_hooks: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_npc(&npc).await.expect("Failed to save");

    // Update NPC
    let mut updated = npc.clone();
    updated.role = "Master Builder".to_string();
    updated.notes = Some("Promoted after completing the castle".to_string());
    updated.stats_json = Some(r#"{"strength":16}"#.to_string());
    db.save_npc(&updated).await.expect("Failed to update");

    let retrieved = db.get_npc("npc-update").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.role, "Master Builder");
    assert!(retrieved.notes.unwrap().contains("castle"));
}

#[tokio::test]
async fn test_npc_delete() {
    let (db, _temp) = create_test_db().await;

    let npc = NpcRecord {
        id: "npc-delete".to_string(),
        campaign_id: None,
        name: "Temporary NPC".to_string(),
        role: "Extra".to_string(),
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
    db.save_npc(&npc).await.expect("Failed to save");

    db.delete_npc("npc-delete").await.expect("Failed to delete");

    let retrieved = db.get_npc("npc-delete").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_npc_list_by_campaign() {
    let (db, _temp) = create_test_db().await;

    // Create two campaigns
    for camp_id in ["camp-a", "camp-b"] {
        let campaign = CampaignRecord::new(
            camp_id.to_string(),
            format!("Campaign {}", camp_id),
            "D&D 5e".to_string(),
        );
        db.create_campaign(&campaign).await.expect("Failed to create campaign");
    }

    // Create NPCs for each campaign
    for i in 1..=3 {
        for camp_id in ["camp-a", "camp-b"] {
            let npc = NpcRecord {
                id: format!("npc-{}-{}", camp_id, i),
                campaign_id: Some(camp_id.to_string()),
                name: format!("NPC {} of {}", i, camp_id),
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
    }

    // List NPCs by campaign
    let npcs_a = db.list_npcs(Some("camp-a")).await.expect("Failed to list");
    assert_eq!(npcs_a.len(), 3);

    let npcs_b = db.list_npcs(Some("camp-b")).await.expect("Failed to list");
    assert_eq!(npcs_b.len(), 3);

    // List all NPCs
    let all_npcs = db.list_npcs(None).await.expect("Failed to list");
    assert_eq!(all_npcs.len(), 6);
}

// =============================================================================
// Extended CRUD Tests - NPC Conversations
// =============================================================================

#[tokio::test]
async fn test_npc_conversation_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-conv-update".to_string(),
        "Conversation Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let npc = NpcRecord {
        id: "npc-conv-update".to_string(),
        campaign_id: Some("camp-conv-update".to_string()),
        name: "Chatty NPC".to_string(),
        role: "Guide".to_string(),
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
        "conv-update".to_string(),
        "npc-conv-update".to_string(),
        "camp-conv-update".to_string(),
    );
    db.save_npc_conversation(&conversation).await.expect("Failed to save");

    // Update conversation with messages
    let mut updated = conversation.clone();
    updated.messages_json = r#"[{"id":"msg1","role":"user","content":"Hello!","created_at":"2024-01-01"}]"#.to_string();
    updated.unread_count = 1;
    updated.last_message_at = chrono::Utc::now().to_rfc3339();
    updated.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_npc_conversation(&updated).await.expect("Failed to update");

    let retrieved = db.get_npc_conversation("npc-conv-update").await.expect("Failed to get").expect("Not found");
    assert!(retrieved.messages_json.contains("Hello!"));
    assert_eq!(retrieved.unread_count, 1);
}

// =============================================================================
// Extended CRUD Tests - Documents
// =============================================================================

#[tokio::test]
async fn test_document_lifecycle() {
    let (db, _temp) = create_test_db().await;

    let doc = DocumentRecord {
        id: "doc-001".to_string(),
        name: "Player's Handbook".to_string(),
        source_type: "pdf".to_string(),
        file_path: Some("/path/to/phb.pdf".to_string()),
        page_count: 320,
        chunk_count: 0,
        status: "pending".to_string(),
        ingested_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_document(&doc).await.expect("Failed to save");

    let docs = db.list_documents().await.expect("Failed to list");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].name, "Player's Handbook");

    // Update status (via save)
    let mut updated = doc.clone();
    updated.status = "ready".to_string();
    updated.chunk_count = 150;
    db.save_document(&updated).await.expect("Failed to update");

    let docs_after = db.list_documents().await.expect("Failed to list");
    assert_eq!(docs_after[0].status, "ready");
    assert_eq!(docs_after[0].chunk_count, 150);

    // Delete
    db.delete_document("doc-001").await.expect("Failed to delete");
    let docs_final = db.list_documents().await.expect("Failed to list");
    assert!(docs_final.is_empty());
}

// =============================================================================
// Extended CRUD Tests - Personalities
// =============================================================================

#[tokio::test]
async fn test_personality_delete() {
    let (db, _temp) = create_test_db().await;

    let personality = PersonalityRecord {
        id: "pers-delete".to_string(),
        name: "Temporary".to_string(),
        source: None,
        data_json: "{}".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    db.save_personality(&personality).await.expect("Failed to save");

    db.delete_personality("pers-delete").await.expect("Failed to delete");

    let retrieved = db.get_personality("pers-delete").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

// =============================================================================
// Extended CRUD Tests - Voice Profiles
// =============================================================================

#[tokio::test]
async fn test_voice_profile_update() {
    let (db, _temp) = create_test_db().await;

    let mut profile = VoiceProfileRecord::new(
        "voice-update".to_string(),
        "Deep Voice".to_string(),
        "elevenlabs".to_string(),
        "voice123".to_string(),
    );
    db.save_voice_profile(&profile).await.expect("Failed to save");

    // Update
    profile.name = "Very Deep Voice".to_string();
    profile.age_range = Some("elderly".to_string());
    profile.gender = Some("male".to_string());
    profile.settings = Some(r#"{"pitch":-2}"#.to_string());
    profile.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_voice_profile(&profile).await.expect("Failed to update");

    let retrieved = db.get_voice_profile("voice-update").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.name, "Very Deep Voice");
    assert_eq!(retrieved.age_range, Some("elderly".to_string()));
}

#[tokio::test]
async fn test_voice_profile_delete() {
    let (db, _temp) = create_test_db().await;

    let profile = VoiceProfileRecord::new(
        "voice-delete".to_string(),
        "Temp Voice".to_string(),
        "openai".to_string(),
        "temp123".to_string(),
    );
    db.save_voice_profile(&profile).await.expect("Failed to save");

    db.delete_voice_profile("voice-delete").await.expect("Failed to delete");

    let retrieved = db.get_voice_profile("voice-delete").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_voice_profile_presets() {
    let (db, _temp) = create_test_db().await;

    // Create preset and custom profiles
    let mut preset = VoiceProfileRecord::new(
        "voice-preset".to_string(),
        "Preset Voice".to_string(),
        "elevenlabs".to_string(),
        "preset123".to_string(),
    );
    preset.is_preset = true;
    db.save_voice_profile(&preset).await.expect("Failed to save");

    let custom = VoiceProfileRecord::new(
        "voice-custom".to_string(),
        "Custom Voice".to_string(),
        "elevenlabs".to_string(),
        "custom123".to_string(),
    );
    db.save_voice_profile(&custom).await.expect("Failed to save");

    // List all
    let all = db.list_voice_profiles().await.expect("Failed to list");
    assert_eq!(all.len(), 2);

    // List presets only
    let presets = db.list_voice_profile_presets().await.expect("Failed to list");
    assert_eq!(presets.len(), 1);
    assert_eq!(presets[0].name, "Preset Voice");
}

// =============================================================================
// Extended CRUD Tests - Session Notes
// =============================================================================

#[tokio::test]
async fn test_session_note_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-note-update".to_string(),
        "Note Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-note-update".to_string(),
        "camp-note-update".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

    let mut note = SessionNoteRecord::new(
        "note-update".to_string(),
        "sess-note-update".to_string(),
        "camp-note-update".to_string(),
        "Initial note".to_string(),
    );
    db.save_session_note(&note).await.expect("Failed to save");

    // Update
    note.content = "Updated note with more details".to_string();
    note.tags = Some(r#"["important","combat"]"#.to_string());
    note.entity_links = Some(r#"[{"type":"npc","id":"npc-001"}]"#.to_string());
    note.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_session_note(&note).await.expect("Failed to update");

    let retrieved = db.get_session_note("note-update").await.expect("Failed to get").expect("Not found");
    assert!(retrieved.content.contains("more details"));
    assert!(retrieved.tags.unwrap().contains("important"));
}

#[tokio::test]
async fn test_session_note_delete() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-note-del".to_string(),
        "Note Delete".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-note-del".to_string(),
        "camp-note-del".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

    let note = SessionNoteRecord::new(
        "note-delete".to_string(),
        "sess-note-del".to_string(),
        "camp-note-del".to_string(),
        "To be deleted".to_string(),
    );
    db.save_session_note(&note).await.expect("Failed to save");

    db.delete_session_note("note-delete").await.expect("Failed to delete");

    let retrieved = db.get_session_note("note-delete").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_list_campaign_notes() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-notes-list".to_string(),
        "Notes List".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Create two sessions
    for i in 1..=2 {
        let session = SessionRecord::new(
            format!("sess-notes-{}", i),
            "camp-notes-list".to_string(),
            i,
        );
        db.create_session(&session).await.expect("Failed to create session");

        // Create notes for each session
        for j in 1..=2 {
            let note = SessionNoteRecord::new(
                format!("note-{}-{}", i, j),
                format!("sess-notes-{}", i),
                "camp-notes-list".to_string(),
                format!("Note {} for session {}", j, i),
            );
            db.save_session_note(&note).await.expect("Failed to save");
        }
    }

    // List all campaign notes
    let all_notes = db.list_campaign_notes("camp-notes-list").await.expect("Failed to list");
    assert_eq!(all_notes.len(), 4);

    // List notes for single session
    let session_notes = db.list_session_notes("sess-notes-1").await.expect("Failed to list");
    assert_eq!(session_notes.len(), 2);
}

// =============================================================================
// Extended CRUD Tests - Session Events
// =============================================================================

#[tokio::test]
async fn test_session_event_delete() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-evt-del".to_string(),
        "Event Delete".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-evt-del".to_string(),
        "camp-evt-del".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

    let event = SessionEventRecord::new(
        "evt-delete".to_string(),
        "sess-evt-del".to_string(),
        "test_event".to_string(),
    );
    db.save_session_event(&event).await.expect("Failed to save");

    db.delete_session_event("evt-delete").await.expect("Failed to delete");

    let retrieved = db.get_session_event("evt-delete").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_session_events_multiple_types() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-evt-types".to_string(),
        "Event Types".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-evt-types".to_string(),
        "camp-evt-types".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

    // Create events of different types
    let event_types = ["combat_start", "combat_end", "npc_interaction", "location_change"];
    for (i, event_type) in event_types.iter().enumerate() {
        let event = SessionEventRecord::new(
            format!("evt-type-{}", i),
            "sess-evt-types".to_string(),
            event_type.to_string(),
        );
        db.save_session_event(&event).await.expect("Failed to save");
    }

    // List all events
    let all_events = db.list_session_events("sess-evt-types").await.expect("Failed to list");
    assert_eq!(all_events.len(), 4);

    // Filter by type
    let combat_starts = db.list_session_events_by_type("sess-evt-types", "combat_start").await.expect("Failed to list");
    assert_eq!(combat_starts.len(), 1);

    let location_changes = db.list_session_events_by_type("sess-evt-types", "location_change").await.expect("Failed to list");
    assert_eq!(location_changes.len(), 1);
}

// =============================================================================
// Extended CRUD Tests - Combat States
// =============================================================================

#[tokio::test]
async fn test_combat_state_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-combat-update".to_string(),
        "Combat Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-combat-update".to_string(),
        "camp-combat-update".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

    let mut combat = CombatStateRecord::new(
        "combat-update".to_string(),
        "sess-combat-update".to_string(),
        r#"[{"name":"Goblin","hp":10}]"#.to_string(),
    );
    db.save_combat_state(&combat).await.expect("Failed to save");

    // Update combat state
    combat.round = 5;
    combat.current_turn = 2;
    combat.combatants = r#"[{"name":"Goblin","hp":3},{"name":"Fighter","hp":20}]"#.to_string();
    combat.notes = Some("Intense battle!".to_string());
    combat.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_combat_state(&combat).await.expect("Failed to update");

    let retrieved = db.get_combat_state("combat-update").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.round, 5);
    assert_eq!(retrieved.current_turn, 2);
    assert!(retrieved.notes.unwrap().contains("Intense"));
}

#[tokio::test]
async fn test_combat_state_delete() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-combat-del".to_string(),
        "Combat Delete".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-combat-del".to_string(),
        "camp-combat-del".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

    let combat = CombatStateRecord::new(
        "combat-delete".to_string(),
        "sess-combat-del".to_string(),
        "[]".to_string(),
    );
    db.save_combat_state(&combat).await.expect("Failed to save");

    db.delete_combat_state("combat-delete").await.expect("Failed to delete");

    let retrieved = db.get_combat_state("combat-delete").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_list_session_combats() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-combats-list".to_string(),
        "Combats List".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-combats-list".to_string(),
        "camp-combats-list".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");

    // Create multiple combats
    for i in 1..=3 {
        let combat = CombatStateRecord::new(
            format!("combat-list-{}", i),
            "sess-combats-list".to_string(),
            format!(r#"[{{"encounter":{}}}]"#, i),
        );
        db.save_combat_state(&combat).await.expect("Failed to save");
    }

    let combats = db.list_session_combats("sess-combats-list").await.expect("Failed to list");
    assert_eq!(combats.len(), 3);
}

// =============================================================================
// Extended CRUD Tests - Locations
// =============================================================================

#[tokio::test]
async fn test_location_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-loc-update".to_string(),
        "Location Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let mut location = LocationRecord::new(
        "loc-update".to_string(),
        "camp-loc-update".to_string(),
        "Old Inn".to_string(),
        "tavern".to_string(),
    );
    db.save_location(&location).await.expect("Failed to save");

    // Update location
    location.name = "The Rusty Nail Inn".to_string();
    location.description = Some("A popular tavern on the crossroads".to_string());
    location.npcs_present_json = r#"["npc-bartender","npc-bard"]"#.to_string();
    location.features_json = r#"["fireplace","stage","rooms_upstairs"]"#.to_string();
    location.secrets_json = r#"["trapdoor_behind_bar"]"#.to_string();
    location.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_location(&location).await.expect("Failed to update");

    let retrieved = db.get_location("loc-update").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.name, "The Rusty Nail Inn");
    assert!(retrieved.npcs_present_json.contains("bartender"));
}

#[tokio::test]
async fn test_location_delete() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-loc-del".to_string(),
        "Location Delete".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let location = LocationRecord::new(
        "loc-delete".to_string(),
        "camp-loc-del".to_string(),
        "Temp Location".to_string(),
        "misc".to_string(),
    );
    db.save_location(&location).await.expect("Failed to save");

    db.delete_location("loc-delete").await.expect("Failed to delete");

    let retrieved = db.get_location("loc-delete").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_location_delete_unlinks_children() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-loc-cascade".to_string(),
        "Location Cascade".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Create parent location
    let parent = LocationRecord::new(
        "loc-parent".to_string(),
        "camp-loc-cascade".to_string(),
        "City".to_string(),
        "city".to_string(),
    );
    db.save_location(&parent).await.expect("Failed to save");

    // Create child location
    let mut child = LocationRecord::new(
        "loc-child".to_string(),
        "camp-loc-cascade".to_string(),
        "Tavern".to_string(),
        "tavern".to_string(),
    );
    child.parent_id = Some("loc-parent".to_string());
    db.save_location(&child).await.expect("Failed to save");

    // Delete parent
    db.delete_location("loc-parent").await.expect("Failed to delete");

    // Child should still exist but with no parent
    let child_after = db.get_location("loc-child").await.expect("Failed to get").expect("Not found");
    assert!(child_after.parent_id.is_none());
}

// =============================================================================
// Extended CRUD Tests - Entity Relationships
// =============================================================================

#[tokio::test]
async fn test_entity_relationship_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-rel-update".to_string(),
        "Relationship Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let mut relationship = EntityRelationshipRecord::new(
        "rel-update".to_string(),
        "camp-rel-update".to_string(),
        EntityType::Npc,
        "npc-a".to_string(),
        EntityType::Npc,
        "npc-b".to_string(),
        "acquaintance".to_string(),
    );
    db.save_entity_relationship(&relationship).await.expect("Failed to save");

    // Update relationship
    relationship.relationship_type = "friend".to_string();
    relationship.strength = 0.8;
    relationship.description = Some("Became friends after the adventure".to_string());
    relationship.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_entity_relationship(&relationship).await.expect("Failed to update");

    let retrieved = db.get_entity_relationship("rel-update").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.relationship_type, "friend");
    assert_eq!(retrieved.strength, 0.8);
}

#[tokio::test]
async fn test_entity_relationship_delete() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-rel-del".to_string(),
        "Relationship Delete".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let relationship = EntityRelationshipRecord::new(
        "rel-delete".to_string(),
        "camp-rel-del".to_string(),
        EntityType::Npc,
        "npc-x".to_string(),
        EntityType::Character,
        "char-y".to_string(),
        "enemy".to_string(),
    );
    db.save_entity_relationship(&relationship).await.expect("Failed to save");

    db.delete_entity_relationship("rel-delete").await.expect("Failed to delete");

    let retrieved = db.get_entity_relationship("rel-delete").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_delete_relationships_for_entity() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-rel-entity-del".to_string(),
        "Entity Relationships".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Create multiple relationships for one entity
    let target_entity = "npc-target";
    for i in 1..=5 {
        let relationship = EntityRelationshipRecord::new(
            format!("rel-entity-{}", i),
            "camp-rel-entity-del".to_string(),
            EntityType::Npc,
            target_entity.to_string(),
            EntityType::Npc,
            format!("npc-other-{}", i),
            "ally".to_string(),
        );
        db.save_entity_relationship(&relationship).await.expect("Failed to save");
    }

    // Delete all relationships for the entity
    let deleted_count = db.delete_relationships_for_entity("npc", target_entity).await.expect("Failed to delete");
    assert_eq!(deleted_count, 5);

    let remaining = db.list_relationships_for_entity("npc", target_entity).await.expect("Failed to list");
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn test_list_relationships_by_type() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-rel-type".to_string(),
        "Relationship Types".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    // Create relationships of different types
    let types = ["ally", "enemy", "ally", "family", "ally"];
    for (i, rel_type) in types.iter().enumerate() {
        let relationship = EntityRelationshipRecord::new(
            format!("rel-type-{}", i),
            "camp-rel-type".to_string(),
            EntityType::Npc,
            format!("npc-{}", i),
            EntityType::Npc,
            format!("npc-other-{}", i),
            rel_type.to_string(),
        );
        db.save_entity_relationship(&relationship).await.expect("Failed to save");
    }

    let allies = db.list_relationships_by_type("camp-rel-type", "ally").await.expect("Failed to list");
    assert_eq!(allies.len(), 3);

    let enemies = db.list_relationships_by_type("camp-rel-type", "enemy").await.expect("Failed to list");
    assert_eq!(enemies.len(), 1);
}

#[tokio::test]
async fn test_list_campaign_relationships() {
    let (db, _temp) = create_test_db().await;

    // Create two campaigns with relationships
    for camp_id in ["camp-a", "camp-b"] {
        let campaign = CampaignRecord::new(
            camp_id.to_string(),
            format!("Campaign {}", camp_id),
            "D&D 5e".to_string(),
        );
        db.create_campaign(&campaign).await.expect("Failed to create campaign");

        for i in 1..=3 {
            let relationship = EntityRelationshipRecord::new(
                format!("rel-{}-{}", camp_id, i),
                camp_id.to_string(),
                EntityType::Npc,
                format!("npc-{}-a", camp_id),
                EntityType::Npc,
                format!("npc-{}-b-{}", camp_id, i),
                "ally".to_string(),
            );
            db.save_entity_relationship(&relationship).await.expect("Failed to save");
        }
    }

    let camp_a_rels = db.list_campaign_relationships("camp-a").await.expect("Failed to list");
    assert_eq!(camp_a_rels.len(), 3);

    let camp_b_rels = db.list_campaign_relationships("camp-b").await.expect("Failed to list");
    assert_eq!(camp_b_rels.len(), 3);
}

// =============================================================================
// Extended CRUD Tests - Campaign Versions
// =============================================================================

#[tokio::test]
async fn test_campaign_version_delete() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-ver-del".to_string(),
        "Version Delete".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let version = CampaignVersionRecord::new(
        "ver-delete".to_string(),
        "camp-ver-del".to_string(),
        1,
        "manual".to_string(),
        "{}".to_string(),
    );
    db.save_campaign_version(&version).await.expect("Failed to save");

    db.delete_campaign_version("ver-delete").await.expect("Failed to delete");

    let retrieved = db.get_campaign_version("ver-delete").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_campaign_version_with_diff() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-ver-diff".to_string(),
        "Version Diff".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let mut version = CampaignVersionRecord::new(
        "ver-with-diff".to_string(),
        "camp-ver-diff".to_string(),
        2,
        "auto_save".to_string(),
        r#"{"name":"Updated Campaign"}"#.to_string(),
    );
    version.description = Some("Added new NPC".to_string());
    version.diff_data = Some(r#"{"added":["npc-001"]}"#.to_string());

    db.save_campaign_version(&version).await.expect("Failed to save");

    let retrieved = db.get_campaign_version("ver-with-diff").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.description, Some("Added new NPC".to_string()));
    assert!(retrieved.diff_data.unwrap().contains("npc-001"));
}

// =============================================================================
// Usage Tracking Extended Tests
// =============================================================================

#[tokio::test]
async fn test_usage_multiple_providers() {
    let (db, _temp) = create_test_db().await;

    // Record usage for multiple providers
    let providers = vec![
        ("claude", "claude-3-sonnet", 1000, 500),
        ("claude", "claude-3-haiku", 2000, 1000),
        ("openai", "gpt-4", 500, 250),
        ("gemini", "gemini-pro", 1500, 750),
    ];

    for (provider, model, input, output) in providers {
        let usage = UsageRecord::new(
            provider.to_string(),
            model.to_string(),
            input,
            output,
        );
        db.record_usage(&usage).await.expect("Failed to record usage");
    }

    let by_provider = db.get_usage_by_provider().await.expect("Failed to get by provider");
    assert_eq!(by_provider.len(), 3); // claude, openai, gemini

    // Find Claude stats
    let claude_stats = by_provider.iter().find(|s| s.provider == "claude").expect("Claude not found");
    assert_eq!(claude_stats.requests, 2);
    assert_eq!(claude_stats.input_tokens, 3000); // 1000 + 2000
}

#[tokio::test]
async fn test_reset_usage_stats() {
    let (db, _temp) = create_test_db().await;

    let usage = UsageRecord::new(
        "test".to_string(),
        "test-model".to_string(),
        1000,
        500,
    );
    db.record_usage(&usage).await.expect("Failed to record usage");

    let stats_before = db.get_total_usage().await.expect("Failed to get");
    assert_eq!(stats_before.total_requests, 1);

    db.reset_usage_stats().await.expect("Failed to reset");

    let stats_after = db.get_total_usage().await.expect("Failed to get");
    assert_eq!(stats_after.total_requests, 0);
    assert_eq!(stats_after.total_input_tokens, 0);
}

// =============================================================================
// Transaction and Concurrent Operation Tests
// =============================================================================

#[tokio::test]
async fn test_concurrent_reads() {
    let (db, _temp) = create_test_db().await;

    // Setup data
    for i in 1..=10 {
        let campaign = CampaignRecord::new(
            format!("camp-concurrent-{:02}", i),
            format!("Concurrent Test {}", i),
            "D&D 5e".to_string(),
        );
        db.create_campaign(&campaign).await.expect("Failed to create campaign");
    }

    // Spawn multiple concurrent reads
    let db_clone = db.clone();
    let handles: Vec<_> = (1..=5)
        .map(|i| {
            let db = db_clone.clone();
            tokio::spawn(async move {
                for _ in 0..10 {
                    let campaigns = db.list_campaigns().await.expect("Failed to list");
                    assert_eq!(campaigns.len(), 10);
                    let camp = db.get_campaign(&format!("camp-concurrent-{:02}", i)).await
                        .expect("Failed to get");
                    assert!(camp.is_some());
                }
            })
        })
        .collect();

    for handle in handles {
        handle.await.expect("Task panicked");
    }
}

#[tokio::test]
async fn test_concurrent_writes_different_records() {
    let (db, _temp) = create_test_db().await;

    // Spawn multiple concurrent writes to different records
    let db_clone = db.clone();
    let handles: Vec<_> = (1..=10)
        .map(|i| {
            let db = db_clone.clone();
            tokio::spawn(async move {
                let campaign = CampaignRecord::new(
                    format!("camp-write-{:02}", i),
                    format!("Write Test {}", i),
                    "D&D 5e".to_string(),
                );
                db.create_campaign(&campaign).await.expect("Failed to create");

                // Update the campaign
                let mut updated = campaign.clone();
                updated.description = Some(format!("Updated by task {}", i));
                updated.updated_at = chrono::Utc::now().to_rfc3339();
                db.update_campaign(&updated).await.expect("Failed to update");
            })
        })
        .collect();

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // Verify all records exist and were updated
    let campaigns = db.list_campaigns().await.expect("Failed to list");
    assert_eq!(campaigns.len(), 10);

    for camp in campaigns {
        assert!(camp.description.is_some());
    }
}

#[tokio::test]
async fn test_concurrent_read_write() {
    let (db, _temp) = create_test_db().await;

    // Create initial data
    let campaign = CampaignRecord::new(
        "camp-rw".to_string(),
        "Read Write Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create");

    let db_clone = db.clone();

    // Writer task - continuously updates the campaign
    let writer = {
        let db = db_clone.clone();
        tokio::spawn(async move {
            for i in 0..20 {
                let mut camp = db.get_campaign("camp-rw").await
                    .expect("Failed to get")
                    .expect("Not found");
                camp.description = Some(format!("Update {}", i));
                camp.updated_at = chrono::Utc::now().to_rfc3339();
                db.update_campaign(&camp).await.expect("Failed to update");
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
    };

    // Reader tasks - continuously read the campaign
    let readers: Vec<_> = (0..5)
        .map(|_| {
            let db = db_clone.clone();
            tokio::spawn(async move {
                for _ in 0..50 {
                    let camp = db.get_campaign("camp-rw").await
                        .expect("Failed to get");
                    assert!(camp.is_some());
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            })
        })
        .collect();

    writer.await.expect("Writer panicked");
    for reader in readers {
        reader.await.expect("Reader panicked");
    }
}

// =============================================================================
// Migration Scenario Tests
// =============================================================================

#[tokio::test]
async fn test_migrations_on_fresh_database() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db = Database::new(temp_dir.path())
        .await
        .expect("Failed to create database");

    // Database should be functional after migrations
    let campaigns = db.list_campaigns().await.expect("Failed to list");
    assert!(campaigns.is_empty());

    // Create some data to verify schema is correct
    let campaign = CampaignRecord::new(
        "test-fresh".to_string(),
        "Fresh DB Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create campaign");

    let session = SessionRecord::new(
        "sess-fresh".to_string(),
        "test-fresh".to_string(),
        1,
    );
    db.create_session(&session).await.expect("Failed to create session");
}

#[tokio::test]
async fn test_migrations_preserve_data() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");

    // Create database and add data
    {
        let db = Database::new(temp_dir.path())
            .await
            .expect("Failed to create database");

        let campaign = CampaignRecord::new(
            "test-preserve".to_string(),
            "Preserve Test".to_string(),
            "D&D 5e".to_string(),
        );
        db.create_campaign(&campaign).await.expect("Failed to create");
    }

    // Reopen database (migrations should run again idempotently)
    {
        let db = Database::new(temp_dir.path())
            .await
            .expect("Failed to reopen database");

        let campaign = db.get_campaign("test-preserve")
            .await
            .expect("Failed to get")
            .expect("Data not preserved");
        assert_eq!(campaign.name, "Preserve Test");
    }
}

// =============================================================================
// Edge Case Tests - Duplicate Keys
// =============================================================================

#[tokio::test]
async fn test_duplicate_campaign_id() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-dup".to_string(),
        "Original".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign).await.expect("Failed to create");

    // Attempt to create duplicate
    let duplicate = CampaignRecord::new(
        "camp-dup".to_string(),
        "Duplicate".to_string(),
        "PF2e".to_string(),
    );
    let result = db.create_campaign(&duplicate).await;
    assert!(result.is_err(), "Should fail on duplicate ID");
}

#[tokio::test]
async fn test_update_nonexistent_campaign() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "nonexistent".to_string(),
        "Does Not Exist".to_string(),
        "D&D 5e".to_string(),
    );

    // Update should succeed but affect 0 rows
    db.update_campaign(&campaign).await.expect("Update should not error");

    // Verify it wasn't created
    let retrieved = db.get_campaign("nonexistent").await.expect("Query should succeed");
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_record() {
    let (db, _temp) = create_test_db().await;

    // Delete nonexistent records should not error
    db.delete_campaign("nonexistent").await.expect("Delete should not error");
    db.delete_character("nonexistent").await.expect("Delete should not error");
    db.delete_npc("nonexistent").await.expect("Delete should not error");
    db.delete_session_note("nonexistent").await.expect("Delete should not error");
}

// =============================================================================
// Edge Case Tests - Empty and Null Values
// =============================================================================

#[tokio::test]
async fn test_campaign_with_null_optional_fields() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord {
        id: "camp-null".to_string(),
        name: "Null Fields".to_string(),
        system: "D&D 5e".to_string(),
        description: None,
        setting: None,
        current_in_game_date: None,
        house_rules: None,
        world_state: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        archived_at: None,
    };
    db.create_campaign(&campaign).await.expect("Failed to create");

    let retrieved = db.get_campaign("camp-null").await.expect("Failed to get").expect("Not found");
    assert!(retrieved.description.is_none());
    assert!(retrieved.setting.is_none());
    assert!(retrieved.house_rules.is_none());
}

#[tokio::test]
async fn test_empty_string_values() {
    let (db, _temp) = create_test_db().await;

    let mut campaign = CampaignRecord::new(
        "camp-empty".to_string(),
        "".to_string(), // Empty name
        "".to_string(), // Empty system
    );
    campaign.description = Some("".to_string()); // Empty optional

    db.create_campaign(&campaign).await.expect("Failed to create");

    let retrieved = db.get_campaign("camp-empty").await.expect("Failed to get").expect("Not found");
    assert_eq!(retrieved.name, "");
    assert_eq!(retrieved.system, "");
    assert_eq!(retrieved.description, Some("".to_string()));
}

// =============================================================================
// Search Analytics Tests
// =============================================================================

#[tokio::test]
async fn test_search_analytics_record() {
    let (db, _temp) = create_test_db().await;

    let record = SearchAnalyticsRecord::new(
        "dragon lore".to_string(),
        5,
        150,
        "hybrid".to_string(),
        false,
    );
    db.record_search(&record).await.expect("Failed to record search");

    let analytics = db.get_search_analytics(24).await.expect("Failed to get analytics");
    assert_eq!(analytics.len(), 1);
    assert_eq!(analytics[0].query, "dragon lore");
    assert_eq!(analytics[0].results_count, 5);
}

#[tokio::test]
async fn test_search_analytics_summary() {
    let (db, _temp) = create_test_db().await;

    // Record multiple searches
    for i in 0..10 {
        let record = SearchAnalyticsRecord::new(
            format!("query {}", i % 3),
            i,
            100 + i as i32,
            "hybrid".to_string(),
            i % 2 == 0, // Alternate cache hits
        );
        db.record_search(&record).await.expect("Failed to record");
    }

    let summary = db.get_search_analytics_summary(24).await.expect("Failed to get summary");
    assert_eq!(summary.total_searches, 10);
}

#[tokio::test]
async fn test_cleanup_search_analytics() {
    let (db, _temp) = create_test_db().await;

    // Record a search
    let record = SearchAnalyticsRecord::new(
        "old query".to_string(),
        3,
        100,
        "full_text".to_string(),
        false,
    );
    db.record_search(&record).await.expect("Failed to record");

    // Cleanup with 0 days retention (should delete all)
    let deleted = db.cleanup_search_analytics(0).await.expect("Failed to cleanup");
    assert_eq!(deleted, 1);

    let remaining = db.get_search_analytics(24).await.expect("Failed to get");
    assert!(remaining.is_empty());
}
