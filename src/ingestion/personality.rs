use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub tone: String,
    pub writing_style: String,
    pub keywords: Vec<String>,
}

pub struct PersonalityExtractor;

impl PersonalityExtractor {
    pub fn extract(text: &str) -> PersonalityProfile {
        // Porting logic: Analyze text for tone markers
        // E.g. "grim", "dark", "whimsical"

        let text_lower = text.to_lowercase();
        let mut keywords = Vec::new();

        // Simple keyword scanning
        if text_lower.contains("blood") || text_lower.contains("darkness") {
            keywords.push("Dark".to_string());
        }
        if text_lower.contains("honor") || text_lower.contains("glory") {
            keywords.push("Heroic".to_string());
        }
        if text_lower.contains("joke") || text_lower.contains("laugh") {
            keywords.push("Comedic".to_string());
        }

        let tone = if keywords.contains(&"Dark".to_string()) {
            "Grim".to_string()
        } else if keywords.contains(&"Comedic".to_string()) {
            "Lighthearted".to_string()
        } else {
            "Neutral".to_string()
        };

        PersonalityProfile {
            tone,
            writing_style: "Standard".to_string(), // Placeholder for complexity analysis
            keywords,
        }
    }
}
