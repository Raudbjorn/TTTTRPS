//! Location Generation Module
//!
//! AI-powered generation of locations including taverns, dungeons, cities,
//! wilderness areas, and other points of interest for TTRPG campaigns.
//!
//! Supports both procedural (template-based) and AI-enhanced generation.
//! Each location includes:
//! - Rich descriptions and atmosphere
//! - Notable features (interactive and decorative)
//! - Inhabitants/NPCs with personalities
//! - Secrets and hidden elements
//! - Potential encounters
//! - Connected locations
//! - Map reference placeholders

mod data;
mod types;

pub use types::*;

use crate::core::llm::{ChatMessage, ChatRequest, LLMClient, LLMConfig, MessageRole};
use chrono::Utc;
use rand::seq::SliceRandom;
use uuid::Uuid;

// ============================================================================
// Location Generator
// ============================================================================

pub struct LocationGenerator {
    llm_client: Option<LLMClient>,
}

impl Default for LocationGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl LocationGenerator {
    pub fn new() -> Self {
        Self { llm_client: None }
    }

    pub fn with_llm(llm_config: LLMConfig) -> Self {
        Self {
            llm_client: Some(LLMClient::new(llm_config)),
        }
    }

    /// Generate a location without LLM (uses templates)
    pub fn generate_quick(&self, options: &LocationGenerationOptions) -> Location {
        let mut rng = rand::thread_rng();

        let location_type = options
            .location_type
            .as_deref()
            .map(LocationType::from_str)
            .unwrap_or(LocationType::Tavern);

        let name = options
            .name
            .clone()
            .unwrap_or_else(|| self.generate_name(&location_type, &mut rng));

        let description = self.generate_description(&location_type, &options.theme, &mut rng);
        let atmosphere = self.generate_atmosphere(&location_type, &mut rng);
        let notable_features = self.generate_features(&location_type, &mut rng);

        let inhabitants = if options.include_inhabitants {
            self.generate_inhabitants(&location_type, &mut rng)
        } else {
            vec![]
        };

        let secrets = if options.include_secrets {
            self.generate_secrets(&location_type, &mut rng)
        } else {
            vec![]
        };

        let encounters = if options.include_encounters {
            self.generate_encounters(&location_type, options.danger_level.clone(), &mut rng)
        } else {
            vec![]
        };

        let loot_potential = if options.include_loot {
            Some(self.generate_loot(&location_type, &mut rng))
        } else {
            None
        };

        let tags = self.generate_tags(&location_type);
        let now = Utc::now();

        Location {
            id: Uuid::new_v4().to_string(),
            campaign_id: options.campaign_id.clone(),
            name,
            location_type,
            description,
            atmosphere,
            notable_features,
            inhabitants,
            secrets,
            encounters,
            connected_locations: vec![],
            loot_potential,
            map_reference: None,
            tags,
            notes: String::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate a location using LLM for rich descriptions
    pub async fn generate_detailed(&self, options: &LocationGenerationOptions) -> Result<Location> {
        let llm = self
            .llm_client
            .as_ref()
            .ok_or_else(|| LocationGenError::GenerationFailed("No LLM configured".to_string()))?;

        let prompt = self.build_prompt(options);
        let system = self.build_system_prompt();

        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: prompt,
                images: None,
                name: None,
                tool_calls: None,
                tool_call_id: None,
            }],
            system_prompt: Some(system),
            temperature: Some(0.8),
            max_tokens: Some(2000),
            provider: None,
            tools: None,
            tool_choice: None,
        };

        let response = llm
            .chat(request)
            .await
            .map_err(|e| LocationGenError::LLMError(e.to_string()))?;

