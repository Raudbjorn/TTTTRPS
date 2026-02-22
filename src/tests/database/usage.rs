//! Usage Database Tests
//!
//! Tests for usage tracking, analytics, and voice profile operations.

use crate::database::{VoiceProfileOps, VoiceProfileRecord};
use crate::tests::common::create_test_db;

// =============================================================================
// Voice Profile Tests
// =============================================================================

#[tokio::test]
async fn test_voice_profile_save_and_get() {
    let (db, _temp) = create_test_db().await;

    let profile = VoiceProfileRecord::new(
        "voice-001".to_string(),
        "Deep Voice".to_string(),
        "elevenlabs".to_string(),
        "voice123".to_string(),
    );

    db.save_voice_profile(&profile)
        .await
        .expect("Failed to save voice profile");

    let retrieved = db
        .get_voice_profile("voice-001")
        .await
        .expect("Failed to get voice profile")
        .expect("Voice profile not found");

    assert_eq!(retrieved.name, "Deep Voice");
    assert_eq!(retrieved.provider, "elevenlabs");
}

#[tokio::test]
async fn test_voice_profile_update() {
    let (db, _temp) = create_test_db().await;

    let mut profile = VoiceProfileRecord::new(
        "voice-update".to_string(),
        "Deep Voice".to_string(),
        "elevenlabs".to_string(),
        "voice123".to_string(),
    );
    db.save_voice_profile(&profile)
        .await
        .expect("Failed to save");

    // Update
    profile.name = "Very Deep Voice".to_string();
    profile.age_range = Some("elderly".to_string());
    profile.gender = Some("male".to_string());
    profile.settings = Some(r#"{"pitch":-2}"#.to_string());
    profile.updated_at = chrono::Utc::now().to_rfc3339();
    db.save_voice_profile(&profile)
        .await
        .expect("Failed to update");

    let retrieved = db
        .get_voice_profile("voice-update")
        .await
        .expect("Failed to get")
        .expect("Not found");
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
    db.save_voice_profile(&profile)
        .await
        .expect("Failed to save");

    db.delete_voice_profile("voice-delete")
        .await
        .expect("Failed to delete");

    let retrieved = db
        .get_voice_profile("voice-delete")
        .await
        .expect("Query should succeed");
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
    db.save_voice_profile(&preset)
        .await
        .expect("Failed to save");

    let custom = VoiceProfileRecord::new(
        "voice-custom".to_string(),
        "Custom Voice".to_string(),
        "elevenlabs".to_string(),
        "custom123".to_string(),
    );
    db.save_voice_profile(&custom)
        .await
        .expect("Failed to save");

    // List all
    let all = db.list_voice_profiles().await.expect("Failed to list");
    assert_eq!(all.len(), 2);

    // List presets only
    let presets = db
        .list_voice_profile_presets()
        .await
        .expect("Failed to list");
    assert_eq!(presets.len(), 1);
    assert_eq!(presets[0].name, "Preset Voice");
}

#[tokio::test]
async fn test_list_voice_profiles() {
    let (db, _temp) = create_test_db().await;

    for i in 1..=3 {
        let profile = VoiceProfileRecord::new(
            format!("voice-{:03}", i),
            format!("Voice {}", i),
            "elevenlabs".to_string(),
            format!("voice-id-{}", i),
        );
        db.save_voice_profile(&profile)
            .await
            .expect("Failed to save");
    }

    let profiles = db.list_voice_profiles().await.expect("Failed to list");
    assert_eq!(profiles.len(), 3);
}
