//! NPC Generator Module
//!
//! Generates non-player characters with rich personalities, motivations,
//! and connections to the game world.

use crate::core::character_gen::{Character, GenerationOptions, CharacterGenerator};
use crate::core::llm::{LLMClient, LLMConfig, ChatMessage, ChatRequest, MessageRole};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::Rng;
use uuid::Uuid;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum NPCGenError {
    #[error("Generation failed: {0}")]
    GenerationFailed(String),

    #[error("LLM error: {0}")]
    LLMError(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),
}

pub type Result<T> = std::result::Result<T, NPCGenError>;

// ============================================================================
// NPC Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPC {
    pub id: String,
    pub name: String,
    pub role: NPCRole,
    pub appearance: AppearanceDescription,
    pub personality: NPCPersonality,
    pub personality_id: Option<String>,
    pub voice: VoiceDescription,
    pub stats: Option<Character>,
    pub relationships: Vec<NPCRelationship>,
    pub secrets: Vec<String>,
    pub hooks: Vec<PlotHook>,
    pub notes: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NPCRole {
    Ally,
    Enemy,
    Neutral,
    Merchant,
    QuestGiver,
    Authority,
    Informant,
    Rival,
    Mentor,
    Minion,
    Boss,
    Bystander,
    Custom(String),
}

