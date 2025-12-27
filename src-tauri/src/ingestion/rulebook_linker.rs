use serde::{Deserialize, Serialize};
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceLink {
    pub source_id: String,
    pub source_title: String,
    pub page: i32,
    pub match_text: String,
    pub confidence: f32,
}

pub struct RulebookLinker {
    // Determine mapping from common abbreviations to source IDs
    // e.g. "PHB" -> "Player's Handbook"
    abbreviations: HashMap<String, String>,
    // simple regex cache
    patterns: Vec<(Regex, String)>,
}

impl RulebookLinker {
    pub fn new() -> Self {
        // Pre-compile common patterns
        // E.g. "PHB 102", "DMG p. 50"
        let patterns = vec![
            (Regex::new(r"(?i)\bPHB\s+(?:p\.?|page)?\s*(\d+)").unwrap(), "PHB".to_string()),
            (Regex::new(r"(?i)\bDMG\s+(?:p\.?|page)?\s*(\d+)").unwrap(), "DMG".to_string()),
            (Regex::new(r"(?i)\bMM\s+(?:p\.?|page)?\s*(\d+)").unwrap(), "MM".to_string()),
        ];

        let mut abbreviations = HashMap::new();
        abbreviations.insert("PHB".to_string(), "Player's Handbook".to_string());
        abbreviations.insert("DMG".to_string(), "Dungeon Master's Guide".to_string());
        abbreviations.insert("MM".to_string(), "Monster Manual".to_string());

        Self {
            abbreviations,
            patterns,
        }
    }

    pub fn find_links(&self, text: &str) -> Vec<ReferenceLink> {
        let mut links = Vec::new();

        for (regex, abbr) in &self.patterns {
            for cap in regex.captures_iter(text) {
                if let Some(page_match) = cap.get(1) {
                    if let Ok(page_num) = page_match.as_str().parse::<i32>() {
                         let title = self.abbreviations.get(abbr).cloned().unwrap_or(abbr.clone());

                         links.push(ReferenceLink {
                            source_id: abbr.clone(), // Placeholder ID
                            source_title: title,
                            page: page_num,
                            match_text: cap.get(0).unwrap().as_str().to_string(),
                            confidence: 0.9,
                         });
                    }
                }
            }
        }

        // Advanced: Could search for explicit titles if we had a full index of known books

        links
    }
}
