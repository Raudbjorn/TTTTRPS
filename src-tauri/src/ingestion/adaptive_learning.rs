use regex::Regex;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::sync::RwLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternTemplate {
    pub pattern_type: String,
    pub system: String,
    // Store regex patterns as strings to be serializable
    pub field_patterns: HashMap<String, Vec<String>>,
    pub usage_count: u32,
    pub success_rate: f32,
}

impl PatternTemplate {
    pub fn new(pattern_type: String, system: String) -> Self {
        Self {
            pattern_type,
            system,
            field_patterns: HashMap::new(),
            usage_count: 0,
            success_rate: 1.0,
        }
    }

    pub fn apply(&self, text: &str) -> HashMap<String, String> {
        let mut results = HashMap::new();
        for (field, patterns) in &self.field_patterns {
            for pattern_str in patterns {
                if let Ok(re) = Regex::new(pattern_str) {
                    if let Some(caps) = re.captures(text) {
                        if let Some(match_val) = caps.get(1) {
                            results.insert(field.clone(), match_val.as_str().to_string());
                            break;
                        }
                    }
                }
            }
        }
        results
    }
}

pub struct AdaptiveLearningSystem {
    patterns: RwLock<HashMap<String, HashMap<String, PatternTemplate>>>, // System -> Type -> Template
}

impl AdaptiveLearningSystem {
    pub fn new() -> Self {
        Self {
            patterns: RwLock::new(HashMap::new()),
        }
    }

    pub fn learn_from_document(&self, text: &str, system: &str) {
        // Implementation of learning logic (porting from Python)
        // ...
    }
}
