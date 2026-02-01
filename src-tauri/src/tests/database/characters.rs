//! Character Database Tests
//!
//! Tests for character CRUD operations.

use crate::database::{CampaignOps, CampaignRecord, CharacterOps, CharacterRecord};
use crate::tests::common::create_test_db;

#[tokio::test]
async fn test_save_character() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-char".to_string(),
        "Character Test".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

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

    db.save_character(&character)
        .await
        .expect("Failed to save character");

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
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

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
        db.save_character(&character)
            .await
            .expect("Failed to save character");
    }

    let characters = db
        .list_characters(Some("camp-chars"))
        .await
        .expect("Failed to list characters");
    assert_eq!(characters.len(), 3);
}

#[tokio::test]
async fn test_character_update() {
    let (db, _temp) = create_test_db().await;

    let campaign = CampaignRecord::new(
        "camp-char-update".to_string(),
        "Character Update".to_string(),
        "D&D 5e".to_string(),
    );
    db.create_campaign(&campaign)
        .await
        .expect("Failed to create campaign");

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
    db.save_character(&character)
        .await
        .expect("Failed to save");

    // Level up
    let mut updated = character.clone();
    updated.level = Some(20);
    updated.data_json = r#"{"class":"Wizard","subclass":"Divination"}"#.to_string();
    updated.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_character(&updated)
        .await
        .expect("Failed to update");

    let retrieved = db
        .get_character("char-update")
        .await
        .expect("Failed to get")
        .expect("Not found");
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
    db.save_character(&character)
        .await
        .expect("Failed to save");

    db.delete_character("char-delete")
        .await
        .expect("Failed to delete");

    let retrieved = db
        .get_character("char-delete")
        .await
        .expect("Query should succeed");
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
        db.save_character(&character)
            .await
            .expect("Failed to save");
    }

    // List all characters (no campaign filter)
    let all_chars = db.list_characters(None).await.expect("Failed to list");
    assert_eq!(all_chars.len(), 3);
}
