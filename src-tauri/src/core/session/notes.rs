//! Session Notes Module (TASK-017)
//!
//! Provides note CRUD operations, tag-based organization, entity linking,
//! search within notes, and AI-powered categorization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::{HashMap, HashSet};

// ============================================================================
// Note Types
// ============================================================================

/// Category of a session note
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NoteCategory {
    /// General session notes
    General,
    /// Combat-related notes
    Combat,
    /// NPC/character notes
    Character,
    /// Location/setting notes
    Location,
    /// Plot/story notes
    Plot,
    /// Quest/objective notes
    Quest,
    /// Loot/treasure notes
    Loot,
    /// Rules/mechanics notes
    Rules,
    /// Player/meta notes
    Meta,
    /// World-building notes
    Worldbuilding,
    /// Dialogue/conversations
    Dialogue,
    /// Secrets/reveals
    Secret,
    /// Custom category
    Custom(String),
}

impl NoteCategory {
    /// Convert to display string
    pub fn display(&self) -> String {
        match self {
            Self::General => "General".to_string(),
            Self::Combat => "Combat".to_string(),
            Self::Character => "Character".to_string(),
            Self::Location => "Location".to_string(),
            Self::Plot => "Plot".to_string(),
            Self::Quest => "Quest".to_string(),
            Self::Loot => "Loot".to_string(),
            Self::Rules => "Rules".to_string(),
            Self::Meta => "Meta".to_string(),
            Self::Worldbuilding => "Worldbuilding".to_string(),
            Self::Dialogue => "Dialogue".to_string(),
            Self::Secret => "Secret".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    /// Get icon identifier
    pub fn icon(&self) -> &'static str {
        match self {
            Self::General => "file-text",
            Self::Combat => "swords",
            Self::Character => "user",
            Self::Location => "map-pin",
            Self::Plot => "book-open",
            Self::Quest => "target",
            Self::Loot => "package",
            Self::Rules => "book",
            Self::Meta => "settings",
            Self::Worldbuilding => "globe",
            Self::Dialogue => "message-square",
            Self::Secret => "lock",
            Self::Custom(_) => "tag",
        }
    }

    /// Get color for UI
    pub fn color(&self) -> &'static str {
        match self {
            Self::General => "#71717a",
            Self::Combat => "#ef4444",
            Self::Character => "#3b82f6",
            Self::Location => "#22c55e",
            Self::Plot => "#a855f7",
            Self::Quest => "#f59e0b",
            Self::Loot => "#eab308",
            Self::Rules => "#64748b",
            Self::Meta => "#6b7280",
            Self::Worldbuilding => "#06b6d4",
            Self::Dialogue => "#ec4899",
            Self::Secret => "#7c3aed",
            Self::Custom(_) => "#94a3b8",
        }
    }
}

/// Type of entity a note can be linked to
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    NPC,
    Player,
    Location,
    Item,
    Quest,
    Session,
    Campaign,
    Combat,
    Custom(String),
}

/// A reference to an entity from a note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityLink {
    /// Type of entity
    pub entity_type: EntityType,
    /// Entity ID
    pub entity_id: String,
    /// Display name
    pub display_name: String,
    /// Character range in note content where reference occurs
    pub text_range: Option<(usize, usize)>,
}

// ============================================================================
// Session Note
// ============================================================================

