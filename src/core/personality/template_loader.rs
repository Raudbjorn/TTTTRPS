//! YAML Template Loader (TASK-PERS-007)
//!
//! Provides functionality to load personality templates from YAML files.
//! Templates are loaded from the `assets/settings/` directory for built-in
//! templates and `~/.local/share/ttrpg-assistant/templates/` for user templates.
//!
//! ## Features
//!
//! - Loads `.yaml` files from configured directories
//! - Validates each template after parsing
//! - Logs errors for corrupted files, continues loading others
//! - Supports both built-in and user-created templates
//!
//! ## Example
//!
//! ```rust,ignore
//! use personality::template_loader::TemplateLoader;
//!
//! let loader = TemplateLoader::new()?;
//!
//! // Load all templates from assets/settings/
//! let builtin = loader.load_builtin_templates().await?;
//!
//! // Load user templates
//! let user = loader.load_user_templates().await?;
//!
//! // Load from a specific directory
//! let custom = loader.load_from_directory("/path/to/templates").await?;
//! ```

use super::errors::{PersonalityExtensionError, TemplateError};
use super::templates::{SettingTemplate, TemplateValidationConfig, TemplateYaml};
use super::types::TemplateId;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ============================================================================
// Constants
// ============================================================================

/// Default directory name for user templates.
const USER_TEMPLATES_DIR: &str = "templates";

/// App data directory name.
const APP_DATA_DIR: &str = "ttrpg-assistant";

/// Built-in templates directory relative to app resources.
const BUILTIN_TEMPLATES_DIR: &str = "assets/settings";

// ============================================================================
// Load Result
// ============================================================================

/// Result of loading templates from a directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateLoadResult {
    /// Successfully loaded templates.
    pub templates: Vec<SettingTemplate>,

    /// Files that failed to load with error messages.
    pub errors: Vec<LoadError>,

    /// Total number of files processed.
    pub files_processed: usize,

    /// Directory that was scanned.
    pub source_directory: String,
}

impl TemplateLoadResult {
    /// Create a new empty result.
    pub fn new(source_directory: impl Into<String>) -> Self {
        Self {
            templates: Vec::new(),
            errors: Vec::new(),
            files_processed: 0,
            source_directory: source_directory.into(),
        }
    }

    /// Check if all files loaded successfully.
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the count of successfully loaded templates.
    pub fn templates_loaded(&self) -> usize {
        self.templates.len()
    }

    /// Get the count of failed loads.
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

/// An error that occurred while loading a specific template file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadError {
    /// Path to the file that failed to load.
    pub file_path: String,

    /// Error message describing what went wrong.
    pub message: String,

    /// Line number where the error occurred (if applicable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,

    /// Error kind for categorization.
    pub kind: LoadErrorKind,
}

/// Categories of template loading errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadErrorKind {
    /// File could not be read (I/O error).
    IoError,
    /// YAML parsing failed.
    ParseError,
    /// Template validation failed.
    ValidationError,
    /// Unknown error.
    Unknown,
}

// ============================================================================
// Template Loader
// ============================================================================

/// Loader for personality templates from YAML files.
pub struct TemplateLoader {
    /// Path to built-in templates directory.
    builtin_dir: PathBuf,

    /// Path to user templates directory.
    user_dir: PathBuf,

    /// Validation configuration for loaded templates.
    validation_config: TemplateValidationConfig,
}

impl TemplateLoader {
    /// Create a new template loader with default directories.
    pub fn new() -> Result<Self, PersonalityExtensionError> {
        let builtin_dir = Self::default_builtin_dir()?;
        let user_dir = Self::default_user_dir()?;

        Ok(Self {
            builtin_dir,
            user_dir,
            validation_config: TemplateValidationConfig::default(),
        })
    }

