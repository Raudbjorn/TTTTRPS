//! Setting Pack Loader for the Archetype Registry.
//!
//! The [`SettingPackLoader`] manages the lifecycle of setting packs:
//!
//! - **Loading**: Parse and validate setting packs from YAML/JSON files
//! - **Validation**: Ensure required fields and semantic version format
//! - **Activation**: Enable a pack for a specific campaign
//! - **Deactivation**: Remove a pack from a campaign
//! - **Version Management**: Store and retrieve multiple versions of packs
//!
//! # Load-Then-Activate Pattern
//!
//! Setting packs follow a strict load-then-activate pattern:
//!
//! ```text
//! ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
//! │    Load      │────>│   Validate   │────>│   Register   │
//! │  (from disk) │     │  (schema +   │     │  (available) │
//! │              │     │   semver)    │     │              │
//! └──────────────┘     └──────────────┘     └──────────────┘
//!                                                 │
//!                                                 v
//! ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
//! │   Resolve    │<────│    Apply     │<────│   Activate   │
//! │ (with pack)  │     │  (overrides) │     │(for campaign)│
//! └──────────────┘     └──────────────┘     └──────────────┘
//! ```
//!
//! **Key Principle**: Loading a setting pack does not affect any campaigns
//! until explicitly activated.
//!
//! # Thread Safety (CRITICAL-ARCH-002)
//!
//! All mutable state is protected by `tokio::sync::RwLock` for async-safe
//! access in the Tauri async command context.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::core::archetype::setting_pack_loader::SettingPackLoader;
//!
//! let loader = SettingPackLoader::new();
//!
//! // Load a setting pack from file
//! let pack_id = loader.load_from_file("/path/to/forgotten_realms.yaml").await?;
//!
//! // Activate for a campaign (validates archetype references)
//! loader.activate(&pack_id, "campaign_123", &registry).await?;
//!
//! // Get active pack for campaign
//! if let Some(active_pack_id) = loader.get_active("campaign_123").await {
//!     println!("Active pack: {}", active_pack_id);
//! }
//!
//! // Deactivate
//! loader.deactivate("campaign_123").await?;
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::error::{ArchetypeError, Result};
use super::setting_pack::{compare_semver, SettingPack, SettingPackSummary};

// ============================================================================
// Constants
// ============================================================================

/// Minimum required application version field (optional compatibility check).
/// Format: "MAJOR.MINOR.PATCH"
pub const DEFAULT_MIN_APP_VERSION: &str = "1.0.0";

// ============================================================================
// Event Types (stubs for future event system)
// ============================================================================

/// Events emitted by the setting pack loader.
#[derive(Debug, Clone)]
pub enum SettingPackEvent {
    /// A setting pack was loaded from file.
    Loaded {
        /// Pack ID
        pack_id: String,
        /// Pack version
        version: String,
    },

    /// A setting pack was activated for a campaign.
    Activated {
        /// Pack ID
        pack_id: String,
        /// Campaign ID
        campaign_id: String,
    },

    /// A setting pack was deactivated.
    Deactivated {
        /// Pack ID
        pack_id: String,
        /// Campaign ID
        campaign_id: String,
    },
}

// ============================================================================
// Version Key
// ============================================================================

/// Create a version key for storing multiple versions of the same pack.
///
/// Format: `{pack_id}@{version}`
///
/// # Examples
///
/// ```rust,ignore
/// let key = version_key("forgotten_realms", "1.0.0");
/// assert_eq!(key, "forgotten_realms@1.0.0");
/// ```
fn version_key(pack_id: &str, version: &str) -> String {
    format!("{}@{}", pack_id, version)
}

/// Parse a version key into (pack_id, version).
///
/// # Returns
///
/// `Some((pack_id, version))` if the key is valid, `None` otherwise.
fn parse_version_key(key: &str) -> Option<(&str, &str)> {
    key.rsplit_once('@')
}

// ============================================================================
// SettingPackLoader
// ============================================================================

/// Manages setting pack lifecycle: loading, validation, activation, and versioning.
///
/// # Architecture
///
/// ```text
///                   SettingPackLoader
///                         │
///     ┌───────────────────┼───────────────────┐
///     │                   │                   │
/// loaded_packs       active_packs       version_index
/// (versioned)        (campaign->pack)   (pack->versions)
/// ```
///
/// # Thread Safety
///
/// All async-accessed fields use `tokio::sync::RwLock` for proper async operation.
pub struct SettingPackLoader {
    /// Loaded packs indexed by version key: `{pack_id}@{version}`.
    ///
    /// This allows storing multiple versions of the same pack.
    loaded_packs: Arc<RwLock<HashMap<String, SettingPack>>>,

    /// Active pack per campaign: `campaign_id -> pack_version_key`.
    ///
    /// Only one pack can be active per campaign at a time.
    active_packs: Arc<RwLock<HashMap<String, String>>>,