/// A session note with full metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNote {
    /// Unique identifier
    pub id: String,
    /// Session this note belongs to
    pub session_id: String,
    /// Campaign ID
    pub campaign_id: String,
    /// Note title/heading
    pub title: String,
    /// Note content (markdown supported)
    pub content: String,
    /// Primary category
    pub category: NoteCategory,
    /// Additional categories (AI or user suggested)
    pub additional_categories: Vec<NoteCategory>,
    /// User-defined tags
    pub tags: Vec<String>,
    /// Linked entities
    pub entity_links: Vec<EntityLink>,
    /// Is this a pinned/important note
    pub is_pinned: bool,
    /// Is this note private (GM only)
    pub is_private: bool,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Author (user ID or "system")
    pub author: String,
    /// AI-generated summary (optional)
    pub ai_summary: Option<String>,
    /// AI-suggested categories
    pub ai_suggested_categories: Vec<NoteCategory>,
    /// AI confidence score for categorization
    pub ai_confidence: Option<f32>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SessionNote {
    /// Create a new note
    pub fn new(
        session_id: impl Into<String>,
        campaign_id: impl Into<String>,
        title: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.into(),
            campaign_id: campaign_id.into(),
            title: title.into(),
            content: content.into(),
            category: NoteCategory::General,
            additional_categories: Vec::new(),
            tags: Vec::new(),
            entity_links: Vec::new(),
            is_pinned: false,
            is_private: false,
            created_at: now,
            updated_at: now,
            author: "user".to_string(),
            ai_summary: None,
            ai_suggested_categories: Vec::new(),
            ai_confidence: None,
            metadata: HashMap::new(),
        }
    }

    /// Builder: set category
    pub fn with_category(mut self, category: NoteCategory) -> Self {
        self.category = category;
        self
    }

    /// Builder: add tags
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Builder: add entity link
    pub fn with_entity_link(mut self, entity_type: EntityType, entity_id: impl Into<String>, name: impl Into<String>) -> Self {
        self.entity_links.push(EntityLink {
            entity_type,
            entity_id: entity_id.into(),
            display_name: name.into(),
            text_range: None,
        });
        self
    }

    /// Builder: mark as pinned
    pub fn pinned(mut self) -> Self {
        self.is_pinned = true;
        self
    }

    /// Builder: mark as private
    pub fn private(mut self) -> Self {
        self.is_private = true;
        self
    }

    /// Builder: set author
    pub fn by(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    /// Update the note content
    pub fn update_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.updated_at = Utc::now();
    }

    /// Update the note title
    pub fn update_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.updated_at = Utc::now();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Add an entity link
    pub fn link_entity(&mut self, entity_type: EntityType, entity_id: impl Into<String>, name: impl Into<String>) {
        self.entity_links.push(EntityLink {
            entity_type,
            entity_id: entity_id.into(),
            display_name: name.into(),
            text_range: None,
        });
        self.updated_at = Utc::now();
    }

    /// Remove an entity link
    pub fn unlink_entity(&mut self, entity_id: &str) {
        self.entity_links.retain(|l| l.entity_id != entity_id);
        self.updated_at = Utc::now();
    }

    /// Set AI categorization results
    pub fn set_ai_categorization(&mut self, categories: Vec<NoteCategory>, confidence: f32) {
        self.ai_suggested_categories = categories;
        self.ai_confidence = Some(confidence);
        self.updated_at = Utc::now();
    }

    /// Apply AI suggestions as the actual categories
    pub fn apply_ai_categories(&mut self) {
        if let Some(first) = self.ai_suggested_categories.first() {
            self.category = first.clone();
        }
        if self.ai_suggested_categories.len() > 1 {
            self.additional_categories = self.ai_suggested_categories[1..].to_vec();
        }
        self.updated_at = Utc::now();
    }

    /// Check if note contains a search term (case-insensitive)
    pub fn matches_search(&self, query: &str) -> bool {
        let query_lower = query.to_lowercase();
        self.title.to_lowercase().contains(&query_lower)
            || self.content.to_lowercase().contains(&query_lower)
            || self.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
    }

    /// Get word count
    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }

    /// Extract preview text
    pub fn preview(&self, max_chars: usize) -> String {
        if self.content.len() <= max_chars {
            self.content.clone()
        } else {
            format!("{}...", &self.content[..max_chars])
        }
    }
}

// ============================================================================
// Notes Manager
// ============================================================================

/// Manages session notes for a campaign
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NotesManager {
    /// All notes indexed by ID
    notes: HashMap<String, SessionNote>,
    /// Index: session_id -> note IDs
    by_session: HashMap<String, Vec<String>>,
    /// Index: tag -> note IDs
    by_tag: HashMap<String, HashSet<String>>,
    /// Index: category -> note IDs
    by_category: HashMap<NoteCategory, HashSet<String>>,
    /// Index: entity -> note IDs
    by_entity: HashMap<String, HashSet<String>>,
}

