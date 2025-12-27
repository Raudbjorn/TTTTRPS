use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::core::models::Campaign;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSnapshot {
    pub id: String,
    pub campaign_id: String,
    pub timestamp: DateTime<Utc>,
    pub data: Campaign, // The full state for now
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNote {
    pub id: String,
    pub campaign_id: String,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub tags: Vec<String>,
}

pub struct CampaignManager {
    // In a real app, this would be a DB connection pool (sqlx)
    // For now, using in-memory store for rapid prototyping
    campaigns: Arc<Mutex<HashMap<String, Campaign>>>,
    snapshots: Arc<Mutex<HashMap<String, Vec<CampaignSnapshot>>>>,
    notes: Arc<Mutex<HashMap<String, Vec<SessionNote>>>>,
}

impl CampaignManager {
    pub fn new() -> Self {
        Self {
            campaigns: Arc::new(Mutex::new(HashMap::new())),
            snapshots: Arc::new(Mutex::new(HashMap::new())),
            notes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_campaign(&self, name: &str, system: &str) -> Campaign {
        let campaign = Campaign {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            system: system.to_string(),
            description: Some(String::new()),
            current_date: "Start".to_string(),
            notes: vec![],
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        self.campaigns.lock().unwrap().insert(campaign.id.clone(), campaign.clone());
        campaign
    }

    pub fn get_campaign(&self, id: &str) -> Option<Campaign> {
        self.campaigns.lock().unwrap().get(id).cloned()
    }

    pub fn list_campaigns(&self) -> Vec<Campaign> {
        self.campaigns.lock().unwrap().values().cloned().collect()
    }

    pub fn update_campaign(&self, campaign: Campaign) {
        // Auto-create snapshot before major updates?
        // For now, manual snapshots.
        self.campaigns.lock().unwrap().insert(campaign.id.clone(), campaign);
    }

    // Versioning
    pub fn create_snapshot(&self, campaign_id: &str, description: &str) -> Result<String, String> {
        let campaigns = self.campaigns.lock().unwrap();
        let campaign = campaigns.get(campaign_id).ok_or("Campaign not found")?;

        let snapshot = CampaignSnapshot {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            timestamp: Utc::now(),
            data: campaign.clone(),
            description: description.to_string(),
        };

        let mut snapshots = self.snapshots.lock().unwrap();
        snapshots.entry(campaign_id.to_string()).or_default().push(snapshot.clone());

        Ok(snapshot.id)
    }

    pub fn restore_snapshot(&self, campaign_id: &str, snapshot_id: &str) -> Result<(), String> {
        let snapshots = self.snapshots.lock().unwrap();
        let campaign_snapshots = snapshots.get(campaign_id).ok_or("No snapshots for campaign")?;

        let snapshot = campaign_snapshots.iter().find(|s| s.id == snapshot_id).ok_or("Snapshot not found")?;

        let mut campaigns = self.campaigns.lock().unwrap();
        campaigns.insert(campaign_id.to_string(), snapshot.data.clone());

        Ok(())
    }

    pub fn add_note(&self, campaign_id: &str, content: &str, tags: Vec<String>) -> SessionNote {
        let note = SessionNote {
            id: Uuid::new_v4().to_string(),
            campaign_id: campaign_id.to_string(),
            timestamp: Utc::now(),
            content: content.to_string(),
            tags,
        };

        let mut notes = self.notes.lock().unwrap();
        notes.entry(campaign_id.to_string()).or_default().push(note.clone());
        note
    }
}
