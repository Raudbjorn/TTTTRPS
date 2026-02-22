//! Model Tests
//!
//! Unit tests for all model types.

#[cfg(test)]
mod core_tests {
    use crate::database::models::*;

    #[test]
    fn test_campaign_record_new() {
        let campaign = CampaignRecord::new(
            "camp-1".to_string(),
            "Test Campaign".to_string(),
            "D&D 5e".to_string(),
        );
        assert_eq!(campaign.id, "camp-1");
        assert_eq!(campaign.name, "Test Campaign");
        assert_eq!(campaign.system, "D&D 5e");
        assert!(campaign.archived_at.is_none());
    }

    #[test]
    fn test_session_record_new() {
        let session = SessionRecord::new(
            "sess-1".to_string(),
            "camp-1".to_string(),
            1,
        );
        assert_eq!(session.session_number, 1);
        assert_eq!(session.status, "active");
        assert!(session.ended_at.is_none());
    }

    #[test]
    fn test_entity_type_conversion() {
        assert_eq!(EntityType::Npc.as_str(), "npc");
        assert_eq!(EntityType::from_str("location"), Some(EntityType::Location));
        assert_eq!(EntityType::from_str("invalid"), None);
    }

    #[test]
    fn test_location_record_new() {
        let location = LocationRecord::new(
            "loc-1".to_string(),
            "camp-1".to_string(),
            "The Rusty Dragon".to_string(),
            "tavern".to_string(),
        );
        assert_eq!(location.name, "The Rusty Dragon");
        assert!(location.parent_id.is_none());
    }
}

#[cfg(test)]
mod chat_tests {
    use crate::database::models::*;

    #[test]
    fn test_chat_session_status_conversion() {
        assert_eq!(ChatSessionStatus::Active.as_str(), "active");
        assert_eq!(ChatSessionStatus::try_from("archived"), Ok(ChatSessionStatus::Archived));
        assert!(ChatSessionStatus::try_from("invalid").is_err());
    }

    #[test]
    fn test_message_role_conversion() {
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::try_from("assistant"), Ok(MessageRole::Assistant));
        assert!(MessageRole::try_from("invalid").is_err());
    }

    #[test]
    fn test_global_chat_session_new() {
        let session = GlobalChatSessionRecord::new();
        assert!(session.is_active());
        assert!(session.linked_game_session_id.is_none());
    }

    #[test]
    fn test_chat_message_record() {
        let msg = ChatMessageRecord::with_role(
            "sess-1".to_string(),
            MessageRole::User,
            "Hello!".to_string(),
        );
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello!");
    }
}

#[cfg(test)]
mod generation_tests {
    use crate::database::models::*;

    #[test]
    fn test_wizard_step_navigation() {
        assert_eq!(WizardStep::Basics.next(), Some(WizardStep::Intent));
        assert_eq!(WizardStep::Review.next(), None);
        assert_eq!(WizardStep::Basics.previous(), None);
        assert_eq!(WizardStep::Intent.previous(), Some(WizardStep::Basics));
    }

    #[test]
    fn test_wizard_step_conversion() {
        assert_eq!(WizardStep::Basics.as_str(), "basics");
        assert_eq!(WizardStep::try_from("party_composition"), Ok(WizardStep::PartyComposition));
        assert!(WizardStep::try_from("invalid").is_err());
    }

    #[test]
    fn test_wizard_state_record() {
        let record = WizardStateRecord::new("wizard-1".to_string(), true);
        assert_eq!(record.current_step, "basics");
        assert!(record.is_ai_assisted());
        assert_eq!(record.completed_steps_vec(), Vec::<String>::new());
    }

    #[test]
    fn test_conversation_thread_record() {
        let thread = ConversationThreadRecord::new(
            "thread-1".to_string(),
            ConversationPurpose::CampaignCreation,
        );
        assert_eq!(thread.purpose, "campaign_creation");
        assert!(!thread.is_archived());
    }

    #[test]
    fn test_conversation_message_record() {
        let msg = ConversationMessageRecord::new(
            "msg-1".to_string(),
            "thread-1".to_string(),
            ConversationRole::User,
            "Hello, world!".to_string(),
        );
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello, world!");
        assert!(msg.suggestions.is_none());
    }

    #[test]
    fn test_trust_level_conversion() {
        assert_eq!(TrustLevel::Canonical.as_str(), "canonical");
        assert!(TrustLevel::Canonical.is_reliable());
        assert!(!TrustLevel::Creative.is_reliable());
        assert_eq!(TrustLevel::try_from("derived"), Ok(TrustLevel::Derived));
    }

    #[test]
    fn test_canon_status_transitions() {
        assert!(CanonStatus::Draft.can_transition_to(CanonStatus::Approved));
        assert!(CanonStatus::Approved.can_transition_to(CanonStatus::Canonical));
        assert!(!CanonStatus::Draft.can_transition_to(CanonStatus::Canonical));
        assert!(!CanonStatus::Canonical.can_transition_to(CanonStatus::Draft));
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::page(42);
        assert_eq!(loc.page, Some(42));
        assert!(loc.section.is_none());

        let loc2 = SourceLocation::section("Combat Rules");
        assert!(loc2.page.is_none());
        assert_eq!(loc2.section, Some("Combat Rules".to_string()));
    }

