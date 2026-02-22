//! Stat Block Parsing Module
//!
//! Parses creature and NPC stat blocks into structured data.
//! Handles D&D 5e and similar system stat block formats.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Parsed stat block data with all creature statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct StatBlockData {
    /// Creature name
    pub name: String,
    /// Size category (Tiny, Small, Medium, Large, Huge, Gargantuan)
    pub size: Option<String>,
    /// Creature type (humanoid, undead, dragon, etc.)
    pub creature_type: Option<String>,
    /// Alignment (lawful good, neutral evil, etc.)
    pub alignment: Option<String>,
    /// Armor Class (with armor type if specified)
    pub armor_class: Option<ArmorClass>,
    /// Hit Points (with dice formula)
    pub hit_points: Option<HitPoints>,
    /// Movement speeds
    pub speed: Speed,
    /// Ability scores
    pub ability_scores: AbilityScores,
    /// Saving throw proficiencies
    pub saving_throws: HashMap<String, i32>,
    /// Skill proficiencies
    pub skills: HashMap<String, i32>,
    /// Damage vulnerabilities
    pub damage_vulnerabilities: Vec<String>,
    /// Damage resistances
    pub damage_resistances: Vec<String>,
    /// Damage immunities
    pub damage_immunities: Vec<String>,
    /// Condition immunities
    pub condition_immunities: Vec<String>,
    /// Senses (darkvision, etc.)
    pub senses: Vec<String>,
    /// Languages known
    pub languages: Vec<String>,
    /// Challenge Rating
    pub challenge_rating: Option<ChallengeRating>,
    /// Traits (passive abilities)
    pub traits: Vec<Feature>,
    /// Actions
    pub actions: Vec<Feature>,
    /// Bonus Actions
    pub bonus_actions: Vec<Feature>,
    /// Reactions
    pub reactions: Vec<Feature>,
    /// Legendary Actions
    pub legendary_actions: Vec<Feature>,
    /// Raw text that couldn't be parsed
    pub unparsed_sections: Vec<String>,
}


/// Armor Class with optional armor type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmorClass {
    pub value: i32,
    pub armor_type: Option<String>,
}

/// Hit Points with dice formula.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitPoints {
    pub average: i32,
    pub formula: Option<String>,
}

/// Movement speeds.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Speed {
    pub walk: Option<i32>,
    pub fly: Option<i32>,
    pub swim: Option<i32>,
    pub climb: Option<i32>,
    pub burrow: Option<i32>,
    pub hover: bool,
}

/// Six ability scores.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AbilityScores {
    pub strength: Option<i32>,
    pub dexterity: Option<i32>,
    pub constitution: Option<i32>,
    pub intelligence: Option<i32>,
    pub wisdom: Option<i32>,
    pub charisma: Option<i32>,
}

impl AbilityScores {
    /// Calculate modifier from ability score.
    pub fn modifier(score: i32) -> i32 {
        // D&D modifier formula: floor((score - 10) / 2)
        // Using integer formula: score/2 - 5 gives correct floor behavior
        (score / 2) - 5
    }

    /// Get a score by name (case-insensitive).
    pub fn get(&self, name: &str) -> Option<i32> {
        match name.to_lowercase().as_str() {
            "str" | "strength" => self.strength,
            "dex" | "dexterity" => self.dexterity,
            "con" | "constitution" => self.constitution,
            "int" | "intelligence" => self.intelligence,
            "wis" | "wisdom" => self.wisdom,
            "cha" | "charisma" => self.charisma,
            _ => None,
        }
    }
}

/// Challenge Rating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeRating {
    pub value: f32,
    pub xp: Option<i32>,
}

/// A creature feature (trait, action, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub name: String,
    pub description: String,
    /// For attacks: damage dice
    pub damage: Option<String>,
    /// For attacks: attack bonus
    pub attack_bonus: Option<i32>,
    /// For attacks: reach/range
    pub reach: Option<String>,
    /// For legendary actions: cost
    pub cost: Option<i32>,
}

impl Feature {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            damage: None,
            attack_bonus: None,
            reach: None,
            cost: None,
        }
    }
}

// ============================================================================
// Parser
// ============================================================================

