//! Location Manager Module
//!
//! Manages campaign locations with hierarchical relationships and full support
//! for generated locations from the location_gen module.

use chrono::Utc;
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

use crate::core::location_gen::{
    Location, LocationType, LocationConnection, Inhabitant, Secret,
    Encounter, MapReference,
};

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Error)]
pub enum LocationManagerError {
    #[error("Location not found: {0}")]
    NotFound(String),
    #[error("Campaign not found: {0}")]
    CampaignNotFound(String),
    #[error("Lock error: {0}")]
    LockError(String),
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

pub type Result<T> = std::result::Result<T, LocationManagerError>;

// ============================================================================
// Location Manager
// ============================================================================

/// Manages generated locations for campaigns
/// Works with the rich Location type from location_gen
pub struct LocationManager {
    /// Locations by ID
    locations: RwLock<HashMap<String, Location>>,
    /// Index: campaign_id -> location_ids
    campaign_index: RwLock<HashMap<String, Vec<String>>>,
}

impl LocationManager {
    pub fn new() -> Self {
        Self {
            locations: RwLock::new(HashMap::new()),
            campaign_index: RwLock::new(HashMap::new()),
        }
    }

    /// Save a generated location
    pub fn save_location(&self, location: Location) -> Result<String> {
        let id = location.id.clone();
        let campaign_id = location.campaign_id.clone();

        let mut locations = self.locations.write()
            .map_err(|e| LocationManagerError::LockError(e.to_string()))?;
        locations.insert(id.clone(), location);

        // Update campaign index
        if let Some(cid) = campaign_id {
            let mut index = self.campaign_index.write()
                .map_err(|e| LocationManagerError::LockError(e.to_string()))?;
            index.entry(cid).or_insert_with(Vec::new).push(id.clone());
        }

        Ok(id)
    }

    /// Get a location by ID
    pub fn get_location(&self, id: &str) -> Option<Location> {
        let locations = self.locations.read().ok()?;
        locations.get(id).cloned()
    }

    /// Update a location
    pub fn update_location(&self, mut location: Location) -> Result<()> {
        let mut locations = self.locations.write()
            .map_err(|e| LocationManagerError::LockError(e.to_string()))?;

        if !locations.contains_key(&location.id) {
            return Err(LocationManagerError::NotFound(location.id));
        }

        location.updated_at = Utc::now();
        locations.insert(location.id.clone(), location);
        Ok(())
    }

    /// Delete a location
    pub fn delete_location(&self, id: &str) -> Result<()> {
        let mut locations = self.locations.write()
            .map_err(|e| LocationManagerError::LockError(e.to_string()))?;

        if let Some(location) = locations.remove(id) {
            // Remove from campaign index
            if let Some(campaign_id) = &location.campaign_id {
                if let Ok(mut index) = self.campaign_index.write() {
                    if let Some(ids) = index.get_mut(campaign_id) {
                        ids.retain(|lid| lid != id);
                    }
                }
            }

            // Remove connections from other locations
            for other in locations.values_mut() {
                other.connected_locations.retain(|c| c.target_id.as_deref() != Some(id));
            }

            Ok(())
        } else {
            Err(LocationManagerError::NotFound(id.to_string()))
        }
    }

    /// List all locations for a campaign
    pub fn list_locations_for_campaign(&self, campaign_id: &str) -> Vec<Location> {
        let locations = match self.locations.read() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        locations
            .values()
            .filter(|l| l.campaign_id.as_deref() == Some(campaign_id))
            .cloned()
            .collect()
    }

    /// Add a connection to a location
    pub fn add_connection(&self, location_id: &str, connection: LocationConnection) -> Result<()> {
        let mut locations = self.locations.write()
            .map_err(|e| LocationManagerError::LockError(e.to_string()))?;

        if let Some(location) = locations.get_mut(location_id) {
            // Check if connection already exists
            if !location.connected_locations.iter().any(|c| c.target_id == connection.target_id) {
                location.connected_locations.push(connection);
                location.updated_at = Utc::now();
            }
            Ok(())
        } else {
            Err(LocationManagerError::NotFound(location_id.to_string()))
        }
    }