    #[test]
    fn test_acceptance_decision_conversion() {
        assert_eq!(AcceptanceDecision::Approved.as_str(), "approved");
        assert_eq!(AcceptanceDecision::try_from("approved"), Ok(AcceptanceDecision::Approved));
        assert_eq!(AcceptanceDecision::try_from("rejected"), Ok(AcceptanceDecision::Rejected));
        assert_eq!(AcceptanceDecision::try_from("modified"), Ok(AcceptanceDecision::Modified));
        assert!(AcceptanceDecision::try_from("invalid").is_err());
    }

    #[test]
    fn test_generation_draft_record() {
        let data = serde_json::json!({"name": "Test NPC", "role": "merchant"});
        let record = GenerationDraftRecord::new("draft-1".to_string(), "npc".to_string(), data);

        assert_eq!(record.entity_type, "npc");
        assert_eq!(record.status, "draft");
        assert_eq!(record.trust_level, "creative");
        assert!(record.is_editable());
    }
}

#[cfg(test)]
mod ttrpg_tests {
    use crate::database::models::*;

    #[test]
    fn test_npc_record() {
        let npc = NpcRecord::new("npc-1".to_string(), "Bartender Bob".to_string(), "shopkeeper".to_string())
            .with_campaign("camp-1".to_string());

        assert_eq!(npc.name, "Bartender Bob");
        assert_eq!(npc.role, "shopkeeper");
        assert_eq!(npc.campaign_id, Some("camp-1".to_string()));
    }

    #[test]
    fn test_combat_state_record() {
        let mut combat = CombatStateRecord::new(
            "combat-1".to_string(),
            "sess-1".to_string(),
            r#"[{"name":"Goblin","initiative":15}]"#.to_string(),
        )
            .with_name("Goblin Ambush".to_string());

        assert!(combat.is_active);
        assert_eq!(combat.round, 1);
        assert!(combat.ended_at.is_none());

        combat.end();
        assert!(!combat.is_active);
        assert!(combat.ended_at.is_some());
    }

    #[test]
    fn test_random_table_record() {
        let table = RandomTableRecord::new("Encounter Table".to_string(), "d20".to_string())
            .with_category("encounters".to_string())
            .with_tags(&["combat".to_string(), "random".to_string()]);

        assert_eq!(table.name, "Encounter Table");
        assert_eq!(table.dice_notation, "d20");
        assert_eq!(table.category, Some("encounters".to_string()));
        assert!(!table.is_system_table());
    }

    #[test]
    fn test_random_table_entry_record() {
        let entry = RandomTableEntryRecord::new(
            "table-1".to_string(),
            1,
            5,
            "Goblin ambush".to_string(),
        );

        assert!(entry.matches_roll(1));
        assert!(entry.matches_roll(3));
        assert!(entry.matches_roll(5));
        assert!(!entry.matches_roll(0));
        assert!(!entry.matches_roll(6));
    }

    #[test]
    fn test_roll_history_record() {
        let roll = RollHistoryRecord::new("d20".to_string(), 15, 3)
            .with_context("Attack roll".to_string());

        assert_eq!(roll.raw_roll, 15);
        assert_eq!(roll.modifier, 3);
        assert_eq!(roll.final_result, 18);
        assert_eq!(roll.context, Some("Attack roll".to_string()));
    }

    #[test]
    fn test_table_type_conversion() {
        assert_eq!(RandomTableType::Standard.as_str(), "standard");
        assert_eq!(RandomTableType::try_from("weighted"), Ok(RandomTableType::Weighted));
        assert_eq!(RandomTableType::try_from("d66"), Ok(RandomTableType::D66));
        assert!(RandomTableType::try_from("invalid").is_err());
    }
}

#[cfg(test)]
mod recap_tests {
    use crate::database::models::*;

    #[test]
    fn test_session_recap_record() {
        let mut recap = SessionRecapRecord::new(
            "session-1".to_string(),
            "campaign-1".to_string(),
        )
        .with_prose("The party ventured into the dark forest...".to_string())
        .with_cliffhanger("A shadowy figure watched from the treeline.".to_string());

        assert_eq!(recap.session_id, "session-1");
        assert!(recap.prose_text.is_some());
        assert!(recap.cliffhanger.is_some());
        assert_eq!(recap.status_enum(), Ok(RecapStatus::Pending));

        recap.mark_complete();
        assert_eq!(recap.status_enum(), Ok(RecapStatus::Complete));
        assert!(recap.generated_at.is_some());
    }

