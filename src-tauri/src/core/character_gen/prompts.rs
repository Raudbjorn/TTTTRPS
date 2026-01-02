//! Prompt Templates for AI-Powered Character Generation
//!
//! Provides structured prompt templates for generating backstories, personality traits,
//! and character descriptions that match campaign settings and game system conventions.

use crate::core::character_gen::{Character, GameSystem, BackstoryLength};
use super::backstory::{BackstoryRequest, BackstoryStyle};

/// System prompt builder for backstory generation
pub struct BackstoryPromptBuilder;

impl BackstoryPromptBuilder {
    /// Build the system prompt based on game system and style preferences
    pub fn build_system_prompt(request: &BackstoryRequest) -> String {
        let system = &request.character.system;
        let genre_context = Self::get_genre_context(system);
        let style_instructions = Self::get_style_instructions(&request.style);
        let campaign_context = Self::get_campaign_context(&request.campaign_setting);

        format!(
            "{genre_context}\n\n\
             {style_instructions}\n\n\
             {campaign_context}\n\n\
             WRITING GUIDELINES:\n\
             - Write compelling, original backstories that feel authentic to the game system\n\
             - Provide hooks for the GM to use in the campaign\n\
             - Include memorable NPCs with clear relationships to the character\n\
             - Explain why the character has their abilities, traits, and motivations\n\
             - Leave some mysteries to explore during play\n\
             - Avoid cliches unless they serve the story\n\
             - Use specific details rather than vague descriptions\n\n\
             OUTPUT FORMAT:\n\
             Always respond with valid JSON containing:\n\
             - \"text\": the full backstory narrative\n\
             - \"summary\": a 1-2 sentence summary\n\
             - \"key_events\": array of 3-5 important life events\n\
             - \"mentioned_npcs\": array of {{\"name\": string, \"relationship\": string, \"status\": string}}\n\
             - \"mentioned_locations\": array of place names\n\
             - \"plot_hooks\": array of 2-4 potential story hooks\n\
             - \"suggested_traits\": array of personality traits based on the backstory"
        )
    }

    /// Build the user prompt with character details
    pub fn build_user_prompt(request: &BackstoryRequest) -> String {
        let char = &request.character;
        let (min_words, max_words) = request.length.word_count();

        let mut prompt = String::with_capacity(2000);

        // Header with length guidance
        prompt.push_str(&format!(
            "Generate a character backstory ({min_words}-{max_words} words) for the following character:\n\n"
        ));

        // Core character info
        prompt.push_str("=== CHARACTER INFORMATION ===\n");
        prompt.push_str(&format!("Name: {}\n", char.name));
        prompt.push_str(&format!("Game System: {}\n", char.system.display_name()));

        if let Some(race) = &char.race {
            prompt.push_str(&format!("Race/Ancestry: {}\n", race));
        }

        if let Some(class) = &char.class {
            prompt.push_str(&format!("Class/Role: {}\n", class));
        }

        prompt.push_str(&format!("Level/Rank: {}\n", char.level));

        if !char.concept.is_empty() {
            prompt.push_str(&format!("Character Concept: {}\n", char.concept));
        }

        // Background information
        if !char.background.origin.is_empty() || char.background.occupation.is_some() ||
           !char.background.motivation.is_empty() {
            prompt.push_str("\n=== BACKGROUND ===\n");

            if !char.background.origin.is_empty() {
                prompt.push_str(&format!("Origin/Background Type: {}\n", char.background.origin));
            }

            if let Some(occupation) = &char.background.occupation {
                prompt.push_str(&format!("Occupation: {}\n", occupation));
            }

            if !char.background.motivation.is_empty() {
                prompt.push_str(&format!("Core Motivation: {}\n", char.background.motivation));
            }

            if !char.background.connections.is_empty() {
                prompt.push_str(&format!("Known Connections: {}\n", char.background.connections.join(", ")));
            }
        }

        // Traits
        if !char.traits.is_empty() {
            prompt.push_str("\n=== PERSONALITY & TRAITS ===\n");
            for trait_item in &char.traits {
                prompt.push_str(&format!(
                    "- {} ({:?}): {}\n",
                    trait_item.name,
                    trait_item.trait_type,
                    trait_item.description
                ));
            }
        }

        // Campaign setting
        if let Some(setting) = &request.campaign_setting {
            prompt.push_str(&format!("\n=== CAMPAIGN SETTING ===\n{}\n", setting));
        }

        // Style preferences
        prompt.push_str("\n=== STYLE REQUIREMENTS ===\n");

        if let Some(tone) = &request.style.tone {
            prompt.push_str(&format!("Tone: {}\n", tone));
        }

        if let Some(perspective) = &request.style.perspective {
            prompt.push_str(&format!("Narrative Perspective: {}\n", perspective));
        }

        if let Some(focus) = &request.style.focus {
            prompt.push_str(&format!("Story Focus: {}\n", focus));
        }

        // Include/exclude elements
        if !request.include_elements.is_empty() {
            prompt.push_str(&format!(
                "\nMUST INCLUDE these elements: {}\n",
                request.include_elements.join(", ")
            ));
        }

        if !request.exclude_elements.is_empty() {
            prompt.push_str(&format!(
                "AVOID these elements: {}\n",
                request.exclude_elements.join(", ")
            ));
        }

        // Custom instructions
        if let Some(custom) = &request.style.custom_instructions {
            prompt.push_str(&format!("\nADDITIONAL INSTRUCTIONS: {}\n", custom));
        }

        prompt
    }

