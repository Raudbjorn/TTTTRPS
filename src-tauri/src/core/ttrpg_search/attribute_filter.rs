//! Attribute Filter Module
//!
//! Builds Meilisearch filter strings from QueryConstraints.

use super::QueryConstraints;
use crate::ingestion::ttrpg::{GameVocabulary, DnD5eVocabulary};

// ============================================================================
// Attribute Filter Builder
// ============================================================================

/// Builds Meilisearch filter strings from query constraints
pub struct AttributeFilter;

impl AttributeFilter {
    /// Build a Meilisearch filter string from constraints
    ///
    /// # Example output
    /// ```text
    /// (damage_types = "fire" OR damage_types = "radiant") AND NOT creature_types = "undead" AND challenge_rating >= 1 AND challenge_rating <= 5
    /// ```
    pub fn build_filter_string(constraints: &QueryConstraints) -> String {
        let mut filters = Vec::new();

        // Group required attributes by category
        let mut attrs_by_category: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for attr in &constraints.required_attributes {
            if attr.required {
                attrs_by_category
                    .entry(attr.category.clone())
                    .or_default()
                    .push(attr.value.clone());
            }
        }

        // Build OR filters for each category
        for (category, values) in attrs_by_category {
            let field_name = Self::category_to_field(&category);
            let or_parts: Vec<String> = values
                .iter()
                .map(|v| format!("{} = \"{}\"", field_name, Self::escape_value(v)))
                .collect();

            if !or_parts.is_empty() {
                if or_parts.len() == 1 {
                    filters.push(or_parts[0].clone());
                } else {
                    filters.push(format!("({})", or_parts.join(" OR ")));
                }
            }
        }

        // Build NOT filters for excluded attributes
        for excl in &constraints.excluded_attributes {
            // Try to match to a field
            let field = Self::guess_field_for_value(excl);
            filters.push(format!("NOT {} = \"{}\"", field, Self::escape_value(excl)));
        }

        // Build CR range filter
        if let Some((min_cr, max_cr)) = constraints.cr_range {
            if (min_cr - max_cr).abs() < f32::EPSILON {
                filters.push(format!("challenge_rating = {}", min_cr));
            } else {
                filters.push(format!("challenge_rating >= {}", min_cr));
                filters.push(format!("challenge_rating <= {}", max_cr));
            }
        }

        // Build level range filter
        if let Some((min_lvl, max_lvl)) = constraints.level_range {
            if min_lvl == max_lvl {
                filters.push(format!("level = {}", min_lvl));
            } else {
                filters.push(format!("level >= {}", min_lvl));
                filters.push(format!("level <= {}", max_lvl));
            }
        }

        filters.join(" AND ")
    }

    /// Build a filter for specific element types
    pub fn build_element_type_filter(element_types: &[&str]) -> String {
        if element_types.is_empty() {
            return String::new();
        }

        let parts: Vec<String> = element_types
            .iter()
            .map(|t| format!("element_type = \"{}\"", Self::escape_value(t)))
            .collect();

        if parts.len() == 1 {
            parts[0].clone()
        } else {
            format!("({})", parts.join(" OR "))
        }
    }

    /// Build a filter for game system
    pub fn build_game_system_filter(game_system: &str) -> String {
        format!("game_system = \"{}\"", Self::escape_value(game_system))
    }

    /// Build a filter for source document
    pub fn build_source_filter(source: &str) -> String {
        format!("source = \"{}\"", Self::escape_value(source))
    }

    /// Combine multiple filter strings with AND
    pub fn combine_and(filters: &[String]) -> String {
        filters
            .iter()
            .filter(|f| !f.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" AND ")
    }

    /// Combine multiple filter strings with OR
    pub fn combine_or(filters: &[String]) -> String {
        let non_empty: Vec<_> = filters
            .iter()
            .filter(|f| !f.is_empty())
            .cloned()
            .collect();

        if non_empty.is_empty() {
            String::new()
        } else if non_empty.len() == 1 {
            non_empty[0].clone()
        } else {
            format!("({})", non_empty.join(" OR "))
        }
    }

