//! Voice Profile System (TASK-004)
//!
//! Manages voice profiles linked to NPCs and personalities.
//! Provides CRUD operations and NPC-to-profile linking.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

use super::types::{VoiceProviderType, VoiceSettings};

// ============================================================================
// Profile Types
// ============================================================================

/// Age range categories for voice profiles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum AgeRange {
    Child,
    YoungAdult,
    #[default]
    Adult,
    MiddleAged,
    Elderly,
}


impl AgeRange {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Child => "Child (0-12)",
            Self::YoungAdult => "Young Adult (13-25)",
            Self::Adult => "Adult (26-45)",
            Self::MiddleAged => "Middle-Aged (46-65)",
            Self::Elderly => "Elderly (65+)",
        }
    }
}

/// Gender categories for voice profiles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum Gender {
    Male,
    Female,
    #[default]
    Neutral,
    NonBinary,
}


impl Gender {
    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Male => "Male",
            Self::Female => "Female",
            Self::Neutral => "Neutral",
            Self::NonBinary => "Non-Binary",
        }
    }
}

/// Metadata for voice profiles including personality traits
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileMetadata {
    /// Age range of the voice
    pub age_range: AgeRange,
    /// Gender of the voice
    pub gender: Gender,
    /// Personality traits (e.g., "gruff", "cheerful", "mysterious")
    pub personality_traits: Vec<String>,
    /// IDs of NPCs linked to this profile
    pub linked_npc_ids: Vec<String>,
    /// Optional description of the voice
    pub description: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl ProfileMetadata {
    /// Create new metadata with basic info
    pub fn new(age_range: AgeRange, gender: Gender) -> Self {
        Self {
            age_range,
            gender,
            personality_traits: Vec::new(),
            linked_npc_ids: Vec::new(),
            description: None,
            tags: Vec::new(),
        }
    }

    /// Add a personality trait
    pub fn with_trait(mut self, trait_name: &str) -> Self {
        self.personality_traits.push(trait_name.to_string());
        self
    }

    /// Add multiple personality traits
    pub fn with_traits(mut self, traits: &[&str]) -> Self {
        self.personality_traits.extend(traits.iter().map(|s| s.to_string()));
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }
}

/// A complete voice profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceProfile {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Voice provider to use
    pub provider: VoiceProviderType,
    /// Provider-specific voice ID (e.g., ElevenLabs voice ID)
    pub voice_id: String,
    /// Voice settings (stability, similarity, etc.)
    pub settings: VoiceSettings,
    /// Profile metadata
    pub metadata: ProfileMetadata,
    /// Whether this is a built-in preset
    pub is_preset: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl VoiceProfile {
    /// Create a new voice profile
    pub fn new(
        name: &str,
        provider: VoiceProviderType,
        voice_id: &str,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            provider,
            voice_id: voice_id.to_string(),
            settings: VoiceSettings::default(),
            metadata: ProfileMetadata::default(),
            is_preset: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a preset profile (built-in, non-editable)
    pub fn preset(
        id: &str,
        name: &str,
        provider: VoiceProviderType,
        voice_id: &str,
        metadata: ProfileMetadata,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            provider,
            voice_id: voice_id.to_string(),
            settings: VoiceSettings::default(),
            metadata,
            is_preset: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set custom voice settings
    pub fn with_settings(mut self, settings: VoiceSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: ProfileMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Link this profile to an NPC
    pub fn link_npc(&mut self, npc_id: &str) {
        if !self.metadata.linked_npc_ids.contains(&npc_id.to_string()) {
            self.metadata.linked_npc_ids.push(npc_id.to_string());
            self.updated_at = Utc::now();
        }
    }

    /// Unlink this profile from an NPC
    pub fn unlink_npc(&mut self, npc_id: &str) {
        self.metadata.linked_npc_ids.retain(|id| id != npc_id);
        self.updated_at = Utc::now();
    }

    /// Check if this profile is linked to an NPC
    pub fn is_linked_to(&self, npc_id: &str) -> bool {
        self.metadata.linked_npc_ids.contains(&npc_id.to_string())
    }
}

// ============================================================================
// Profile Manager
// ============================================================================

/// Error type for profile operations
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("Profile not found: {0}")]
    NotFound(String),

    #[error("Cannot modify preset profile: {0}")]
    CannotModifyPreset(String),

    #[error("Profile already exists: {0}")]
    AlreadyExists(String),

    #[error("NPC already linked to another profile: {0}")]
    NpcAlreadyLinked(String),

    #[error("Invalid profile data: {0}")]
    InvalidData(String),

    #[error("Storage error: {0}")]
    StorageError(String),
}

pub type ProfileResult<T> = Result<T, ProfileError>;

/// Manages voice profiles with CRUD operations and NPC linking
#[derive(Debug)]
pub struct VoiceProfileManager {
    /// User-created profiles
    profiles: HashMap<String, VoiceProfile>,
    /// Built-in preset profiles
    presets: Vec<VoiceProfile>,
    /// Reverse lookup: NPC ID -> Profile ID
    npc_to_profile: HashMap<String, String>,
}

impl VoiceProfileManager {
    /// Create a new profile manager with presets loaded
    pub fn new() -> Self {
        let presets = super::presets::get_dm_presets();
        Self {
            profiles: HashMap::new(),
            presets,
            npc_to_profile: HashMap::new(),
        }
    }

    /// Create a new profile
    pub fn create_profile(&mut self, mut profile: VoiceProfile) -> ProfileResult<String> {
        if self.profiles.contains_key(&profile.id) || self.get_preset(&profile.id).is_some() {
            return Err(ProfileError::AlreadyExists(profile.id));
        }

        profile.is_preset = false;
        profile.updated_at = Utc::now();
        let id = profile.id.clone();
        self.profiles.insert(id.clone(), profile);
        Ok(id)
    }

    /// Get a profile by ID (checks user profiles first, then presets)
    pub fn get_profile(&self, id: &str) -> Option<&VoiceProfile> {
        self.profiles.get(id).or_else(|| self.get_preset(id))
    }

    /// Get a mutable reference to a user profile (presets cannot be modified)
    pub fn get_profile_mut(&mut self, id: &str) -> ProfileResult<&mut VoiceProfile> {
        if self.get_preset(id).is_some() {
            return Err(ProfileError::CannotModifyPreset(id.to_string()));
        }
        self.profiles
            .get_mut(id)
            .ok_or_else(|| ProfileError::NotFound(id.to_string()))
    }

    /// Update an existing profile
    pub fn update_profile(&mut self, profile: VoiceProfile) -> ProfileResult<()> {
        if profile.is_preset || self.get_preset(&profile.id).is_some() {
            return Err(ProfileError::CannotModifyPreset(profile.id));
        }

        if !self.profiles.contains_key(&profile.id) {
            return Err(ProfileError::NotFound(profile.id));
        }

        let mut updated = profile;
        updated.updated_at = Utc::now();
        self.profiles.insert(updated.id.clone(), updated);
        Ok(())
    }

    /// Delete a user profile
    pub fn delete_profile(&mut self, id: &str) -> ProfileResult<()> {
        if self.get_preset(id).is_some() {
            return Err(ProfileError::CannotModifyPreset(id.to_string()));
        }

        // Remove NPC linkages
        let linked_npcs: Vec<String> = self.npc_to_profile
            .iter()
            .filter(|(_, pid)| *pid == id)
            .map(|(nid, _)| nid.clone())
            .collect();

        for npc_id in linked_npcs {
            self.npc_to_profile.remove(&npc_id);
        }

        self.profiles
            .remove(id)
            .ok_or_else(|| ProfileError::NotFound(id.to_string()))?;

        Ok(())
    }

    /// List all user profiles
    pub fn list_profiles(&self) -> Vec<&VoiceProfile> {
        self.profiles.values().collect()
    }

    /// List all preset profiles
    pub fn list_presets(&self) -> &[VoiceProfile] {
        &self.presets
    }

    /// List all profiles (user + presets)
    pub fn list_all(&self) -> Vec<&VoiceProfile> {
        let mut all: Vec<&VoiceProfile> = self.profiles.values().collect();
        all.extend(self.presets.iter());
        all
    }

    /// Get a preset by ID
    fn get_preset(&self, id: &str) -> Option<&VoiceProfile> {
        self.presets.iter().find(|p| p.id == id)
    }

    /// Link an NPC to a voice profile
    pub fn link_to_npc(&mut self, profile_id: &str, npc_id: &str) -> ProfileResult<()> {
        // Verify profile exists
        if self.get_profile(profile_id).is_none() {
            return Err(ProfileError::NotFound(profile_id.to_string()));
        }

        // Check if NPC is already linked to a different profile
        if let Some(existing_profile) = self.npc_to_profile.get(npc_id) {
            if existing_profile != profile_id {
                // Unlink from existing profile first
                self.unlink_from_npc(npc_id)?;
            }
        }

        // Update the profile's linked_npc_ids
        if let Some(profile) = self.profiles.get_mut(profile_id) {
            profile.link_npc(npc_id);
        }
        // Note: Presets can be linked but not modified in-place

        // Update reverse lookup
        self.npc_to_profile.insert(npc_id.to_string(), profile_id.to_string());

        Ok(())
    }

    /// Unlink an NPC from its voice profile
    pub fn unlink_from_npc(&mut self, npc_id: &str) -> ProfileResult<()> {
        if let Some(profile_id) = self.npc_to_profile.remove(npc_id) {
            // Update the profile's linked_npc_ids if it's a user profile
            if let Some(profile) = self.profiles.get_mut(&profile_id) {
                profile.unlink_npc(npc_id);
            }
        }
        Ok(())
    }

    /// Get the profile linked to an NPC
    pub fn get_profile_for_npc(&self, npc_id: &str) -> Option<&VoiceProfile> {
        self.npc_to_profile
            .get(npc_id)
            .and_then(|pid| self.get_profile(pid))
    }

    /// Get the profile ID for an NPC
    pub fn get_profile_id_for_npc(&self, npc_id: &str) -> Option<&String> {
        self.npc_to_profile.get(npc_id)
    }

    /// Search profiles by name or traits
    pub fn search(&self, query: &str) -> Vec<&VoiceProfile> {
        let query_lower = query.to_lowercase();
        self.list_all()
            .into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower)
                    || p.metadata.personality_traits.iter().any(|t| t.to_lowercase().contains(&query_lower))
                    || p.metadata.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                    || p.metadata.description.as_ref().is_some_and(|d| d.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Filter profiles by gender
    pub fn filter_by_gender(&self, gender: Gender) -> Vec<&VoiceProfile> {
        self.list_all()
            .into_iter()
            .filter(|p| p.metadata.gender == gender)
            .collect()
    }

    /// Filter profiles by age range
    pub fn filter_by_age(&self, age_range: AgeRange) -> Vec<&VoiceProfile> {
        self.list_all()
            .into_iter()
            .filter(|p| p.metadata.age_range == age_range)
            .collect()
    }

    /// Filter profiles by provider
    pub fn filter_by_provider(&self, provider: VoiceProviderType) -> Vec<&VoiceProfile> {
        self.list_all()
            .into_iter()
            .filter(|p| p.provider == provider)
            .collect()
    }

    /// Get profiles with a specific personality trait
    pub fn with_trait(&self, trait_name: &str) -> Vec<&VoiceProfile> {
        let trait_lower = trait_name.to_lowercase();
        self.list_all()
            .into_iter()
            .filter(|p| {
                p.metadata.personality_traits.iter().any(|t| t.to_lowercase() == trait_lower)
            })
            .collect()
    }

    /// Export all user profiles to JSON
    pub fn export_profiles(&self) -> Result<String, ProfileError> {
        let profiles: Vec<&VoiceProfile> = self.profiles.values().collect();
        serde_json::to_string_pretty(&profiles)
            .map_err(|e| ProfileError::StorageError(e.to_string()))
    }

    /// Import profiles from JSON
    pub fn import_profiles(&mut self, json: &str) -> ProfileResult<usize> {
        let profiles: Vec<VoiceProfile> = serde_json::from_str(json)
            .map_err(|e| ProfileError::InvalidData(e.to_string()))?;

        let mut count = 0;
        for mut profile in profiles {
            profile.is_preset = false;  // Imported profiles are never presets
            if !self.profiles.contains_key(&profile.id) {
                self.profiles.insert(profile.id.clone(), profile);
                count += 1;
            }
        }
        Ok(count)
    }

    /// Get statistics about profiles
    pub fn stats(&self) -> ProfileStats {
        ProfileStats {
            total_user_profiles: self.profiles.len(),
            total_presets: self.presets.len(),
            linked_npcs: self.npc_to_profile.len(),
            profiles_by_provider: self.count_by_provider(),
            profiles_by_gender: self.count_by_gender(),
        }
    }

    fn count_by_provider(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for profile in self.list_all() {
            let key = format!("{:?}", profile.provider);
            *counts.entry(key).or_insert(0) += 1;
        }
        counts
    }

    fn count_by_gender(&self) -> HashMap<String, usize> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for profile in self.list_all() {
            let key = profile.metadata.gender.display_name().to_string();
            *counts.entry(key).or_insert(0) += 1;
        }
        counts
    }
}

impl Default for VoiceProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about voice profiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileStats {
    pub total_user_profiles: usize,
    pub total_presets: usize,
    pub linked_npcs: usize,
    pub profiles_by_provider: HashMap<String, usize>,
    pub profiles_by_gender: HashMap<String, usize>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_profile() {
        let mut manager = VoiceProfileManager::new();
        let profile = VoiceProfile::new("Test Voice", VoiceProviderType::OpenAI, "alloy");

        let id = manager.create_profile(profile.clone()).unwrap();
        assert!(!id.is_empty());

        let retrieved = manager.get_profile(&id).unwrap();
        assert_eq!(retrieved.name, "Test Voice");
    }

    #[test]
    fn test_link_npc() {
        let mut manager = VoiceProfileManager::new();
        let profile = VoiceProfile::new("NPC Voice", VoiceProviderType::OpenAI, "echo");
        let profile_id = manager.create_profile(profile).unwrap();

        let npc_id = "npc-123";
        manager.link_to_npc(&profile_id, npc_id).unwrap();

        let linked = manager.get_profile_for_npc(npc_id).unwrap();
        assert_eq!(linked.id, profile_id);
    }

    #[test]
    fn test_presets_exist() {
        let manager = VoiceProfileManager::new();
        let presets = manager.list_presets();
        assert!(!presets.is_empty(), "Should have preset profiles");
        assert!(presets.len() >= 13, "Should have at least 13 DM personas");
    }

    #[test]
    fn test_cannot_modify_preset() {
        let mut manager = VoiceProfileManager::new();
        let preset_id = &manager.list_presets()[0].id.clone();

        let result = manager.get_profile_mut(preset_id);
        assert!(matches!(result, Err(ProfileError::CannotModifyPreset(_))));
    }

    #[test]
    fn test_search_profiles() {
        let mut manager = VoiceProfileManager::new();
        let mut profile = VoiceProfile::new("Gruff Warrior", VoiceProviderType::OpenAI, "onyx");
        profile.metadata.personality_traits = vec!["gruff".to_string(), "battle-hardened".to_string()];
        manager.create_profile(profile).unwrap();

        let results = manager.search("gruff");
        assert!(!results.is_empty());
        assert!(results.iter().any(|p| p.name == "Gruff Warrior"));
    }

    #[test]
    fn test_filter_by_gender() {
        let manager = VoiceProfileManager::new();
        let male_profiles = manager.filter_by_gender(Gender::Male);
        let female_profiles = manager.filter_by_gender(Gender::Female);

        // Presets should include both genders
        assert!(!male_profiles.is_empty() || !female_profiles.is_empty());
    }
}