/// Parses stat block text into structured data.
pub struct StatBlockParser {
    // Compiled regex patterns
    size_type_alignment: Regex,
    armor_class: Regex,
    hit_points: Regex,
    speed: Regex,
    ability_scores: Regex,
    challenge_rating: Regex,
    feature: Regex,
}

impl Default for StatBlockParser {
    fn default() -> Self {
        Self::new()
    }
}

impl StatBlockParser {
    /// Create a new stat block parser.
    pub fn new() -> Self {
        Self {
            size_type_alignment: Regex::new(
                r"(?i)(tiny|small|medium|large|huge|gargantuan)\s+(\w+(?:\s+\([^)]+\))?),?\s*([\w\s]+)?"
            ).unwrap(),
            armor_class: Regex::new(
                r"(?i)armor\s+class\s+(\d+)\s*(?:\(([^)]+)\))?"
            ).unwrap(),
            hit_points: Regex::new(
                r"(?i)hit\s+points\s+(\d+)\s*(?:\(([^)]+)\))?"
            ).unwrap(),
            speed: Regex::new(
                r"(?i)speed\s+([\d\w\s,\.]+(?:ft\.?)?)"
            ).unwrap(),
            ability_scores: Regex::new(
                r"(?i)(str|dex|con|int|wis|cha)\s+(\d+)\s*\(([+-]?\d+)\)"
            ).unwrap(),
            challenge_rating: Regex::new(
                r"(?i)challenge\s+(\d+(?:/\d+)?)\s*(?:\(([,\d]+)\s*xp\))?"
            ).unwrap(),
            // Simple pattern without lookahead - we'll parse features line by line instead
            feature: Regex::new(
                r"^([A-Z][A-Za-z\s'-]+)\.\s*(.+)"
            ).unwrap(),
        }
    }

    /// Parse stat block text into structured data.
    ///
    /// # Arguments
    /// * `text` - The stat block text
    ///
    /// # Returns
    /// * `Result<StatBlockData, String>` - Parsed data or error message
    pub fn parse(&self, text: &str) -> Result<StatBlockData, String> {
        let mut data = StatBlockData::default();
        let lines: Vec<&str> = text.lines().map(|l| l.trim()).collect();

        if lines.is_empty() {
            return Err("Empty stat block".to_string());
        }

        // First non-empty line is usually the name
        for line in &lines {
            if !line.is_empty() {
                data.name = line.to_string();
                break;
            }
        }

        let text_lower = text.to_lowercase();

        // Parse size/type/alignment
        if let Some(caps) = self.size_type_alignment.captures(&text_lower) {
            data.size = caps.get(1).map(|m| m.as_str().to_string());
            data.creature_type = caps.get(2).map(|m| m.as_str().to_string());
            data.alignment = caps.get(3).map(|m| m.as_str().trim().to_string());
        }

        // Parse AC
        if let Some(caps) = self.armor_class.captures(text) {
            if let Ok(ac) = caps.get(1).unwrap().as_str().parse::<i32>() {
                data.armor_class = Some(ArmorClass {
                    value: ac,
                    armor_type: caps.get(2).map(|m| m.as_str().to_string()),
                });
            }
        }

        // Parse HP
        if let Some(caps) = self.hit_points.captures(text) {
            if let Ok(hp) = caps.get(1).unwrap().as_str().parse::<i32>() {
                data.hit_points = Some(HitPoints {
                    average: hp,
                    formula: caps.get(2).map(|m| m.as_str().to_string()),
                });
            }
        }

        // Parse speed
        if let Some(caps) = self.speed.captures(text) {
            let speed_text = caps.get(1).unwrap().as_str().to_lowercase();
            data.speed = self.parse_speed(&speed_text);
        }

        // Parse ability scores
        for caps in self.ability_scores.captures_iter(text) {
            let ability = caps.get(1).unwrap().as_str().to_lowercase();
            if let Ok(score) = caps.get(2).unwrap().as_str().parse::<i32>() {
                match ability.as_str() {
                    "str" => data.ability_scores.strength = Some(score),
                    "dex" => data.ability_scores.dexterity = Some(score),
                    "con" => data.ability_scores.constitution = Some(score),
                    "int" => data.ability_scores.intelligence = Some(score),
                    "wis" => data.ability_scores.wisdom = Some(score),
                    "cha" => data.ability_scores.charisma = Some(score),
                    _ => {}
                }
            }
        }

        // Parse CR
        if let Some(caps) = self.challenge_rating.captures(text) {
            let cr_str = caps.get(1).unwrap().as_str();
            let cr_value = if cr_str.contains('/') {
                let parts: Vec<&str> = cr_str.split('/').collect();
                if parts.len() == 2 {
                    let num = parts[0].parse::<f32>().unwrap_or(0.0);
                    let denom = parts[1].parse::<f32>().unwrap_or(1.0);
                    if denom.abs() < f32::EPSILON {
                        0.0
                    } else {
                        num / denom
                    }
                } else {
                    0.0
                }
            } else {
                cr_str.parse::<f32>().unwrap_or(0.0)
            };

            let xp = caps.get(2).and_then(|m| {
                m.as_str().replace(',', "").parse::<i32>().ok()
            });

            data.challenge_rating = Some(ChallengeRating {
                value: cr_value,
                xp,
            });
        }

        // Parse damage resistances/immunities
        self.parse_damage_types(&text_lower, &mut data);

        // Parse condition immunities
        self.parse_condition_immunities(&text_lower, &mut data);

        // Parse senses
        self.parse_senses(&text_lower, &mut data);

        // Parse languages
        self.parse_languages(&text_lower, &mut data);

        // Parse features/actions
        self.parse_features(text, &mut data);

        Ok(data)
    }

