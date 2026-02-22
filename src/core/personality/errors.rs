//! Personality Extension Error Types
//!
//! Defines error types for the personality blending and context detection system.
//! Uses `thiserror` for ergonomic error handling with source context.

use thiserror::Error;

// ============================================================================
// Template Errors
// ============================================================================

/// Errors related to personality template operations.
#[derive(Error, Debug)]
pub enum TemplateError {
    /// Template with the given ID was not found.
    #[error("template not found: {id}")]
    NotFound {
        /// The template ID that was not found.
        id: String,
    },

    /// Failed to parse template YAML/JSON.
    #[error("template parse error in '{file}' at line {line}: {message}")]
    ParseError {
        /// File path or identifier where the error occurred.
        file: String,
        /// Line number where the error occurred (0 if unknown).
        line: usize,
        /// Description of the parse error.
        message: String,
        /// Underlying error source.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Template validation failed (e.g., missing required fields).
    #[error("template validation failed for '{template_id}': {message}")]
    ValidationError {
        /// The template ID that failed validation.
        template_id: String,
        /// Description of the validation failure.
        message: String,
    },

    /// Base profile referenced by template does not exist.
    #[error("base profile '{base_profile}' not found for template '{template_id}'")]
    BaseProfileNotFound {
        /// The template ID that references the missing profile.
        template_id: String,
        /// The base profile ID that was not found.
        base_profile: String,
    },

    /// I/O error while loading template file.
    #[error("failed to read template file '{path}': {message}")]
    IoError {
        /// Path to the file that could not be read.
        path: String,
        /// Description of the I/O error.
        message: String,
        /// Underlying I/O error.
        #[source]
        source: Option<std::io::Error>,
    },

    /// Meilisearch operation failed.
    #[error("meilisearch error for template '{template_id}': {message}")]
    MeilisearchError {
        /// The template ID involved in the operation.
        template_id: String,
        /// Description of the Meilisearch error.
        message: String,
    },
}

impl TemplateError {
    /// Create a new NotFound error.
    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound { id: id.into() }
    }

    /// Create a new ParseError.
    pub fn parse_error(
        file: impl Into<String>,
        line: usize,
        message: impl Into<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::ParseError {
            file: file.into(),
            line,
            message: message.into(),
            source,
        }
    }

    /// Create a new ValidationError.
    pub fn validation_error(template_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ValidationError {
            template_id: template_id.into(),
            message: message.into(),
        }
    }

    /// Create a new BaseProfileNotFound error.
    pub fn base_profile_not_found(
        template_id: impl Into<String>,
        base_profile: impl Into<String>,
    ) -> Self {
        Self::BaseProfileNotFound {
            template_id: template_id.into(),
            base_profile: base_profile.into(),
        }
    }

    /// Create a new IoError.
    pub fn io_error(path: impl Into<String>, source: std::io::Error) -> Self {
        Self::IoError {
            path: path.into(),
            message: source.to_string(),
            source: Some(source),
        }
    }

    /// Create a new MeilisearchError.
    pub fn meilisearch_error(template_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::MeilisearchError {
            template_id: template_id.into(),
            message: message.into(),
        }
    }
}

// ============================================================================
// Blend Errors
// ============================================================================

/// Errors related to personality blending operations.
#[derive(Error, Debug)]
pub enum BlendError {
    /// No personality components provided for blending.
    #[error("cannot create blend with zero components")]
    EmptyComponents,

    /// Blend weights do not sum to 1.0 (within tolerance).
    #[error("blend weights sum to {actual:.4}, expected 1.0 (tolerance: {tolerance:.4})")]
    InvalidWeightSum {
        /// The actual sum of weights.
        actual: f32,
        /// The tolerance used for comparison.
        tolerance: f32,
    },

    /// Individual weight is out of valid range [0.0, 1.0].
    #[error("weight {weight:.4} for component '{component_id}' is out of range [0.0, 1.0]")]
    WeightOutOfRange {
        /// The component ID with the invalid weight.
        component_id: String,
        /// The invalid weight value.
        weight: f32,
    },

    /// Referenced personality profile not found during blend creation.
    #[error("personality profile '{profile_id}' not found for blend component")]
    ProfileNotFound {
        /// The profile ID that was not found.
        profile_id: String,
    },

    /// Interpolation failed for a specific dimension.
    #[error("interpolation failed for dimension '{dimension}': {message}")]
    InterpolationError {
        /// The dimension that failed interpolation.
        dimension: String,
        /// Description of the interpolation failure.
        message: String,
    },

    /// Blend context is incompatible with requested operation.
    #[error("blend context '{context}' is incompatible: {message}")]
    IncompatibleContext {
        /// The context that caused the incompatibility.
        context: String,
        /// Description of why it is incompatible.
        message: String,
    },
}

impl BlendError {
    /// Create a new InvalidWeightSum error.
    pub fn invalid_weight_sum(actual: f32, tolerance: f32) -> Self {
        Self::InvalidWeightSum { actual, tolerance }
    }