    /// Build prompt for regenerating a specific section
    pub fn build_regeneration_prompt(
        original_text: &str,
        section: &str,
        feedback: Option<&str>,
    ) -> String {
        let feedback_str = feedback
            .map(|f| format!("\n\nUser feedback: {}", f))
            .unwrap_or_default();

        format!(
            "Here is an existing character backstory:\n\n\
             ---\n{original_text}\n---\n\n\
             Please rewrite the {section} section of this backstory.{feedback_str}\n\n\
             Keep the overall story consistent, but improve and expand this specific part.\n\
             Maintain the same voice and style as the original.\n\
             Return the COMPLETE updated backstory in the same JSON format."
        )
    }

    /// Build prompt for editing based on user feedback
    pub fn build_edit_prompt(original_text: &str, edit_instructions: &str) -> String {
        format!(
            "Here is an existing character backstory:\n\n\
             ---\n{original_text}\n---\n\n\
             Please modify the backstory according to these instructions:\n\
             {edit_instructions}\n\n\
             Maintain consistency with any elements not mentioned in the instructions.\n\
             Return the COMPLETE updated backstory in the same JSON format."
        )
    }

    /// Get genre-specific context for the system prompt
    fn get_genre_context(system: &GameSystem) -> String {
        match system {
            GameSystem::DnD5e => {
                "You are a master storyteller specializing in D&D 5th Edition character backstories.\n\
                 Draw from classic fantasy tropes: ancient prophecies, magical academies, noble houses,\n\
                 guild intrigue, monster-haunted wildernesses, and the eternal struggle between good and evil.\n\
                 Consider the Forgotten Realms as a default setting unless specified otherwise.\n\
                 Reference appropriate deities, factions, and iconic D&D elements.".to_string()
            }
            GameSystem::Pathfinder2e => {
                "You are a master storyteller specializing in Pathfinder 2nd Edition character backstories.\n\
                 Embrace Golarion's rich tapestry: the Inner Sea region, Absalom, the Pathfinder Society,\n\
                 diverse ancestries with unique cultures, and the complex pantheon of deities.\n\
                 Include references to appropriate nations, organizations, and Pathfinder lore.\n\
                 Balance high fantasy elements with the system's more tactical, grounded feel.".to_string()
            }
            GameSystem::CallOfCthulhu => {
                "You are a master storyteller specializing in Call of Cthulhu character backstories.\n\
                 Create investigators with mundane professions and hidden depths.\n\
                 Set stories in the 1920s by default (Jazz Age America, post-WWI Europe).\n\
                 Hint at cosmic horror without being explicit - the unknown should terrify.\n\
                 Focus on psychological complexity, academic pursuits, and creeping dread.\n\
                 Characters should feel like ordinary people who will face extraordinary horrors.".to_string()
            }
            GameSystem::Cyberpunk => {
                "You are a master storyteller specializing in Cyberpunk Red character backstories.\n\
                 Set in Night City, 2045 - after the Fourth Corporate War and the Time of the Red.\n\
                 Explore themes of corporate oppression, street survival, transhumanism, and rebellion.\n\
                 Include chrome (cyberware), gangs, fixers, corps, and the struggle for humanity.\n\
                 Balance gritty street-level stories with high-tech corporate intrigue.\n\
                 Reference Trauma Team, Militech, Arasaka, and other setting elements.".to_string()
            }
            GameSystem::Shadowrun => {
                "You are a master storyteller specializing in Shadowrun character backstories.\n\
                 In 2080, magic has returned and megacorporations rule. Characters are shadowrunners -\n\
                 deniable assets doing dirty work for those who can pay.\n\
                 Blend cyberpunk dystopia with urban fantasy: elves, dwarves, orks, trolls, dragons,\n\
                 shamans, deckers, and street samurai coexist in neon-lit sprawls.\n\
                 Reference the Big Ten megacorps, the Sixth World's history, and the shadows' culture.".to_string()
            }
            GameSystem::FateCore => {
                "You are a master storyteller specializing in Fate Core character backstories.\n\
                 Focus on dramatic moments and character-defining decisions that create Aspects.\n\
                 Structure the backstory around Phase Trio moments: the character's adventure,\n\
                 a crossing paths story, and another character connection.\n\
                 Emphasize relationships, beliefs, and dramatic potential over mechanics.\n\
                 Every element should be compellable or invokable in play.".to_string()
            }
            GameSystem::WorldOfDarkness => {
                "You are a master storyteller specializing in World of Darkness character backstories.\n\
                 Write for Chronicles of Darkness or classic WoD as appropriate to the character type.\n\
                 Balance mundane human elements with supernatural horror and personal tragedy.\n\
                 Focus on the monster within - the struggle to maintain humanity against the Beast.\n\
                 Include mortal life before the embrace/awakening/change, the transition trauma,\n\
                 and the character's place in supernatural society.".to_string()
            }
            GameSystem::DungeonWorld => {
                "You are a master storyteller specializing in Dungeon World character backstories.\n\
                 Write fiction-first narratives that set up dramatic questions for the table.\n\
                 Focus on bonds with other characters and open-ended threats.\n\
                 Leave blanks for collaborative worldbuilding - suggest rather than define.\n\
                 Emphasize the character's first adventure and why they became an adventurer.\n\
                 Keep it punchy and actionable for Powered by the Apocalypse play.".to_string()
            }
            GameSystem::GURPS => {
                "You are a master storyteller specializing in GURPS character backstories.\n\
                 Write detailed, realistic backstories that justify specific advantages and disadvantages.\n\
                 GURPS can cover any genre - match the tone to the campaign setting provided.\n\
                 Focus on concrete experiences that explain skills and abilities.\n\
                 Include specific training, mentors, and formative experiences.\n\
                 Balance point-buy mechanics with narrative coherence.".to_string()
            }
            GameSystem::Warhammer => {
                "You are a master storyteller specializing in Warhammer Fantasy Roleplay backstories.\n\
                 Write grimdark tales of the Old World - where life is brutal, short, and full of rats.\n\
                 Characters are humble folk: rat catchers, road wardens, grave robbers turned heroes.\n\
                 Include the Empire's provinces, Chaos corruption, Skaven schemes, and Sigmar's faith.\n\
                 Embrace dark humor and the absurdity of survival in a doomed world.\n\
                 Reference appropriate careers, social standing, and the ever-present threat of Chaos.".to_string()
            }
            GameSystem::Custom(name) => {
                format!(
                    "You are a master storyteller creating a character backstory for the {} system.\n\
                     Adapt your style to match any setting information provided.\n\
                     Focus on universal storytelling elements: motivation, conflict, relationships,\n\
                     and growth potential. Create hooks that work in any genre.",
                    name
                )
            }
        }
    }