    fn parse_speed(&self, text: &str) -> Speed {
        let mut speed = Speed::default();
        let number = Regex::new(r"(\d+)").unwrap();

        // Walk speed (default)
        if let Some(caps) = number.captures(text) {
            speed.walk = caps.get(1).and_then(|m| m.as_str().parse().ok());
        }

        // Other speeds
        let fly_re = Regex::new(r"fly\s+(\d+)").unwrap();
        let swim_re = Regex::new(r"swim\s+(\d+)").unwrap();
        let climb_re = Regex::new(r"climb\s+(\d+)").unwrap();
        let burrow_re = Regex::new(r"burrow\s+(\d+)").unwrap();

        if let Some(caps) = fly_re.captures(text) {
            speed.fly = caps.get(1).and_then(|m| m.as_str().parse().ok());
            speed.hover = text.contains("hover");
        }
        if let Some(caps) = swim_re.captures(text) {
            speed.swim = caps.get(1).and_then(|m| m.as_str().parse().ok());
        }
        if let Some(caps) = climb_re.captures(text) {
            speed.climb = caps.get(1).and_then(|m| m.as_str().parse().ok());
        }
        if let Some(caps) = burrow_re.captures(text) {
            speed.burrow = caps.get(1).and_then(|m| m.as_str().parse().ok());
        }

        speed
    }

    fn parse_damage_types(&self, text: &str, data: &mut StatBlockData) {
        let damage_types = [
            "acid", "bludgeoning", "cold", "fire", "force", "lightning",
            "necrotic", "piercing", "poison", "psychic", "radiant",
            "slashing", "thunder",
        ];

        // Check for vulnerabilities
        if let Some(start) = text.find("damage vulnerabilities") {
            let section = &text[start..];
            if let Some(end) = section.find('\n') {
                let line = &section[..end];
                for dt in &damage_types {
                    if line.contains(dt) {
                        data.damage_vulnerabilities.push(dt.to_string());
                    }
                }
            }
        }

        // Check for resistances
        if let Some(start) = text.find("damage resistances") {
            let section = &text[start..];
            if let Some(end) = section.find('\n') {
                let line = &section[..end];
                for dt in &damage_types {
                    if line.contains(dt) {
                        data.damage_resistances.push(dt.to_string());
                    }
                }
            }
        }

        // Check for immunities
        if let Some(start) = text.find("damage immunities") {
            let section = &text[start..];
            if let Some(end) = section.find('\n') {
                let line = &section[..end];
                for dt in &damage_types {
                    if line.contains(dt) {
                        data.damage_immunities.push(dt.to_string());
                    }
                }
            }
        }
    }

    fn parse_condition_immunities(&self, text: &str, data: &mut StatBlockData) {
        let conditions = [
            "blinded", "charmed", "deafened", "exhaustion", "frightened",
            "grappled", "incapacitated", "invisible", "paralyzed", "petrified",
            "poisoned", "prone", "restrained", "stunned", "unconscious",
        ];

        if let Some(start) = text.find("condition immunities") {
            let section = &text[start..];
            if let Some(end) = section.find('\n') {
                let line = &section[..end];
                for cond in &conditions {
                    if line.contains(cond) {
                        data.condition_immunities.push(cond.to_string());
                    }
                }
            }
        }
    }

