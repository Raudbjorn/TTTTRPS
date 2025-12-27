//! Personality Module
//!
//! Extracts and manages personality profiles from source text.
//! Used for generating NPC personalities based on source material.

use crate::core::llm::{LLMClient, LLMConfig, LLMError, ChatMessage, ChatRequest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;
use chrono::Utc;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum PersonalityError {
    #[error("LLM error: {0}")]
    LLMError(#[from] LLMError),

    #[error("Personality not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),
}

pub type Result<T> = std::result::Result<T, PersonalityError>;

// ============================================================================
// Personality Profile Types
// ============================================================================

/// A complete personality profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    /// Unique identifier
    pub id: String,
    /// Profile name
    pub name: String,
    /// Source material this was extracted from
    pub source: Option<String>,
    /// Speaking style and mannerisms
    pub speech_patterns: SpeechPatterns,
    /// Core personality traits
    pub traits: Vec<PersonalityTrait>,
    /// Knowledge domains
    pub knowledge_areas: Vec<String>,
    /// Typical responses to situations
    pub behavioral_tendencies: BehavioralTendencies,
    /// Example phrases that capture the voice
    pub example_phrases: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
}

/// Speech patterns and verbal mannerisms
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpeechPatterns {
    /// Formality level (1-10, 1=casual, 10=formal)
    pub formality: u8,
    /// Common phrases or catchphrases
    pub common_phrases: Vec<String>,
    /// Vocabulary style
    pub vocabulary_style: String,
    /// Accent or dialect notes
    pub dialect_notes: Option<String>,
    /// Pacing/tempo description
    pub pacing: String,
}

/// A personality trait with intensity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityTrait {
    /// Trait name (e.g., "curious", "cautious", "aggressive")
    pub trait_name: String,
    /// Intensity (1-10)
    pub intensity: u8,
    /// Description of how this manifests
    pub manifestation: String,
}

/// How the personality tends to behave in situations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehavioralTendencies {
    /// Response to conflict
    pub conflict_response: String,
    /// Response to strangers
    pub stranger_response: String,
    /// Response to authority
    pub authority_response: String,
    /// Response to requests for help
    pub help_response: String,
    /// General attitude
    pub general_attitude: String,
}

// ============================================================================
// Personality Extractor
// ============================================================================

pub struct PersonalityExtractor {
    llm_client: LLMClient,
}

impl PersonalityExtractor {
    pub fn new(llm_config: LLMConfig) -> Self {
        Self {
            llm_client: LLMClient::new(llm_config),
        }
    }