    /// Map category name to Meilisearch field name
    fn category_to_field(category: &str) -> &'static str {
        match category.to_lowercase().as_str() {
            "damage_type" | "damage" => "damage_types",
            "creature_type" | "creature" => "creature_types",
            "condition" => "conditions",
            "alignment" => "alignments",
            "rarity" => "rarities",
            "size" => "sizes",
            "spell_school" | "school" => "spell_schools",
            _ => "metadata",
        }
    }

    /// Guess which field a value might belong to using GameVocabulary
    ///
    /// Uses the D&D 5e vocabulary by default but could be extended to support
    /// other game systems by passing a vocabulary parameter.
    fn guess_field_for_value(value: &str) -> &'static str {
        Self::guess_field_for_value_with_vocabulary(value, &DnD5eVocabulary)
    }

    /// Guess which field a value might belong to using a specific vocabulary
    fn guess_field_for_value_with_vocabulary(value: &str, vocabulary: &dyn GameVocabulary) -> &'static str {
        let lower = value.to_lowercase();

        // Check damage types from vocabulary
        if vocabulary.damage_types().iter().any(|d| lower == *d) {
            return "damage_types";
        }

        // Check creature types from vocabulary
        if vocabulary.creature_types().iter().any(|c| lower == *c) {
            return "creature_types";
        }

        // Check conditions from vocabulary
        if vocabulary.conditions().iter().any(|c| lower == *c) {
            return "conditions";
        }

        // Check sizes from vocabulary
        if vocabulary.sizes().iter().any(|s| lower == *s) {
            return "sizes";
        }

        // Check rarities from vocabulary
        if vocabulary.rarities().iter().any(|r| lower == *r) {
            return "rarities";
        }

        // Default
        "metadata"
    }

    /// Escape special characters in filter values
    fn escape_value(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::RequiredAttribute;

    #[test]
    fn test_single_attribute_filter() {
        let constraints = QueryConstraints {
            required_attributes: vec![RequiredAttribute {
                category: "damage_type".to_string(),
                value: "fire".to_string(),
                required: true,
            }],
            ..Default::default()
        };

        let filter = AttributeFilter::build_filter_string(&constraints);
        assert!(filter.contains("damage_types = \"fire\""));
    }

    #[test]
    fn test_multiple_attributes_same_category() {
        let constraints = QueryConstraints {
            required_attributes: vec![
                RequiredAttribute {
                    category: "damage_type".to_string(),
                    value: "fire".to_string(),
                    required: true,
                },
                RequiredAttribute {
                    category: "damage_type".to_string(),
                    value: "cold".to_string(),
                    required: true,
                },
            ],
            ..Default::default()
        };

        let filter = AttributeFilter::build_filter_string(&constraints);
        assert!(filter.contains("OR"));
        assert!(filter.contains("fire"));
        assert!(filter.contains("cold"));
    }

    #[test]
    fn test_exclusion_filter() {
        let constraints = QueryConstraints {
            excluded_attributes: vec!["undead".to_string()],
            ..Default::default()
        };

        let filter = AttributeFilter::build_filter_string(&constraints);
        assert!(filter.contains("NOT"));
        assert!(filter.contains("undead"));
    }

    #[test]
    fn test_cr_range_filter() {
        let constraints = QueryConstraints {
            cr_range: Some((1.0, 5.0)),
            ..Default::default()
        };

        let filter = AttributeFilter::build_filter_string(&constraints);
        assert!(filter.contains("challenge_rating >= 1"));
        assert!(filter.contains("challenge_rating <= 5"));
    }

    #[test]
    fn test_level_filter() {
        let constraints = QueryConstraints {
            level_range: Some((3, 3)),
            ..Default::default()
        };

        let filter = AttributeFilter::build_filter_string(&constraints);
        assert!(filter.contains("level = 3"));
    }

    #[test]
    fn test_element_type_filter() {
        let filter = AttributeFilter::build_element_type_filter(&["stat_block", "spell"]);
        assert!(filter.contains("element_type = \"stat_block\""));
        assert!(filter.contains("element_type = \"spell\""));
        assert!(filter.contains(" OR "));
    }

    #[test]
    fn test_combine_filters() {
        let filter1 = "damage_types = \"fire\"".to_string();
        let filter2 = "challenge_rating >= 1".to_string();

        let combined = AttributeFilter::combine_and(&[filter1, filter2]);
        assert!(combined.contains(" AND "));
    }

    #[test]
    fn test_escape_value() {
        let value = "\"quoted\" and \\slashed";
        let escaped = AttributeFilter::escape_value(value);
        assert!(escaped.contains("\\\""));
        assert!(escaped.contains("\\\\"));
    }

    #[test]
    fn test_guess_field() {
        assert_eq!(AttributeFilter::guess_field_for_value("fire"), "damage_types");
        assert_eq!(AttributeFilter::guess_field_for_value("undead"), "creature_types");
        assert_eq!(AttributeFilter::guess_field_for_value("poisoned"), "conditions");
        assert_eq!(AttributeFilter::guess_field_for_value("medium"), "sizes");
        assert_eq!(AttributeFilter::guess_field_for_value("legendary"), "rarities");
    }
}