    /// Get style-specific instructions
    fn get_style_instructions(style: &BackstoryStyle) -> String {
        let mut instructions = Vec::new();

        match style.tone.as_deref() {
            Some("heroic") => instructions.push(
                "TONE: Write an inspiring, heroic narrative. The character overcomes adversity \
                 through courage and virtue. Include moments of triumph and noble sacrifice.".to_string()
            ),
            Some("tragic") => instructions.push(
                "TONE: Write a tragic narrative marked by loss and sacrifice. The character has \
                 experienced significant hardship. Balance pathos with resilience.".to_string()
            ),
            Some("comedic") => instructions.push(
                "TONE: Write with wit and humor. Include absurd situations, ironic twists, and \
                 lighthearted moments. The character's journey has comedic elements without \
                 becoming parody.".to_string()
            ),
            Some("mysterious") => instructions.push(
                "TONE: Write with an air of mystery. Leave questions unanswered, hint at secrets, \
                 and include unexplained events. The character has hidden depths to explore.".to_string()
            ),
            Some("gritty") => instructions.push(
                "TONE: Write a realistic, grounded narrative. The character has faced hard choices \
                 with real consequences. Include moral ambiguity and practical concerns.".to_string()
            ),
            Some("dark") => instructions.push(
                "TONE: Write a dark, brooding narrative. The character has witnessed or done \
                 terrible things. Explore themes of corruption, violence, and moral compromise.".to_string()
            ),
            Some("epic") => instructions.push(
                "TONE: Write a sweeping, epic narrative. The character is destined for great things \
                 and their past reflects that potential. Include grand events and fateful meetings.".to_string()
            ),
            Some(custom) => instructions.push(format!("TONE: {}", custom)),
            None => {}
        }

        match style.perspective.as_deref() {
            Some("first_person") => instructions.push(
                "PERSPECTIVE: Write in first person, as if the character is telling their own story. \
                 Use 'I' and 'me' throughout. Include personal reflections and direct emotional content.".to_string()
            ),
            Some("third_person") => instructions.push(
                "PERSPECTIVE: Write in third person, as a narrator describing the character's history. \
                 Use the character's name and 'they/them' pronouns as appropriate.".to_string()
            ),
            Some("journal") => instructions.push(
                "PERSPECTIVE: Write as journal entries or letters. Include dates where appropriate. \
                 Let the character's voice come through in the writing style.".to_string()
            ),
            _ => {}
        }

        match style.focus.as_deref() {
            Some("personal") => instructions.push(
                "FOCUS: Emphasize personal relationships, family, and emotional growth. \
                 The character's story is about who they love and who they've lost.".to_string()
            ),
            Some("political") => instructions.push(
                "FOCUS: Emphasize political intrigue, faction allegiances, and power dynamics. \
                 The character is connected to larger forces and conflicts.".to_string()
            ),
            Some("adventurous") => instructions.push(
                "FOCUS: Emphasize action, exploration, and daring deeds. \
                 The character has led an exciting life full of narrow escapes.".to_string()
            ),
            Some("philosophical") => instructions.push(
                "FOCUS: Emphasize the character's beliefs, moral struggles, and search for meaning. \
                 Include moments of reflection and philosophical turning points.".to_string()
            ),
            Some("professional") => instructions.push(
                "FOCUS: Emphasize the character's career, training, and professional development. \
                 How did they become so skilled? Who taught them?".to_string()
            ),
            Some(custom) => instructions.push(format!("FOCUS: {}", custom)),
            _ => {}
        }

        if instructions.is_empty() {
            "STYLE: Write in a balanced narrative style, mixing action with reflection, \
             and personal moments with broader context.".to_string()
        } else {
            instructions.join("\n\n")
        }
    }

