use std::path::{Path, PathBuf};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

const PIPER_HF_BASE: &str = "https://huggingface.co/rhasspy/piper-voices/resolve/main";
const PIPER_VOICES_JSON: &str = "https://huggingface.co/rhasspy/piper-voices/raw/main/voices.json";

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Voice not found: {0}")]
    VoiceNotFound(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Download canceled")]
    Canceled,
}

pub type DownloadResult<T> = std::result::Result<T, DownloadError>;

/// Available Piper voice from the Hugging Face repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailablePiperVoice {
    pub key: String,
    pub name: String,
    pub language: PiperLanguage,
    pub quality: String,
    pub num_speakers: u32,
    pub sample_rate: u32,
    pub files: PiperVoiceFiles,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperLanguage {
    pub code: String,
    pub family: String,
    pub region: String,
    pub name_native: String,
    pub name_english: String,
    pub country_english: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperVoiceFiles {
    pub model: PiperFileInfo,
    pub config: PiperFileInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiperFileInfo {
    pub size_bytes: u64,
    pub md5_digest: String,
}

/// Download progress callback
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Parsed components of a Piper voice key
#[derive(Debug, Clone)]
struct ParsedVoiceKey {
    lang_region: String,  // e.g., "en_US"
    voice_name: String,   // e.g., "lessac"
    quality: String,      // e.g., "medium"
}

impl ParsedVoiceKey {
    /// Parse a voice key like "en_US-lessac-medium"
    fn parse(voice_key: &str, default_quality: Option<&str>) -> Result<Self, DownloadError> {
        let parts: Vec<&str> = voice_key.split('-').collect();
        if parts.len() < 2 {
            return Err(DownloadError::VoiceNotFound(voice_key.to_string()));
        }

        let quality = default_quality
            .or_else(|| parts.get(2).filter(|s| !s.is_empty()).copied())
            .unwrap_or("medium")
            .to_string();

        Ok(Self {
            lang_region: parts[0].to_string(),
            voice_name: parts[1].to_string(),
            quality,
        })
    }

    /// Get the language code (e.g., "en" from "en_US")
    fn lang_code(&self) -> &str {
        self.lang_region.split('_').next().unwrap_or("en")
    }

    /// Construct the HuggingFace path for this voice
    fn hf_base_path(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.lang_code(),
            self.lang_region,
            self.voice_name,
            self.quality
        )
    }

    /// Get the model filename (e.g., "en_US-lessac-medium.onnx")
    fn model_filename(&self) -> String {
        format!("{}-{}-{}.onnx", self.lang_region, self.voice_name, self.quality)
    }

    /// Get the config filename (e.g., "en_US-lessac-medium.onnx.json")
    fn config_filename(&self) -> String {
        format!("{}.json", self.model_filename())
    }
}

/// Voice downloader for Piper models from Hugging Face
pub struct VoiceDownloader {
    client: Client,
    models_dir: PathBuf,
}

impl VoiceDownloader {
    pub fn new(models_dir: PathBuf) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .expect("Failed to create HTTP client"),
            models_dir,
        }
    }

    /// List all available Piper voices from Hugging Face
    pub async fn list_available_voices(&self) -> DownloadResult<Vec<AvailablePiperVoice>> {
        info!("Fetching available Piper voices from Hugging Face");

        let response = self.client.get(PIPER_VOICES_JSON).send().await?;

        if !response.status().is_success() {
            return Err(DownloadError::Network(
                response.error_for_status().unwrap_err()
            ));
        }

        let json: serde_json::Value = response.json().await?;

        let mut voices = Vec::new();

        // Parse the voices.json structure
        if let Some(obj) = json.as_object() {
            for (key, value) in obj {
                if let Ok(voice) = self.parse_voice_entry(key, value) {
                    voices.push(voice);
                }
            }
        }

        info!(count = voices.len(), "Found available Piper voices");
        Ok(voices)
    }

    fn parse_voice_entry(&self, key: &str, value: &serde_json::Value) -> DownloadResult<AvailablePiperVoice> {
        let language = value.get("language")
            .ok_or_else(|| DownloadError::Parse("Missing language".to_string()))?;

        let files = value.get("files")
            .and_then(|f| f.as_object())
            .ok_or_else(|| DownloadError::Parse("Missing files".to_string()))?;

        // Get the first available quality variant
        let (quality, file_info) = files.iter().next()
            .ok_or_else(|| DownloadError::Parse("No quality variants".to_string()))?;

        let model_file = file_info.get("model")
            .ok_or_else(|| DownloadError::Parse("Missing model file info".to_string()))?;
        let config_file = file_info.get("config")
            .ok_or_else(|| DownloadError::Parse("Missing config file info".to_string()))?;

        Ok(AvailablePiperVoice {
            key: key.to_string(),
            name: value.get("name")
                .and_then(|n| n.as_str())
                .unwrap_or(key)
                .to_string(),
            language: PiperLanguage {
                code: language.get("code").and_then(|c| c.as_str()).unwrap_or("").to_string(),
                family: language.get("family").and_then(|f| f.as_str()).unwrap_or("").to_string(),
                region: language.get("region").and_then(|r| r.as_str()).unwrap_or("").to_string(),
                name_native: language.get("name_native").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                name_english: language.get("name_english").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                country_english: language.get("country_english").and_then(|c| c.as_str()).unwrap_or("").to_string(),
            },
            quality: quality.clone(),
            num_speakers: value.get("num_speakers").and_then(|n| n.as_u64()).unwrap_or(1) as u32,
            sample_rate: file_info.get("sample_rate").and_then(|s| s.as_u64()).unwrap_or(22050) as u32,
            files: PiperVoiceFiles {
                model: PiperFileInfo {
                    size_bytes: model_file.get("size_bytes").and_then(|s| s.as_u64()).unwrap_or(0),
                    md5_digest: model_file.get("md5_digest").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                },
                config: PiperFileInfo {
                    size_bytes: config_file.get("size_bytes").and_then(|s| s.as_u64()).unwrap_or(0),
                    md5_digest: config_file.get("md5_digest").and_then(|m| m.as_str()).unwrap_or("").to_string(),
                },
            },
        })
    }

    /// Download a Piper voice by key (e.g., "en_US-lessac-medium")
    pub async fn download_voice(
        &self,
        voice_key: &str,
        quality: Option<&str>,
        progress: Option<ProgressCallback>,
    ) -> DownloadResult<PathBuf> {
        info!(voice = voice_key, "Downloading Piper voice");

        // Ensure models directory exists
        tokio::fs::create_dir_all(&self.models_dir).await?;

        // Parse voice key using helper struct
        let parsed = ParsedVoiceKey::parse(voice_key, quality)?;

        // Construct URLs using parsed components
        let model_url = format!(
            "{}/{}/{}",
            PIPER_HF_BASE,
            parsed.hf_base_path(),
            parsed.model_filename()
        );
        let config_url = format!(
            "{}/{}/{}",
            PIPER_HF_BASE,
            parsed.hf_base_path(),
            parsed.config_filename()
        );

        let model_path = self.models_dir.join(parsed.model_filename());
        let config_path = self.models_dir.join(parsed.config_filename());

        // Download model file
        debug!(url = %model_url, "Downloading model file");
        self.download_file(&model_url, &model_path, progress.as_ref()).await?;

        // Download config file
        debug!(url = %config_url, "Downloading config file");
        self.download_file(&config_url, &config_path, None).await?;

        info!(path = ?model_path, "Voice download complete");
        Ok(model_path)
    }

    async fn download_file(
        &self,
        url: &str,
        dest: &Path,
        progress: Option<&ProgressCallback>,
    ) -> DownloadResult<()> {
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(DownloadError::Network(
                response.error_for_status().unwrap_err()
            ));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;

        let mut file = tokio::fs::File::create(dest).await?;
        let mut stream = response.bytes_stream();

        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if let Some(ref cb) = progress {
                cb(downloaded, total_size);
            }
        }

        file.flush().await?;
        Ok(())
    }

    /// Check if a voice is already downloaded
    pub fn is_voice_downloaded(&self, voice_key: &str) -> bool {
        match ParsedVoiceKey::parse(voice_key, None) {
            Ok(parsed) => {
                let model_path = self.models_dir.join(parsed.model_filename());
                let config_path = self.models_dir.join(parsed.config_filename());
                model_path.exists() && config_path.exists()
            }
            Err(_) => false,
        }
    }

    /// Delete a downloaded voice
    pub async fn delete_voice(&self, voice_key: &str) -> DownloadResult<()> {
        let parsed = ParsedVoiceKey::parse(voice_key, None)?;

        let model_path = self.models_dir.join(parsed.model_filename());
        let config_path = self.models_dir.join(parsed.config_filename());

        if model_path.exists() {
            tokio::fs::remove_file(&model_path).await?;
        }
        if config_path.exists() {
            tokio::fs::remove_file(&config_path).await?;
        }

        info!(voice = voice_key, "Deleted voice files");
        Ok(())
    }

    /// Get the models directory
    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }
}

