#[cfg(test)]
mod tests {
    use crate::bindings::ai::ChatRequestPayload;
    use crate::bindings::campaign::Campaign;
    use crate::bindings::mechanics::GenerationOptions;
    use serde_json::json;

    #[test]
    fn test_chat_request_serialization() {
        let payload = ChatRequestPayload {
            message: "Hello".to_string(),
            system_prompt: Some("System".to_string()),
            personality_id: None,
            context: None,
            use_rag: true,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["message"], "Hello");
        assert_eq!(json["use_rag"], true);
    }

    #[test]
    fn test_campaign_deserialization() {
        let json = json!({
            "id": "123",
            "name": "Test Campaign",
            "system": "D&D 5e",
            "created_at": "2023-01-01",
            "updated_at": "2023-01-02",
            "settings": {
                "theme": "fantasy",
                "theme_weights": {},
                "voice_enabled": false,
                "auto_transcribe": true
            }
        });
        let campaign: Campaign = serde_json::from_value(json).unwrap();
        assert_eq!(campaign.id, "123");
        assert_eq!(campaign.name, "Test Campaign");
        assert!(campaign.settings.auto_transcribe);
    }

    #[test]
    fn test_generation_options_default() {
        let options = GenerationOptions::default();
        assert_eq!(options.random_stats, false);
        assert_eq!(options.include_equipment, false);
    }
}