    /// Get campaign-specific context
    fn get_campaign_context(campaign_setting: &Option<String>) -> String {
        match campaign_setting {
            Some(setting) if !setting.is_empty() => {
                format!(
                    "CAMPAIGN CONTEXT:\n\
                     The character exists within this campaign setting:\n\
                     {}\n\n\
                     Ensure the backstory fits naturally within this world and its conventions. \
                     Reference specific locations, factions, or events from the setting where appropriate.",
                    setting
                )
            }
            _ => {
                "CAMPAIGN CONTEXT: No specific campaign setting provided. Create a backstory \
                 that can be easily adapted to common settings for this game system.".to_string()
            }
        }
    }
}

/// Predefined backstory templates for quick generation
pub struct BackstoryTemplates;

impl BackstoryTemplates {
    /// Get a backstory structure template based on genre
    pub fn get_structure_template(system: &GameSystem) -> Vec<&'static str> {
        match system {
            GameSystem::DnD5e | GameSystem::Pathfinder2e | GameSystem::DungeonWorld => vec![
                "Early life and upbringing",
                "Discovery of abilities or calling",
                "Mentor or formative relationship",
                "First real adventure or trial",
                "Tragedy or triumph that defined them",
                "Why they adventure now",
                "A secret or unresolved conflict",
            ],
            GameSystem::CallOfCthulhu => vec![
                "Professional background and education",
                "The normal life before",
                "First brush with the unknown",
                "Growing awareness of cosmic truths",
                "What drives them to investigate",
                "Connections to other investigators",
                "A memory they'd rather forget",
            ],
            GameSystem::Cyberpunk | GameSystem::Shadowrun => vec![
                "Life before the streets/shadows",
                "The fall or the choice that changed everything",
                "First job or run",
                "A betrayal or hard lesson",
                "Notable chrome/augmentation (if any) and why",
                "Current reputation and connections",
                "What they're working toward",
            ],
            GameSystem::WorldOfDarkness => vec![
                "Mortal life before the change",
                "The embrace/awakening/first change",
                "Coming to terms with what they are",
                "Finding their place in supernatural society",
                "The humanity/balance they struggle to maintain",
                "Unfinished business from their mortal life",
                "What anchors them to humanity",
            ],
            GameSystem::FateCore => vec![
                "Their first adventure (establishing High Concept)",
                "A story with another PC (first crossing paths)",
                "Another character connection (second crossing paths)",
                "What they want more than anything",
                "What gets them into trouble",
            ],
            GameSystem::Warhammer => vec![
                "Birth and early years in the Old World",
                "How they came to their first career",
                "A brush with Chaos or the supernatural",
                "The event that made them leave their old life",
                "Scars both physical and mental",
                "What they hope to achieve or escape",
                "Their relationship with faith and Sigmar",
            ],
            GameSystem::GURPS | GameSystem::Custom(_) => vec![
                "Origins and early life",
                "Education and training",
                "Formative experiences",
                "Career and achievements",
                "Relationships and connections",
                "Current situation and goals",
                "Secrets and complications",
            ],
        }
    }

    /// Get example plot hooks for a genre
    pub fn get_example_hooks(system: &GameSystem) -> Vec<&'static str> {
        match system {
            GameSystem::DnD5e | GameSystem::Pathfinder2e => vec![
                "A family heirloom was stolen by a known thieves' guild",
                "Their mentor disappeared while investigating ancient ruins",
                "A childhood friend has become involved with a dark cult",
                "They owe a significant debt to a powerful noble",
                "Visions of a coming catastrophe haunt their dreams",
                "An old enemy has resurfaced with new allies",
                "They're the last known heir to a disputed title",
            ],
            GameSystem::CallOfCthulhu => vec![
                "Strange letters arrive from a colleague who went missing years ago",
                "Their research has attracted the attention of a secretive organization",
                "A family member's death revealed disturbing documents",
                "They witnessed something they can't explain and can't forget",
                "An artifact they acquired is changing them in subtle ways",
                "Someone is following them, leaving cryptic warnings",
            ],
            GameSystem::Cyberpunk | GameSystem::Shadowrun => vec![
                "They have data that a megacorp wants buried",
                "A loved one is being held as leverage by a fixer",
                "Their old crew is being hunted and eliminated one by one",
                "A ghost in the machine knows their real identity",
                "They need a specific piece of cyberware to survive",
                "Someone framed them for a job gone wrong",
            ],
            _ => vec![
                "Unfinished business with a former ally",
                "A secret that could destroy them if revealed",
                "A promise made long ago that must be kept",
                "Someone from their past has returned",
                "They possess something others desperately want",
                "A mystery from their past demands answers",
            ],
        }
    }

    /// Get opening hooks for different tones
    pub fn get_opening_hook(tone: Option<&str>) -> &'static str {
        match tone {
            Some("heroic") => "From humble beginnings came one destined for greatness...",
            Some("tragic") => "In the end, all that remained were memories of what was lost...",
            Some("comedic") => "It all started with a misunderstanding and a stolen goat...",
            Some("mysterious") => "There are things in the past best left buried, but some graves refuse to stay closed...",
            Some("gritty") => "Survival was never guaranteed, only earned through blood and compromise...",
            Some("dark") => "The path to this moment was paved with choices that cannot be undone...",
            Some("epic") => "When the stars aligned and fate drew its first breath, a story began...",
            _ => "Every hero's journey begins with a single step into the unknown...",
        }
    }
}