    /// Create a loader with custom directories.
    pub fn with_directories(
        builtin_dir: impl Into<PathBuf>,
        user_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            builtin_dir: builtin_dir.into(),
            user_dir: user_dir.into(),
            validation_config: TemplateValidationConfig::default(),
        }
    }

    /// Set a custom validation configuration.
    pub fn with_validation_config(mut self, config: TemplateValidationConfig) -> Self {
        self.validation_config = config;
        self
    }

    /// Get the default directory for built-in templates.
    fn default_builtin_dir() -> Result<PathBuf, PersonalityExtensionError> {
        // In development, use the local assets directory
        // In production, this would be resolved from the app bundle
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok();

        if let Some(dir) = manifest_dir {
            // Development mode - relative to Cargo.toml
            Ok(PathBuf::from(dir).join(BUILTIN_TEMPLATES_DIR))
        } else {
            // Production mode - try to find relative to executable
            let exe_path = std::env::current_exe().map_err(|e| {
                PersonalityExtensionError::internal(format!(
                    "Failed to get executable path: {}",
                    e
                ))
            })?;

            // Go up from binary to find assets
            let app_dir = exe_path
                .parent()
                .and_then(|p| p.parent())
                .ok_or_else(|| {
                    PersonalityExtensionError::internal("Failed to find app directory")
                })?;

            Ok(app_dir.join(BUILTIN_TEMPLATES_DIR))
        }
    }

    /// Get the default directory for user templates.
    fn default_user_dir() -> Result<PathBuf, PersonalityExtensionError> {
        let data_dir = dirs::data_local_dir().ok_or_else(|| {
            PersonalityExtensionError::internal("Failed to get local data directory")
        })?;

        Ok(data_dir.join(APP_DATA_DIR).join(USER_TEMPLATES_DIR))
    }

    /// Get the built-in templates directory path.
    pub fn builtin_dir(&self) -> &Path {
        &self.builtin_dir
    }

    /// Get the user templates directory path.
    pub fn user_dir(&self) -> &Path {
        &self.user_dir
    }

    /// Ensure the user templates directory exists.
    pub fn ensure_user_dir(&self) -> Result<(), PersonalityExtensionError> {
        if !self.user_dir.exists() {
            std::fs::create_dir_all(&self.user_dir).map_err(|e| {
                TemplateError::io_error(self.user_dir.display().to_string(), e)
            })?;
            log::info!("Created user templates directory: {}", self.user_dir.display());
        }
        Ok(())
    }

    // ========================================================================
    // Loading Methods
    // ========================================================================

    /// Load all built-in templates from the assets directory.
    pub async fn load_builtin_templates(&self) -> Result<TemplateLoadResult, PersonalityExtensionError> {
        self.load_from_directory(&self.builtin_dir, true).await
    }

    /// Load all user templates from the user directory.
    pub async fn load_user_templates(&self) -> Result<TemplateLoadResult, PersonalityExtensionError> {
        // Ensure directory exists
        self.ensure_user_dir()?;
        self.load_from_directory(&self.user_dir, false).await
    }

    /// Load templates from a specific directory.
    ///
    /// # Arguments
    /// * `dir` - Directory to load from
    /// * `mark_builtin` - Whether to mark loaded templates as built-in
    pub async fn load_from_directory(
        &self,
        dir: &Path,
        mark_builtin: bool,
    ) -> Result<TemplateLoadResult, PersonalityExtensionError> {
        let mut result = TemplateLoadResult::new(dir.display().to_string());

        if !dir.exists() {
            log::warn!("Template directory does not exist: {}", dir.display());
            return Ok(result);
        }

        if !dir.is_dir() {
            return Err(PersonalityExtensionError::internal(format!(
                "Path is not a directory: {}",
                dir.display()
            )));
        }

        // Find all YAML files
        let entries = std::fs::read_dir(dir).map_err(|e| {
            TemplateError::io_error(dir.display().to_string(), e)
        })?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    result.errors.push(LoadError {
                        file_path: dir.display().to_string(),
                        message: format!("Failed to read directory entry: {}", e),
                        line: None,
                        kind: LoadErrorKind::IoError,
                    });
                    continue;
                }
            };

            let path = entry.path();

            // Skip non-YAML files
            if !Self::is_yaml_file(&path) {
                continue;
            }

            result.files_processed += 1;

            // Load and validate the template
            match self.load_single_file(&path, mark_builtin).await {
                Ok(template) => {
                    log::debug!(
                        "Loaded template '{}' from {}",
                        template.name,
                        path.display()
                    );
                    result.templates.push(template);
                }
                Err(e) => {
                    let (message, line, kind) = Self::categorize_error(&e);
                    result.errors.push(LoadError {
                        file_path: path.display().to_string(),
                        message,
                        line,
                        kind,
                    });
                    log::warn!(
                        "Failed to load template from {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        log::info!(
            "Loaded {} templates from {}, {} errors",
            result.templates_loaded(),
            dir.display(),
            result.error_count()
        );

        Ok(result)
    }

    /// Load a single template from a YAML file.
    pub async fn load_single_file(
        &self,
        path: &Path,
        mark_builtin: bool,
    ) -> Result<SettingTemplate, PersonalityExtensionError> {
        // Read file contents
        let content = std::fs::read_to_string(path).map_err(|e| {
            TemplateError::io_error(path.display().to_string(), e)
        })?;

        // Parse YAML
        self.parse_yaml_content(&content, path, mark_builtin)
    }

    /// Parse YAML content into a template.
    pub fn parse_yaml_content(
        &self,
        content: &str,
        source_path: &Path,
        mark_builtin: bool,
    ) -> Result<SettingTemplate, PersonalityExtensionError> {
        // Parse the YAML
        let yaml: TemplateYaml = serde_yaml_ng::from_str(content).map_err(|e| {
            let line = e.location().map(|loc| loc.line());
            TemplateError::ParseError {
                file: source_path.display().to_string(),
                line: line.unwrap_or(0),
                message: e.to_string(),
                source: Some(Box::new(e)),
            }
        })?;

        // Convert to SettingTemplate
        let mut template: SettingTemplate = yaml.try_into()?;

        // Update vocabulary keys
        template.update_vocabulary_keys();

        // Mark as built-in if requested
        if mark_builtin {
            template.mark_builtin();
        }

        // Validate the template
        template.validate_with_config(&self.validation_config)?;

        Ok(template)
    }

    /// Check if a path is a YAML file.
    ///
    /// For actual file loading, checks both extension and that it's a real file.
    /// For testing extension matching only, use `has_yaml_extension`.
    fn is_yaml_file(path: &Path) -> bool {
        path.is_file() && Self::has_yaml_extension(path)
    }

    /// Check if a path has a YAML extension (without checking if file exists).
    fn has_yaml_extension(path: &Path) -> bool {
        path.extension()
            .map(|ext| ext == "yaml" || ext == "yml")
            .unwrap_or(false)
    }

    /// Categorize an error for reporting.
    fn categorize_error(e: &PersonalityExtensionError) -> (String, Option<usize>, LoadErrorKind) {
        match e {
            PersonalityExtensionError::Template(TemplateError::IoError { message, .. }) => {
                (message.clone(), None, LoadErrorKind::IoError)
            }
            PersonalityExtensionError::Template(TemplateError::ParseError {
                line, message, ..
            }) => (message.clone(), Some(*line), LoadErrorKind::ParseError),
            PersonalityExtensionError::Template(TemplateError::ValidationError {
                message, ..
            }) => (message.clone(), None, LoadErrorKind::ValidationError),
            _ => (e.to_string(), None, LoadErrorKind::Unknown),
        }
    }

    // ========================================================================
    // Export Methods
    // ========================================================================

    /// Export a template to YAML string.
    pub fn export_to_yaml(&self, template: &SettingTemplate) -> Result<String, PersonalityExtensionError> {
        let yaml: TemplateYaml = template.clone().into();
        serde_yaml_ng::to_string(&yaml).map_err(|e| {
            PersonalityExtensionError::internal(format!("Failed to serialize template to YAML: {}", e))
        })
    }

    /// Export a template to a YAML file.
    pub async fn export_to_file(
        &self,
        template: &SettingTemplate,
        path: &Path,
    ) -> Result<(), PersonalityExtensionError> {
        let yaml = self.export_to_yaml(template)?;

        std::fs::write(path, yaml).map_err(|e| {
            TemplateError::io_error(path.display().to_string(), e)
        })?;

        log::info!("Exported template '{}' to {}", template.name, path.display());
        Ok(())
    }

    /// Save a template to the user templates directory.
    pub async fn save_user_template(
        &self,
        template: &SettingTemplate,
    ) -> Result<PathBuf, PersonalityExtensionError> {
        self.ensure_user_dir()?;

        // Generate filename from template ID
        let filename = format!("{}.yaml", template.id);
        let path = self.user_dir.join(&filename);

        self.export_to_file(template, &path).await?;

        Ok(path)
    }

    /// Delete a user template file.
    pub async fn delete_user_template(&self, id: &TemplateId) -> Result<(), PersonalityExtensionError> {
        let filename = format!("{}.yaml", id);
        let path = self.user_dir.join(&filename);

        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| {
                TemplateError::io_error(path.display().to_string(), e)
            })?;
            log::info!("Deleted user template file: {}", path.display());
        } else {
            log::warn!("Template file not found: {}", path.display());
        }

        Ok(())
    }

    // ========================================================================
    // Import Methods
    // ========================================================================

    /// Import a template from YAML string.
    pub fn import_from_yaml(&self, yaml: &str) -> Result<SettingTemplate, PersonalityExtensionError> {
        let template_yaml: TemplateYaml = serde_yaml_ng::from_str(yaml).map_err(|e| {
            TemplateError::ParseError {
                file: "<string>".to_string(),
                line: e.location().map(|loc| loc.line()).unwrap_or(0),
                message: e.to_string(),
                source: Some(Box::new(e)),
            }
        })?;

        let mut template: SettingTemplate = template_yaml.try_into()?;
        template.update_vocabulary_keys();
        template.validate_with_config(&self.validation_config)?;

        Ok(template)
    }

    /// Import a template from YAML, generating a new ID to avoid conflicts.
    pub fn import_from_yaml_new_id(&self, yaml: &str) -> Result<SettingTemplate, PersonalityExtensionError> {
        let mut template = self.import_from_yaml(yaml)?;
        template.id = TemplateId::generate();
        template.touch();
        Ok(template)
    }

    /// Check if a template with the given name already exists.
    pub async fn check_duplicate_name(
        &self,
        name: &str,
    ) -> Result<bool, PersonalityExtensionError> {
        // Load user templates and check for duplicate
        let result = self.load_user_templates().await?;
        Ok(result.templates.iter().any(|t| t.name == name))
    }
}