/// Popular pre-defined Piper voices for quick access
pub fn popular_piper_voices() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("en_US-lessac-medium", "Lessac (US English)", "Medium quality, natural sounding"),
        ("en_US-amy-medium", "Amy (US English)", "Female voice, clear enunciation"),
        ("en_US-ryan-medium", "Ryan (US English)", "Male voice, warm tone"),
        ("en_GB-alan-medium", "Alan (British English)", "British male, professional"),
        ("en_GB-alba-medium", "Alba (British English)", "British female, clear"),
        ("de_DE-thorsten-medium", "Thorsten (German)", "German male, natural"),
        ("fr_FR-upmc-medium", "UPMC (French)", "French voice, standard"),
        ("es_ES-davefx-medium", "Davefx (Spanish)", "Spanish male voice"),
        ("it_IT-riccardo-medium", "Riccardo (Italian)", "Italian male voice"),
        ("pl_PL-gosia-medium", "Gosia (Polish)", "Polish female voice"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // =========================================================================
    // Unit Tests: ParsedVoiceKey
    // =========================================================================

    mod parsed_voice_key {
        use super::*;

        #[test]
        fn parse_full_voice_key() {
            let parsed = ParsedVoiceKey::parse("en_US-lessac-medium", None).unwrap();
            assert_eq!(parsed.lang_region, "en_US");
            assert_eq!(parsed.voice_name, "lessac");
            assert_eq!(parsed.quality, "medium");
        }

        #[test]
        fn parse_voice_key_without_quality_defaults_to_medium() {
            let parsed = ParsedVoiceKey::parse("en_US-lessac", None).unwrap();
            assert_eq!(parsed.lang_region, "en_US");
            assert_eq!(parsed.voice_name, "lessac");
            assert_eq!(parsed.quality, "medium");
        }

        #[test]
        fn parse_voice_key_with_override_quality() {
            let parsed = ParsedVoiceKey::parse("en_US-lessac-low", Some("high")).unwrap();
            assert_eq!(parsed.quality, "high");
        }

        #[test]
        fn parse_voice_key_with_empty_quality_defaults_to_medium() {
            // Simulates a key like "en_US-lessac-" where quality part is empty
            let parsed = ParsedVoiceKey::parse("en_US-lessac-", None).unwrap();
            assert_eq!(parsed.quality, "medium");
        }

        #[test]
        fn parse_invalid_voice_key_too_short() {
            let result = ParsedVoiceKey::parse("invalid", None);
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), DownloadError::VoiceNotFound(_)));
        }

        #[test]
        fn parse_empty_voice_key() {
            let result = ParsedVoiceKey::parse("", None);
            assert!(result.is_err());
        }

        #[test]
        fn lang_code_extracts_correctly() {
            let parsed = ParsedVoiceKey::parse("en_US-lessac-medium", None).unwrap();
            assert_eq!(parsed.lang_code(), "en");

            let parsed_de = ParsedVoiceKey::parse("de_DE-thorsten-high", None).unwrap();
            assert_eq!(parsed_de.lang_code(), "de");
        }

        #[test]
        fn hf_base_path_constructs_correctly() {
            let parsed = ParsedVoiceKey::parse("en_US-lessac-medium", None).unwrap();
            assert_eq!(parsed.hf_base_path(), "en/en_US/lessac/medium");
        }

        #[test]
        fn model_filename_constructs_correctly() {
            let parsed = ParsedVoiceKey::parse("en_US-lessac-medium", None).unwrap();
            assert_eq!(parsed.model_filename(), "en_US-lessac-medium.onnx");
        }

        #[test]
        fn config_filename_constructs_correctly() {
            let parsed = ParsedVoiceKey::parse("en_US-lessac-medium", None).unwrap();
            assert_eq!(parsed.config_filename(), "en_US-lessac-medium.onnx.json");
        }

        #[test]
        fn parse_various_languages() {
            let test_cases = vec![
                ("fr_FR-upmc-medium", "fr_FR", "upmc", "medium"),
                ("de_DE-thorsten-high", "de_DE", "thorsten", "high"),
                ("ja_JP-voice-low", "ja_JP", "voice", "low"),
                ("zh_CN-speaker-x_low", "zh_CN", "speaker", "x_low"),
            ];

            for (key, expected_region, expected_name, expected_quality) in test_cases {
                let parsed = ParsedVoiceKey::parse(key, None).unwrap();
                assert_eq!(parsed.lang_region, expected_region, "Failed for key: {}", key);
                assert_eq!(parsed.voice_name, expected_name, "Failed for key: {}", key);
                assert_eq!(parsed.quality, expected_quality, "Failed for key: {}", key);
            }
        }
    }

    // =========================================================================
    // Unit Tests: VoiceDownloader
    // =========================================================================

    mod voice_downloader {
        use super::*;

        #[test]
        fn new_creates_downloader_with_correct_path() {
            let dir = PathBuf::from("/tmp/test-voices");
            let downloader = VoiceDownloader::new(dir.clone());
            assert_eq!(downloader.models_dir(), dir);
        }

        #[test]
        fn is_voice_downloaded_returns_false_for_missing_files() {
            let temp_dir = TempDir::new().unwrap();
            let downloader = VoiceDownloader::new(temp_dir.path().to_path_buf());

            assert!(!downloader.is_voice_downloaded("en_US-lessac-medium"));
        }

        #[test]
        fn is_voice_downloaded_returns_true_when_files_exist() {
            let temp_dir = TempDir::new().unwrap();
            let downloader = VoiceDownloader::new(temp_dir.path().to_path_buf());

            // Create mock model and config files
            let model_path = temp_dir.path().join("en_US-lessac-medium.onnx");
            let config_path = temp_dir.path().join("en_US-lessac-medium.onnx.json");
            std::fs::write(&model_path, b"mock model").unwrap();
            std::fs::write(&config_path, b"{}").unwrap();

            assert!(downloader.is_voice_downloaded("en_US-lessac-medium"));
        }

        #[test]
        fn is_voice_downloaded_returns_false_for_partial_files() {
            let temp_dir = TempDir::new().unwrap();
            let downloader = VoiceDownloader::new(temp_dir.path().to_path_buf());

            // Only create model file, not config
            let model_path = temp_dir.path().join("en_US-lessac-medium.onnx");
            std::fs::write(&model_path, b"mock model").unwrap();

            assert!(!downloader.is_voice_downloaded("en_US-lessac-medium"));
        }

        #[test]
        fn is_voice_downloaded_returns_false_for_invalid_key() {
            let temp_dir = TempDir::new().unwrap();
            let downloader = VoiceDownloader::new(temp_dir.path().to_path_buf());

            assert!(!downloader.is_voice_downloaded("invalid"));
        }
    }

    // =========================================================================
    // Unit Tests: popular_piper_voices
    // =========================================================================

    mod popular_voices {
        use super::*;

        #[test]
        fn returns_non_empty_list() {
            let voices = popular_piper_voices();
            assert!(!voices.is_empty());
        }

        #[test]
        fn all_voices_have_valid_keys() {
            let voices = popular_piper_voices();
            for (key, name, desc) in &voices {
                // All keys should be parseable
                let result = ParsedVoiceKey::parse(key, None);
                assert!(result.is_ok(), "Invalid key: {} ({})", key, name);

                // Name and description should not be empty
                assert!(!name.is_empty(), "Empty name for key: {}", key);
                assert!(!desc.is_empty(), "Empty description for key: {}", key);
            }
        }

        #[test]
        fn contains_english_voices() {
            let voices = popular_piper_voices();
            let english_count = voices.iter()
                .filter(|(key, _, _)| key.starts_with("en_"))
                .count();
            assert!(english_count >= 2, "Should have at least 2 English voices");
        }
    }

    // =========================================================================
    // Integration Tests: VoiceDownloader
    // =========================================================================

    mod integration {
        use super::*;

        #[tokio::test]
        async fn delete_voice_removes_files() {
            let temp_dir = TempDir::new().unwrap();
            let downloader = VoiceDownloader::new(temp_dir.path().to_path_buf());

            // Create mock files
            let model_path = temp_dir.path().join("en_US-lessac-medium.onnx");
            let config_path = temp_dir.path().join("en_US-lessac-medium.onnx.json");
            std::fs::write(&model_path, b"mock model").unwrap();
            std::fs::write(&config_path, b"{}").unwrap();

            assert!(model_path.exists());
            assert!(config_path.exists());

            // Delete the voice
            downloader.delete_voice("en_US-lessac-medium").await.unwrap();

            assert!(!model_path.exists());
            assert!(!config_path.exists());
        }

        #[tokio::test]
        async fn delete_voice_handles_missing_files_gracefully() {
            let temp_dir = TempDir::new().unwrap();
            let downloader = VoiceDownloader::new(temp_dir.path().to_path_buf());

            // Should not error even if files don't exist
            let result = downloader.delete_voice("en_US-nonexistent-medium").await;
            assert!(result.is_ok());
        }

        // Note: Actual download tests are marked as ignored since they require network
        #[tokio::test]
        #[ignore = "requires network access to Hugging Face"]
        async fn download_voice_from_huggingface() {
            let temp_dir = TempDir::new().unwrap();
            let downloader = VoiceDownloader::new(temp_dir.path().to_path_buf());

            let result = downloader.download_voice("en_US-lessac-medium", None, None).await;
            assert!(result.is_ok());

            let model_path = result.unwrap();
            assert!(model_path.exists());
        }

        #[tokio::test]
        #[ignore = "requires network access to Hugging Face"]
        async fn list_available_voices_returns_voices() {
            let temp_dir = TempDir::new().unwrap();
            let downloader = VoiceDownloader::new(temp_dir.path().to_path_buf());

            let result = downloader.list_available_voices().await;
            assert!(result.is_ok());

            let voices = result.unwrap();
            assert!(!voices.is_empty(), "Should return at least some voices");
        }
    }
}