impl NPCRole {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "ally" => Self::Ally,
            "enemy" => Self::Enemy,
            "neutral" => Self::Neutral,
            "merchant" | "shopkeeper" | "vendor" => Self::Merchant,
            "questgiver" | "quest giver" | "patron" => Self::QuestGiver,
            "authority" | "guard" | "official" => Self::Authority,
            "informant" | "spy" | "contact" => Self::Informant,
            "rival" => Self::Rival,
            "mentor" | "teacher" => Self::Mentor,
            "minion" | "henchman" => Self::Minion,
            "boss" | "villain" | "bbeg" => Self::Boss,
            "bystander" | "commoner" => Self::Bystander,
            other => Self::Custom(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceDescription {
    pub age: String,
    pub height: String,
    pub build: String,
    pub hair: String,
    pub eyes: String,
    pub skin: String,
    pub distinguishing_features: Vec<String>,
    pub clothing: String,
    pub demeanor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPCPersonality {
    pub traits: Vec<String>,
    pub ideals: Vec<String>,
    pub bonds: Vec<String>,
    pub flaws: Vec<String>,
    pub mannerisms: Vec<String>,
    pub speech_patterns: Vec<String>,
    pub motivations: Vec<String>,
    pub fears: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceDescription {
    pub pitch: String,
    pub pace: String,
    pub accent: Option<String>,
    pub vocabulary: String,
    pub sample_phrases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NPCRelationship {
    pub target_id: Option<String>,
    pub target_name: String,
    pub relationship_type: String,
    pub disposition: i32, // -100 to 100
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotHook {
    pub description: String,
    pub hook_type: PlotHookType,
    pub urgency: Urgency,
    pub reward_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlotHookType {
    Quest,
    Rumor,
    Secret,
    Conflict,
    Opportunity,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Urgency {
    Low,
    Medium,
    High,
    Critical,
}

// ============================================================================
// Generation Options
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NPCGenerationOptions {
    pub system: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub race: Option<String>,
    pub occupation: Option<String>,
    pub location: Option<String>,
    pub theme: Option<String>,
    pub generate_stats: bool,
    pub generate_backstory: bool,
    pub personality_depth: PersonalityDepth,
    pub include_hooks: bool,
    pub include_secrets: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum PersonalityDepth {
    #[default]
    Basic,
    Standard,
    Detailed,
    Comprehensive,
}

// ============================================================================
// NPC Generator
// ============================================================================

pub struct NPCGenerator {
    llm_client: Option<LLMClient>,
}

impl Default for NPCGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl NPCGenerator {
    pub fn new() -> Self {
        Self { llm_client: None }
    }

    pub fn with_llm(llm_config: LLMConfig) -> Self {
        Self {
            llm_client: Some(LLMClient::new(llm_config)),
        }
    }

    /// Generate a quick NPC without LLM
    pub fn generate_quick(&self, options: &NPCGenerationOptions) -> NPC {
        let mut rng = rand::thread_rng();

        let name = options.name.clone()
            .unwrap_or_else(|| self.random_name(&mut rng, options.race.as_deref()));

        let role = options.role.as_deref()
            .map(NPCRole::from_str)
            .unwrap_or(NPCRole::Neutral);

        let appearance = self.generate_appearance(&mut rng, options);
        let personality = self.generate_personality(&mut rng, &role, options.personality_depth.clone());
        let voice = self.generate_voice(&mut rng, &personality);

        let stats = if options.generate_stats {
            let char_options = GenerationOptions {
                system: options.system.clone(),
                name: Some(name.clone()),
                race: options.race.clone(),
                class: options.occupation.clone(),
                random_stats: true,
                include_equipment: true,
                ..Default::default()
            };
            CharacterGenerator::generate(&char_options).ok()
        } else {
            None
        };

        let secrets = if options.include_secrets {
            self.generate_secrets(&mut rng, &role)
        } else {
            vec![]
        };

        let hooks = if options.include_hooks {
            self.generate_plot_hooks(&mut rng, &role)
        } else {
            vec![]
        };

        NPC {
            id: Uuid::new_v4().to_string(),
            name,
            role,
            appearance,
            personality,
            personality_id: None,
            voice,
            stats,
            relationships: vec![],
            secrets,
            hooks,
            notes: String::new(),
            tags: vec![],
        }
    }

    /// Generate a detailed NPC using LLM
    pub async fn generate_detailed(&self, options: &NPCGenerationOptions) -> Result<NPC> {
        let llm = self.llm_client.as_ref()
            .ok_or_else(|| NPCGenError::GenerationFailed("No LLM configured".to_string()))?;

        let prompt = self.build_generation_prompt(options);

        let request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: MessageRole::User,
                    content: prompt,
                    images: None,
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
            ],
            system_prompt: Some("You are a creative TTRPG game master assistant specializing in NPC creation. Generate detailed, memorable NPCs with rich personalities and interesting hooks. Always respond with valid JSON.".to_string()),
            temperature: Some(0.8),
            max_tokens: Some(2000),
            provider: None,
            tools: None,
            tool_choice: None,
        };

        let response = llm.chat(request).await
            .map_err(|e| NPCGenError::LLMError(e.to_string()))?;

        self.parse_npc_response(&response.content, options)
    }

    fn build_generation_prompt(&self, options: &NPCGenerationOptions) -> String {
        let mut prompt = String::from("Generate a detailed NPC with the following parameters:\n\n");

        if let Some(system) = &options.system {
            prompt.push_str(&format!("Game System: {}\n", system));
        }

        if let Some(role) = &options.role {
            prompt.push_str(&format!("Role: {}\n", role));
        }

        if let Some(race) = &options.race {
            prompt.push_str(&format!("Race/Species: {}\n", race));
        }

        if let Some(occupation) = &options.occupation {
            prompt.push_str(&format!("Occupation: {}\n", occupation));
        }

        if let Some(location) = &options.location {
            prompt.push_str(&format!("Location: {}\n", location));
        }

        if let Some(theme) = &options.theme {
            prompt.push_str(&format!("Theme/Tone: {}\n", theme));
        }

        prompt.push_str(&format!("\nPersonality Depth: {:?}\n", options.personality_depth));
        prompt.push_str(&format!("Include Plot Hooks: {}\n", options.include_hooks));
        prompt.push_str(&format!("Include Secrets: {}\n", options.include_secrets));

        prompt.push_str(r#"

Respond with a JSON object containing:
{
  "name": "string",
  "appearance": {
    "age": "string",
    "height": "string",
    "build": "string",
    "hair": "string",
    "eyes": "string",
    "skin": "string",
    "distinguishing_features": ["string"],
    "clothing": "string",
    "demeanor": "string"
  },
  "personality": {
    "traits": ["string"],
    "ideals": ["string"],
    "bonds": ["string"],
    "flaws": ["string"],
    "mannerisms": ["string"],
    "speech_patterns": ["string"],
    "motivations": ["string"],
    "fears": ["string"]
  },
  "voice": {
    "pitch": "string",
    "pace": "string",
    "accent": "string or null",
    "vocabulary": "string",
    "sample_phrases": ["string"]
  },
  "secrets": ["string"],
  "hooks": [{"description": "string", "hook_type": "Quest|Rumor|Secret|Conflict|Opportunity|Warning", "urgency": "Low|Medium|High|Critical", "reward_hint": "string or null"}]
}
"#);

        prompt
    }

    fn parse_npc_response(&self, response: &str, options: &NPCGenerationOptions) -> Result<NPC> {
        // Try to extract JSON from the response
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| NPCGenError::GenerationFailed(format!("Failed to parse response: {}", e)))?;

        let role = options.role.as_deref()
            .map(NPCRole::from_str)
            .unwrap_or(NPCRole::Neutral);

        Ok(NPC {
            id: Uuid::new_v4().to_string(),
            name: parsed["name"].as_str().unwrap_or("Unknown").to_string(),
            role,
            appearance: self.parse_appearance(&parsed["appearance"]),
            personality: self.parse_personality(&parsed["personality"]),
            personality_id: None,
            voice: self.parse_voice(&parsed["voice"]),
            stats: None,
            relationships: vec![],
            secrets: parsed["secrets"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            hooks: self.parse_hooks(&parsed["hooks"]),
            notes: String::new(),
            tags: vec![],
        })
    }

    fn parse_appearance(&self, value: &serde_json::Value) -> AppearanceDescription {
        AppearanceDescription {
            age: value["age"].as_str().unwrap_or("Adult").to_string(),
            height: value["height"].as_str().unwrap_or("Average").to_string(),
            build: value["build"].as_str().unwrap_or("Medium").to_string(),
            hair: value["hair"].as_str().unwrap_or("Brown").to_string(),
            eyes: value["eyes"].as_str().unwrap_or("Brown").to_string(),
            skin: value["skin"].as_str().unwrap_or("Fair").to_string(),
            distinguishing_features: value["distinguishing_features"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            clothing: value["clothing"].as_str().unwrap_or("Common clothes").to_string(),
            demeanor: value["demeanor"].as_str().unwrap_or("Neutral").to_string(),
        }
    }

    fn parse_personality(&self, value: &serde_json::Value) -> NPCPersonality {
        let parse_array = |key: &str| -> Vec<String> {
            value[key].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default()
        };

        NPCPersonality {
            traits: parse_array("traits"),
            ideals: parse_array("ideals"),
            bonds: parse_array("bonds"),
            flaws: parse_array("flaws"),
            mannerisms: parse_array("mannerisms"),
            speech_patterns: parse_array("speech_patterns"),
            motivations: parse_array("motivations"),
            fears: parse_array("fears"),
        }
    }

    fn parse_voice(&self, value: &serde_json::Value) -> VoiceDescription {
        VoiceDescription {
            pitch: value["pitch"].as_str().unwrap_or("Medium").to_string(),
            pace: value["pace"].as_str().unwrap_or("Normal").to_string(),
            accent: value["accent"].as_str().map(String::from),
            vocabulary: value["vocabulary"].as_str().unwrap_or("Common").to_string(),
            sample_phrases: value["sample_phrases"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        }
    }

    fn parse_hooks(&self, value: &serde_json::Value) -> Vec<PlotHook> {
        value.as_array()
            .map(|arr| {
                arr.iter().filter_map(|v| {
                    Some(PlotHook {
                        description: v["description"].as_str()?.to_string(),
                        hook_type: match v["hook_type"].as_str().unwrap_or("Quest") {
                            "Quest" => PlotHookType::Quest,
                            "Rumor" => PlotHookType::Rumor,
                            "Secret" => PlotHookType::Secret,
                            "Conflict" => PlotHookType::Conflict,
                            "Opportunity" => PlotHookType::Opportunity,
                            "Warning" => PlotHookType::Warning,
                            _ => PlotHookType::Quest,
                        },
                        urgency: match v["urgency"].as_str().unwrap_or("Medium") {
                            "Low" => Urgency::Low,
                            "High" => Urgency::High,
                            "Critical" => Urgency::Critical,
                            _ => Urgency::Medium,
                        },
                        reward_hint: v["reward_hint"].as_str().map(String::from),
                    })
                }).collect()
            })
            .unwrap_or_default()
    }

    // ========================================================================
    // Random Generation Helpers
    // ========================================================================

    fn random_name(&self, rng: &mut impl Rng, race: Option<&str>) -> String {
        let first_names = match race {
            Some("Elf") | Some("elf") => vec![
                "Aelindra", "Caelynn", "Eryndor", "Faelar", "Galinndan",
                "Liadon", "Mirthal", "Naivara", "Siannodel", "Thamior",
            ],
            Some("Dwarf") | Some("dwarf") => vec![
                "Barendd", "Dain", "Eberk", "Gardain", "Harbek",
                "Kildrak", "Morgran", "Orsik", "Thoradin", "Vondal",
            ],
            Some("Halfling") | Some("halfling") => vec![
                "Alton", "Cade", "Eldon", "Garret", "Lyle",
                "Milo", "Osborn", "Roscoe", "Wellby", "Wendel",
            ],
            _ => vec![
                "Marcus", "Elena", "Theron", "Lyra", "Cedric",
                "Mirabel", "Aldric", "Seraphina", "Victor", "Adelaide",
            ],
        };

        let last_names = vec![
            "Blackwood", "Ironforge", "Silverleaf", "Thornwood", "Stormwind",
            "Ravencrest", "Goldstein", "Darkhollow", "Brightwater", "Shadowmere",
        ];

        format!("{} {}",
            first_names[rng.gen_range(0..first_names.len())],
            last_names[rng.gen_range(0..last_names.len())]
        )
    }

    fn generate_appearance(&self, rng: &mut impl Rng, options: &NPCGenerationOptions) -> AppearanceDescription {
        let ages = ["Young adult", "Adult", "Middle-aged", "Elderly"];
        let heights = ["Short", "Average height", "Tall", "Very tall"];
        let builds = ["Thin", "Average", "Athletic", "Heavyset", "Muscular"];
        let hair_colors = ["Black", "Brown", "Blonde", "Red", "Gray", "White", "Bald"];
        let eye_colors = ["Brown", "Blue", "Green", "Gray", "Hazel", "Amber"];
        let demeanors = ["Friendly", "Suspicious", "Tired", "Alert", "Bored", "Nervous"];

        let features = vec![
            "Scar on left cheek",
            "Missing finger",
            "Tattoo on arm",
            "Eye patch",
            "Prominent nose",
            "Gap in teeth",
            "Birthmark on neck",
            "Calloused hands",
            "Crooked smile",
            "Piercing gaze",
        ];

        let mut distinguishing = vec![];
        if rng.gen_bool(0.5) {
            distinguishing.push(features[rng.gen_range(0..features.len())].to_string());
        }

        AppearanceDescription {
            age: ages[rng.gen_range(0..ages.len())].to_string(),
            height: heights[rng.gen_range(0..heights.len())].to_string(),
            build: builds[rng.gen_range(0..builds.len())].to_string(),
            hair: hair_colors[rng.gen_range(0..hair_colors.len())].to_string(),
            eyes: eye_colors[rng.gen_range(0..eye_colors.len())].to_string(),
            skin: "Fair".to_string(),
            distinguishing_features: distinguishing,
            clothing: self.random_clothing(rng, options.occupation.as_deref()),
            demeanor: demeanors[rng.gen_range(0..demeanors.len())].to_string(),
        }
    }

    fn random_clothing(&self, rng: &mut impl Rng, occupation: Option<&str>) -> String {
        match occupation {
            Some("Merchant") | Some("merchant") => "Fine merchant's robes with a coin purse",
            Some("Guard") | Some("guard") => "Worn leather armor with a city emblem",
            Some("Noble") | Some("noble") => "Elegant silk garments with gold trim",
            Some("Peasant") | Some("peasant") => "Simple homespun clothes, patched",
            Some("Priest") | Some("priest") => "Religious vestments with holy symbols",
            Some("Thief") | Some("thief") => "Dark, practical clothing with many pockets",
            _ => {
                let options = [
                    "Common traveling clothes",
                    "Work-worn practical garments",
                    "Clean but simple attire",
                    "Weather-beaten cloak over sturdy clothes",
                ];
                options[rng.gen_range(0..options.len())]
            }
        }.to_string()
    }

    fn generate_personality(&self, rng: &mut impl Rng, role: &NPCRole, depth: PersonalityDepth) -> NPCPersonality {
        let trait_count = match depth {
            PersonalityDepth::Basic => 1,
            PersonalityDepth::Standard => 2,
            PersonalityDepth::Detailed => 3,
            PersonalityDepth::Comprehensive => 4,
        };

        let all_traits = [
            "Honest", "Greedy", "Cautious", "Bold", "Friendly", "Suspicious",
            "Curious", "Pessimistic", "Optimistic", "Sarcastic", "Generous",
            "Paranoid", "Patient", "Impulsive", "Loyal", "Deceitful",
        ];

        let all_ideals = [
            "Justice above all", "Wealth opens doors", "Knowledge is power",
            "Family comes first", "Freedom is precious", "Order maintains peace",
            "Power through strength", "Kindness costs nothing",
        ];

        let all_bonds = [
            "Family back home", "A mentor who taught everything",
            "A rival who must be surpassed", "A debt that must be repaid",
            "A lost love never forgotten", "A homeland that needs protection",
        ];

        let all_flaws = [
            "Drinks too much", "Trusts too easily", "Quick to anger",
            "Holds grudges", "Addicted to gambling", "Proud to a fault",
            "Tells lies to avoid conflict", "Cowardly when threatened",
        ];

        let mannerisms_by_role: Vec<&str> = match role {
            NPCRole::Merchant => vec!["Constantly counting coins", "Eyes potential customers", "Haggling reflex"],
            NPCRole::Authority => vec!["Formal stance", "Evaluating gaze", "Commands attention"],
            NPCRole::Informant => vec!["Whispers unnecessarily", "Checks for eavesdroppers", "Nervous twitches"],
            _ => vec!["Scratches chin when thinking", "Avoids eye contact", "Fidgets with hands"],
        };

        let traits: Vec<String> = (0..trait_count)
            .map(|_| all_traits[rng.gen_range(0..all_traits.len())].to_string())
            .collect();

        NPCPersonality {
            traits,
            ideals: vec![all_ideals[rng.gen_range(0..all_ideals.len())].to_string()],
            bonds: vec![all_bonds[rng.gen_range(0..all_bonds.len())].to_string()],
            flaws: vec![all_flaws[rng.gen_range(0..all_flaws.len())].to_string()],
            mannerisms: vec![mannerisms_by_role[rng.gen_range(0..mannerisms_by_role.len())].to_string()],
            speech_patterns: vec![self.random_speech_pattern(rng)],
            motivations: vec![self.random_motivation(rng, role)],
            fears: vec![self.random_fear(rng)],
        }
    }

    fn random_speech_pattern(&self, rng: &mut impl Rng) -> String {
        let patterns = [
            "Speaks slowly and deliberately",
            "Uses big words incorrectly",
            "Ends sentences with questions",
            "Frequently clears throat",
            "Speaks in third person occasionally",
            "Uses old-fashioned expressions",
            "Mumbles when nervous",
            "Speaks rapidly when excited",
        ];
        patterns[rng.gen_range(0..patterns.len())].to_string()
    }

    fn random_motivation(&self, rng: &mut impl Rng, role: &NPCRole) -> String {
        let motivations = match role {
            NPCRole::Merchant => vec!["Profit and expansion", "Paying off debts", "Providing for family"],
            NPCRole::Enemy => vec!["Revenge for past wrongs", "Power at any cost", "Proving superiority"],
            NPCRole::QuestGiver => vec!["Solving a problem", "Recovering something lost", "Protecting the community"],
            NPCRole::Authority => vec!["Maintaining order", "Climbing the ranks", "Serving justice"],
            _ => vec!["Survival", "Finding purpose", "Protecting loved ones", "Seeking adventure"],
        };
        motivations[rng.gen_range(0..motivations.len())].to_string()
    }

    fn random_fear(&self, rng: &mut impl Rng) -> String {
        let fears = [
            "Being forgotten", "Losing control", "Death of loved ones",
            "Poverty", "The dark", "Being discovered", "Magic",
            "Monsters", "Crowds", "Being alone", "Failure",
        ];
        fears[rng.gen_range(0..fears.len())].to_string()
    }

    fn generate_voice(&self, rng: &mut impl Rng, _personality: &NPCPersonality) -> VoiceDescription {
        let pitches = ["Low", "Medium", "High", "Gravelly", "Melodic"];
        let paces = ["Slow", "Normal", "Fast", "Halting", "Measured"];
        let accents = [
            None, Some("Regional"), Some("Foreign"), Some("Noble"),
            Some("Rural"), Some("Street"), Some("Academic"),
        ];
        let vocabularies = [
            "Simple", "Common", "Educated", "Refined", "Street slang", "Technical",
        ];

        let sample_phrases = vec![
            "Well now, that's interesting...",
            "I've seen things, friend.",
            "Coin first, information later.",
            "You don't want to know.",
        ];

        VoiceDescription {
            pitch: pitches[rng.gen_range(0..pitches.len())].to_string(),
            pace: paces[rng.gen_range(0..paces.len())].to_string(),
            accent: accents[rng.gen_range(0..accents.len())].map(String::from),
            vocabulary: vocabularies[rng.gen_range(0..vocabularies.len())].to_string(),
            sample_phrases: vec![sample_phrases[rng.gen_range(0..sample_phrases.len())].to_string()],
        }
    }

    fn generate_secrets(&self, rng: &mut impl Rng, role: &NPCRole) -> Vec<String> {
        let secrets = match role {
            NPCRole::Merchant => vec![
                "Secretly deals in stolen goods",
                "Owes money to dangerous people",
                "Has a hidden second family",
            ],
            NPCRole::Authority => vec![
                "Takes bribes from criminals",
                "Is actually a spy for another faction",
                "Killed someone in their past",
            ],
            NPCRole::Enemy => vec![
                "Has a tragic backstory that explains their villainy",
                "Is being blackmailed into their actions",
                "Secretly loves one of the party members' allies",
            ],
            _ => vec![
                "Has a hidden talent",
                "Is not who they claim to be",
                "Witnessed something they shouldn't have",
            ],
        };

        if rng.gen_bool(0.6) {
            vec![secrets[rng.gen_range(0..secrets.len())].to_string()]
        } else {
            vec![]
        }
    }

    fn generate_plot_hooks(&self, rng: &mut impl Rng, role: &NPCRole) -> Vec<PlotHook> {
        let hooks = match role {
            NPCRole::QuestGiver => vec![
                PlotHook {
                    description: "Needs someone to retrieve a stolen heirloom".to_string(),
                    hook_type: PlotHookType::Quest,
                    urgency: Urgency::Medium,
                    reward_hint: Some("Gold and a favor".to_string()),
                },
                PlotHook {
                    description: "Has information about a local threat".to_string(),
                    hook_type: PlotHookType::Rumor,
                    urgency: Urgency::High,
                    reward_hint: None,
                },
            ],
            NPCRole::Merchant => vec![
                PlotHook {
                    description: "Shipment went missing on the road".to_string(),
                    hook_type: PlotHookType::Quest,
                    urgency: Urgency::Medium,
                    reward_hint: Some("Discount on future purchases".to_string()),
                },
            ],
            NPCRole::Informant => vec![
                PlotHook {
                    description: "Has heard whispers about something big".to_string(),
                    hook_type: PlotHookType::Secret,
                    urgency: Urgency::Low,
                    reward_hint: None,
                },
            ],
            _ => vec![
                PlotHook {
                    description: "Mentions something unusual they witnessed".to_string(),
                    hook_type: PlotHookType::Rumor,
                    urgency: Urgency::Low,
                    reward_hint: None,
                },
            ],
        };

        if rng.gen_bool(0.7) {
            vec![hooks[rng.gen_range(0..hooks.len())].clone()]
        } else {
            vec![]
        }
    }
}

// ============================================================================
// NPC Store
// ============================================================================

use std::sync::RwLock;

pub struct NPCStore {
    npcs: RwLock<HashMap<String, NPC>>,
    by_campaign: RwLock<HashMap<String, Vec<String>>>,
}

impl Default for NPCStore {
    fn default() -> Self {
        Self::new()
    }
}

impl NPCStore {
    pub fn new() -> Self {
        Self {
            npcs: RwLock::new(HashMap::new()),
            by_campaign: RwLock::new(HashMap::new()),
        }
    }

    pub fn add(&self, npc: NPC, campaign_id: Option<&str>) {
        let id = npc.id.clone();
        self.npcs.write().unwrap().insert(id.clone(), npc);

        if let Some(cid) = campaign_id {
            self.by_campaign.write().unwrap()
                .entry(cid.to_string())
                .or_default()
                .push(id);
        }
    }

    pub fn get(&self, id: &str) -> Option<NPC> {
        self.npcs.read().unwrap().get(id).cloned()
    }

    pub fn list(&self, campaign_id: Option<&str>) -> Vec<NPC> {
        let npcs = self.npcs.read().unwrap();

        match campaign_id {
            Some(cid) => {
                self.by_campaign.read().unwrap()
                    .get(cid)
                    .map(|ids| {
                        ids.iter()
                            .filter_map(|id| npcs.get(id).cloned())
                            .collect()
                    })
                    .unwrap_or_default()
            }
            None => npcs.values().cloned().collect(),
        }
    }

    pub fn update(&self, npc: NPC) {
        self.npcs.write().unwrap().insert(npc.id.clone(), npc);
    }

    pub fn delete(&self, id: &str) {
        self.npcs.write().unwrap().remove(id);

        // Remove from campaign associations
        let mut by_campaign = self.by_campaign.write().unwrap();
        for ids in by_campaign.values_mut() {
            ids.retain(|i| i != id);
        }
    }

    pub fn search(&self, query: &str, campaign_id: Option<&str>) -> Vec<NPC> {
        let query_lower = query.to_lowercase();

        self.list(campaign_id)
            .into_iter()
            .filter(|npc| {
                npc.name.to_lowercase().contains(&query_lower) ||
                npc.personality.traits.iter().any(|t| t.to_lowercase().contains(&query_lower)) ||
                npc.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_generation() {
        let generator = NPCGenerator::new();
        let options = NPCGenerationOptions {
            name: Some("Test NPC".to_string()),
            role: Some("merchant".to_string()),
            generate_stats: false,
            ..Default::default()
        };

        let npc = generator.generate_quick(&options);
        assert_eq!(npc.name, "Test NPC");
        assert_eq!(npc.role, NPCRole::Merchant);
        assert!(!npc.personality.traits.is_empty());
    }

    #[test]
    fn test_random_generation() {
        let generator = NPCGenerator::new();
        let options = NPCGenerationOptions {
            role: Some("enemy".to_string()),
            include_secrets: true,
            include_hooks: true,
            ..Default::default()
        };

        let npc = generator.generate_quick(&options);
        assert_eq!(npc.role, NPCRole::Enemy);
        assert!(!npc.name.is_empty());
    }

    #[test]
    fn test_npc_store() {
        let store = NPCStore::new();
        let generator = NPCGenerator::new();

        let npc = generator.generate_quick(&NPCGenerationOptions::default());
        let id = npc.id.clone();

        store.add(npc, Some("campaign-1"));

        assert!(store.get(&id).is_some());
        assert_eq!(store.list(Some("campaign-1")).len(), 1);
        assert_eq!(store.list(None).len(), 1);

        store.delete(&id);
        assert!(store.get(&id).is_none());
    }

    #[test]
    fn test_role_parsing() {
        assert_eq!(NPCRole::from_str("merchant"), NPCRole::Merchant);
        assert_eq!(NPCRole::from_str("shopkeeper"), NPCRole::Merchant);
        assert_eq!(NPCRole::from_str("bbeg"), NPCRole::Boss);
        assert_eq!(NPCRole::from_str("custom"), NPCRole::Custom("custom".to_string()));
    }
}
