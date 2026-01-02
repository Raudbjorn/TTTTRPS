//! Voice Presets (TASK-004)
//!
//! Built-in DM persona voice profiles for various narrative styles.
//! These presets are designed to cover common TTRPG archetypes.

use super::profiles::{AgeRange, Gender, ProfileMetadata, VoiceProfile};
use super::types::VoiceProviderType;

/// Get all built-in DM persona presets (13+ voices)
pub fn get_dm_presets() -> Vec<VoiceProfile> {
    vec![
        // =========================================================================
        // Classic Fantasy Personas
        // =========================================================================
        VoiceProfile::preset(
            "preset-wise-sage",
            "The Wise Sage",
            VoiceProviderType::OpenAI,
            "echo",
            ProfileMetadata::new(AgeRange::Elderly, Gender::Male)
                .with_traits(&["wise", "calm", "measured", "thoughtful"])
                .with_description("A venerable sage who speaks with ancient wisdom and measured patience. Perfect for lore dumps, mystical encounters, and elder NPCs.")
                .with_tag("fantasy")
                .with_tag("narrator"),
        ),

        VoiceProfile::preset(
            "preset-booming-commander",
            "The Booming Commander",
            VoiceProviderType::OpenAI,
            "onyx",
            ProfileMetadata::new(AgeRange::MiddleAged, Gender::Male)
                .with_traits(&["commanding", "authoritative", "loud", "confident"])
                .with_description("A powerful military leader with a voice that carries across battlefields. Ideal for generals, warlords, and authority figures.")
                .with_tag("fantasy")
                .with_tag("military"),
        ),

        VoiceProfile::preset(
            "preset-mysterious-oracle",
            "The Mysterious Oracle",
            VoiceProviderType::OpenAI,
            "shimmer",
            ProfileMetadata::new(AgeRange::Adult, Gender::Female)
                .with_traits(&["ethereal", "mysterious", "prophetic", "otherworldly"])
                .with_description("An enigmatic seer who speaks in riddles and prophecies. Perfect for fortune tellers, seers, and mystical entities.")
                .with_tag("fantasy")
                .with_tag("mystical"),
        ),

        VoiceProfile::preset(
            "preset-jovial-innkeeper",
            "The Jovial Innkeeper",
            VoiceProviderType::OpenAI,
            "fable",
            ProfileMetadata::new(AgeRange::MiddleAged, Gender::Male)
                .with_traits(&["friendly", "jovial", "warm", "welcoming"])
                .with_description("A hearty tavern keeper who makes everyone feel at home. Great for merchants, friendly NPCs, and quest givers.")
                .with_tag("fantasy")
                .with_tag("social"),
        ),

        VoiceProfile::preset(
            "preset-cunning-rogue",
            "The Cunning Rogue",
            VoiceProviderType::OpenAI,
            "alloy",
            ProfileMetadata::new(AgeRange::YoungAdult, Gender::Neutral)
                .with_traits(&["sly", "cunning", "quick-witted", "charming"])
                .with_description("A street-smart character with a silver tongue. Perfect for thieves, spies, and morally flexible allies.")
                .with_tag("fantasy")
                .with_tag("stealth"),
        ),

        // =========================================================================
        // Dark/Horror Personas
        // =========================================================================
        VoiceProfile::preset(
            "preset-sinister-villain",
            "The Sinister Villain",
            VoiceProviderType::OpenAI,
            "onyx",
            ProfileMetadata::new(AgeRange::Adult, Gender::Male)
                .with_traits(&["menacing", "cold", "calculating", "cruel"])
                .with_description("A chilling antagonist whose every word drips with malice. Ideal for BBEGs, dark lords, and intimidating foes.")
                .with_tag("horror")
                .with_tag("villain"),
        ),

        VoiceProfile::preset(
            "preset-haunted-spirit",
            "The Haunted Spirit",
            VoiceProviderType::OpenAI,
            "shimmer",
            ProfileMetadata::new(AgeRange::Adult, Gender::Female)
                .with_traits(&["haunting", "sorrowful", "ethereal", "tragic"])
                .with_description("A ghostly presence speaking from beyond the grave. Perfect for ghosts, spirits, and tragic figures.")
                .with_tag("horror")
                .with_tag("undead"),
        ),

        VoiceProfile::preset(
            "preset-eldritch-horror",
            "The Eldritch Voice",
            VoiceProviderType::OpenAI,
            "nova",
            ProfileMetadata::new(AgeRange::Adult, Gender::Neutral)
                .with_traits(&["alien", "unsettling", "ancient", "incomprehensible"])
                .with_description("A voice from beyond mortal understanding. Use for cosmic horrors, aberrations, and otherworldly entities.")
                .with_tag("horror")
                .with_tag("cosmic"),
        ),

        // =========================================================================
        // Heroic/Adventure Personas
        // =========================================================================
        VoiceProfile::preset(
            "preset-brave-knight",
            "The Brave Knight",
            VoiceProviderType::OpenAI,
            "echo",
            ProfileMetadata::new(AgeRange::Adult, Gender::Male)
                .with_traits(&["noble", "brave", "honorable", "inspiring"])
                .with_description("A paragon of virtue and courage. Ideal for paladins, heroic NPCs, and noble leaders.")
                .with_tag("fantasy")
                .with_tag("heroic"),
        ),

        VoiceProfile::preset(
            "preset-fierce-warrior",
            "The Fierce Warrior",
            VoiceProviderType::OpenAI,
            "nova",
            ProfileMetadata::new(AgeRange::Adult, Gender::Female)
                .with_traits(&["fierce", "determined", "bold", "powerful"])
                .with_description("A battle-hardened warrior whose voice commands respect. Perfect for barbarians, fighters, and amazon-type characters.")
                .with_tag("fantasy")
                .with_tag("combat"),
        ),

        VoiceProfile::preset(
            "preset-young-hero",
            "The Young Hero",
            VoiceProviderType::OpenAI,
            "alloy",
            ProfileMetadata::new(AgeRange::YoungAdult, Gender::Neutral)
                .with_traits(&["eager", "hopeful", "determined", "youthful"])
                .with_description("An aspiring adventurer full of hope and determination. Great for apprentices, squires, and coming-of-age characters.")
                .with_tag("fantasy")
                .with_tag("heroic"),
        ),

        // =========================================================================
        // Sci-Fi/Modern Personas
        // =========================================================================
        VoiceProfile::preset(
            "preset-ai-companion",
            "The AI Companion",
            VoiceProviderType::OpenAI,
            "alloy",
            ProfileMetadata::new(AgeRange::Adult, Gender::Neutral)
                .with_traits(&["precise", "helpful", "analytical", "slightly-robotic"])
                .with_description("An artificial intelligence companion. Perfect for sci-fi campaigns, robots, and synthetic beings.")
                .with_tag("scifi")
                .with_tag("ai"),
        ),

        VoiceProfile::preset(
            "preset-grizzled-detective",
            "The Grizzled Detective",
            VoiceProviderType::OpenAI,
            "onyx",
            ProfileMetadata::new(AgeRange::MiddleAged, Gender::Male)
                .with_traits(&["world-weary", "cynical", "observant", "gravelly"])
                .with_description("A noir-style investigator who's seen it all. Ideal for mystery campaigns, hardboiled characters, and investigators.")
                .with_tag("noir")
                .with_tag("modern"),
        ),

        VoiceProfile::preset(
            "preset-corporate-exec",
            "The Corporate Executive",
            VoiceProviderType::OpenAI,
            "shimmer",
            ProfileMetadata::new(AgeRange::Adult, Gender::Female)
                .with_traits(&["polished", "calculating", "professional", "cold"])
                .with_description("A sleek corporate power player. Great for cyberpunk megacorp executives, manipulative nobles, or political schemers.")
                .with_tag("scifi")
                .with_tag("cyberpunk"),
        ),

        // =========================================================================
        // Narrator Personas
        // =========================================================================
        VoiceProfile::preset(
            "preset-epic-narrator",
            "The Epic Narrator",
            VoiceProviderType::OpenAI,
            "echo",
            ProfileMetadata::new(AgeRange::MiddleAged, Gender::Male)
                .with_traits(&["dramatic", "resonant", "epic", "theatrical"])
                .with_description("A dramatic storyteller for epic moments and grand narratives. Perfect for scene-setting and dramatic reveals.")
                .with_tag("narrator")
                .with_tag("epic"),
        ),

        VoiceProfile::preset(
            "preset-whimsical-storyteller",
            "The Whimsical Storyteller",
            VoiceProviderType::OpenAI,
            "fable",
            ProfileMetadata::new(AgeRange::Adult, Gender::Female)
                .with_traits(&["playful", "whimsical", "warm", "enchanting"])
                .with_description("A fairy-tale narrator with a touch of magic. Ideal for lighthearted campaigns, fey encounters, and children's storybook vibes.")
                .with_tag("narrator")
                .with_tag("whimsical"),
        ),

        // =========================================================================
        // Creature/Monster Personas
        // =========================================================================
        VoiceProfile::preset(
            "preset-ancient-dragon",
            "The Ancient Dragon",
            VoiceProviderType::OpenAI,
            "onyx",
            ProfileMetadata::new(AgeRange::Elderly, Gender::Neutral)
                .with_traits(&["ancient", "powerful", "arrogant", "rumbling"])
                .with_description("A primordial dragon whose voice shakes the very earth. For dragons, titans, and ancient beings of immense power.")
                .with_tag("fantasy")
                .with_tag("creature"),
        ),

        VoiceProfile::preset(
            "preset-mischievous-fey",
            "The Mischievous Fey",
            VoiceProviderType::OpenAI,
            "shimmer",
            ProfileMetadata::new(AgeRange::YoungAdult, Gender::NonBinary)
                .with_traits(&["playful", "mischievous", "mercurial", "enchanting"])
                .with_description("A trickster fey creature with unpredictable moods. Perfect for fairies, pixies, and capricious forest spirits.")
                .with_tag("fantasy")
                .with_tag("fey"),
        ),

        VoiceProfile::preset(
            "preset-gruff-dwarf",
            "The Gruff Dwarf",
            VoiceProviderType::OpenAI,
            "fable",
            ProfileMetadata::new(AgeRange::MiddleAged, Gender::Male)
                .with_traits(&["gruff", "proud", "stubborn", "hearty"])
                .with_description("A stout dwarven voice, perfect for miners, smiths, and mountain-dwelling folk.")
                .with_tag("fantasy")
                .with_tag("dwarf"),
        ),

        // =========================================================================
        // Utility Personas
        // =========================================================================
        VoiceProfile::preset(
            "preset-neutral-narrator",
            "The Neutral Narrator",
            VoiceProviderType::OpenAI,
            "nova",
            ProfileMetadata::new(AgeRange::Adult, Gender::Neutral)
                .with_traits(&["clear", "neutral", "informative", "balanced"])
                .with_description("A clear, unbiased narrator for rules readings, descriptions, and informational content.")
                .with_tag("narrator")
                .with_tag("utility"),
        ),
    ]
}