    fn parse_senses(&self, text: &str, data: &mut StatBlockData) {
        if let Some(start) = text.find("senses") {
            let section = &text[start..];
            if let Some(end) = section.find('\n') {
                let line = &section[..end].replace("senses", "").trim().to_string();
                data.senses = line.split(',').map(|s| s.trim().to_string()).collect();
            }
        }
    }

    fn parse_languages(&self, text: &str, data: &mut StatBlockData) {
        if let Some(start) = text.find("languages") {
            let section = &text[start..];
            if let Some(end) = section.find('\n') {
                let line = &section[..end].replace("languages", "").trim().to_string();
                if line != "—" && !line.is_empty() {
                    data.languages = line.split(',').map(|s| s.trim().to_string()).collect();
                }
            }
        }
    }

    fn parse_features(&self, text: &str, data: &mut StatBlockData) {
        // Look for Actions section
        let actions_start = text.to_lowercase().find("actions");
        let reactions_start = text.to_lowercase().find("reactions");
        let legendary_start = text.to_lowercase().find("legendary actions");

        // Parse features based on sections
        // This is a simplified implementation - full parsing would be more complex
        for caps in self.feature.captures_iter(text) {
            let name = caps.get(1).unwrap().as_str().trim().to_string();
            let desc = caps.get(2).unwrap().as_str().trim().to_string();

            let feature = Feature::new(name, desc);

            // Determine which section this feature belongs to
            let feature_pos = caps.get(0).unwrap().start();

            if let Some(leg_start) = legendary_start {
                if feature_pos > leg_start {
                    data.legendary_actions.push(feature);
                    continue;
                }
            }
            if let Some(react_start) = reactions_start {
                if feature_pos > react_start {
                    data.reactions.push(feature);
                    continue;
                }
            }
            if let Some(act_start) = actions_start {
                if feature_pos > act_start {
                    data.actions.push(feature);
                    continue;
                }
            }

            data.traits.push(feature);
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ability_modifier() {
        assert_eq!(AbilityScores::modifier(10), 0);
        assert_eq!(AbilityScores::modifier(8), -1);
        assert_eq!(AbilityScores::modifier(14), 2);
        assert_eq!(AbilityScores::modifier(20), 5);
        assert_eq!(AbilityScores::modifier(1), -5);
    }

    #[test]
    fn test_parse_goblin() {
        let parser = StatBlockParser::new();
        let text = r#"
            Goblin
            Small humanoid (goblinoid), neutral evil
            Armor Class 15 (leather armor, shield)
            Hit Points 7 (2d6)
            Speed 30 ft.
            STR 8 (-1) DEX 14 (+2) CON 10 (+0) INT 10 (+0) WIS 8 (-1) CHA 8 (-1)
            Skills Stealth +6
            Senses darkvision 60 ft., passive Perception 9
            Languages Common, Goblin
            Challenge 1/4 (50 XP)
        "#;

        let result = parser.parse(text);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.size, Some("small".to_string()));
        assert_eq!(data.creature_type, Some("humanoid (goblinoid)".to_string()));
        assert_eq!(data.armor_class.as_ref().map(|ac| ac.value), Some(15));
        assert_eq!(data.hit_points.as_ref().map(|hp| hp.average), Some(7));
        assert_eq!(data.ability_scores.strength, Some(8));
        assert_eq!(data.ability_scores.dexterity, Some(14));
        assert!(data.challenge_rating.is_some());
        assert!((data.challenge_rating.as_ref().unwrap().value - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_parse_speed() {
        let parser = StatBlockParser::new();

        let speed = parser.parse_speed("30 ft., fly 60 ft., swim 30 ft.");
        assert_eq!(speed.walk, Some(30));
        assert_eq!(speed.fly, Some(60));
        assert_eq!(speed.swim, Some(30));
        assert!(!speed.hover);

        let speed = parser.parse_speed("0 ft., fly 30 ft. (hover)");
        assert_eq!(speed.walk, Some(0));
        assert_eq!(speed.fly, Some(30));
        assert!(speed.hover);
    }
}