/// Token estimation for backstory lengths
pub fn estimate_tokens(length: &BackstoryLength) -> u32 {
    match length {
        BackstoryLength::Brief => 800,    // ~50-100 words + JSON overhead
        BackstoryLength::Medium => 1500,  // ~150-300 words + JSON overhead
        BackstoryLength::Detailed => 2500, // ~400-600 words + JSON overhead
    }
}

/// Get recommended temperature for backstory generation
pub fn recommended_temperature(length: &BackstoryLength) -> f32 {
    match length {
        BackstoryLength::Brief => 0.7,    // More focused for brevity
        BackstoryLength::Medium => 0.8,   // Balanced creativity
        BackstoryLength::Detailed => 0.85, // More creative for longer content
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::character_gen::{Character, CharacterBackground, GameSystem};

    fn create_test_character() -> Character {
        Character {
            id: "test-id".to_string(),
            name: "Aldric Stormwind".to_string(),
            system: GameSystem::DnD5e,
            concept: "A battle-hardened veteran seeking redemption".to_string(),
            race: Some("Human".to_string()),
            class: Some("Paladin".to_string()),
            level: 5,
            attributes: std::collections::HashMap::new(),
            skills: std::collections::HashMap::new(),
            traits: vec![],
            equipment: vec![],
            background: CharacterBackground {
                origin: "Soldier".to_string(),
                occupation: Some("Former knight".to_string()),
                motivation: "Atone for past failures".to_string(),
                connections: vec!["Order of the Silver Dawn".to_string()],
                secrets: vec![],
                history: String::new(),
            },
            backstory: None,
            notes: String::new(),
            portrait_prompt: None,
        }
    }

    #[test]
    fn test_system_prompt_generation() {
        let char = create_test_character();
        let request = BackstoryRequest {
            character: char,
            length: BackstoryLength::Medium,
            campaign_setting: Some("Forgotten Realms - Sword Coast".to_string()),
            style: BackstoryStyle::default(),
            include_elements: vec![],
            exclude_elements: vec![],
        };

        let prompt = BackstoryPromptBuilder::build_system_prompt(&request);
        assert!(prompt.contains("D&D 5th Edition"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_user_prompt_generation() {
        let char = create_test_character();
        let request = BackstoryRequest {
            character: char,
            length: BackstoryLength::Medium,
            campaign_setting: None,
            style: BackstoryStyle {
                tone: Some("heroic".to_string()),
                perspective: Some("third_person".to_string()),
                focus: None,
                custom_instructions: None,
            },
            include_elements: vec!["dragons".to_string()],
            exclude_elements: vec!["orcs".to_string()],
        };

        let prompt = BackstoryPromptBuilder::build_user_prompt(&request);
        assert!(prompt.contains("Aldric Stormwind"));
        assert!(prompt.contains("Paladin"));
        assert!(prompt.contains("dragons"));
        assert!(prompt.contains("orcs"));
    }

    #[test]
    fn test_structure_templates() {
        let dnd_structure = BackstoryTemplates::get_structure_template(&GameSystem::DnD5e);
        assert!(!dnd_structure.is_empty());

        let coc_structure = BackstoryTemplates::get_structure_template(&GameSystem::CallOfCthulhu);
        assert!(coc_structure.iter().any(|s| s.contains("unknown")));
    }

    #[test]
    fn test_token_estimation() {
        assert!(estimate_tokens(&BackstoryLength::Brief) < estimate_tokens(&BackstoryLength::Medium));
        assert!(estimate_tokens(&BackstoryLength::Medium) < estimate_tokens(&BackstoryLength::Detailed));
    }
}