    /// Remove a connection from a location
    pub fn remove_connection(&self, location_id: &str, target_id: &str) -> Result<()> {
        let mut locations = self.locations.write()
            .map_err(|e| LocationManagerError::LockError(e.to_string()))?;

        if let Some(location) = locations.get_mut(location_id) {
            location.connected_locations.retain(|c| c.target_id.as_deref() != Some(target_id));
            location.updated_at = Utc::now();
            Ok(())
        } else {
            Err(LocationManagerError::NotFound(location_id.to_string()))
        }
    }

    /// Get connected locations
    pub fn get_connected_locations(&self, location_id: &str) -> Vec<Location> {
        let locations = match self.locations.read() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        if let Some(location) = locations.get(location_id) {
            location.connected_locations
                .iter()
                .filter_map(|conn| conn.target_id.as_ref().and_then(|id| locations.get(id)).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Add an inhabitant to a location
    pub fn add_inhabitant(&self, location_id: &str, inhabitant: Inhabitant) -> Result<()> {
        let mut locations = self.locations.write()
            .map_err(|e| LocationManagerError::LockError(e.to_string()))?;

        if let Some(location) = locations.get_mut(location_id) {
            location.inhabitants.push(inhabitant);
            location.updated_at = Utc::now();
            Ok(())
        } else {
            Err(LocationManagerError::NotFound(location_id.to_string()))
        }
    }

    /// Remove an inhabitant from a location by name
    pub fn remove_inhabitant(&self, location_id: &str, name: &str) -> Result<()> {
        let mut locations = self.locations.write()
            .map_err(|e| LocationManagerError::LockError(e.to_string()))?;

        if let Some(location) = locations.get_mut(location_id) {
            location.inhabitants.retain(|i| i.name != name);
            location.updated_at = Utc::now();
            Ok(())
        } else {
            Err(LocationManagerError::NotFound(location_id.to_string()))
        }
    }

    /// Add a secret to a location
    pub fn add_secret(&self, location_id: &str, secret: Secret) -> Result<()> {
        let mut locations = self.locations.write()
            .map_err(|e| LocationManagerError::LockError(e.to_string()))?;

        if let Some(location) = locations.get_mut(location_id) {
            location.secrets.push(secret);
            location.updated_at = Utc::now();
            Ok(())
        } else {
            Err(LocationManagerError::NotFound(location_id.to_string()))
        }
    }

    /// Add an encounter to a location
    pub fn add_encounter(&self, location_id: &str, encounter: Encounter) -> Result<()> {
        let mut locations = self.locations.write()
            .map_err(|e| LocationManagerError::LockError(e.to_string()))?;

        if let Some(location) = locations.get_mut(location_id) {
            location.encounters.push(encounter);
            location.updated_at = Utc::now();
            Ok(())
        } else {
            Err(LocationManagerError::NotFound(location_id.to_string()))
        }
    }

    /// Set map reference for a location
    pub fn set_map_reference(&self, location_id: &str, map_ref: MapReference) -> Result<()> {
        let mut locations = self.locations.write()
            .map_err(|e| LocationManagerError::LockError(e.to_string()))?;

        if let Some(location) = locations.get_mut(location_id) {
            location.map_reference = Some(map_ref);
            location.updated_at = Utc::now();
            Ok(())
        } else {
            Err(LocationManagerError::NotFound(location_id.to_string()))
        }
    }

    /// Search locations by various criteria
    pub fn search_locations(
        &self,
        campaign_id: Option<String>,
        location_type: Option<String>,
        tags: Option<Vec<String>>,
        query: Option<String>,
    ) -> Vec<Location> {
        let locations = match self.locations.read() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        let query_lower = query.map(|q| q.to_lowercase());
        let loc_type = location_type.map(|t| LocationType::from_str(&t));

        locations
            .values()
            .filter(|l| {
                // Campaign filter
                if let Some(ref cid) = campaign_id {
                    if l.campaign_id.as_deref() != Some(cid.as_str()) {
                        return false;
                    }
                }

                // Location type filter
                if let Some(ref lt) = loc_type {
                    if &l.location_type != lt {
                        return false;
                    }
                }

                // Tags filter
                if let Some(ref filter_tags) = tags {
                    if !filter_tags.iter().any(|t| l.tags.contains(t)) {
                        return false;
                    }
                }

                // Query filter (search in name, description, notes)
                if let Some(ref q) = query_lower {
                    let matches = l.name.to_lowercase().contains(q)
                        || l.description.to_lowercase().contains(q)
                        || l.notes.to_lowercase().contains(q)
                        || l.tags.iter().any(|t| t.to_lowercase().contains(q));
                    if !matches {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }

    /// Get locations by type
    pub fn get_by_type(&self, campaign_id: &str, location_type: &LocationType) -> Vec<Location> {
        let locations = match self.locations.read() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        locations
            .values()
            .filter(|l| l.campaign_id.as_deref() == Some(campaign_id) && &l.location_type == location_type)
            .cloned()
            .collect()
    }

    /// Get all locations (no campaign filter)
    pub fn list_all(&self) -> Vec<Location> {
        let locations = match self.locations.read() {
            Ok(l) => l,
            Err(_) => return Vec::new(),
        };

        locations.values().cloned().collect()
    }

    /// Get location count
    pub fn count(&self) -> usize {
        self.locations.read().map(|l| l.len()).unwrap_or(0)
    }

    /// Get location count for a campaign
    pub fn count_for_campaign(&self, campaign_id: &str) -> usize {
        let locations = match self.locations.read() {
            Ok(l) => l,
            Err(_) => return 0,
        };

        locations
            .values()
            .filter(|l| l.campaign_id.as_deref() == Some(campaign_id))
            .count()
    }
}

impl Default for LocationManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::location_gen::{LocationGenerator, LocationGenerationOptions};

    #[test]
    fn test_save_and_get_location() {
        let manager = LocationManager::new();
        let generator = LocationGenerator::new();

        let options = LocationGenerationOptions {
            location_type: Some("tavern".to_string()),
            campaign_id: Some("campaign-1".to_string()),
            include_inhabitants: true,
            ..Default::default()
        };

        let location = generator.generate_quick(&options);
        let id = manager.save_location(location.clone()).unwrap();

        let retrieved = manager.get_location(&id).unwrap();
        assert_eq!(retrieved.name, location.name);
        assert_eq!(retrieved.location_type, LocationType::Tavern);
    }

    #[test]
    fn test_list_by_campaign() {
        let manager = LocationManager::new();
        let generator = LocationGenerator::new();

        // Create locations for different campaigns
        for i in 0..3 {
            let options = LocationGenerationOptions {
                location_type: Some("tavern".to_string()),
                campaign_id: Some("campaign-1".to_string()),
                ..Default::default()
            };
            let location = generator.generate_quick(&options);
            manager.save_location(location).unwrap();
        }

        for i in 0..2 {
            let options = LocationGenerationOptions {
                location_type: Some("dungeon".to_string()),
                campaign_id: Some("campaign-2".to_string()),
                ..Default::default()
            };
            let location = generator.generate_quick(&options);
            manager.save_location(location).unwrap();
        }

        let campaign1_locations = manager.list_locations_for_campaign("campaign-1");
        assert_eq!(campaign1_locations.len(), 3);

        let campaign2_locations = manager.list_locations_for_campaign("campaign-2");
        assert_eq!(campaign2_locations.len(), 2);
    }

    #[test]
    fn test_connections() {
        let manager = LocationManager::new();
        let generator = LocationGenerator::new();

        let opt1 = LocationGenerationOptions {
            location_type: Some("tavern".to_string()),
            campaign_id: Some("campaign-1".to_string()),
            ..Default::default()
        };
        let loc1 = generator.generate_quick(&opt1);
        let id1 = manager.save_location(loc1).unwrap();

        let opt2 = LocationGenerationOptions {
            location_type: Some("shop".to_string()),
            campaign_id: Some("campaign-1".to_string()),
            ..Default::default()
        };
        let loc2 = generator.generate_quick(&opt2);
        let id2 = manager.save_location(loc2).unwrap();

        // Add connection
        let connection = LocationConnection {
            target_id: Some(id2.clone()),
            target_name: "Test Shop".to_string(),
            connection_type: crate::core::location_gen::ConnectionType::Road,
            description: None,
            travel_time: Some("5 minutes".to_string()),
            hazards: vec![],
        };
        manager.add_connection(&id1, connection).unwrap();

        let connected = manager.get_connected_locations(&id1);
        assert_eq!(connected.len(), 1);
        assert_eq!(connected[0].id, id2);
    }
}