    /// Version index: `pack_id -> Vec<version>` (sorted by semver).
    ///
    /// Tracks all loaded versions of each pack for version management.
    version_index: Arc<RwLock<HashMap<String, Vec<String>>>>,

    /// Event listeners (stub for future event system).
    #[allow(dead_code)]
    event_listeners: Arc<RwLock<Vec<Box<dyn Fn(SettingPackEvent) + Send + Sync>>>>,
}

impl Default for SettingPackLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingPackLoader {
    /// Create a new setting pack loader.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let loader = SettingPackLoader::new();
    /// ```
    pub fn new() -> Self {
        Self {
            loaded_packs: Arc::new(RwLock::new(HashMap::new())),
            active_packs: Arc::new(RwLock::new(HashMap::new())),
            version_index: Arc::new(RwLock::new(HashMap::new())),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    // ========================================================================
    // Loading Operations (TASK-ARCH-050)
    // ========================================================================

    /// Load a setting pack from a file (YAML or JSON).
    ///
    /// This method:
    /// 1. Reads the file content using async I/O
    /// 2. Parses as YAML or JSON based on file extension
    /// 3. Validates required fields and semantic version
    /// 4. Stores the pack in the loaded packs map (not activated)
    /// 5. Emits `SETTING_PACK_LOADED` event
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the setting pack file (.yaml, .yml, or .json)
    ///
    /// # Returns
    ///
    /// The version key of the loaded pack (`{pack_id}@{version}`).
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::Io` if the file cannot be read
    /// - `ArchetypeError::SettingPackInvalid` if validation fails
    /// - `ArchetypeError::YamlParse` if YAML parsing fails
    /// - `ArchetypeError::Serialization` if JSON parsing fails
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let key = loader.load_from_file("content_packs/forgotten_realms.yaml").await?;
    /// // key = "forgotten_realms@1.0.0"
    /// ```
    pub async fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let path = path.as_ref();

        // Read file content using tokio::fs (async I/O)
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ArchetypeError::SettingPackInvalid {
                pack_id: path.display().to_string(),
                reason: format!("Failed to read file: {}", e),
            })?;

        // Determine format from extension
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        let pack: SettingPack = match extension.as_str() {
            "yaml" | "yml" => serde_yaml_ng::from_str(&content).map_err(|e| {
                ArchetypeError::SettingPackInvalid {
                    pack_id: path.display().to_string(),
                    reason: format!("Invalid YAML: {}", e),
                }
            })?,
            "json" => serde_json::from_str(&content).map_err(|e| {
                ArchetypeError::SettingPackInvalid {
                    pack_id: path.display().to_string(),
                    reason: format!("Invalid JSON: {}", e),
                }
            })?,
            _ => {
                // Try YAML first, then JSON
                serde_yaml_ng::from_str(&content)
                    .or_else(|_| serde_json::from_str(&content).map_err(ArchetypeError::from))
                    .map_err(|_| ArchetypeError::SettingPackInvalid {
                        pack_id: path.display().to_string(),
                        reason: "File must be valid YAML or JSON".to_string(),
                    })?
            }
        };

        // Validate and store the pack
        self.load_pack(pack).await
    }

    /// Load a setting pack from a string (YAML format).
    ///
    /// # Arguments
    ///
    /// * `content` - YAML string content
    ///
    /// # Returns
    ///
    /// The version key of the loaded pack.
    pub async fn load_from_yaml(&self, content: &str) -> Result<String> {
        let pack: SettingPack = serde_yaml_ng::from_str(content)?;
        self.load_pack(pack).await
    }

    /// Load a setting pack from a string (JSON format).
    ///
    /// # Arguments
    ///
    /// * `content` - JSON string content
    ///
    /// # Returns
    ///
    /// The version key of the loaded pack.
    pub async fn load_from_json(&self, content: &str) -> Result<String> {
        let pack: SettingPack = serde_json::from_str(content)?;
        self.load_pack(pack).await
    }

    /// Load a setting pack object directly.
    ///
    /// This validates and stores the pack without parsing from a file.
    ///
    /// # Arguments
    ///
    /// * `pack` - The setting pack to load
    ///
    /// # Returns
    ///
    /// The version key of the loaded pack.
    pub async fn load_pack(&self, pack: SettingPack) -> Result<String> {
        // Validate the pack
        self.validate_pack(&pack)?;

        let pack_id = pack.id.clone();
        let version = pack.version.clone();
        let vkey = version_key(&pack_id, &version);

        // Store the pack
        {
            let mut loaded = self.loaded_packs.write().await;
            loaded.insert(vkey.clone(), pack);
        }

        // Update version index
        self.add_to_version_index(&pack_id, &version).await;

        // Emit event (stub)
        self.emit_event(SettingPackEvent::Loaded {
            pack_id,
            version,
        })
        .await;

        log::info!("Loaded setting pack: {}", vkey);

        Ok(vkey)
    }

