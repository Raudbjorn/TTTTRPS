//! NPC Database Tests
//!
//! Tests for NPC CRUD operations, conversations, and personalities.

use crate::database::{
    CampaignOps, CampaignRecord, NpcOps, NpcRecord, NpcConversation, PersonalityRecord,
};
use crate::tests::common::create_test_db;

// =============================================================================
// Basic CRUD Tests
// =============================================================================

#[tokio::test]
async fn test_save_and_get_npc() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-npc".to_string(),
        "NPC Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let npc = NpcRecord {
        id: "npc-001".to_string(),
        campaign_id: Some("camp-npc".to_string()),
        name: "Elara the Wise".to_string(),
        role: "Sage".to_string(),
        personality_id: None,
        personality_json: r#"{"traits":["wise","patient"]}"#.to_string(),
        data_json: None,
        stats_json: None,
        notes: Some("Knows ancient lore".to_string()),
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

    assert_eq!(retrieved.name, "Elara the Wise");
    assert_eq!(retrieved.role, "Sage");
}

#[tokio::test]
async fn test_list_npcs() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-npcs".to_string(),
        "NPCs Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    for i in 1..=5 {
        let npc = NpcRecord {
            id: format!("npc-{:03}", i),
            campaign_id: Some("camp-npcs".to_string()),
            name: format!("NPC {}", i),
            role: "Commoner".to_string(),
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

    let npcs = db
        .list_npcs(Some("camp-npcs"))
        .await
        .expect("Failed to list NPCs");
    assert_eq!(npcs.len(), 5);
}

#[tokio::test]
async fn test_npc_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-npc-update".to_string(),
        "NPC Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

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

    let retrieved = db
        .get_npc("npc-update")
        .await
        .expect("Failed to get")
        .expect("Not found");
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

    db.delete_npc("npc-delete")
        .await
        .expect("Failed to delete");

    let retrieved = db
        .get_npc("npc-delete")
        .await
        .expect("Query should succeed");
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
        db.create_campaign(&campaign)
            .await
            .expect("Failed to create campaign");
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
    let npcs_a = db
        .list_npcs(Some("camp-a"))
        .await
        .expect("Failed to list");
    assert_eq!(npcs_a.len(), 3);

    let npcs_b = db
        .list_npcs(Some("camp-b"))
        .await
        .expect("Failed to list");
    assert_eq!(npcs_b.len(), 3);

    // List all NPCs
    let all_npcs = db.list_npcs(None).await.expect("Failed to list");
    assert_eq!(all_npcs.len(), 6);
}

// =============================================================================
// NPC Conversation Tests
// =============================================================================

#[tokio::test]
async fn test_npc_conversation() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-conv".to_string(),
        "Conversation Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    let npc = NpcRecord {
        id: "npc-conv".to_string(),
        campaign_id: Some("camp-conv".to_string()),
        name: "Talkative NPC".to_string(),
        role: "Merchant".to_string(),
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

#[tokio::test]
async fn test_npc_conversation_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-conv-update".to_string(),
        "Conversation Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

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
    db.save_npc_conversation(&conversation)
        .await
        .expect("Failed to save");

    // Update conversation with messages
    let mut updated = conversation.clone();
    updated.messages_json =
        r#"[{"id":"msg1","role":"user","content":"Hello!","created_at":"2024-01-01"}]"#.to_string();
    updated.unread_count = 1;
    updated.last_message_at = chrono::Utc::now().to_rfc3339();
    updated.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_npc_conversation(&updated)
        .await
        .expect("Failed to update");

    let retrieved = db
        .get_npc_conversation("npc-conv-update")
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert!(retrieved.messages_json.contains("Hello!"));
    assert_eq!(retrieved.unread_count, 1);
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
    db.save_personality(&personality)
        .await
        .expect("Failed to save");

    db.delete_personality("pers-delete")
        .await
        .expect("Failed to delete");

    let retrieved = db
        .get_personality("pers-delete")
        .await
        .expect("Query should succeed");
    assert!(retrieved.is_none());
}