    /// Create a new WeightOutOfRange error.
    pub fn weight_out_of_range(component_id: impl Into<String>, weight: f32) -> Self {
        Self::WeightOutOfRange {
            component_id: component_id.into(),
            weight,
        }
    }

    /// Create a new ProfileNotFound error.
    pub fn profile_not_found(profile_id: impl Into<String>) -> Self {
        Self::ProfileNotFound {
            profile_id: profile_id.into(),
        }
    }

    /// Create a new InterpolationError.
    pub fn interpolation_error(dimension: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InterpolationError {
            dimension: dimension.into(),
            message: message.into(),
        }
    }

    /// Create a new IncompatibleContext error.
    pub fn incompatible_context(context: impl Into<String>, message: impl Into<String>) -> Self {
        Self::IncompatibleContext {
            context: context.into(),
            message: message.into(),
        }
    }
}

// ============================================================================
// Blend Rule Errors
// ============================================================================

/// Errors related to blend rule operations.
#[derive(Error, Debug)]
pub enum BlendRuleError {
    /// Blend rule with the given ID was not found.
    #[error("blend rule not found: {id}")]
    NotFound {
        /// The rule ID that was not found.
        id: String,
    },

    /// Rule definition is invalid.
    #[error("invalid blend rule '{rule_id}': {message}")]
    InvalidRule {
        /// The rule ID that is invalid.
        rule_id: String,
        /// Description of why the rule is invalid.
        message: String,
    },

    /// Rule conflicts with existing rules.
    #[error("blend rule '{rule_id}' conflicts with existing rule '{conflicting_rule_id}'")]
    RuleConflict {
        /// The new rule ID that conflicts.
        rule_id: String,
        /// The existing rule ID that it conflicts with.
        conflicting_rule_id: String,
    },

    /// Rule evaluation failed.
    #[error("failed to evaluate blend rule '{rule_id}': {message}")]
    EvaluationError {
        /// The rule ID that failed evaluation.
        rule_id: String,
        /// Description of the evaluation failure.
        message: String,
    },

    /// Meilisearch operation failed.
    #[error("meilisearch error for rule '{rule_id}': {message}")]
    MeilisearchError {
        /// The rule ID involved in the operation.
        rule_id: String,
        /// Description of the Meilisearch error.
        message: String,
    },
}

impl BlendRuleError {
    /// Create a new NotFound error.
    pub fn not_found(id: impl Into<String>) -> Self {
        Self::NotFound { id: id.into() }
    }

    /// Create a new InvalidRule error.
    pub fn invalid_rule(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidRule {
            rule_id: rule_id.into(),
            message: message.into(),
        }
    }

    /// Create a new RuleConflict error.
    pub fn rule_conflict(
        rule_id: impl Into<String>,
        conflicting_rule_id: impl Into<String>,
    ) -> Self {
        Self::RuleConflict {
            rule_id: rule_id.into(),
            conflicting_rule_id: conflicting_rule_id.into(),
        }
    }

    /// Create a new EvaluationError.
    pub fn evaluation_error(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::EvaluationError {
            rule_id: rule_id.into(),
            message: message.into(),
        }
    }

    /// Create a new MeilisearchError.
    pub fn meilisearch_error(rule_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::MeilisearchError {
            rule_id: rule_id.into(),
            message: message.into(),
        }
    }
}

// ============================================================================
// Context Detection Errors
// ============================================================================

/// Errors related to gameplay context detection.
#[derive(Error, Debug)]
pub enum ContextDetectionError {
    /// Input text is too short for reliable detection.
    #[error("input text too short for context detection (min: {min_length}, actual: {actual_length})")]
    InputTooShort {
        /// Minimum required length.
        min_length: usize,
        /// Actual input length.
        actual_length: usize,
    },

    /// Detection confidence is below threshold.
    #[error("context detection confidence {confidence:.2} is below threshold {threshold:.2}")]
    LowConfidence {
        /// The confidence score achieved.
        confidence: f32,
        /// The minimum required threshold.
        threshold: f32,
    },

    /// Multiple contexts detected with similar confidence (ambiguous).
    #[error("ambiguous context: {contexts:?} with similar confidence scores")]
    AmbiguousContext {
        /// The contexts detected with similar confidence.
        contexts: Vec<String>,
    },

    /// Keyword configuration is invalid.
    #[error("invalid keyword configuration: {message}")]
    InvalidKeywordConfig {
        /// Description of the configuration error.
        message: String,
    },

    /// Context cache error.
    #[error("context cache error: {message}")]
    CacheError {
        /// Description of the cache error.
        message: String,
    },
}

impl ContextDetectionError {
    /// Create a new InputTooShort error.
    pub fn input_too_short(min_length: usize, actual_length: usize) -> Self {
        Self::InputTooShort {
            min_length,
            actual_length,
        }
    }

    /// Create a new LowConfidence error.
    pub fn low_confidence(confidence: f32, threshold: f32) -> Self {
        Self::LowConfidence {
            confidence,
            threshold,
        }
    }

    /// Create a new AmbiguousContext error.
    pub fn ambiguous_context(contexts: Vec<String>) -> Self {
        Self::AmbiguousContext { contexts }
    }