    #[test]
    fn test_arc_recap_record() {
        let recap = ArcRecapRecord::new(
            "arc-1".to_string(),
            "campaign-1".to_string(),
            "The Dragon's Shadow".to_string(),
        )
        .with_summary("An epic tale of heroes facing an ancient evil.".to_string())
        .with_sessions(&["session-1".to_string(), "session-2".to_string()]);

        assert_eq!(recap.title, "The Dragon's Shadow");
        assert!(recap.summary.is_some());
        assert_eq!(recap.session_ids_vec().len(), 2);
    }

    #[test]
    fn test_pc_knowledge_filter() {
        let filter = PCKnowledgeFilterRecord::new(
            "recap-1".to_string(),
            "character-1".to_string(),
        )
        .with_known_npcs(&["npc-1".to_string(), "npc-2".to_string()])
        .with_private_notes("The character suspects the innkeeper.".to_string());

        assert_eq!(filter.knows_npc_ids_vec().len(), 2);
        assert!(filter.private_notes.is_some());
    }

    #[test]
    fn test_recap_status_conversion() {
        assert_eq!(RecapStatus::Pending.as_str(), "pending");
        assert_eq!(RecapStatus::try_from("complete"), Ok(RecapStatus::Complete));
        assert_eq!(RecapStatus::try_from("generating"), Ok(RecapStatus::Generating));
        assert!(RecapStatus::try_from("invalid").is_err());
    }
}

#[cfg(test)]
mod cards_tests {
    use crate::database::models::*;

    #[test]
    fn test_card_entity_type_conversion() {
        assert_eq!(CardEntityType::Npc.as_str(), "npc");
        assert_eq!(CardEntityType::try_from("npc"), Ok(CardEntityType::Npc));
        assert_eq!(CardEntityType::try_from("location"), Ok(CardEntityType::Location));
        assert!(CardEntityType::try_from("invalid").is_err());
    }

    #[test]
    fn test_disclosure_level_conversion() {
        assert_eq!(DisclosureLevel::Summary.as_str(), "summary");
        assert_eq!(DisclosureLevel::try_from("minimal"), Ok(DisclosureLevel::Minimal));
        assert_eq!(DisclosureLevel::try_from("complete"), Ok(DisclosureLevel::Complete));
        assert!(DisclosureLevel::try_from("invalid").is_err());
    }

    #[test]
    fn test_include_status_conversion() {
        assert_eq!(IncludeStatus::Always.as_str(), "always");
        assert_eq!(IncludeStatus::default(), IncludeStatus::Auto);
        assert!(IncludeStatus::try_from("invalid").is_err());
    }

    #[test]
    fn test_pinned_card_record() {
        let card = PinnedCardRecord::new(
            "session-1".to_string(),
            CardEntityType::Npc,
            "npc-1".to_string(),
            0,
        );
        assert_eq!(card.entity_type, "npc");
        assert_eq!(card.disclosure_level, "summary");
        assert_eq!(card.display_order, 0);
    }

    #[test]
    fn test_cheat_sheet_preference_record() {
        let pref = CheatSheetPreferenceRecord::new(
            "campaign-1".to_string(),
            PreferenceType::Category,
        )
        .with_entity_type(CardEntityType::Npc)
        .with_include_status(IncludeStatus::Always)
        .with_priority(75);

        assert_eq!(pref.preference_type, "category");
        assert_eq!(pref.entity_type, Some("npc".to_string()));
        assert_eq!(pref.include_status, "always");
        assert_eq!(pref.priority, 75);
    }

    #[test]
    fn test_card_cache_record() {
        let cache = CardCacheRecord::new(
            CardEntityType::Npc,
            "npc-1".to_string(),
            DisclosureLevel::Summary,
            "<div>NPC Card</div>".to_string(),
            24,
        );
        assert_eq!(cache.entity_type, "npc");
        assert!(!cache.is_expired());
    }

    #[test]
    fn test_card_cache_expiration() {
        let mut cache = CardCacheRecord::new(
            CardEntityType::Npc,
            "npc-1".to_string(),
            DisclosureLevel::Summary,
            "<div>Expired</div>".to_string(),
            24,
        );
        // Set expires_at to the past
        cache.expires_at = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        assert!(cache.is_expired());
    }
}

#[cfg(test)]
mod analytics_tests {
    use crate::database::models::*;

    #[test]
    fn test_search_analytics_record() {
        let record = SearchAnalyticsRecord::new(
            "test query".to_string(),
            10,
            50,
            "hybrid".to_string(),
            false,
        );

        assert_eq!(record.results_count, 10);
        assert!(!record.cache_hit);
        assert!(!record.is_zero_result());
        assert!(!record.has_selection());
    }

    #[test]
    fn test_search_selection_record() {
        let selection = SearchSelectionRecord::new(
            "search-1".to_string(),
            "test query".to_string(),
            0,
            "rulebook".to_string(),
            1500,
        )
        .with_helpfulness(true);

        assert_eq!(selection.result_index, 0);
        assert_eq!(selection.was_helpful, Some(true));
        assert_eq!(selection.selection_delay_ms, 1500);
    }
}