impl NotesManager {
    pub fn new() -> Self {
        Self::default()
    }

    // ========================================================================
    // CRUD Operations
    // ========================================================================

    /// Create a new note
    pub fn create_note(&mut self, note: SessionNote) -> &SessionNote {
        let note_id = note.id.clone();
        let session_id = note.session_id.clone();

        // Update indices
        self.by_session
            .entry(session_id)
            .or_default()
            .push(note_id.clone());

        for tag in &note.tags {
            self.by_tag
                .entry(tag.clone())
                .or_default()
                .insert(note_id.clone());
        }

        self.by_category
            .entry(note.category.clone())
            .or_default()
            .insert(note_id.clone());

        for link in &note.entity_links {
            self.by_entity
                .entry(link.entity_id.clone())
                .or_default()
                .insert(note_id.clone());
        }

        self.notes.insert(note_id.clone(), note);
        self.notes.get(&note_id).unwrap()
    }

    /// Get a note by ID
    pub fn get_note(&self, note_id: &str) -> Option<&SessionNote> {
        self.notes.get(note_id)
    }

    /// Get a mutable note by ID
    pub fn get_note_mut(&mut self, note_id: &str) -> Option<&mut SessionNote> {
        self.notes.get_mut(note_id)
    }

    /// Update a note (replaces existing)
    pub fn update_note(&mut self, note: SessionNote) -> Result<&SessionNote, String> {
        let note_id = note.id.clone();

        // Check if note exists
        if !self.notes.contains_key(&note_id) {
            return Err(format!("Note not found: {}", note_id));
        }

        // Get old note for index cleanup
        let old_note = self.notes.get(&note_id).unwrap().clone();

        // Update tag index
        for tag in &old_note.tags {
            if let Some(set) = self.by_tag.get_mut(tag) {
                set.remove(&note_id);
            }
        }
        for tag in &note.tags {
            self.by_tag
                .entry(tag.clone())
                .or_default()
                .insert(note_id.clone());
        }

        // Update category index
        if let Some(set) = self.by_category.get_mut(&old_note.category) {
            set.remove(&note_id);
        }
        self.by_category
            .entry(note.category.clone())
            .or_default()
            .insert(note_id.clone());

        // Update entity index
        for link in &old_note.entity_links {
            if let Some(set) = self.by_entity.get_mut(&link.entity_id) {
                set.remove(&note_id);
            }
        }
        for link in &note.entity_links {
            self.by_entity
                .entry(link.entity_id.clone())
                .or_default()
                .insert(note_id.clone());
        }

        self.notes.insert(note_id.clone(), note);
        Ok(self.notes.get(&note_id).unwrap())
    }

    /// Delete a note
    pub fn delete_note(&mut self, note_id: &str) -> Option<SessionNote> {
        let note = self.notes.remove(note_id)?;

        // Clean up indices
        if let Some(ids) = self.by_session.get_mut(&note.session_id) {
            ids.retain(|id| id != note_id);
        }

        for tag in &note.tags {
            if let Some(set) = self.by_tag.get_mut(tag) {
                set.remove(note_id);
            }
        }

        if let Some(set) = self.by_category.get_mut(&note.category) {
            set.remove(note_id);
        }

        for link in &note.entity_links {
            if let Some(set) = self.by_entity.get_mut(&link.entity_id) {
                set.remove(note_id);
            }
        }

        Some(note)
    }

    // ========================================================================
    // Query Operations
    // ========================================================================