// =========================================================================
// Property-Based Tests
// =========================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating valid language regions
    fn lang_region_strategy() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "en_US", "en_GB", "de_DE", "fr_FR", "es_ES", "it_IT",
            "pl_PL", "pt_BR", "ru_RU", "zh_CN", "ja_JP", "ko_KR",
        ]).prop_map(String::from)
    }

    // Strategy for generating voice names
    fn voice_name_strategy() -> impl Strategy<Value = String> {
        "[a-z]{3,12}".prop_map(String::from)
    }

    // Strategy for generating quality levels
    fn quality_strategy() -> impl Strategy<Value = String> {
        prop::sample::select(vec!["x_low", "low", "medium", "high"])
            .prop_map(String::from)
    }

    proptest! {
        /// Any valid voice key should parse successfully
        #[test]
        fn parse_valid_voice_keys(
            lang_region in lang_region_strategy(),
            voice_name in voice_name_strategy(),
            quality in quality_strategy()
        ) {
            let voice_key = format!("{}-{}-{}", lang_region, voice_name, quality);
            let result = ParsedVoiceKey::parse(&voice_key, None);

            prop_assert!(result.is_ok(), "Failed to parse valid key: {}", voice_key);

            let parsed = result.unwrap();
            prop_assert_eq!(parsed.lang_region, lang_region);
            prop_assert_eq!(parsed.voice_name, voice_name);
            prop_assert_eq!(parsed.quality, quality);
        }

        /// Voice keys without quality should default to medium
        #[test]
        fn parse_keys_without_quality_default_to_medium(
            lang_region in lang_region_strategy(),
            voice_name in voice_name_strategy()
        ) {
            let voice_key = format!("{}-{}", lang_region, voice_name);
            let result = ParsedVoiceKey::parse(&voice_key, None);

            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap().quality, "medium");
        }

        /// Override quality should always be used when provided
        #[test]
        fn override_quality_takes_precedence(
            lang_region in lang_region_strategy(),
            voice_name in voice_name_strategy(),
            original_quality in quality_strategy(),
            override_quality in quality_strategy()
        ) {
            let voice_key = format!("{}-{}-{}", lang_region, voice_name, original_quality);
            let result = ParsedVoiceKey::parse(&voice_key, Some(&override_quality));

            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap().quality, override_quality);
        }

        /// Model filename should always end with .onnx
        #[test]
        fn model_filename_ends_with_onnx(
            lang_region in lang_region_strategy(),
            voice_name in voice_name_strategy(),
            quality in quality_strategy()
        ) {
            let voice_key = format!("{}-{}-{}", lang_region, voice_name, quality);
            let parsed = ParsedVoiceKey::parse(&voice_key, None).unwrap();

            prop_assert!(parsed.model_filename().ends_with(".onnx"));
        }

        /// Config filename should always end with .onnx.json
        #[test]
        fn config_filename_ends_with_onnx_json(
            lang_region in lang_region_strategy(),
            voice_name in voice_name_strategy(),
            quality in quality_strategy()
        ) {
            let voice_key = format!("{}-{}-{}", lang_region, voice_name, quality);
            let parsed = ParsedVoiceKey::parse(&voice_key, None).unwrap();

            prop_assert!(parsed.config_filename().ends_with(".onnx.json"));
        }

        /// HuggingFace path should contain all components
        #[test]
        fn hf_path_contains_all_components(
            lang_region in lang_region_strategy(),
            voice_name in voice_name_strategy(),
            quality in quality_strategy()
        ) {
            let voice_key = format!("{}-{}-{}", lang_region, voice_name, quality);
            let parsed = ParsedVoiceKey::parse(&voice_key, None).unwrap();
            let hf_path = parsed.hf_base_path();

            prop_assert!(hf_path.contains(&lang_region));
            prop_assert!(hf_path.contains(&voice_name));
            prop_assert!(hf_path.contains(&quality));
        }

        /// Single-part strings should always fail to parse
        #[test]
        fn single_part_keys_fail(s in "[a-zA-Z0-9_]+") {
            // Only test strings without hyphens
            if !s.contains('-') {
                let result = ParsedVoiceKey::parse(&s, None);
                prop_assert!(result.is_err());
            }
        }
    }
}