        self.parse_response(&response.content, options)
    }

    // ========================================================================
    // Tag Generation
    // ========================================================================

    fn generate_tags(&self, loc_type: &LocationType) -> Vec<String> {
        let mut tags = vec![loc_type.display_name().to_lowercase()];

        match loc_type {
            LocationType::Tavern
            | LocationType::Inn
            | LocationType::Shop
            | LocationType::Guild
            | LocationType::Temple
            | LocationType::Market => {
                tags.push("urban".to_string());
                tags.push("social".to_string());
            }
            LocationType::Castle | LocationType::Manor | LocationType::Stronghold => {
                tags.push("fortification".to_string());
                tags.push("noble".to_string());
            }
            LocationType::City | LocationType::Town | LocationType::Village => {
                tags.push("settlement".to_string());
            }
            LocationType::Forest
            | LocationType::Mountain
            | LocationType::Swamp
            | LocationType::Desert
            | LocationType::Plains
            | LocationType::Coast
            | LocationType::Island
            | LocationType::River
            | LocationType::Lake => {
                tags.push("wilderness".to_string());
                tags.push("outdoor".to_string());
            }
            LocationType::Dungeon
            | LocationType::Cave
            | LocationType::Ruins
            | LocationType::Tower
            | LocationType::Tomb
            | LocationType::Mine
            | LocationType::Lair => {
                tags.push("adventure".to_string());
                tags.push("dangerous".to_string());
            }
            LocationType::Shrine | LocationType::Portal | LocationType::Planar => {
                tags.push("magical".to_string());
            }
            _ => {}
        }
        tags
    }

    // ========================================================================
    // Name Generation
    // ========================================================================

    fn generate_name(&self, loc_type: &LocationType, rng: &mut impl rand::Rng) -> String {
        let adjective = data::NAME_ADJECTIVES[rng.gen_range(0..data::NAME_ADJECTIVES.len())];

        match loc_type {
            LocationType::Tavern | LocationType::Inn => {
                let noun = data::TAVERN_NOUNS[rng.gen_range(0..data::TAVERN_NOUNS.len())];
                format!("The {} {}", adjective, noun)
            }
            LocationType::Shop => {
                let shop_type = data::SHOP_TYPES[rng.gen_range(0..data::SHOP_TYPES.len())];
                format!("{}'s {}", self.random_name(rng), shop_type)
            }
            LocationType::Dungeon | LocationType::Ruins => {
                let dungeon_type =
                    data::DUNGEON_NAME_TYPES[rng.gen_range(0..data::DUNGEON_NAME_TYPES.len())];
                format!("{} {}", adjective, dungeon_type)
            }
            LocationType::Forest => {
                let forest_type =
                    data::FOREST_NAME_TYPES[rng.gen_range(0..data::FOREST_NAME_TYPES.len())];
                format!("{} {}", adjective, forest_type)
            }
            LocationType::Mountain => {
                let mountain_type =
                    data::MOUNTAIN_NAME_TYPES[rng.gen_range(0..data::MOUNTAIN_NAME_TYPES.len())];
                format!("{} {}", adjective, mountain_type)
            }
            _ => format!("{} {}", adjective, loc_type.display_name()),
        }
    }

    fn random_name(&self, rng: &mut impl rand::Rng) -> String {
        data::NPC_NAMES[rng.gen_range(0..data::NPC_NAMES.len())].to_string()
    }

    // ========================================================================
    // Description Generation
    // ========================================================================

    fn generate_description(
        &self,
        loc_type: &LocationType,
        theme: &Option<String>,
        _rng: &mut impl rand::Rng,
    ) -> String {
        let type_key = loc_type.display_name().to_lowercase();
        let base = data::DESCRIPTIONS
            .iter()
            .find(|(key, _)| *key == type_key)
            .map(|(_, desc)| *desc)
            .unwrap_or("A location of interest in the world.");

        match theme {
            Some(t) => format!("{} The atmosphere carries a distinctly {} feeling.", base, t),
            None => base.to_string(),
        }
    }

    // ========================================================================
    // Atmosphere Generation
    // ========================================================================

    fn generate_atmosphere(&self, loc_type: &LocationType, _rng: &mut impl rand::Rng) -> Atmosphere {
        match loc_type {
            LocationType::Tavern | LocationType::Inn => Atmosphere {
                lighting: "Warm candlelight and flickering fireplace".to_string(),
                sounds: vec![
                    "Murmured conversations".to_string(),
                    "Clinking glasses".to_string(),
                    "Crackling fire".to_string(),
                ],
                smells: vec![
                    "Roasting meat".to_string(),
                    "Spilled ale".to_string(),
                    "Wood smoke".to_string(),
                ],
                mood: "Welcoming but watchful".to_string(),
                weather: None,
                time_of_day_effects: Some("Busier in the evening, quieter at midday".to_string()),
            },
            LocationType::Dungeon | LocationType::Cave => Atmosphere {
                lighting: "Pitch darkness, torches required".to_string(),
                sounds: vec![
                    "Dripping water".to_string(),
                    "Distant echoes".to_string(),
                    "Scuttling creatures".to_string(),
                ],
                smells: vec![
                    "Damp stone".to_string(),
                    "Decay".to_string(),
                    "Stale air".to_string(),
                ],
                mood: "Oppressive and dangerous".to_string(),
                weather: None,
                time_of_day_effects: None,
            },
            LocationType::Forest => Atmosphere {
                lighting: "Dappled sunlight through the canopy".to_string(),
                sounds: vec![
                    "Birdsong".to_string(),
                    "Rustling leaves".to_string(),
                    "Distant wildlife".to_string(),
                ],
                smells: vec![
                    "Pine".to_string(),
                    "Damp earth".to_string(),
                    "Wild flowers".to_string(),
                ],
                mood: "Serene but watchful".to_string(),
                weather: Some("Partly cloudy".to_string()),
                time_of_day_effects: Some("More dangerous at night".to_string()),
            },
            _ => Atmosphere::default(),
        }
    }

    // ========================================================================
    // Feature Generation
    // ========================================================================

    fn generate_features(
        &self,
        loc_type: &LocationType,
        rng: &mut impl rand::Rng,
    ) -> Vec<NotableFeature> {
        let features_pool = self.get_features_for_type(loc_type);
        let count = rng.gen_range(3..=5).min(features_pool.len());
        let selected: Vec<_> = features_pool.choose_multiple(rng, count).collect();

        selected
            .iter()
            .map(|(name, desc, interactive, hidden, effect)| NotableFeature {
                name: name.to_string(),
                description: desc.to_string(),
                interactive: *interactive,
                hidden: *hidden,
                mechanical_effect: effect.map(|s| s.to_string()),
            })
            .collect()
    }

    fn get_features_for_type(&self, loc_type: &LocationType) -> &'static [data::FeatureData] {
        match loc_type {
            LocationType::Tavern | LocationType::Inn => data::TAVERN_FEATURES,
            LocationType::Dungeon
            | LocationType::Cave
            | LocationType::Ruins
            | LocationType::Tomb => data::DUNGEON_FEATURES,
            LocationType::Forest | LocationType::Mountain | LocationType::Swamp => {
                data::FOREST_FEATURES
            }
            LocationType::Shop | LocationType::Market => data::SHOP_FEATURES,
            LocationType::Castle | LocationType::Stronghold | LocationType::Manor => {
                data::CASTLE_FEATURES
            }
            LocationType::Temple | LocationType::Shrine => data::TEMPLE_FEATURES,
            _ => data::TAVERN_FEATURES, // Default fallback
        }
    }

    // ========================================================================
    // Inhabitant Generation
    // ========================================================================

    fn generate_inhabitants(
        &self,
        loc_type: &LocationType,
        rng: &mut impl rand::Rng,
    ) -> Vec<Inhabitant> {
        match loc_type {
            LocationType::Tavern | LocationType::Inn => {
                self.generate_tavern_inhabitants(rng)
            }
            LocationType::Shop | LocationType::Market => {
                vec![self.create_shopkeeper(rng)]
            }
            LocationType::Temple | LocationType::Shrine => {
                self.generate_temple_inhabitants(rng)
            }
            LocationType::Castle | LocationType::Stronghold | LocationType::Manor => {
                self.generate_castle_inhabitants(rng)
            }
            LocationType::Guild => {
                vec![self.create_guildmaster(rng)]
            }
            LocationType::Dungeon | LocationType::Cave | LocationType::Ruins => {
                if rng.gen_bool(0.3) {
                    vec![self.create_hermit(rng)]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }

    fn generate_tavern_inhabitants(&self, rng: &mut impl rand::Rng) -> Vec<Inhabitant> {
        let mut inhabitants = vec![Inhabitant {
            name: self.random_name(rng),
            role: "Barkeep".to_string(),
            description: data::BARKEEP_DESCRIPTIONS
                .choose(rng)
                .unwrap()
                .to_string(),
            disposition: Disposition::Neutral,
            secrets: vec![
                "Knows local rumors and gossip".to_string(),
                "Has connections to the underground".to_string(),
            ],
            services: vec![
                "Drinks".to_string(),
                "Food".to_string(),
                "Rooms".to_string(),
                "Local information".to_string(),
            ],
        }];

        if rng.gen_bool(0.7) {
            inhabitants.push(Inhabitant {
                name: self.random_name(rng),
                role: "Server".to_string(),
                description: data::BARMAID_DESCRIPTIONS
                    .choose(rng)
                    .unwrap()
                    .to_string(),
                disposition: Disposition::Friendly,
                secrets: vec!["Overhears private conversations".to_string()],
                services: vec!["Table service".to_string(), "Room cleaning".to_string()],
            });
        }

        if rng.gen_bool(0.5) {
            inhabitants.push(Inhabitant {
                name: self.random_name(rng),
                role: "Regular Patron".to_string(),
                description: "A local who practically lives at the bar".to_string(),
                disposition: Disposition::Varies,
                secrets: vec!["Has gambling debts".to_string()],
                services: vec!["Local history".to_string(), "Introductions".to_string()],
            });
        }

        inhabitants
    }

    fn generate_temple_inhabitants(&self, rng: &mut impl rand::Rng) -> Vec<Inhabitant> {
        let mut inhabitants = vec![Inhabitant {
            name: self.random_name(rng),
            role: "Head Priest".to_string(),
            description: data::PRIEST_DESCRIPTIONS.choose(rng).unwrap().to_string(),
            disposition: Disposition::Friendly,
            secrets: vec!["Knows dark secrets from confessions".to_string()],
            services: vec![
                "Healing".to_string(),
                "Blessings".to_string(),
                "Spiritual guidance".to_string(),
            ],
        }];

        if rng.gen_bool(0.6) {
            inhabitants.push(Inhabitant {
                name: self.random_name(rng),
                role: "Acolyte".to_string(),
                description: "A young initiate learning the ways of the faith".to_string(),
                disposition: Disposition::Friendly,
                secrets: vec!["Witnessed something they shouldn't have".to_string()],
                services: vec!["Minor healing".to_string(), "Temple tours".to_string()],
            });
        }

        inhabitants
    }

    fn generate_castle_inhabitants(&self, rng: &mut impl rand::Rng) -> Vec<Inhabitant> {
        vec![
            Inhabitant {
                name: self.random_name(rng),
                role: "Guard Captain".to_string(),
                description: data::GUARD_DESCRIPTIONS.choose(rng).unwrap().to_string(),
                disposition: Disposition::Wary,
                secrets: vec!["Knows the patrol schedules".to_string()],
                services: vec!["Security".to_string(), "Escort".to_string()],
            },
            Inhabitant {
                name: self.random_name(rng),
                role: "Steward".to_string(),
                description: "A meticulous administrator who manages the household affairs"
                    .to_string(),
                disposition: Disposition::Neutral,
                secrets: vec!["Knows all the castle's secret passages".to_string()],
                services: vec![
                    "Appointments".to_string(),
                    "Lodging arrangements".to_string(),
                ],
            },
        ]
    }

    fn create_shopkeeper(&self, rng: &mut impl rand::Rng) -> Inhabitant {
        Inhabitant {
            name: self.random_name(rng),
            role: "Shopkeeper".to_string(),
            description: data::SHOPKEEPER_DESCRIPTIONS
                .choose(rng)
                .unwrap()
                .to_string(),
            disposition: Disposition::Friendly,
            secrets: vec![
                "Has rare items for trusted customers".to_string(),
                "Fences stolen goods on the side".to_string(),
            ],
            services: vec![
                "Buy/Sell goods".to_string(),
                "Identify items".to_string(),
                "Special orders".to_string(),
            ],
        }
    }

    fn create_guildmaster(&self, rng: &mut impl rand::Rng) -> Inhabitant {
        Inhabitant {
            name: self.random_name(rng),
            role: "Guildmaster".to_string(),
            description: "A wealthy professional who rose through the ranks".to_string(),
            disposition: Disposition::Neutral,
            secrets: vec!["Controls more of the city than people realize".to_string()],
            services: vec![
                "Guild membership".to_string(),
                "Contracts".to_string(),
                "Training".to_string(),
            ],
        }
    }

    fn create_hermit(&self, rng: &mut impl rand::Rng) -> Inhabitant {
        Inhabitant {
            name: self.random_name(rng),
            role: "Hermit".to_string(),
            description: "A reclusive figure who has made this dark place their home".to_string(),
            disposition: Disposition::Wary,
            secrets: vec!["Knows safe paths through the area".to_string()],
            services: vec!["Guidance".to_string(), "Shelter".to_string()],
        }
    }

    // ========================================================================
    // Secret Generation
    // ========================================================================

    fn generate_secrets(&self, loc_type: &LocationType, rng: &mut impl rand::Rng) -> Vec<Secret> {
        let secrets_pool = self.get_secrets_for_type(loc_type);
        let count = rng.gen_range(1..=3).min(secrets_pool.len());
        let selected: Vec<_> = secrets_pool.choose_multiple(rng, count).collect();

        selected
            .iter()
            .map(|(desc, diff, consequences, clues)| Secret {
                description: desc.to_string(),
                difficulty_to_discover: diff.clone(),
                consequences_if_revealed: consequences.to_string(),
                clues: clues.iter().map(|s| s.to_string()).collect(),
            })
            .collect()
    }

    fn get_secrets_for_type(&self, loc_type: &LocationType) -> &'static [data::SecretData] {
        match loc_type {
            LocationType::Tavern
            | LocationType::Inn
            | LocationType::Shop
            | LocationType::Market
            | LocationType::Guild => data::URBAN_SECRETS,
            LocationType::Dungeon
            | LocationType::Cave
            | LocationType::Ruins
            | LocationType::Tomb
            | LocationType::Mine => data::DUNGEON_SECRETS,
            LocationType::Forest
            | LocationType::Mountain
            | LocationType::Swamp
            | LocationType::Desert
            | LocationType::Plains => data::WILDERNESS_SECRETS,
            LocationType::Temple | LocationType::Shrine => data::TEMPLE_SECRETS,
            _ => data::URBAN_SECRETS,
        }
    }

    // ========================================================================
    // Encounter Generation
    // ========================================================================

    fn generate_encounters(
        &self,
        loc_type: &LocationType,
        danger: Option<Difficulty>,
        rng: &mut impl rand::Rng,
    ) -> Vec<Encounter> {
        let difficulty = danger.unwrap_or(Difficulty::Medium);
        let encounters_pool = self.get_encounters_for_type(loc_type);
        let count = rng.gen_range(2..=4).min(encounters_pool.len());
        let selected: Vec<_> = encounters_pool.choose_multiple(rng, count).collect();

        selected
            .iter()
            .map(|(name, desc, trigger, rewards, optional)| Encounter {
                name: name.to_string(),
                description: desc.to_string(),
                trigger: trigger.to_string(),
                difficulty: difficulty.clone(),
                rewards: rewards.iter().map(|s| s.to_string()).collect(),
                optional: *optional,
            })
            .collect()
    }

    fn get_encounters_for_type(&self, loc_type: &LocationType) -> &'static [data::EncounterData] {
        match loc_type {
            LocationType::Dungeon
            | LocationType::Cave
            | LocationType::Ruins
            | LocationType::Tomb
            | LocationType::Mine
            | LocationType::Lair => data::DUNGEON_ENCOUNTERS,
            LocationType::Forest
            | LocationType::Mountain
            | LocationType::Swamp
            | LocationType::Desert
            | LocationType::Plains
            | LocationType::Coast => data::WILDERNESS_ENCOUNTERS,
            LocationType::Tavern
            | LocationType::Inn
            | LocationType::City
            | LocationType::Town
            | LocationType::Village
            | LocationType::Market => data::URBAN_ENCOUNTERS,
            _ => data::WILDERNESS_ENCOUNTERS,
        }
    }

    // ========================================================================
    // Loot Generation
    // ========================================================================

    fn generate_loot(&self, loc_type: &LocationType, rng: &mut impl rand::Rng) -> LootPotential {
        match loc_type {
            LocationType::Dungeon | LocationType::Tomb | LocationType::Ruins | LocationType::Lair => {
                let count = rng.gen_range(2..=4);
                let items: Vec<String> = data::DUNGEON_LOOT_ITEMS
                    .choose_multiple(rng, count)
                    .map(|s| s.to_string())
                    .collect();
                LootPotential {
                    treasure_level: TreasureLevel::Rich,
                    notable_items: items,
                    hidden_caches: rng.gen_range(1..=3),
                }
            }
            LocationType::Cave | LocationType::Mine => {
                let count = rng.gen_range(1..=2);
                let items: Vec<String> = data::DUNGEON_LOOT_ITEMS
                    .choose_multiple(rng, count)
                    .map(|s| s.to_string())
                    .collect();
                LootPotential {
                    treasure_level: TreasureLevel::Average,
                    notable_items: items,
                    hidden_caches: rng.gen_range(0..=2),
                }
            }
            LocationType::Forest | LocationType::Mountain | LocationType::Swamp => {
                let count = rng.gen_range(1..=2);
                let items: Vec<String> = data::WILDERNESS_LOOT_ITEMS
                    .choose_multiple(rng, count)
                    .map(|s| s.to_string())
                    .collect();
                LootPotential {
                    treasure_level: TreasureLevel::Modest,
                    notable_items: items,
                    hidden_caches: rng.gen_range(0..=1),
                }
            }
            LocationType::Castle | LocationType::Manor | LocationType::Stronghold => LootPotential {
                treasure_level: TreasureLevel::Rich,
                notable_items: vec![
                    "Noble treasures".to_string(),
                    "Artwork".to_string(),
                    "Jeweled items".to_string(),
                ],
                hidden_caches: rng.gen_range(1..=2),
            },
            LocationType::Tavern | LocationType::Inn => LootPotential {
                treasure_level: TreasureLevel::Poor,
                notable_items: vec![],
                hidden_caches: 0,
            },
            LocationType::Shop | LocationType::Market => {
                let count = rng.gen_range(0..=1);
                let items: Vec<String> = data::URBAN_LOOT_ITEMS
                    .choose_multiple(rng, count)
                    .map(|s| s.to_string())
                    .collect();
                LootPotential {
                    treasure_level: TreasureLevel::Modest,
                    notable_items: items,
                    hidden_caches: rng.gen_range(0..=1),
                }
            }
            LocationType::Temple | LocationType::Shrine => LootPotential {
                treasure_level: TreasureLevel::Average,
                notable_items: vec!["Religious artifacts".to_string(), "Offerings".to_string()],
                hidden_caches: 1,
            },
            _ => LootPotential {
                treasure_level: TreasureLevel::Modest,
                notable_items: vec![],
                hidden_caches: rng.gen_range(0..=1),
            },
        }
    }

    // ========================================================================
    // LLM Prompt Building
    // ========================================================================

    fn build_prompt(&self, options: &LocationGenerationOptions) -> String {
        let mut prompt = String::from("Generate a detailed location for a TTRPG campaign.\n\n");

        if let Some(loc_type) = &options.location_type {
            prompt.push_str(&format!("Location Type: {}\n", loc_type));
        }

        if let Some(name) = &options.name {
            prompt.push_str(&format!("Name: {}\n", name));
        }

        if let Some(size) = &options.size {
            prompt.push_str(&format!("Size: {:?}\n", size));
        }

        if let Some(theme) = &options.theme {
            prompt.push_str(&format!("Theme: {}\n", theme));
        }

        if let Some(setting) = &options.setting {
            prompt.push_str(&format!("Setting: {}\n", setting));
        }

        if let Some(danger) = &options.danger_level {
            prompt.push_str(&format!("Danger Level: {:?}\n", danger));
        }

        prompt.push_str("\nInclude:\n");
        if options.include_inhabitants {
            prompt.push_str("- NPCs/Inhabitants\n");
        }
        if options.include_secrets {
            prompt.push_str("- Secrets and hidden elements\n");
        }
        if options.include_encounters {
            prompt.push_str("- Possible encounters\n");
        }
        if options.include_loot {
            prompt.push_str("- Treasure and rewards\n");
        }

        prompt.push_str("\nProvide a rich, detailed description suitable for a game master to use.");
        prompt
    }

    fn build_system_prompt(&self) -> String {
        "You are a creative TTRPG location designer. Generate detailed, \
         atmospheric locations with interesting features, NPCs, and secrets. \
         Make locations feel alive and full of adventure potential. \
         Return your response as a JSON object with the following structure:\n\
         {\"name\", \"description\", \"atmosphere\", \"notable_features\", \
         \"inhabitants\", \"secrets\", \"encounters\", \"loot_potential\"}"
            .to_string()
    }

    // ========================================================================
    // Response Parsing
    // ========================================================================

    fn parse_response(
        &self,
        content: &str,
        options: &LocationGenerationOptions,
    ) -> Result<Location> {
        // Extract JSON from response
        let json_str = content
            .find('{')
            .and_then(|start| content.rfind('}').map(|end| &content[start..=end]))
            .unwrap_or(content);

        // Try to parse JSON and build location
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
            return self.build_location_from_json(&parsed, options);
        }

        // Fall back to creating a basic location from the raw content
        self.build_fallback_location(content, options)
    }

    fn build_location_from_json(
        &self,
        json: &serde_json::Value,
        options: &LocationGenerationOptions,
    ) -> Result<Location> {
        let location_type = options
            .location_type
            .as_deref()
            .map(LocationType::from_str)
            .unwrap_or(LocationType::Tavern);

        let name = json
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| options.name.clone())
            .unwrap_or_else(|| "Generated Location".to_string());

        let description = json
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let atmosphere = self.parse_atmosphere(json.get("atmosphere"));
        let notable_features = self.parse_features(json.get("notable_features"));
        let inhabitants = self.parse_inhabitants(json.get("inhabitants"));
        let secrets = self.parse_secrets(json.get("secrets"));
        let encounters = self.parse_encounters(json.get("encounters"));
        let loot_potential = self.parse_loot_potential(json.get("loot_potential"));

        let tags = self.generate_tags(&location_type);
        let now = Utc::now();

        Ok(Location {
            id: Uuid::new_v4().to_string(),
            campaign_id: options.campaign_id.clone(),
            name,
            location_type,
            description,
            atmosphere,
            notable_features,
            inhabitants,
            secrets,
            encounters,
            connected_locations: vec![],
            loot_potential,
            map_reference: options.map_reference.clone(),
            tags,
            notes: String::new(),
            created_at: now,
            updated_at: now,
        })
    }

    fn build_fallback_location(
        &self,
        content: &str,
        options: &LocationGenerationOptions,
    ) -> Result<Location> {
        let location_type = options
            .location_type
            .as_deref()
            .map(LocationType::from_str)
            .unwrap_or(LocationType::Tavern);

        let tags = self.generate_tags(&location_type);
        let now = Utc::now();

        Ok(Location {
            id: Uuid::new_v4().to_string(),
            campaign_id: options.campaign_id.clone(),
            name: options
                .name
                .clone()
                .unwrap_or_else(|| "Generated Location".to_string()),
            location_type,
            description: content.to_string(),
            atmosphere: Atmosphere::default(),
            notable_features: vec![],
            inhabitants: vec![],
            secrets: vec![],
            encounters: vec![],
            connected_locations: vec![],
            loot_potential: None,
            map_reference: None,
            tags,
            notes: String::new(),
            created_at: now,
            updated_at: now,
        })
    }

    // ========================================================================
    // JSON Parsing Helpers
    // ========================================================================

    fn parse_atmosphere(&self, value: Option<&serde_json::Value>) -> Atmosphere {
        let Some(atm) = value else {
            return Atmosphere::default();
        };

        Atmosphere {
            lighting: atm
                .get("lighting")
                .and_then(|v| v.as_str())
                .unwrap_or("Variable")
                .to_string(),
            sounds: self.parse_string_array(atm.get("sounds")),
            smells: self.parse_string_array(atm.get("smells")),
            mood: atm
                .get("mood")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            weather: atm.get("weather").and_then(|v| v.as_str()).map(String::from),
            time_of_day_effects: atm
                .get("time_of_day_effects")
                .and_then(|v| v.as_str())
                .map(String::from),
        }
    }

    fn parse_features(&self, value: Option<&serde_json::Value>) -> Vec<NotableFeature> {
        let Some(arr) = value.and_then(|v| v.as_array()) else {
            return vec![];
        };

        arr.iter()
            .filter_map(|f| {
                Some(NotableFeature {
                    name: f.get("name")?.as_str()?.to_string(),
                    description: f
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    interactive: f
                        .get("interactive")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    hidden: f.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false),
                    mechanical_effect: f
                        .get("mechanical_effect")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                })
            })
            .collect()
    }

    fn parse_inhabitants(&self, value: Option<&serde_json::Value>) -> Vec<Inhabitant> {
        let Some(arr) = value.and_then(|v| v.as_array()) else {
            return vec![];
        };

        arr.iter()
            .filter_map(|i| {
                Some(Inhabitant {
                    name: i.get("name")?.as_str()?.to_string(),
                    role: i
                        .get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    description: i
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    disposition: Disposition::from_str(
                        i.get("disposition")
                            .and_then(|v| v.as_str())
                            .unwrap_or("neutral"),
                    ),
                    secrets: self.parse_string_array(i.get("secrets")),
                    services: self.parse_string_array(i.get("services")),
                })
            })
            .collect()
    }

    fn parse_secrets(&self, value: Option<&serde_json::Value>) -> Vec<Secret> {
        let Some(arr) = value.and_then(|v| v.as_array()) else {
            return vec![];
        };

        arr.iter()
            .filter_map(|s| {
                Some(Secret {
                    description: s.get("description")?.as_str()?.to_string(),
                    difficulty_to_discover: Difficulty::from_str(
                        s.get("difficulty")
                            .and_then(|v| v.as_str())
                            .unwrap_or("medium"),
                    ),
                    consequences_if_revealed: s
                        .get("consequences")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    clues: self.parse_string_array(s.get("clues")),
                })
            })
            .collect()
    }

    fn parse_encounters(&self, value: Option<&serde_json::Value>) -> Vec<Encounter> {
        let Some(arr) = value.and_then(|v| v.as_array()) else {
            return vec![];
        };

        arr.iter()
            .filter_map(|e| {
                Some(Encounter {
                    name: e.get("name")?.as_str()?.to_string(),
                    description: e
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    trigger: e
                        .get("trigger")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    difficulty: Difficulty::from_str(
                        e.get("difficulty")
                            .and_then(|v| v.as_str())
                            .unwrap_or("medium"),
                    ),
                    rewards: self.parse_string_array(e.get("rewards")),
                    optional: e.get("optional").and_then(|v| v.as_bool()).unwrap_or(true),
                })
            })
            .collect()
    }

    fn parse_string_array(&self, value: Option<&serde_json::Value>) -> Vec<String> {
        value
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn parse_loot_potential(&self, value: Option<&serde_json::Value>) -> Option<LootPotential> {
        let loot = value?;
        Some(LootPotential {
            treasure_level: TreasureLevel::from_str(
                loot.get("treasure_level")
                    .and_then(|v| v.as_str())
                    .or_else(|| loot.get("level").and_then(|v| v.as_str()))
                    .unwrap_or("none"),
            ),
            notable_items: self.parse_string_array(loot.get("notable_items").or_else(|| loot.get("items"))),
            hidden_caches: loot
                .get("hidden_caches")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_location_type_parsing() {
        assert_eq!(LocationType::from_str("tavern"), LocationType::Tavern);
        assert_eq!(LocationType::from_str("dungeon"), LocationType::Dungeon);
        assert_eq!(LocationType::from_str("prison"), LocationType::Prison);
        assert_eq!(LocationType::from_str("forest"), LocationType::Forest);
    }

    #[test]
    fn test_quick_generation() {
        let generator = LocationGenerator::new();
        let options = LocationGenerationOptions {
            location_type: Some("tavern".to_string()),
            include_inhabitants: true,
            include_secrets: true,
            ..Default::default()
        };

        let location = generator.generate_quick(&options);
        assert!(!location.name.is_empty());
        assert_eq!(location.location_type, LocationType::Tavern);
    }

    #[test]
    fn test_difficulty_parsing() {
        assert_eq!(Difficulty::from_str("easy"), Difficulty::Easy);
        assert_eq!(Difficulty::from_str("hard"), Difficulty::Hard);
        assert_eq!(Difficulty::from_str("very_hard"), Difficulty::VeryHard);
        assert_eq!(Difficulty::from_str("unknown"), Difficulty::Medium);
    }

    #[test]
    fn test_disposition_parsing() {
        assert_eq!(Disposition::from_str("friendly"), Disposition::Friendly);
        assert_eq!(Disposition::from_str("hostile"), Disposition::Hostile);
        assert_eq!(Disposition::from_str("unknown"), Disposition::Neutral);
    }
}