impl Default for TemplateLoader {
    fn default() -> Self {
        match Self::new() {
            Ok(loader) => loader,
            Err(e) => {
                log::warn!(
                    "Failed to create TemplateLoader with default paths: {}. \
                     Using fallback paths.",
                    e
                );
                // Fallback to current directory-based paths
                Self::with_directories(
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join("assets/settings"),
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join("user_templates"),
                )
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_yaml() -> &'static str {
        r#"id: test_template
name: Test Template
description: A test template for unit testing purposes.
game_system: dnd5e
setting_name: Test Setting
base_profile: storyteller
vocabulary:
  ancient texts: 0.05
  arcane knowledge: 0.04
  mystical arts: 0.03
  eldritch power: 0.02
  tome of lore: 0.03
  the Weave: 0.05
  divine providence: 0.02
  magical ward: 0.03
  enchanted scroll: 0.04
  ritual circle: 0.02
common_phrases:
  - As the ancient texts foretell
  - By the power of the arcane
  - The mystical arts reveal
  - Behold the eldritch truth
  - As written in the tome
deity_references:
  - Mystra
  - Oghma
tags:
  - test
  - magic
tone_overrides:
  scholarly: 0.8
  mysterious: 0.6
cultural_markers:
  - References ancient scrolls
  - Uses archaic language
"#
    }

    fn minimal_yaml() -> &'static str {
        r#"id: minimal
name: Minimal Template
description: A minimal test template.
base_profile: default
vocabulary:
  term1: 0.1
  term2: 0.1
  term3: 0.1
  term4: 0.1
  term5: 0.1
  term6: 0.1
  term7: 0.1
  term8: 0.1
  term9: 0.1
  term10: 0.1
"#
    }

    fn invalid_yaml() -> &'static str {
        r#"id: invalid
name:
base_profile: default
"#
    }

