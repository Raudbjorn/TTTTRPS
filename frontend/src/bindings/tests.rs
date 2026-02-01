#[cfg(test)]
mod tests {
    use crate::bindings::ai::ChatRequestPayload;
    use crate::bindings::campaign::Campaign;
    use crate::bindings::mechanics::{GenerationOptions, CombatState, Combatant, Character, CharacterAttributeValue};
    use crate::bindings::audio::{VoiceConfig, ElevenLabsConfig, VoiceProviderType};
    use crate::bindings::world::{WorldState, InGameDate};
    use crate::bindings::library::IngestOptions;
    use serde_json::json;
    use std::collections::HashMap;

    // --- AI Module Tests ---
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
        assert_eq!(json["system_prompt"], "System");
        assert_eq!(json["use_rag"], true);
    }

    // --- Campaign Module Tests ---
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

    // --- Mechanics Module Tests ---
    #[test]
    fn test_generation_options_default() {
        let options = GenerationOptions::default();
        assert_eq!(options.random_stats, false);
        assert_eq!(options.include_equipment, false);
    }

    #[test]
    fn test_combat_state_serialization() {
        let combatant = Combatant {
            id: "c1".to_string(),
            name: "Goblin".to_string(),
            initiative: 12,
            hp_current: 5,
            hp_max: 7,
            ac: Some(12),
            hp_temp: None,
            combatant_type: "npc".to_string(),
            conditions: vec!["prone".to_string()],
            is_active: true,
        };
        
        let state = CombatState {
            id: "combat1".to_string(),
            round: 1,
            current_turn: 0,
            combatants: vec![combatant],
            is_active: true,
        };

        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["id"], "combat1");
        assert_eq!(json["combatants"][0]["name"], "Goblin");
        // Serialization uses struct field names
        assert_eq!(json["combatants"][0]["hp_current"], 5); 
        assert_eq!(json["combatants"][0]["ac"], 12);
    }

    #[test]
    fn test_character_structure() {
        let mut attrs = HashMap::new();
        attrs.insert("str".to_string(), CharacterAttributeValue { base: 10, modifier: 0, temp_bonus: 0 });

        let char = Character {
            id: "ch1".to_string(),
            name: "Hero".to_string(),
            system: "dnd5e".to_string(),
            concept: "Fighter".to_string(),
            race: Some("Human".to_string()),
            character_class: Some("Fighter".to_string()),
            level: 1,
            attributes: attrs,
            skills: HashMap::new(),
            traits: vec![],
            equipment: vec![],
            background: Default::default(),
            backstory: None,
            notes: "".to_string(),
            portrait_prompt: None,
        };

        let json = serde_json::to_value(&char).unwrap();
        // Verify 'class' rename
        assert_eq!(json["class"], "Fighter"); 
        assert_eq!(json["attributes"]["str"]["base"], 10);
    }

    // --- Audio Module Tests ---
    #[test]
    fn test_voice_config_serialization() {
        let config = VoiceConfig {
            provider: "elevenlabs".to_string(),
            cache_dir: None,
            default_voice_id: None,
            elevenlabs: Some(ElevenLabsConfig {
                api_key: "secret".to_string(),
                model_id: None,
            }),
            fish_audio: None,
            ollama: None,
            openai: None,
            piper: None,
            coqui: None,
        };

        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["provider"], "elevenlabs");
        assert_eq!(json["elevenlabs"]["api_key"], "secret");
    }

    #[test]
    fn test_voice_provider_enum() {
        let p = VoiceProviderType::ElevenLabs;
        assert_eq!(p.to_string_key(), "elevenlabs");
        assert_eq!(p.display_name(), "ElevenLabs");
    }

    // --- World Module Tests ---
    #[test]
    fn test_world_state_date() {
        let date = InGameDate {
            year: 1492,
            month: 5,
            day: 12,
            era: Some("DR".to_string()),
            calendar: "harptos".to_string(),
            time: None,
        };
        let json = serde_json::to_value(&date).unwrap();
        assert_eq!(json["year"], 1492);
        assert_eq!(json["era"], "DR");
    }

    #[test]
    fn test_world_state_structure() {
        let date = InGameDate {
            year: 1492,
            month: 5,
            day: 12,
            era: Some("DR".to_string()),
            calendar: "harptos".to_string(),
            time: None,
        };
        
        let state = WorldState {
            campaign_id: "c1".to_string(),
            current_date: date,
            events: vec![],
            locations: HashMap::new(),
            npc_relationships: vec![],
            custom_fields: HashMap::new(),
            updated_at: "2023-01-01".to_string(),
            calendar_config: crate::bindings::world::CalendarConfig {
                name: "Harptos".to_string(),
                months_per_year: 12,
                days_per_month: vec![30],
                month_names: vec![],
                week_days: vec![],
                eras: vec![],
            },
        };

        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["campaign_id"], "c1");
        assert_eq!(json["current_date"]["year"], 1492);
    }

    // --- Library Module Tests ---
    #[test]
    fn test_ingest_options() {
        let opts = IngestOptions {
            source_type: "pdf".to_string(),
            campaign_id: Some("camp1".to_string()),
        };
        let json = serde_json::to_value(&opts).unwrap();
        assert_eq!(json["source_type"], "pdf");
        assert_eq!(json["campaign_id"], "camp1");
    }
}