    /// Create a new InvalidKeywordConfig error.
    pub fn invalid_keyword_config(message: impl Into<String>) -> Self {
        Self::InvalidKeywordConfig {
            message: message.into(),
        }
    }

    /// Create a new CacheError.
    pub fn cache_error(message: impl Into<String>) -> Self {
        Self::CacheError {
            message: message.into(),
        }
    }
}

// ============================================================================
// Unified Error Type
// ============================================================================

/// Unified error type for all personality extension operations.
#[derive(Error, Debug)]
pub enum PersonalityExtensionError {
    /// Template-related error.
    #[error(transparent)]
    Template(#[from] TemplateError),

    /// Blend-related error.
    #[error(transparent)]
    Blend(#[from] BlendError),

    /// Blend rule-related error.
    #[error(transparent)]
    BlendRule(#[from] BlendRuleError),

    /// Context detection error.
    #[error(transparent)]
    ContextDetection(#[from] ContextDetectionError),

    /// Base personality error (from personality_base module).
    #[error("personality error: {0}")]
    PersonalityBase(String),

    /// Generic internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl PersonalityExtensionError {
    /// Create a new Internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Create a PersonalityBase error from a string message.
    pub fn personality_base(message: impl Into<String>) -> Self {
        Self::PersonalityBase(message.into())
    }
}

/// Result type alias for personality extension operations.
pub type Result<T> = std::result::Result<T, PersonalityExtensionError>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_error_display() {
        let err = TemplateError::not_found("forgotten_realms_sage");
        assert_eq!(
            err.to_string(),
            "template not found: forgotten_realms_sage"
        );

        let err = TemplateError::parse_error("templates/sage.yaml", 42, "unexpected token", None);
        assert_eq!(
            err.to_string(),
            "template parse error in 'templates/sage.yaml' at line 42: unexpected token"
        );

        let err =
            TemplateError::validation_error("broken_template", "missing required field 'name'");
        assert_eq!(
            err.to_string(),
            "template validation failed for 'broken_template': missing required field 'name'"
        );

        let err = TemplateError::base_profile_not_found("my_template", "storyteller");
        assert_eq!(
            err.to_string(),
            "base profile 'storyteller' not found for template 'my_template'"
        );
    }

    #[test]
    fn test_blend_error_display() {
        let err = BlendError::EmptyComponents;
        assert_eq!(err.to_string(), "cannot create blend with zero components");

        let err = BlendError::invalid_weight_sum(1.5, 0.001);
        assert!(err.to_string().contains("1.5"));
        assert!(err.to_string().contains("1.0"));

        let err = BlendError::weight_out_of_range("tactical_advisor", -0.5);
        assert!(err.to_string().contains("tactical_advisor"));
        assert!(err.to_string().contains("-0.5"));

        let err = BlendError::profile_not_found("nonexistent_profile");
        assert_eq!(
            err.to_string(),
            "personality profile 'nonexistent_profile' not found for blend component"
        );
    }

    #[test]
    fn test_blend_rule_error_display() {
        let err = BlendRuleError::not_found("rule_123");
        assert_eq!(err.to_string(), "blend rule not found: rule_123");

        let err = BlendRuleError::rule_conflict("new_rule", "existing_rule");
        assert_eq!(
            err.to_string(),
            "blend rule 'new_rule' conflicts with existing rule 'existing_rule'"
        );
    }

    #[test]
    fn test_context_detection_error_display() {
        let err = ContextDetectionError::input_too_short(50, 10);
        assert!(err.to_string().contains("50"));
        assert!(err.to_string().contains("10"));

        let err = ContextDetectionError::low_confidence(0.35, 0.5);
        assert!(err.to_string().contains("0.35"));
        assert!(err.to_string().contains("0.5"));

        let err = ContextDetectionError::ambiguous_context(vec![
            "combat".to_string(),
            "exploration".to_string(),
        ]);
        assert!(err.to_string().contains("combat"));
        assert!(err.to_string().contains("exploration"));
    }

    #[test]
    fn test_unified_error_from_variants() {
        let template_err = TemplateError::not_found("test");
        let unified: PersonalityExtensionError = template_err.into();
        assert!(matches!(unified, PersonalityExtensionError::Template(_)));

        let blend_err = BlendError::EmptyComponents;
        let unified: PersonalityExtensionError = blend_err.into();
        assert!(matches!(unified, PersonalityExtensionError::Blend(_)));

        let rule_err = BlendRuleError::not_found("test");
        let unified: PersonalityExtensionError = rule_err.into();
        assert!(matches!(unified, PersonalityExtensionError::BlendRule(_)));

        let context_err = ContextDetectionError::input_too_short(50, 10);
        let unified: PersonalityExtensionError = context_err.into();
        assert!(matches!(
            unified,
            PersonalityExtensionError::ContextDetection(_)
        ));
    }

    #[test]
    fn test_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TemplateError>();
        assert_send_sync::<BlendError>();
        assert_send_sync::<BlendRuleError>();
        assert_send_sync::<ContextDetectionError>();
        assert_send_sync::<PersonalityExtensionError>();
    }
}
