//! Campaign Database Tests
//!
//! Tests for campaign CRUD operations, versioning, and relationships.

use crate::database::{
    CampaignOps, CampaignRecord, CampaignVersionRecord, EntityRelationshipRecord, EntityType,
    NpcOps, NpcRecord, RelationshipOps,
};
use crate::tests::common::create_test_db;

// =============================================================================
// Basic CRUD Tests
// =============================================================================

#[tokio::test]
async fn test_create_campaign() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-001".to_string(),
        "Dragon's Lair".to_string(),
        "D&D 5e".to_string(),
    );

    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

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
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    campaign.name = "Updated Name".to_string();
    campaign.description = Some("A thrilling adventure".to_string());
    campaign.setting = Some("Forgotten Realms".to_string());
    campaign.updated_at = chrono::Utc::now().to_rfc3339();

    db.update_campaign(&campaign)
        .await
        .expect("Failed to update campaign");

    let retrieved = db
        .get_campaign("camp-002")
        .await
        .expect("Failed to get campaign")
        .expect("Campaign not found");

    assert_eq!(retrieved.name, "Updated Name");
    assert_eq!(
        retrieved.description,
        Some("A thrilling adventure".to_string())
    );
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
        db.create_campaign(&campaign)
            .await
            .expect("Failed to create campaign");
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
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    db.delete_campaign("camp-del")
        .await
        .expect("Failed to delete campaign");

    let retrieved = db
        .get_campaign("camp-del")
        .await
        .expect("Query should succeed");
    assert!(retrieved.is_none(), "Campaign should be deleted");
}

// =============================================================================
// Versioning Tests
// =============================================================================

#[tokio::test]
async fn test_campaign_versioning() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-ver".to_string(),
        "Version Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

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
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

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
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

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

    let from_target = db
        .list_relationships_for_entity("npc", "npc-b")
        .await
        .expect("Failed to list relationships");
    assert_eq!(from_target.len(), 1);
}

// =============================================================================
// Extended CRUD Tests
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

    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

    // Read
    let retrieved = db
        .get_campaign("camp-lifecycle")
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert_eq!(retrieved.description, Some("A test campaign".to_string()));

    // Update
    let mut updated = retrieved.clone();
    updated.name = "Updated Lifecycle".to_string();
    updated.current_in_game_date = Some("Year 1492".to_string());
    updated.updated_at = chrono::Utc::now().to_rfc3339();

    db.update_campaign(&updated)
        .await
        .expect("Failed to update");

    let after_update = db
        .get_campaign("camp-lifecycle")
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert_eq!(after_update.name, "Updated Lifecycle");

    // Archive
    let mut archived = after_update.clone();
    archived.archived_at = Some(chrono::Utc::now().to_rfc3339());
    db.update_campaign(&archived)
        .await
        .expect("Failed to archive");

    let after_archive = db
        .get_campaign("camp-lifecycle")
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert!(after_archive.archived_at.is_some());

    // Delete
    db.delete_campaign("camp-lifecycle")
        .await
        .expect("Failed to delete");
    let after_delete = db
        .get_campaign("camp-lifecycle")
        .await
        .expect("Query should succeed");
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
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create");

    let retrieved = db
        .get_campaign("camp-special")
        .await
        .expect("Failed to get")
        .expect("Not found");
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
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create");

    let retrieved = db
        .get_campaign("camp-unicode")
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert_eq!(retrieved.name, "冒険の世界");
    assert_eq!(retrieved.system, "システム");
}

#[tokio::test]
async fn test_campaign_with_large_json() {
    let (db, _temp) = create_test_db().await;

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

    db.create_campaign(&campaign)
        .await
        .expect("Failed to create");

    let retrieved = db
        .get_campaign("camp-large")
        .await
        .expect("Failed to get")
        .expect("Not found");
    assert_eq!(retrieved.world_state, Some(large_json));
}

// =============================================================================
// Edge Cases
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
async fn test_empty_campaign_list() {
    let (db, _temp) = create_test_db().await;

    let campaigns = db.list_campaigns().await.expect("List should succeed");
    assert!(campaigns.is_empty(), "Should return empty list");
}