    /// Get all notes for a session
    pub fn notes_for_session(&self, session_id: &str) -> Vec<&SessionNote> {
        self.by_session
            .get(session_id)
            .map(|ids| ids.iter().filter_map(|id| self.notes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all notes with a specific tag
    pub fn notes_with_tag(&self, tag: &str) -> Vec<&SessionNote> {
        self.by_tag
            .get(tag)
            .map(|ids| ids.iter().filter_map(|id| self.notes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all notes in a category
    pub fn notes_in_category(&self, category: &NoteCategory) -> Vec<&SessionNote> {
        self.by_category
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.notes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all notes linked to an entity
    pub fn notes_for_entity(&self, entity_id: &str) -> Vec<&SessionNote> {
        self.by_entity
            .get(entity_id)
            .map(|ids| ids.iter().filter_map(|id| self.notes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get pinned notes for a session
    pub fn pinned_notes(&self, session_id: &str) -> Vec<&SessionNote> {
        self.notes_for_session(session_id)
            .into_iter()
            .filter(|n| n.is_pinned)
            .collect()
    }

    /// Search notes by query
    pub fn search(&self, query: &str) -> Vec<&SessionNote> {
        self.notes
            .values()
            .filter(|n| n.matches_search(query))
            .collect()
    }

    /// Search within a session
    pub fn search_in_session(&self, session_id: &str, query: &str) -> Vec<&SessionNote> {
        self.notes_for_session(session_id)
            .into_iter()
            .filter(|n| n.matches_search(query))
            .collect()
    }

    /// Get all unique tags used
    pub fn all_tags(&self) -> Vec<&String> {
        self.by_tag.keys().collect()
    }

    /// Get all notes
    pub fn all_notes(&self) -> Vec<&SessionNote> {
        self.notes.values().collect()
    }

    /// Get recent notes (sorted by updated_at desc)
    pub fn recent_notes(&self, limit: usize) -> Vec<&SessionNote> {
        let mut notes: Vec<&SessionNote> = self.notes.values().collect();
        notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        notes.into_iter().take(limit).collect()
    }

    /// Get note count
    pub fn count(&self) -> usize {
        self.notes.len()
    }
}

// ============================================================================
// AI Categorization
// ============================================================================

/// Request for AI categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorizationRequest {
    /// Note title
    pub title: String,
    /// Note content
    pub content: String,
    /// Available categories
    pub available_categories: Vec<String>,
}

/// Response from AI categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorizationResponse {
    /// Suggested primary category
    pub primary_category: String,
    /// Additional category suggestions
    pub additional_categories: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Optional summary
    pub summary: Option<String>,
    /// Suggested tags
    pub suggested_tags: Vec<String>,
    /// Detected entity mentions
    pub detected_entities: Vec<DetectedEntity>,
}

/// An entity detected in the note content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedEntity {
    /// Entity type
    pub entity_type: String,
    /// Name as mentioned in text
    pub mention: String,
    /// Suggested entity ID (if can be matched)
    pub suggested_id: Option<String>,
    /// Character range in content
    pub text_range: (usize, usize),
}

/// Build the prompt for AI categorization
pub fn build_categorization_prompt(request: &CategorizationRequest) -> String {
    let categories = request.available_categories.join(", ");

    format!(
        r#"Analyze the following TTRPG session note and categorize it.

TITLE: {}

CONTENT:
{}

AVAILABLE CATEGORIES: {}

Please respond in JSON format with:
{{
  "primary_category": "<best matching category>",
  "additional_categories": ["<other relevant categories>"],
  "confidence": <0.0-1.0 confidence score>,
  "summary": "<1-2 sentence summary>",
  "suggested_tags": ["<relevant tags>"],
  "detected_entities": [
    {{
      "entity_type": "npc|location|item|quest",
      "mention": "<text mention>",
      "text_range": [start, end]
    }}
  ]
}}

Focus on:
1. The primary purpose of the note
2. Key entities mentioned (NPCs, locations, items)
3. Relevant tags for organization
4. A concise summary for quick reference"#,
        request.title,
        request.content,
        categories
    )
}

/// Parse AI categorization response
pub fn parse_categorization_response(json_str: &str) -> Result<CategorizationResponse, String> {
    serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse AI response: {}", e))
}

/// Convert AI response to NoteCategories
pub fn response_to_categories(response: &CategorizationResponse) -> (NoteCategory, Vec<NoteCategory>) {
    let primary = string_to_category(&response.primary_category);
    let additional: Vec<NoteCategory> = response
        .additional_categories
        .iter()
        .map(|s| string_to_category(s))
        .collect();

    (primary, additional)
}

fn string_to_category(s: &str) -> NoteCategory {
    match s.to_lowercase().as_str() {
        "general" => NoteCategory::General,
        "combat" => NoteCategory::Combat,
        "character" => NoteCategory::Character,
        "location" => NoteCategory::Location,
        "plot" => NoteCategory::Plot,
        "quest" => NoteCategory::Quest,
        "loot" => NoteCategory::Loot,
        "rules" => NoteCategory::Rules,
        "meta" => NoteCategory::Meta,
        "worldbuilding" => NoteCategory::Worldbuilding,
        "dialogue" => NoteCategory::Dialogue,
        "secret" => NoteCategory::Secret,
        other => NoteCategory::Custom(other.to_string()),
    }
}

// ============================================================================
// Note Export/Import
// ============================================================================

/// Export format for notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteExport {
    pub version: String,
    pub exported_at: DateTime<Utc>,
    pub notes: Vec<SessionNote>,
}

impl NotesManager {
    /// Export all notes for a session
    pub fn export_session(&self, session_id: &str) -> NoteExport {
        NoteExport {
            version: "1.0".to_string(),
            exported_at: Utc::now(),
            notes: self.notes_for_session(session_id).into_iter().cloned().collect(),
        }
    }

    /// Export all notes
    pub fn export_all(&self) -> NoteExport {
        NoteExport {
            version: "1.0".to_string(),
            exported_at: Utc::now(),
            notes: self.notes.values().cloned().collect(),
        }
    }

    /// Import notes from export
    pub fn import(&mut self, export: NoteExport) -> usize {
        let mut imported = 0;
        for note in export.notes {
            if !self.notes.contains_key(&note.id) {
                self.create_note(note);
                imported += 1;
            }
        }
        imported
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_creation() {
        let note = SessionNote::new(
            "session-1",
            "campaign-1",
            "Combat Notes",
            "The party fought a group of goblins in the forest."
        )
        .with_category(NoteCategory::Combat)
        .with_tags(["combat", "goblins", "forest"])
        .with_entity_link(EntityType::Location, "forest-1", "Dark Forest");

        assert_eq!(note.category, NoteCategory::Combat);
        assert_eq!(note.tags.len(), 3);
        assert_eq!(note.entity_links.len(), 1);
    }

    #[test]
    fn test_notes_manager() {
        let mut manager = NotesManager::new();

        let note1 = SessionNote::new("session-1", "campaign-1", "Note 1", "Content 1")
            .with_category(NoteCategory::Combat)
            .with_tags(["combat"]);

        let note2 = SessionNote::new("session-1", "campaign-1", "Note 2", "Content 2")
            .with_category(NoteCategory::Plot)
            .with_tags(["plot"]);

        manager.create_note(note1);
        manager.create_note(note2);

        assert_eq!(manager.count(), 2);
        assert_eq!(manager.notes_for_session("session-1").len(), 2);
        assert_eq!(manager.notes_with_tag("combat").len(), 1);
        assert_eq!(manager.notes_in_category(&NoteCategory::Combat).len(), 1);
    }

    #[test]
    fn test_search() {
        let mut manager = NotesManager::new();

        let note = SessionNote::new("session-1", "campaign-1", "Goblin Battle", "The goblins attacked at dawn.");
        manager.create_note(note);

        assert_eq!(manager.search("goblin").len(), 1);
        assert_eq!(manager.search("dragon").len(), 0);
    }

    #[test]
    fn test_categorization_prompt() {
        let request = CategorizationRequest {
            title: "Meeting the Blacksmith".to_string(),
            content: "The party met Torgin the Blacksmith in the village of Millbrook. He offered to forge new weapons if they bring him steel from the mines.".to_string(),
            available_categories: vec!["Character".to_string(), "Quest".to_string(), "Location".to_string()],
        };

        let prompt = build_categorization_prompt(&request);
        assert!(prompt.contains("TITLE: Meeting the Blacksmith"));
        assert!(prompt.contains("Torgin the Blacksmith"));
    }
}