    /// Validate a setting pack for required fields and format.
    ///
    /// # Validation Rules
    ///
    /// - `id` must not be empty
    /// - `name` must not be empty
    /// - `game_system` must not be empty
    /// - `version` must be valid semantic version (MAJOR.MINOR.PATCH)
    ///
    /// # Note
    ///
    /// Archetype reference validation is deferred to activation time
    /// when the registry is available.
    fn validate_pack(&self, pack: &SettingPack) -> Result<()> {
        // Use the pack's built-in validation
        pack.validate()?;

        // Additional validation for required fields
        if pack.game_system.is_empty() {
            return Err(ArchetypeError::SettingPackInvalid {
                pack_id: pack.id.clone(),
                reason: "Game system is required".to_string(),
            });
        }

        Ok(())
    }

    // ========================================================================
    // Activation Operations (TASK-ARCH-051)
    // ========================================================================

    /// Activate a setting pack for a campaign.
    ///
    /// This method:
    /// 1. Verifies the pack is loaded
    /// 2. Validates all archetype references exist in the registry
    /// 3. Deactivates any previously active pack for this campaign
    /// 4. Activates the new pack
    /// 5. Invalidates cache for the campaign
    /// 6. Emits `SETTING_PACK_ACTIVATED` event
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the pack to activate (uses latest version if no version specified)
    /// * `campaign_id` - ID of the campaign to activate for
    /// * `existing_archetypes` - Set of archetype IDs that exist in the registry
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::SettingPackNotFound` if pack is not loaded
    /// - `ArchetypeError::SettingPackReferenceError` if pack references missing archetypes
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Get existing archetype IDs from registry
    /// let existing: HashSet<String> = registry.list(None).await
    ///     .iter()
    ///     .map(|a| a.id.to_string())
    ///     .collect();
    ///
    /// loader.activate("forgotten_realms", "campaign_123", &existing).await?;
    /// ```
    pub async fn activate(
        &self,
        pack_id: &str,
        campaign_id: &str,
        existing_archetypes: &std::collections::HashSet<String>,
    ) -> Result<()> {
        // If pack_id contains @, use it as-is; otherwise get latest version
        let vkey = if pack_id.contains('@') {
            pack_id.to_string()
        } else {
            self.get_latest_version_key(pack_id).await?
        };

        // Verify pack is loaded and get it for validation
        let pack = {
            let loaded = self.loaded_packs.read().await;
            loaded
                .get(&vkey)
                .cloned()
                .ok_or_else(|| ArchetypeError::SettingPackNotFound(vkey.clone()))?
        };

        // Validate all referenced archetypes exist
        let missing: Vec<String> = pack
            .archetype_overrides
            .keys()
            .filter(|id| !existing_archetypes.contains(*id))
            .cloned()
            .collect();

        if !missing.is_empty() {
            return Err(ArchetypeError::SettingPackReferenceError {
                pack_id: pack.id.clone(),
                missing_ids: missing,
            });
        }

        // Deactivate any existing pack for this campaign
        let old_pack = {
            let active = self.active_packs.read().await;
            active.get(campaign_id).cloned()
        };

        if let Some(old_vkey) = old_pack {
            self.deactivate_internal(campaign_id, &old_vkey).await?;
        }

        // Activate new pack
        {
            let mut active = self.active_packs.write().await;
            active.insert(campaign_id.to_string(), vkey.clone());
        }

        // Emit event
        self.emit_event(SettingPackEvent::Activated {
            pack_id: pack.id.clone(),
            campaign_id: campaign_id.to_string(),
        })
        .await;

        log::info!(
            "Activated setting pack '{}' for campaign '{}'",
            vkey,
            campaign_id
        );

        Ok(())
    }

    /// Activate a specific version of a setting pack.
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the pack
    /// * `version` - Specific version to activate
    /// * `campaign_id` - ID of the campaign
    /// * `existing_archetypes` - Set of archetype IDs that exist
    pub async fn activate_version(
        &self,
        pack_id: &str,
        version: &str,
        campaign_id: &str,
        existing_archetypes: &std::collections::HashSet<String>,
    ) -> Result<()> {
        let vkey = version_key(pack_id, version);
        self.activate(&vkey, campaign_id, existing_archetypes).await
    }

    /// Deactivate the setting pack for a campaign.
    ///
    /// This removes the pack from the campaign without deleting the loaded pack.
    ///
    /// # Arguments
    ///
    /// * `campaign_id` - ID of the campaign to deactivate pack for
    ///
    /// # Returns
    ///
    /// `Ok(())` even if no pack was active (idempotent).
    pub async fn deactivate(&self, campaign_id: &str) -> Result<()> {
        let old_vkey = {
            let mut active = self.active_packs.write().await;
            active.remove(campaign_id)
        };

        if let Some(vkey) = old_vkey {
            // Extract pack_id from version key
            if let Some((pack_id, _)) = parse_version_key(&vkey) {
                self.emit_event(SettingPackEvent::Deactivated {
                    pack_id: pack_id.to_string(),
                    campaign_id: campaign_id.to_string(),
                })
                .await;

                log::info!(
                    "Deactivated setting pack '{}' for campaign '{}'",
                    vkey,
                    campaign_id
                );
            }
        }

        Ok(())
    }