    /// Extract personality from source text
    pub async fn extract_from_text(
        &self,
        text: &str,
        source_name: Option<&str>,
    ) -> Result<PersonalityProfile> {
        let system_prompt = r#"You are an expert at analyzing text and extracting personality profiles.
Analyze the given text and extract a personality profile that captures:
1. Speech patterns and verbal mannerisms
2. Core personality traits with intensity (1-10 scale)
3. Knowledge areas and expertise
4. Behavioral tendencies in different situations
5. Example phrases that capture the voice

Respond in JSON format matching this structure:
{
  "name": "Profile Name",
  "speech_patterns": {
    "formality": 5,
    "common_phrases": ["phrase1", "phrase2"],
    "vocabulary_style": "description",
    "dialect_notes": "optional notes",
    "pacing": "description of speech tempo"
  },
  "traits": [
    {"trait_name": "trait", "intensity": 7, "manifestation": "how it shows"}
  ],
  "knowledge_areas": ["area1", "area2"],
  "behavioral_tendencies": {
    "conflict_response": "description",
    "stranger_response": "description",
    "authority_response": "description",
    "help_response": "description",
    "general_attitude": "description"
  },
  "example_phrases": ["example1", "example2"],
  "tags": ["tag1", "tag2"]
}"#;

        let user_prompt = format!(
            "Analyze this text and extract a personality profile:\n\n{}",
            text
        );

        let request = ChatRequest::new(vec![ChatMessage::user(&user_prompt)])
            .with_system(system_prompt.to_string());

        let response = self.llm_client.chat(request).await?;

        // Parse the JSON response
        let extracted: ExtractedPersonality = self.parse_extraction(&response.content)?;

        // Build the full profile
        Ok(PersonalityProfile {
            id: Uuid::new_v4().to_string(),
            name: extracted.name,
            source: source_name.map(|s| s.to_string()),
            speech_patterns: extracted.speech_patterns,
            traits: extracted.traits,
            knowledge_areas: extracted.knowledge_areas,
            behavioral_tendencies: extracted.behavioral_tendencies,
            example_phrases: extracted.example_phrases,
            tags: extracted.tags,
            metadata: HashMap::new(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        })
    }

    /// Parse LLM extraction response
    fn parse_extraction(&self, response: &str) -> Result<ExtractedPersonality> {
        // Try to find JSON in the response
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        serde_json::from_str(json_str).map_err(|e| {
            PersonalityError::ExtractionFailed(format!(
                "Failed to parse personality: {}. Response: {}",
                e, response
            ))
        })
    }

    /// Generate a personality based on a description
    pub async fn generate_from_description(
        &self,
        description: &str,
    ) -> Result<PersonalityProfile> {
        let system_prompt = r#"You are an expert at creating rich, nuanced character personalities.
Given a brief description, create a detailed personality profile that would make the character
feel authentic and interesting. Include specific quirks and details.

Respond in JSON format matching this structure:
{
  "name": "Character Type Name",
  "speech_patterns": {
    "formality": 5,
    "common_phrases": ["phrase1", "phrase2"],
    "vocabulary_style": "description",
    "dialect_notes": "optional notes",
    "pacing": "description of speech tempo"
  },
  "traits": [
    {"trait_name": "trait", "intensity": 7, "manifestation": "how it shows"}
  ],
  "knowledge_areas": ["area1", "area2"],
  "behavioral_tendencies": {
    "conflict_response": "description",
    "stranger_response": "description",
    "authority_response": "description",
    "help_response": "description",
    "general_attitude": "description"
  },
  "example_phrases": ["example1", "example2"],
  "tags": ["tag1", "tag2"]
}"#;

        let request = ChatRequest::new(vec![ChatMessage::user(description)])
            .with_system(system_prompt.to_string());

        let response = self.llm_client.chat(request).await?;
        let extracted: ExtractedPersonality = self.parse_extraction(&response.content)?;

        Ok(PersonalityProfile {
            id: Uuid::new_v4().to_string(),
            name: extracted.name,
            source: None,
            speech_patterns: extracted.speech_patterns,
            traits: extracted.traits,
            knowledge_areas: extracted.knowledge_areas,
            behavioral_tendencies: extracted.behavioral_tendencies,
            example_phrases: extracted.example_phrases,
            tags: extracted.tags,
            metadata: HashMap::new(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        })
    }
}

/// Intermediate structure for parsing LLM output
#[derive(Debug, Deserialize)]
struct ExtractedPersonality {
    name: String,
    #[serde(default)]
    speech_patterns: SpeechPatterns,
    #[serde(default)]
    traits: Vec<PersonalityTrait>,
    #[serde(default)]
    knowledge_areas: Vec<String>,
    #[serde(default)]
    behavioral_tendencies: BehavioralTendencies,
    #[serde(default)]
    example_phrases: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
}

// ============================================================================
// Personality Store (In-Memory CRUD)
// ============================================================================

pub struct PersonalityStore {
    profiles: std::sync::RwLock<HashMap<String, PersonalityProfile>>,
}

impl PersonalityStore {
    pub fn new() -> Self {
        Self {
            profiles: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Create a new personality profile
    pub fn create(&self, mut profile: PersonalityProfile) -> Result<PersonalityProfile> {
        let now = Utc::now().to_rfc3339();
        profile.id = Uuid::new_v4().to_string();
        profile.created_at = now.clone();
        profile.updated_at = now;

        let id = profile.id.clone();
        let mut profiles = self.profiles.write().unwrap();
        profiles.insert(id, profile.clone());

        log::info!("Created personality profile: {}", profile.name);
        Ok(profile)
    }

    /// Get a personality profile by ID
    pub fn get(&self, id: &str) -> Result<PersonalityProfile> {
        let profiles = self.profiles.read().unwrap();
        profiles
            .get(id)
            .cloned()
            .ok_or_else(|| PersonalityError::NotFound(id.to_string()))
    }

    /// Get all personality profiles
    pub fn list(&self) -> Vec<PersonalityProfile> {
        let profiles = self.profiles.read().unwrap();
        profiles.values().cloned().collect()
    }

    /// Update a personality profile
    pub fn update(&self, id: &str, mut profile: PersonalityProfile) -> Result<PersonalityProfile> {
        let mut profiles = self.profiles.write().unwrap();

        if !profiles.contains_key(id) {
            return Err(PersonalityError::NotFound(id.to_string()));
        }

        profile.id = id.to_string();
        profile.updated_at = Utc::now().to_rfc3339();

        profiles.insert(id.to_string(), profile.clone());

        log::info!("Updated personality profile: {}", profile.name);
        Ok(profile)
    }

    /// Delete a personality profile
    pub fn delete(&self, id: &str) -> Result<()> {
        let mut profiles = self.profiles.write().unwrap();

        if profiles.remove(id).is_none() {
            return Err(PersonalityError::NotFound(id.to_string()));
        }

        log::info!("Deleted personality profile: {}", id);
        Ok(())
    }

    /// Search profiles by tag
    pub fn search_by_tag(&self, tag: &str) -> Vec<PersonalityProfile> {
        let profiles = self.profiles.read().unwrap();
        profiles
            .values()
            .filter(|p| p.tags.iter().any(|t| t.to_lowercase() == tag.to_lowercase()))
            .cloned()
            .collect()
    }

    /// Search profiles by name
    pub fn search_by_name(&self, query: &str) -> Vec<PersonalityProfile> {
        let query_lower = query.to_lowercase();
        let profiles = self.profiles.read().unwrap();
        profiles
            .values()
            .filter(|p| p.name.to_lowercase().contains(&query_lower))
            .cloned()
            .collect()
    }

    /// Export all profiles to JSON
    pub fn export_json(&self) -> Result<String> {
        let profiles = self.profiles.read().unwrap();
        let list: Vec<_> = profiles.values().collect();
        Ok(serde_json::to_string_pretty(&list)?)
    }

    /// Import profiles from JSON
    pub fn import_json(&self, json: &str) -> Result<usize> {
        let imported: Vec<PersonalityProfile> = serde_json::from_str(json)?;
        let count = imported.len();

        let mut profiles = self.profiles.write().unwrap();
        for profile in imported {
            profiles.insert(profile.id.clone(), profile);
        }

        log::info!("Imported {} personality profiles", count);
        Ok(count)
    }
}

impl Default for PersonalityStore {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Preset Personalities
// ============================================================================

pub fn create_preset_personality(preset: &str) -> Option<PersonalityProfile> {
    let now = Utc::now().to_rfc3339();

    match preset {
        "tavern_keeper" => Some(PersonalityProfile {
            id: Uuid::new_v4().to_string(),
            name: "Friendly Tavern Keeper".to_string(),
            source: None,
            speech_patterns: SpeechPatterns {
                formality: 3,
                common_phrases: vec![
                    "What'll it be?".to_string(),
                    "Ye look like ye've traveled far.".to_string(),
                    "Another round for the table!".to_string(),
                ],
                vocabulary_style: "Casual, colloquial, with occasional folksy expressions".to_string(),
                dialect_notes: Some("Slight rural accent".to_string()),
                pacing: "Relaxed but efficient, picks up when busy".to_string(),
            },
            traits: vec![
                PersonalityTrait {
                    trait_name: "Hospitable".to_string(),
                    intensity: 9,
                    manifestation: "Always offers food, drink, or a warm seat".to_string(),
                },
                PersonalityTrait {
                    trait_name: "Observant".to_string(),
                    intensity: 7,
                    manifestation: "Notices everything happening in the tavern".to_string(),
                },
                PersonalityTrait {
                    trait_name: "Gossip-lover".to_string(),
                    intensity: 6,
                    manifestation: "Loves sharing and gathering local news".to_string(),
                },
            ],
            knowledge_areas: vec![
                "Local gossip".to_string(),
                "Regional news".to_string(),
                "Brewing and cooking".to_string(),
                "Traveler routes".to_string(),
            ],
            behavioral_tendencies: BehavioralTendencies {
                conflict_response: "Tries to defuse with humor or free drinks, but has a sturdy club behind the bar".to_string(),
                stranger_response: "Welcoming but curious, asks subtle questions".to_string(),
                authority_response: "Respectful but not subservient, knows their worth".to_string(),
                help_response: "Happy to help with information, expects fair payment for goods".to_string(),
                general_attitude: "Cheerful and business-minded".to_string(),
            },
            example_phrases: vec![
                "Well now, what brings ye to our humble establishment?".to_string(),
                "I've heard tell of some strange goings-on up at the old mill...".to_string(),
                "Best stew this side of the river, if I do say so myself!".to_string(),
            ],
            tags: vec!["npc".to_string(), "tavern".to_string(), "friendly".to_string()],
            metadata: HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
        }),

        "grumpy_merchant" => Some(PersonalityProfile {
            id: Uuid::new_v4().to_string(),
            name: "Grumpy Merchant".to_string(),
            source: None,
            speech_patterns: SpeechPatterns {
                formality: 5,
                common_phrases: vec![
                    "Hmph.".to_string(),
                    "You break it, you buy it.".to_string(),
                    "That's the price, take it or leave it.".to_string(),
                ],
                vocabulary_style: "Curt, business-focused, minimal pleasantries".to_string(),
                dialect_notes: None,
                pacing: "Quick, impatient".to_string(),
            },
            traits: vec![
                PersonalityTrait {
                    trait_name: "Shrewd".to_string(),
                    intensity: 9,
                    manifestation: "Never misses an opportunity for profit".to_string(),
                },
                PersonalityTrait {
                    trait_name: "Impatient".to_string(),
                    intensity: 8,
                    manifestation: "Sighs and taps fingers during negotiations".to_string(),
                },
                PersonalityTrait {
                    trait_name: "Secretly Kind".to_string(),
                    intensity: 4,
                    manifestation: "Occasionally gives discounts to those truly in need".to_string(),
                },
            ],
            knowledge_areas: vec![
                "Trade routes".to_string(),
                "Item values".to_string(),
                "Market trends".to_string(),
            ],
            behavioral_tendencies: BehavioralTendencies {
                conflict_response: "Refuses service and threatens to call guards".to_string(),
                stranger_response: "Suspicious, assumes they'll waste time without buying".to_string(),
                authority_response: "Complies but complains constantly".to_string(),
                help_response: "Only helps if there's profit involved".to_string(),
                general_attitude: "Pessimistic about everything except making money".to_string(),
            },
            example_phrases: vec![
                "You gonna buy something or just gawk all day?".to_string(),
                "Fine craftsmanship costs good coin.".to_string(),
                "*grumble* ...I suppose I can knock off a few coppers. Just this once.".to_string(),
            ],
            tags: vec!["npc".to_string(), "merchant".to_string(), "grumpy".to_string()],
            metadata: HashMap::new(),
            created_at: now.clone(),
            updated_at: now,
        }),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personality_store_crud() {
        let store = PersonalityStore::new();

        // Create
        let profile = PersonalityProfile {
            id: String::new(),
            name: "Test Profile".to_string(),
            source: None,
            speech_patterns: SpeechPatterns::default(),
            traits: vec![],
            knowledge_areas: vec![],
            behavioral_tendencies: BehavioralTendencies::default(),
            example_phrases: vec![],
            tags: vec!["test".to_string()],
            metadata: HashMap::new(),
            created_at: String::new(),
            updated_at: String::new(),
        };

        let created = store.create(profile).unwrap();
        assert!(!created.id.is_empty());
        assert_eq!(created.name, "Test Profile");

        // Read
        let fetched = store.get(&created.id).unwrap();
        assert_eq!(fetched.name, "Test Profile");

        // Update
        let mut updated = fetched.clone();
        updated.name = "Updated Profile".to_string();
        let result = store.update(&created.id, updated).unwrap();
        assert_eq!(result.name, "Updated Profile");

        // Delete
        store.delete(&created.id).unwrap();
        assert!(store.get(&created.id).is_err());
    }

    #[test]
    fn test_preset_personalities() {
        let tavern = create_preset_personality("tavern_keeper");
        assert!(tavern.is_some());
        assert!(tavern.unwrap().name.contains("Tavern"));

        let unknown = create_preset_personality("unknown_preset");
        assert!(unknown.is_none());
    }
}