/// Get presets filtered by tag
pub fn get_presets_by_tag(tag: &str) -> Vec<VoiceProfile> {
    get_dm_presets()
        .into_iter()
        .filter(|p| p.metadata.tags.iter().any(|t| t == tag))
        .collect()
}

/// Get presets by category
pub fn get_presets_by_category() -> std::collections::HashMap<String, Vec<VoiceProfile>> {
    let mut categories: std::collections::HashMap<String, Vec<VoiceProfile>> = std::collections::HashMap::new();

    for preset in get_dm_presets() {
        for tag in &preset.metadata.tags {
            categories
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(preset.clone());
        }
    }

    categories
}

/// Get a single preset by ID
pub fn get_preset_by_id(id: &str) -> Option<VoiceProfile> {
    get_dm_presets().into_iter().find(|p| p.id == id)
}

/// Get all available OpenAI voice IDs used in presets
pub fn get_openai_voice_ids() -> Vec<&'static str> {
    vec!["alloy", "echo", "fable", "onyx", "nova", "shimmer"]
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dm_presets_count() {
        let presets = get_dm_presets();
        assert!(presets.len() >= 13, "Should have at least 13 DM personas, got {}", presets.len());
    }

    #[test]
    fn test_all_presets_have_required_fields() {
        for preset in get_dm_presets() {
            assert!(!preset.id.is_empty(), "Preset should have an ID");
            assert!(!preset.name.is_empty(), "Preset should have a name");
            assert!(!preset.voice_id.is_empty(), "Preset should have a voice_id");
            assert!(preset.is_preset, "Preset should be marked as preset");
            assert!(!preset.metadata.personality_traits.is_empty(), "Preset should have personality traits");
            assert!(preset.metadata.description.is_some(), "Preset should have a description");
            assert!(!preset.metadata.tags.is_empty(), "Preset should have at least one tag");
        }
    }

    #[test]
    fn test_get_presets_by_tag() {
        let fantasy = get_presets_by_tag("fantasy");
        assert!(!fantasy.is_empty(), "Should have fantasy presets");

        let horror = get_presets_by_tag("horror");
        assert!(!horror.is_empty(), "Should have horror presets");

        let narrator = get_presets_by_tag("narrator");
        assert!(!narrator.is_empty(), "Should have narrator presets");
    }

    #[test]
    fn test_get_preset_by_id() {
        let preset = get_preset_by_id("preset-wise-sage");
        assert!(preset.is_some(), "Should find preset by ID");
        assert_eq!(preset.unwrap().name, "The Wise Sage");
    }

    #[test]
    fn test_unique_preset_ids() {
        let presets = get_dm_presets();
        let mut ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for preset in presets {
            assert!(ids.insert(preset.id.clone()), "Preset IDs should be unique: {}", preset.id);
        }
    }

    #[test]
    fn test_gender_variety() {
        let presets = get_dm_presets();
        let has_male = presets.iter().any(|p| matches!(p.metadata.gender, Gender::Male));
        let has_female = presets.iter().any(|p| matches!(p.metadata.gender, Gender::Female));
        let has_neutral = presets.iter().any(|p| matches!(p.metadata.gender, Gender::Neutral));

        assert!(has_male, "Should have male voices");
        assert!(has_female, "Should have female voices");
        assert!(has_neutral, "Should have neutral voices");
    }

    #[test]
    fn test_age_variety() {
        let presets = get_dm_presets();
        let has_adult = presets.iter().any(|p| matches!(p.metadata.age_range, AgeRange::Adult));
        let has_elderly = presets.iter().any(|p| matches!(p.metadata.age_range, AgeRange::Elderly));

        assert!(has_adult, "Should have adult voices");
        assert!(has_elderly, "Should have elderly voices");
    }
}