    /// Internal deactivation without removing from active_packs.
    async fn deactivate_internal(&self, campaign_id: &str, vkey: &str) -> Result<()> {
        if let Some((pack_id, _)) = parse_version_key(vkey) {
            self.emit_event(SettingPackEvent::Deactivated {
                pack_id: pack_id.to_string(),
                campaign_id: campaign_id.to_string(),
            })
            .await;

            log::info!(
                "Deactivated setting pack '{}' for campaign '{}'",
                vkey,
                campaign_id
            );
        }

        Ok(())
    }

    /// Get the active setting pack ID for a campaign.
    ///
    /// # Arguments
    ///
    /// * `campaign_id` - ID of the campaign
    ///
    /// # Returns
    ///
    /// `Some(version_key)` if a pack is active, `None` otherwise.
    pub async fn get_active(&self, campaign_id: &str) -> Option<String> {
        let active = self.active_packs.read().await;
        active.get(campaign_id).cloned()
    }

    /// Get the active setting pack for a campaign.
    ///
    /// # Arguments
    ///
    /// * `campaign_id` - ID of the campaign
    ///
    /// # Returns
    ///
    /// `Some(SettingPack)` if a pack is active, `None` otherwise.
    pub async fn get_active_pack(&self, campaign_id: &str) -> Option<SettingPack> {
        let vkey = self.get_active(campaign_id).await?;
        let loaded = self.loaded_packs.read().await;
        loaded.get(&vkey).cloned()
    }

    /// Check if a pack is active for any campaign.
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the pack to check
    ///
    /// # Returns
    ///
    /// List of campaign IDs where this pack (any version) is active.
    pub async fn get_campaigns_using_pack(&self, pack_id: &str) -> Vec<String> {
        let active = self.active_packs.read().await;
        active
            .iter()
            .filter_map(|(campaign_id, vkey)| {
                if let Some((pid, _)) = parse_version_key(vkey) {
                    if pid == pack_id {
                        return Some(campaign_id.clone());
                    }
                }
                None
            })
            .collect()
    }

    // ========================================================================
    // Version Management Operations (TASK-ARCH-052)
    // ========================================================================