    #[test]
    fn test_import_from_yaml() {
        let loader = TemplateLoader::with_directories("/tmp/builtin", "/tmp/user")
            .with_validation_config(TemplateValidationConfig::lenient());

        let template = loader.import_from_yaml(sample_yaml()).unwrap();

        assert_eq!(template.id.as_str(), "test_template");
        assert_eq!(template.name, "Test Template");
        assert_eq!(template.game_system, Some("dnd5e".to_string()));
        assert_eq!(template.vocabulary.len(), 10);
        assert_eq!(template.common_phrases.len(), 5);
        assert_eq!(template.deity_references.len(), 2);
    }

    #[test]
    fn test_import_minimal_yaml() {
        let loader = TemplateLoader::with_directories("/tmp/builtin", "/tmp/user")
            .with_validation_config(TemplateValidationConfig::minimal());

        let template = loader.import_from_yaml(minimal_yaml()).unwrap();

        assert_eq!(template.name, "Minimal Template");
        assert_eq!(template.vocabulary.len(), 10);
    }

    #[test]
    fn test_import_invalid_yaml() {
        let loader = TemplateLoader::with_directories("/tmp/builtin", "/tmp/user");

        let result = loader.import_from_yaml(invalid_yaml());
        assert!(result.is_err());
    }

