use crate::core::personality_base::PersonalityProfile;

#[derive(Clone, Debug)]
pub struct PersonalityDisplay {
    pub id: String,
    pub name: String,
    pub source: String,
    pub trait_summary: String,
    pub tag_summary: String,
    pub formality: u8,
}

impl PersonalityDisplay {
    pub fn from_profile(p: &PersonalityProfile) -> Self {
        let trait_summary = p
            .traits
            .iter()
            .take(3)
            .map(|t| t.trait_name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let tag_summary = if p.tags.is_empty() {
            "—".to_string()
        } else {
            p.tags.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
        };

        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            source: p.source.clone().unwrap_or_else(|| "custom".to_string()),
            trait_summary,
            tag_summary,
            formality: p.speech_patterns.formality,
        }
    }
}