    /// Get a specific version of a setting pack.
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the pack
    /// * `version` - Version string (e.g., "1.0.0")
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::PackVersionNotFound` if the version is not loaded
    pub async fn get_version(&self, pack_id: &str, version: &str) -> Result<SettingPack> {
        let vkey = version_key(pack_id, version);
        let loaded = self.loaded_packs.read().await;
        loaded
            .get(&vkey)
            .cloned()
            .ok_or_else(|| ArchetypeError::PackVersionNotFound {
                pack_id: pack_id.to_string(),
                version: version.to_string(),
            })
    }

    /// Get the latest (highest semver) version of a setting pack.
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the pack
    ///
    /// # Errors
    ///
    /// - `ArchetypeError::SettingPackNotFound` if no versions are loaded
    pub async fn get_latest(&self, pack_id: &str) -> Result<SettingPack> {
        let vkey = self.get_latest_version_key(pack_id).await?;
        let loaded = self.loaded_packs.read().await;
        loaded
            .get(&vkey)
            .cloned()
            .ok_or_else(|| ArchetypeError::SettingPackNotFound(pack_id.to_string()))
    }

    /// Get the version key for the latest version of a pack.
    async fn get_latest_version_key(&self, pack_id: &str) -> Result<String> {
        let index = self.version_index.read().await;
        let versions = index
            .get(pack_id)
            .ok_or_else(|| ArchetypeError::SettingPackNotFound(pack_id.to_string()))?;

        let latest = versions
            .last()
            .ok_or_else(|| ArchetypeError::SettingPackNotFound(pack_id.to_string()))?;

        Ok(version_key(pack_id, latest))
    }

    /// Get all loaded versions of a setting pack.
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the pack
    ///
    /// # Returns
    ///
    /// Vector of version strings, sorted by semver (oldest to newest).
    pub async fn get_versions(&self, pack_id: &str) -> Vec<String> {
        let index = self.version_index.read().await;
        index.get(pack_id).cloned().unwrap_or_default()
    }

    /// Check if a specific version of a pack is loaded.
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the pack
    /// * `version` - Version to check
    pub async fn has_version(&self, pack_id: &str, version: &str) -> bool {
        let vkey = version_key(pack_id, version);
        let loaded = self.loaded_packs.read().await;
        loaded.contains_key(&vkey)
    }

    /// Check if a pack meets the minimum app version requirement.
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the pack
    /// * `version` - Version of the pack
    /// * `app_version` - Current application version
    ///
    /// # Returns
    ///
    /// `true` if the pack is compatible, `false` otherwise.
    /// Returns `true` if the pack has no `requires_app_version` field.
    pub async fn check_app_compatibility(
        &self,
        pack_id: &str,
        version: &str,
        app_version: &str,
    ) -> Result<bool> {
        let pack = self.get_version(pack_id, version).await?;

        // If no requirement, pack is compatible
        // Note: SettingPack doesn't have requires_app_version field in current schema
        // This is a future-proofing stub that always returns true
        let _ = pack; // suppress unused warning
        let _ = app_version;

        Ok(true)
    }

    /// Add a version to the version index, maintaining semver sort order.
    async fn add_to_version_index(&self, pack_id: &str, version: &str) {
        let mut index = self.version_index.write().await;
        let versions = index.entry(pack_id.to_string()).or_default();

        // Check if version already exists
        if versions.contains(&version.to_string()) {
            return;
        }

        // Add and sort by semver
        versions.push(version.to_string());
        versions.sort_by(|a, b| {
            compare_semver(a, b).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Remove a version from the version index.
    #[allow(dead_code)]
    async fn remove_from_version_index(&self, pack_id: &str, version: &str) {
        let mut index = self.version_index.write().await;
        if let Some(versions) = index.get_mut(pack_id) {
            versions.retain(|v| v != version);
            if versions.is_empty() {
                index.remove(pack_id);
            }
        }
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Get a loaded setting pack by version key.
    ///
    /// # Arguments
    ///
    /// * `version_key` - The version key (`{pack_id}@{version}`)
    pub async fn get_pack(&self, version_key: &str) -> Option<SettingPack> {
        let loaded = self.loaded_packs.read().await;
        loaded.get(version_key).cloned()
    }

    /// Get a loaded setting pack by ID (returns latest version).
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the pack
    pub async fn get_pack_by_id(&self, pack_id: &str) -> Option<SettingPack> {
        self.get_latest(pack_id).await.ok()
    }

    /// List all loaded setting packs (latest version of each).
    ///
    /// # Returns
    ///
    /// Vector of setting pack summaries.
    pub async fn list_packs(&self) -> Vec<SettingPackSummary> {
        let index = self.version_index.read().await;
        let loaded = self.loaded_packs.read().await;

        index
            .iter()
            .filter_map(|(pack_id, versions)| {
                let latest = versions.last()?;
                let vkey = version_key(pack_id, latest);
                loaded.get(&vkey).map(SettingPackSummary::from)
            })
            .collect()
    }

    /// List all loaded packs with all versions.
    ///
    /// # Returns
    ///
    /// Vector of (pack_id, Vec<version>) tuples.
    pub async fn list_all_versions(&self) -> Vec<(String, Vec<String>)> {
        let index = self.version_index.read().await;
        index
            .iter()
            .map(|(pack_id, versions)| (pack_id.clone(), versions.clone()))
            .collect()
    }

    /// Get the count of loaded packs (unique pack IDs).
    pub async fn count(&self) -> usize {
        let index = self.version_index.read().await;
        index.len()
    }

    /// Get the count of all loaded pack versions.
    pub async fn count_all_versions(&self) -> usize {
        let loaded = self.loaded_packs.read().await;
        loaded.len()
    }

    /// Unload a setting pack from memory.
    ///
    /// This removes the pack from loaded_packs but does not affect active campaigns.
    /// Use `deactivate` first if the pack is active.
    ///
    /// # Arguments
    ///
    /// * `pack_id` - ID of the pack to unload
    /// * `version` - Optional specific version to unload; if None, unloads all versions
    ///
    /// # Returns
    ///
    /// Number of versions unloaded.
    pub async fn unload(&self, pack_id: &str, version: Option<&str>) -> usize {
        let mut loaded = self.loaded_packs.write().await;
        let mut index = self.version_index.write().await;

        let mut count = 0;

        if let Some(ver) = version {
            // Unload specific version
            let vkey = version_key(pack_id, ver);
            if loaded.remove(&vkey).is_some() {
                count = 1;
                if let Some(versions) = index.get_mut(pack_id) {
                    versions.retain(|v| v != ver);
                    if versions.is_empty() {
                        index.remove(pack_id);
                    }
                }
            }
        } else {
            // Unload all versions
            if let Some(versions) = index.remove(pack_id) {
                for ver in versions {
                    let vkey = version_key(pack_id, &ver);
                    if loaded.remove(&vkey).is_some() {
                        count += 1;
                    }
                }
            }
        }

        count
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Emit an event to all listeners (stub implementation).
    async fn emit_event(&self, event: SettingPackEvent) {
        let listeners = self.event_listeners.read().await;
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // -------------------------------------------------------------------------
    // Helper functions
    // -------------------------------------------------------------------------

    fn create_test_pack(id: &str, version: &str) -> SettingPack {
        SettingPack::new(id, format!("Test Pack {}", id), "dnd5e", version)
    }

    fn create_test_pack_yaml(id: &str, version: &str) -> String {
        format!(
            r#"
id: "{}"
name: "Test Pack {}"
gameSystem: "dnd5e"
version: "{}"
"#,
            id, id, version
        )
    }

    fn create_test_pack_json(id: &str, version: &str) -> String {
        format!(
            r#"{{
    "id": "{}",
    "name": "Test Pack {}",
    "gameSystem": "dnd5e",
    "version": "{}"
}}"#,
            id, id, version
        )
    }

    // -------------------------------------------------------------------------
    // Version key tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_version_key_format() {
        let key = version_key("forgotten_realms", "1.0.0");
        assert_eq!(key, "forgotten_realms@1.0.0");
    }

    #[test]
    fn test_parse_version_key() {
        let result = parse_version_key("forgotten_realms@1.0.0");
        assert_eq!(result, Some(("forgotten_realms", "1.0.0")));

        let result = parse_version_key("pack@with@multiple@at@1.0.0");
        assert_eq!(result, Some(("pack@with@multiple@at", "1.0.0")));

        let result = parse_version_key("no_version");
        assert_eq!(result, None);
    }

    // -------------------------------------------------------------------------
    // SettingPackLoader creation tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_loader_creation() {
        let loader = SettingPackLoader::new();
        assert_eq!(loader.count().await, 0);
        assert_eq!(loader.count_all_versions().await, 0);
    }

    #[tokio::test]
    async fn test_loader_default() {
        let loader = SettingPackLoader::default();
        assert_eq!(loader.count().await, 0);
    }

    // -------------------------------------------------------------------------
    // Loading tests (TASK-ARCH-050)
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_load_pack_directly() {
        let loader = SettingPackLoader::new();
        let pack = create_test_pack("test_pack", "1.0.0");

        let vkey = loader.load_pack(pack).await.unwrap();
        assert_eq!(vkey, "test_pack@1.0.0");
        assert_eq!(loader.count().await, 1);
    }

    #[tokio::test]
    async fn test_load_from_yaml() {
        let loader = SettingPackLoader::new();
        let yaml = create_test_pack_yaml("yaml_pack", "1.0.0");

        let vkey = loader.load_from_yaml(&yaml).await.unwrap();
        assert_eq!(vkey, "yaml_pack@1.0.0");

        let pack = loader.get_latest("yaml_pack").await.unwrap();
        assert_eq!(pack.name, "Test Pack yaml_pack");
        assert_eq!(pack.game_system, "dnd5e");
    }

    #[tokio::test]
    async fn test_load_from_json() {
        let loader = SettingPackLoader::new();
        let json = create_test_pack_json("json_pack", "2.0.0");

        let vkey = loader.load_from_json(&json).await.unwrap();
        assert_eq!(vkey, "json_pack@2.0.0");

        let pack = loader.get_latest("json_pack").await.unwrap();
        assert_eq!(pack.version, "2.0.0");
    }

    #[tokio::test]
    async fn test_load_invalid_yaml() {
        let loader = SettingPackLoader::new();
        let invalid = "not: valid: yaml: at: all:";

        let result = loader.load_from_yaml(invalid).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_pack_missing_id() {
        let loader = SettingPackLoader::new();
        let yaml = r#"
name: "Test Pack"
gameSystem: "dnd5e"
version: "1.0.0"
"#;
        // This should fail because id is missing
        // Note: serde_yaml may produce an empty string for missing fields
        let result = loader.load_from_yaml(yaml).await;
        // The pack validation should catch this
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_pack_invalid_version() {
        let loader = SettingPackLoader::new();
        let yaml = r#"
id: "test"
name: "Test Pack"
gameSystem: "dnd5e"
version: "invalid"
"#;

        let result = loader.load_from_yaml(yaml).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("version"));
    }

    #[tokio::test]
    async fn test_load_pack_missing_game_system() {
        let loader = SettingPackLoader::new();
        let yaml = r#"
id: "test"
name: "Test Pack"
version: "1.0.0"
"#;

        // gameSystem defaults to empty, which should fail validation
        let result = loader.load_from_yaml(yaml).await;
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // Activation tests (TASK-ARCH-051)
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_activate_pack() {
        let loader = SettingPackLoader::new();
        let pack = create_test_pack("test_pack", "1.0.0");
        loader.load_pack(pack).await.unwrap();

        let existing: HashSet<String> = HashSet::new();
        let result = loader.activate("test_pack", "campaign_1", &existing).await;
        assert!(result.is_ok());

        let active = loader.get_active("campaign_1").await;
        assert_eq!(active, Some("test_pack@1.0.0".to_string()));
    }

    #[tokio::test]
    async fn test_activate_nonexistent_pack() {
        let loader = SettingPackLoader::new();
        let existing: HashSet<String> = HashSet::new();

        let result = loader.activate("nonexistent", "campaign_1", &existing).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::SettingPackNotFound(_) => (),
            _ => panic!("Expected SettingPackNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_activate_pack_with_missing_references() {
        let loader = SettingPackLoader::new();

        // Create pack with archetype overrides
        let mut pack = create_test_pack("test_pack", "1.0.0");
        pack.archetype_overrides.insert(
            "dwarf".to_string(),
            super::super::setting_pack::ArchetypeOverride::new()
                .with_display_name("Shield Dwarf"),
        );

        loader.load_pack(pack).await.unwrap();

        // No existing archetypes
        let existing: HashSet<String> = HashSet::new();
        let result = loader.activate("test_pack", "campaign_1", &existing).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::SettingPackReferenceError { missing_ids, .. } => {
                assert!(missing_ids.contains(&"dwarf".to_string()));
            }
            _ => panic!("Expected SettingPackReferenceError"),
        }
    }

    #[tokio::test]
    async fn test_activate_pack_with_valid_references() {
        let loader = SettingPackLoader::new();

        // Create pack with archetype overrides
        let mut pack = create_test_pack("test_pack", "1.0.0");
        pack.archetype_overrides.insert(
            "dwarf".to_string(),
            super::super::setting_pack::ArchetypeOverride::new()
                .with_display_name("Shield Dwarf"),
        );

        loader.load_pack(pack).await.unwrap();

        // Existing archetypes include "dwarf"
        let existing: HashSet<String> = ["dwarf".to_string()].into_iter().collect();
        let result = loader.activate("test_pack", "campaign_1", &existing).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_activate_replaces_existing() {
        let loader = SettingPackLoader::new();
        let pack1 = create_test_pack("pack_1", "1.0.0");
        let pack2 = create_test_pack("pack_2", "1.0.0");
        loader.load_pack(pack1).await.unwrap();
        loader.load_pack(pack2).await.unwrap();

        let existing: HashSet<String> = HashSet::new();

        // Activate first pack
        loader.activate("pack_1", "campaign_1", &existing).await.unwrap();
        assert_eq!(
            loader.get_active("campaign_1").await,
            Some("pack_1@1.0.0".to_string())
        );

        // Activate second pack (should replace first)
        loader.activate("pack_2", "campaign_1", &existing).await.unwrap();
        assert_eq!(
            loader.get_active("campaign_1").await,
            Some("pack_2@1.0.0".to_string())
        );
    }

    #[tokio::test]
    async fn test_deactivate_pack() {
        let loader = SettingPackLoader::new();
        let pack = create_test_pack("test_pack", "1.0.0");
        loader.load_pack(pack).await.unwrap();

        let existing: HashSet<String> = HashSet::new();
        loader.activate("test_pack", "campaign_1", &existing).await.unwrap();

        // Deactivate
        loader.deactivate("campaign_1").await.unwrap();

        assert!(loader.get_active("campaign_1").await.is_none());
    }

    #[tokio::test]
    async fn test_deactivate_nonexistent_is_ok() {
        let loader = SettingPackLoader::new();

        // Should not error on deactivating a campaign with no active pack
        let result = loader.deactivate("nonexistent_campaign").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_active_pack() {
        let loader = SettingPackLoader::new();
        let pack = create_test_pack("test_pack", "1.0.0");
        loader.load_pack(pack).await.unwrap();

        let existing: HashSet<String> = HashSet::new();
        loader.activate("test_pack", "campaign_1", &existing).await.unwrap();

        let active_pack = loader.get_active_pack("campaign_1").await;
        assert!(active_pack.is_some());
        assert_eq!(active_pack.unwrap().id, "test_pack");
    }

    #[tokio::test]
    async fn test_get_campaigns_using_pack() {
        let loader = SettingPackLoader::new();
        let pack = create_test_pack("test_pack", "1.0.0");
        loader.load_pack(pack).await.unwrap();

        let existing: HashSet<String> = HashSet::new();
        loader.activate("test_pack", "campaign_1", &existing).await.unwrap();
        loader.activate("test_pack", "campaign_2", &existing).await.unwrap();

        let campaigns = loader.get_campaigns_using_pack("test_pack").await;
        assert_eq!(campaigns.len(), 2);
        assert!(campaigns.contains(&"campaign_1".to_string()));
        assert!(campaigns.contains(&"campaign_2".to_string()));
    }

    // -------------------------------------------------------------------------
    // Version management tests (TASK-ARCH-052)
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_load_multiple_versions() {
        let loader = SettingPackLoader::new();

        loader.load_pack(create_test_pack("pack", "1.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "1.1.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "2.0.0")).await.unwrap();

        assert_eq!(loader.count().await, 1); // 1 unique pack ID
        assert_eq!(loader.count_all_versions().await, 3); // 3 versions

        let versions = loader.get_versions("pack").await;
        assert_eq!(versions, vec!["1.0.0", "1.1.0", "2.0.0"]);
    }

    #[tokio::test]
    async fn test_get_specific_version() {
        let loader = SettingPackLoader::new();

        loader.load_pack(create_test_pack("pack", "1.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "2.0.0")).await.unwrap();

        let pack = loader.get_version("pack", "1.0.0").await.unwrap();
        assert_eq!(pack.version, "1.0.0");

        let pack = loader.get_version("pack", "2.0.0").await.unwrap();
        assert_eq!(pack.version, "2.0.0");
    }

    #[tokio::test]
    async fn test_get_nonexistent_version() {
        let loader = SettingPackLoader::new();
        loader.load_pack(create_test_pack("pack", "1.0.0")).await.unwrap();

        let result = loader.get_version("pack", "99.0.0").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArchetypeError::PackVersionNotFound { pack_id, version } => {
                assert_eq!(pack_id, "pack");
                assert_eq!(version, "99.0.0");
            }
            _ => panic!("Expected PackVersionNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_get_latest_version() {
        let loader = SettingPackLoader::new();

        // Load versions out of order
        loader.load_pack(create_test_pack("pack", "2.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "1.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "1.5.0")).await.unwrap();

        let latest = loader.get_latest("pack").await.unwrap();
        assert_eq!(latest.version, "2.0.0");
    }

    #[tokio::test]
    async fn test_has_version() {
        let loader = SettingPackLoader::new();
        loader.load_pack(create_test_pack("pack", "1.0.0")).await.unwrap();

        assert!(loader.has_version("pack", "1.0.0").await);
        assert!(!loader.has_version("pack", "2.0.0").await);
        assert!(!loader.has_version("other", "1.0.0").await);
    }

    #[tokio::test]
    async fn test_activate_specific_version() {
        let loader = SettingPackLoader::new();

        loader.load_pack(create_test_pack("pack", "1.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "2.0.0")).await.unwrap();

        let existing: HashSet<String> = HashSet::new();

        // Activate specific version
        loader
            .activate_version("pack", "1.0.0", "campaign_1", &existing)
            .await
            .unwrap();

        let active = loader.get_active("campaign_1").await;
        assert_eq!(active, Some("pack@1.0.0".to_string()));

        // Verify it's the 1.0.0 version
        let active_pack = loader.get_active_pack("campaign_1").await.unwrap();
        assert_eq!(active_pack.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_version_semver_ordering() {
        let loader = SettingPackLoader::new();

        // Load in random order
        loader.load_pack(create_test_pack("pack", "1.10.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "1.2.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "2.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "1.9.0")).await.unwrap();

        let versions = loader.get_versions("pack").await;

        // Should be sorted by semver (1.2.0 < 1.9.0 < 1.10.0 < 2.0.0)
        assert_eq!(versions, vec!["1.2.0", "1.9.0", "1.10.0", "2.0.0"]);
    }

    // -------------------------------------------------------------------------
    // Unload tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_unload_specific_version() {
        let loader = SettingPackLoader::new();

        loader.load_pack(create_test_pack("pack", "1.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "2.0.0")).await.unwrap();

        let count = loader.unload("pack", Some("1.0.0")).await;
        assert_eq!(count, 1);

        assert!(!loader.has_version("pack", "1.0.0").await);
        assert!(loader.has_version("pack", "2.0.0").await);
    }

    #[tokio::test]
    async fn test_unload_all_versions() {
        let loader = SettingPackLoader::new();

        loader.load_pack(create_test_pack("pack", "1.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack", "2.0.0")).await.unwrap();

        let count = loader.unload("pack", None).await;
        assert_eq!(count, 2);

        assert_eq!(loader.count().await, 0);
    }

    // -------------------------------------------------------------------------
    // List tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_list_packs() {
        let loader = SettingPackLoader::new();

        loader.load_pack(create_test_pack("pack_a", "1.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack_b", "2.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack_a", "1.1.0")).await.unwrap();

        let summaries = loader.list_packs().await;
        assert_eq!(summaries.len(), 2);

        // Find pack_a - should be latest version (1.1.0)
        let pack_a = summaries.iter().find(|s| s.id == "pack_a").unwrap();
        assert_eq!(pack_a.version, "1.1.0");
    }

    #[tokio::test]
    async fn test_list_all_versions() {
        let loader = SettingPackLoader::new();

        loader.load_pack(create_test_pack("pack_a", "1.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack_a", "2.0.0")).await.unwrap();
        loader.load_pack(create_test_pack("pack_b", "1.0.0")).await.unwrap();

        let all = loader.list_all_versions().await;
        assert_eq!(all.len(), 2);

        // Find pack_a
        let pack_a = all.iter().find(|(id, _)| id == "pack_a").unwrap();
        assert_eq!(pack_a.1, vec!["1.0.0", "2.0.0"]);
    }

    // -------------------------------------------------------------------------
    // Event tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_setting_pack_event_clone() {
        let event = SettingPackEvent::Loaded {
            pack_id: "test".to_string(),
            version: "1.0.0".to_string(),
        };

        let cloned = event.clone();
        match cloned {
            SettingPackEvent::Loaded { pack_id, version } => {
                assert_eq!(pack_id, "test");
                assert_eq!(version, "1.0.0");
            }
            _ => panic!("Expected Loaded event"),
        }
    }
}