    #[test]
    fn test_export_to_yaml() {
        let loader = TemplateLoader::with_directories("/tmp/builtin", "/tmp/user")
            .with_validation_config(TemplateValidationConfig::lenient());

        let original = loader.import_from_yaml(sample_yaml()).unwrap();
        let yaml = loader.export_to_yaml(&original).unwrap();

        // Should be valid YAML
        assert!(yaml.contains("name: Test Template"));
        assert!(yaml.contains("game_system: dnd5e"));

        // Roundtrip should work
        let reimported = loader.import_from_yaml(&yaml).unwrap();
        assert_eq!(reimported.name, original.name);
    }

    #[test]
    fn test_import_with_new_id() {
        let loader = TemplateLoader::with_directories("/tmp/builtin", "/tmp/user")
            .with_validation_config(TemplateValidationConfig::lenient());

        let template1 = loader.import_from_yaml(sample_yaml()).unwrap();
        let template2 = loader.import_from_yaml_new_id(sample_yaml()).unwrap();

        assert_eq!(template1.id.as_str(), "test_template");
        assert_ne!(template2.id.as_str(), "test_template");
        assert_eq!(template1.name, template2.name);
    }

    #[tokio::test]
    async fn test_load_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let templates_path = temp_dir.path();

        // Write a valid template
        std::fs::write(
            templates_path.join("valid.yaml"),
            sample_yaml(),
        ).unwrap();

        // Write an invalid template
        std::fs::write(
            templates_path.join("invalid.yaml"),
            invalid_yaml(),
        ).unwrap();

        // Write a non-yaml file (should be skipped)
        std::fs::write(
            templates_path.join("readme.txt"),
            "This is not a template",
        ).unwrap();

        let loader = TemplateLoader::with_directories(templates_path, "/tmp/user")
            .with_validation_config(TemplateValidationConfig::lenient());

        let result = loader.load_from_directory(templates_path, false).await.unwrap();

        // Should have processed 2 YAML files
        assert_eq!(result.files_processed, 2);
        // One should have loaded successfully
        assert_eq!(result.templates_loaded(), 1);
        // One should have failed
        assert_eq!(result.error_count(), 1);
    }

    #[tokio::test]
    async fn test_load_nonexistent_directory() {
        let loader = TemplateLoader::with_directories(
            "/nonexistent/builtin",
            "/nonexistent/user",
        );

        let result = loader
            .load_from_directory(Path::new("/nonexistent/dir"), false)
            .await
            .unwrap();

        assert!(result.templates.is_empty());
        assert_eq!(result.files_processed, 0);
    }

    #[tokio::test]
    async fn test_save_and_load_user_template() {
        let temp_dir = TempDir::new().unwrap();
        let user_path = temp_dir.path();

        let loader = TemplateLoader::with_directories("/tmp/builtin", user_path)
            .with_validation_config(TemplateValidationConfig::lenient());

        // Import a template
        let template = loader.import_from_yaml(sample_yaml()).unwrap();

        // Save to user directory
        let saved_path = loader.save_user_template(&template).await.unwrap();
        assert!(saved_path.exists());

        // Load from user directory
        let result = loader.load_user_templates().await.unwrap();
        assert_eq!(result.templates_loaded(), 1);
        assert_eq!(result.templates[0].name, "Test Template");

        // Delete the template
        loader.delete_user_template(&template.id).await.unwrap();
        assert!(!saved_path.exists());
    }

    #[test]
    fn test_has_yaml_extension() {
        assert!(TemplateLoader::has_yaml_extension(Path::new("/path/to/template.yaml")));
        assert!(TemplateLoader::has_yaml_extension(Path::new("/path/to/template.yml")));
        assert!(!TemplateLoader::has_yaml_extension(Path::new("/path/to/template.json")));
        assert!(!TemplateLoader::has_yaml_extension(Path::new("/path/to/template.txt")));
    }

    #[test]
    fn test_load_error_serialization() {
        let error = LoadError {
            file_path: "/path/to/file.yaml".to_string(),
            message: "Test error".to_string(),
            line: Some(42),
            kind: LoadErrorKind::ParseError,
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"filePath\""));
        assert!(json.contains("\"line\":42"));
        assert!(json.contains("\"kind\":\"parse_error\""));
    }

    #[test]
    fn test_template_load_result() {
        let mut result = TemplateLoadResult::new("/test/path");

        assert!(result.is_success());
        assert_eq!(result.templates_loaded(), 0);
        assert_eq!(result.error_count(), 0);

        result.errors.push(LoadError {
            file_path: "test.yaml".to_string(),
            message: "error".to_string(),
            line: None,
            kind: LoadErrorKind::IoError,
        });

        assert!(!result.is_success());
        assert_eq!(result.error_count(), 1);
    }
}
