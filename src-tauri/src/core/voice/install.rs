//! Voice provider installation and setup
//!
//! Handles checking installation status and providing installation
//! instructions/automation for various TTS providers.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;
use tracing::{info, warn};

use super::download::{VoiceDownloader, DownloadError, AvailablePiperVoice};
use super::types::VoiceProviderType;

/// Maximum length for version strings to avoid excessively long output
const MAX_VERSION_LENGTH: usize = 50;

#[derive(Error, Debug)]
pub enum InstallError {
    #[error("Provider not supported for automatic installation: {0}")]
    NotSupported(String),

    #[error("Installation failed: {0}")]
    InstallFailed(String),

    #[error("Command execution failed: {0}")]
    CommandFailed(#[from] std::io::Error),

    #[error("Download error: {0}")]
    Download(#[from] DownloadError),

    #[error("Provider already installed")]
    AlreadyInstalled,
}

pub type InstallResult<T> = std::result::Result<T, InstallError>;

/// Installation status for a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallStatus {
    pub provider: VoiceProviderType,
    pub installed: bool,
    pub version: Option<String>,
    pub binary_path: Option<String>,
    pub voices_available: u32,
    pub install_method: InstallMethod,
    pub install_instructions: Option<String>,
}

/// How to install a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallMethod {
    /// System package manager (pacman, apt, etc.)
    PackageManager(String),
    /// Python pip/pipx
    Python(String),
    /// Download binary
    Binary(String),
    /// Docker container
    Docker(String),
    /// Manual installation required
    Manual(String),
    /// Already managed by the app (e.g., voice downloads)
    AppManaged,
}

/// Voice provider installer
pub struct ProviderInstaller {
    models_dir: PathBuf,
}

impl ProviderInstaller {
    pub fn new(models_dir: PathBuf) -> Self {
        Self { models_dir }
    }

    /// Check installation status for a specific provider
    pub async fn check_status(&self, provider: &VoiceProviderType) -> InstallStatus {
        match provider {
            VoiceProviderType::Piper => self.check_piper().await,
            VoiceProviderType::Coqui => self.check_coqui().await,
            VoiceProviderType::Ollama => self.check_ollama().await,
            VoiceProviderType::Chatterbox => self.check_chatterbox().await,
            VoiceProviderType::GptSoVits => self.check_gpt_sovits().await,
            VoiceProviderType::XttsV2 => self.check_xtts_v2().await,
            VoiceProviderType::FishSpeech => self.check_fish_speech().await,
            VoiceProviderType::Dia => self.check_dia().await,
            _ => InstallStatus {
                provider: provider.clone(),
                installed: false,
                version: None,
                binary_path: None,
                voices_available: 0,
                install_method: InstallMethod::Manual("Cloud provider - requires API key".to_string()),
                install_instructions: Some("Configure API key in settings".to_string()),
            },
        }
    }

    /// Check all local provider installation statuses
    pub async fn check_all_local(&self) -> Vec<InstallStatus> {
        let providers = vec![
            VoiceProviderType::Piper,
            VoiceProviderType::Coqui,
            VoiceProviderType::Ollama,
            VoiceProviderType::Chatterbox,
            VoiceProviderType::GptSoVits,
            VoiceProviderType::XttsV2,
            VoiceProviderType::FishSpeech,
            VoiceProviderType::Dia,
        ];

        let mut statuses = Vec::new();
        for provider in providers {
            statuses.push(self.check_status(&provider).await);
        }
        statuses
    }

    /// Install a provider (where automatic installation is supported)
    pub async fn install(&self, provider: &VoiceProviderType) -> InstallResult<InstallStatus> {
        match provider {
            VoiceProviderType::Piper => self.install_piper().await,
            VoiceProviderType::Coqui => self.install_coqui().await,
            _ => Err(InstallError::NotSupported(format!(
                "{:?} requires manual installation",
                provider
            ))),
        }
    }

    // =========================================================================
    // Piper
    // =========================================================================

    async fn check_piper(&self) -> InstallStatus {
        let (installed, version, binary_path) = self.check_binary(&["piper", "piper-tts"]).await;
        let voices_available = self.count_piper_voices();

        InstallStatus {
            provider: VoiceProviderType::Piper,
            installed,
            version,
            binary_path,
            voices_available,
            install_method: InstallMethod::PackageManager("paru -S piper-tts-bin".to_string()),
            install_instructions: Some(if installed {
                if voices_available == 0 {
                    "Piper installed. Download voices to get started.".to_string()
                } else {
                    format!("Piper ready with {} voice(s)", voices_available)
                }
            } else {
                "Install: paru -S piper-tts-bin\nOr download from: https://github.com/rhasspy/piper/releases".to_string()
            }),
        }
    }

    fn count_piper_voices(&self) -> u32 {
        let mut count = 0;

        // Check local models dir
        count += count_onnx_files(&self.models_dir, false);

        // Check system voices (recursive)
        let system_dir = PathBuf::from("/usr/share/piper-voices");
        count += count_onnx_files(&system_dir, true);

        count
    }

    async fn install_piper(&self) -> InstallResult<InstallStatus> {
        // Check if already installed
        let status = self.check_piper().await;
        if status.installed {
            return Err(InstallError::AlreadyInstalled);
        }

        // Try to install via paru (Arch)
        info!("Attempting to install piper-tts-bin via paru");

        let output = Command::new("paru")
            .args(["-S", "--noconfirm", "piper-tts-bin"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if output.status.success() {
            info!("Piper installed successfully");
            Ok(self.check_piper().await)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(InstallError::InstallFailed(format!(
                "paru install failed: {}",
                stderr
            )))
        }
    }

    /// Download a Piper voice
    pub async fn download_piper_voice(
        &self,
        voice_key: &str,
        quality: Option<&str>,
    ) -> InstallResult<PathBuf> {
        let downloader = VoiceDownloader::new(self.models_dir.clone());
        let path = downloader.download_voice(voice_key, quality, None).await?;
        Ok(path)
    }

    /// List available Piper voices for download
    pub async fn list_available_piper_voices(&self) -> InstallResult<Vec<AvailablePiperVoice>> {
        let downloader = VoiceDownloader::new(self.models_dir.clone());
        let voices = downloader.list_available_voices().await?;
        Ok(voices)
    }

    // =========================================================================
    // Coqui TTS
    // =========================================================================

    async fn check_coqui(&self) -> InstallStatus {
        // Check for tts-server binary (Python package)
        let (installed, version, binary_path) = self.check_binary(&["tts-server", "tts"]).await;

        InstallStatus {
            provider: VoiceProviderType::Coqui,
            installed,
            version,
            binary_path,
            voices_available: if installed { 4 } else { 0 }, // Default models available
            install_method: InstallMethod::Python("pipx install TTS".to_string()),
            install_instructions: Some(if installed {
                "Coqui TTS ready. Models download automatically on first use.".to_string()
            } else {
                "Install: pipx install TTS\nOr: pip install TTS".to_string()
            }),
        }
    }

    async fn install_coqui(&self) -> InstallResult<InstallStatus> {
        let status = self.check_coqui().await;
        if status.installed {
            return Err(InstallError::AlreadyInstalled);
        }

        info!("Attempting to install Coqui TTS via pipx");

        // Try pipx first
        let output = Command::new("pipx")
            .args(["install", "TTS"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        match output {
            Ok(out) if out.status.success() => {
                info!("Coqui TTS installed successfully via pipx");
                Ok(self.check_coqui().await)
            }
            _ => {
                // Try pip as fallback
                warn!("pipx failed, trying pip");
                let output = Command::new("pip")
                    .args(["install", "--user", "TTS"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await?;

                if output.status.success() {
                    info!("Coqui TTS installed successfully via pip");
                    Ok(self.check_coqui().await)
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(InstallError::InstallFailed(format!(
                        "pip install failed: {}",
                        stderr
                    )))
                }
            }
        }
    }

    // =========================================================================
    // Other providers (detection only, manual install)
    // =========================================================================

    async fn check_ollama(&self) -> InstallStatus {
        let (installed, version, binary_path) = self.check_binary(&["ollama"]).await;

        InstallStatus {
            provider: VoiceProviderType::Ollama,
            installed,
            version,
            binary_path,
            voices_available: 0,
            install_method: InstallMethod::Binary("https://ollama.ai/download".to_string()),
            install_instructions: Some(if installed {
                "Ollama installed. Ensure a TTS model is pulled.".to_string()
            } else {
                "Download from: https://ollama.ai/download\nOr: curl -fsSL https://ollama.ai/install.sh | sh".to_string()
            }),
        }
    }

    async fn check_chatterbox(&self) -> InstallStatus {
        // Chatterbox typically runs as a Python server
        InstallStatus {
            provider: VoiceProviderType::Chatterbox,
            installed: false, // Would need to check if server is running
            version: None,
            binary_path: None,
            voices_available: 0,
            install_method: InstallMethod::Docker("docker run -p 8000:8000 resemble/chatterbox".to_string()),
            install_instructions: Some(
                "Docker: docker run -p 8000:8000 resemble/chatterbox\n\
                 Or clone: https://github.com/resemble-ai/chatterbox".to_string()
            ),
        }
    }

    async fn check_gpt_sovits(&self) -> InstallStatus {
        InstallStatus {
            provider: VoiceProviderType::GptSoVits,
            installed: false,
            version: None,
            binary_path: None,
            voices_available: 0,
            install_method: InstallMethod::Manual("Clone and run locally".to_string()),
            install_instructions: Some(
                "Clone: https://github.com/RVC-Boss/GPT-SoVITS\n\
                 Follow setup instructions in README".to_string()
            ),
        }
    }

    async fn check_xtts_v2(&self) -> InstallStatus {
        // XTTS-v2 uses the Coqui TTS server
        let coqui_status = self.check_coqui().await;

        InstallStatus {
            provider: VoiceProviderType::XttsV2,
            installed: coqui_status.installed,
            version: coqui_status.version,
            binary_path: coqui_status.binary_path,
            voices_available: 1, // XTTS v2 model
            install_method: InstallMethod::Python("pipx install TTS".to_string()),
            install_instructions: Some(if coqui_status.installed {
                "Run: tts-server --model_name tts_models/multilingual/multi-dataset/xtts_v2".to_string()
            } else {
                "Install Coqui TTS first: pipx install TTS".to_string()
            }),
        }
    }

    async fn check_fish_speech(&self) -> InstallStatus {
        InstallStatus {
            provider: VoiceProviderType::FishSpeech,
            installed: false,
            version: None,
            binary_path: None,
            voices_available: 0,
            install_method: InstallMethod::Docker("docker run -p 7860:7860 fishaudio/fish-speech".to_string()),
            install_instructions: Some(
                "Docker: docker run -p 7860:7860 fishaudio/fish-speech\n\
                 Or: https://github.com/fishaudio/fish-speech".to_string()
            ),
        }
    }

    async fn check_dia(&self) -> InstallStatus {
        InstallStatus {
            provider: VoiceProviderType::Dia,
            installed: false,
            version: None,
            binary_path: None,
            voices_available: 0,
            install_method: InstallMethod::Manual("Clone and run locally".to_string()),
            install_instructions: Some(
                "Clone: https://github.com/nari-labs/dia\n\
                 Follow setup instructions".to_string()
            ),
        }
    }

    // =========================================================================
    // Helper methods
    // =========================================================================

    async fn check_binary(&self, names: &[&str]) -> (bool, Option<String>, Option<String>) {
        for name in names {
            // Try 'which' to find the binary
            if let Ok(output) = Command::new("which")
                .arg(name)
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .await
            {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();

                    // Try to get version
                    let version = self.get_version(name).await;

                    return (true, version, Some(path));
                }
            }
        }
        (false, None, None)
    }

    async fn get_version(&self, binary: &str) -> Option<String> {
        // Try common version flags
        for flag in &["--version", "-V", "version"] {
            if let Ok(output) = Command::new(binary)
                .arg(flag)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // Extract first line, first few words
                    if let Some(line) = stdout.lines().next() {
                        return Some(line.chars().take(MAX_VERSION_LENGTH).collect());
                    }
                }
            }
        }
        None
    }
}

/// Quick helper to get popular Piper voices for the UI
pub fn get_recommended_piper_voices() -> Vec<(&'static str, &'static str, &'static str)> {
    super::download::popular_piper_voices()
}

/// Count ONNX model files in a directory
///
/// # Arguments
/// * `dir` - Directory to scan
/// * `recursive` - Whether to scan subdirectories
fn count_onnx_files(dir: &Path, recursive: bool) -> u32 {
    if !dir.exists() {
        return 0;
    }

    if recursive {
        walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "onnx"))
            .count() as u32
    } else {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "onnx"))
                    .count() as u32
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // =========================================================================
    // Unit Tests: count_onnx_files
    // =========================================================================

    mod count_onnx_files_tests {
        use super::*;

        #[test]
        fn returns_zero_for_nonexistent_dir() {
            let count = count_onnx_files(Path::new("/nonexistent/path"), false);
            assert_eq!(count, 0);
        }

        #[test]
        fn returns_zero_for_empty_dir() {
            let temp_dir = TempDir::new().unwrap();
            let count = count_onnx_files(temp_dir.path(), false);
            assert_eq!(count, 0);
        }

        #[test]
        fn counts_onnx_files_non_recursive() {
            let temp_dir = TempDir::new().unwrap();

            // Create some .onnx files
            std::fs::write(temp_dir.path().join("model1.onnx"), b"").unwrap();
            std::fs::write(temp_dir.path().join("model2.onnx"), b"").unwrap();
            std::fs::write(temp_dir.path().join("config.json"), b"").unwrap();

            let count = count_onnx_files(temp_dir.path(), false);
            assert_eq!(count, 2);
        }

        #[test]
        fn ignores_subdirs_when_non_recursive() {
            let temp_dir = TempDir::new().unwrap();

            std::fs::write(temp_dir.path().join("model1.onnx"), b"").unwrap();

            // Create subdir with more onnx files
            let subdir = temp_dir.path().join("subdir");
            std::fs::create_dir(&subdir).unwrap();
            std::fs::write(subdir.join("model2.onnx"), b"").unwrap();

            let count = count_onnx_files(temp_dir.path(), false);
            assert_eq!(count, 1); // Only counts top-level
        }

        #[test]
        fn counts_subdirs_when_recursive() {
            let temp_dir = TempDir::new().unwrap();

            std::fs::write(temp_dir.path().join("model1.onnx"), b"").unwrap();

            // Create subdir with more onnx files
            let subdir = temp_dir.path().join("subdir");
            std::fs::create_dir(&subdir).unwrap();
            std::fs::write(subdir.join("model2.onnx"), b"").unwrap();

            // Nested subdir
            let nested = subdir.join("nested");
            std::fs::create_dir(&nested).unwrap();
            std::fs::write(nested.join("model3.onnx"), b"").unwrap();

            let count = count_onnx_files(temp_dir.path(), true);
            assert_eq!(count, 3);
        }

        #[test]
        fn ignores_non_onnx_files() {
            let temp_dir = TempDir::new().unwrap();

            std::fs::write(temp_dir.path().join("model.onnx"), b"").unwrap();
            std::fs::write(temp_dir.path().join("model.onnx.json"), b"").unwrap();
            std::fs::write(temp_dir.path().join("readme.txt"), b"").unwrap();
            std::fs::write(temp_dir.path().join("model.bin"), b"").unwrap();

            let count = count_onnx_files(temp_dir.path(), false);
            assert_eq!(count, 1);
        }
    }

    // =========================================================================
    // Unit Tests: InstallStatus
    // =========================================================================

    mod install_status_tests {
        use super::*;

        #[test]
        fn install_status_serializes_correctly() {
            let status = InstallStatus {
                provider: VoiceProviderType::Piper,
                installed: true,
                version: Some("1.0.0".to_string()),
                binary_path: Some("/usr/bin/piper".to_string()),
                voices_available: 5,
                install_method: InstallMethod::PackageManager("paru -S piper".to_string()),
                install_instructions: Some("Test instructions".to_string()),
            };

            let json = serde_json::to_string(&status).unwrap();
            assert!(json.contains("\"installed\":true"));
            assert!(json.contains("\"voices_available\":5"));
        }

        #[test]
        fn install_method_variants_serialize() {
            let methods = vec![
                InstallMethod::PackageManager("apt install foo".to_string()),
                InstallMethod::Python("pip install bar".to_string()),
                InstallMethod::Binary("https://example.com".to_string()),
                InstallMethod::Docker("docker run foo".to_string()),
                InstallMethod::Manual("Do it yourself".to_string()),
                InstallMethod::AppManaged,
            ];

            for method in methods {
                let json = serde_json::to_string(&method).unwrap();
                assert!(!json.is_empty());
            }
        }
    }

    // =========================================================================
    // Unit Tests: ProviderInstaller
    // =========================================================================

    mod provider_installer_tests {
        use super::*;

        #[test]
        fn new_creates_installer_with_correct_path() {
            let dir = PathBuf::from("/tmp/test-models");
            let installer = ProviderInstaller::new(dir.clone());
            assert_eq!(installer.models_dir, dir);
        }

        #[tokio::test]
        async fn check_status_returns_status_for_all_providers() {
            let temp_dir = TempDir::new().unwrap();
            let installer = ProviderInstaller::new(temp_dir.path().to_path_buf());

            let providers = vec![
                VoiceProviderType::Piper,
                VoiceProviderType::Coqui,
                VoiceProviderType::Ollama,
                VoiceProviderType::Chatterbox,
                VoiceProviderType::GptSoVits,
                VoiceProviderType::XttsV2,
                VoiceProviderType::FishSpeech,
                VoiceProviderType::Dia,
            ];

            for provider in providers {
                let status = installer.check_status(&provider).await;
                assert_eq!(status.provider, provider);
                assert!(status.install_instructions.is_some());
            }
        }

        #[tokio::test]
        async fn check_all_local_returns_all_local_providers() {
            let temp_dir = TempDir::new().unwrap();
            let installer = ProviderInstaller::new(temp_dir.path().to_path_buf());

            let statuses = installer.check_all_local().await;

            // Should have at least 8 local providers
            assert!(statuses.len() >= 8);

            // All should have install instructions
            for status in &statuses {
                assert!(status.install_instructions.is_some());
            }
        }

        #[tokio::test]
        async fn cloud_providers_return_api_key_instructions() {
            let temp_dir = TempDir::new().unwrap();
            let installer = ProviderInstaller::new(temp_dir.path().to_path_buf());

            let cloud_providers = vec![
                VoiceProviderType::ElevenLabs,
                VoiceProviderType::FishAudio,
                VoiceProviderType::OpenAI,
            ];

            for provider in cloud_providers {
                let status = installer.check_status(&provider).await;
                let instructions = status.install_instructions.unwrap();
                assert!(
                    instructions.contains("API key"),
                    "Cloud provider {:?} should mention API key",
                    provider
                );
            }
        }

        #[test]
        fn count_piper_voices_with_mock_files() {
            let temp_dir = TempDir::new().unwrap();
            let installer = ProviderInstaller::new(temp_dir.path().to_path_buf());

            // Get baseline count (may include system voices from /usr/share/piper-voices)
            let baseline = installer.count_piper_voices();

            // Create mock onnx files
            std::fs::write(temp_dir.path().join("voice1.onnx"), b"").unwrap();
            std::fs::write(temp_dir.path().join("voice2.onnx"), b"").unwrap();

            let count = installer.count_piper_voices();
            assert_eq!(count, baseline + 2, "Should have 2 more voices after adding files");
        }
    }

    // =========================================================================
    // Unit Tests: get_recommended_piper_voices
    // =========================================================================

    mod recommended_voices_tests {
        use super::*;

        #[test]
        fn returns_non_empty_list() {
            let voices = get_recommended_piper_voices();
            assert!(!voices.is_empty());
        }

        #[test]
        fn all_entries_are_valid_tuples() {
            let voices = get_recommended_piper_voices();
            for (key, name, desc) in voices {
                assert!(!key.is_empty());
                assert!(!name.is_empty());
                assert!(!desc.is_empty());
                // Key should contain hyphens (valid voice key format)
                assert!(key.contains('-'), "Key {} should contain hyphens", key);
            }
        }
    }

    // =========================================================================
    // Integration Tests
    // =========================================================================

    mod integration {
        use super::*;

        #[tokio::test]
        async fn install_unsupported_provider_returns_error() {
            let temp_dir = TempDir::new().unwrap();
            let installer = ProviderInstaller::new(temp_dir.path().to_path_buf());

            // GptSoVits doesn't support auto-install
            let result = installer.install(&VoiceProviderType::GptSoVits).await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), InstallError::NotSupported(_)));
        }

        #[tokio::test]
        async fn download_piper_voice_integration() {
            let temp_dir = TempDir::new().unwrap();
            let installer = ProviderInstaller::new(temp_dir.path().to_path_buf());

            // Get baseline count (may include system voices from /usr/share/piper-voices)
            let baseline = installer.count_piper_voices();

            // Create mock voice files to simulate already downloaded
            let model_path = temp_dir.path().join("en_US-lessac-medium.onnx");
            let config_path = temp_dir.path().join("en_US-lessac-medium.onnx.json");
            std::fs::write(&model_path, b"mock").unwrap();
            std::fs::write(&config_path, b"{}").unwrap();

            // Verify count increases by 1
            let count = installer.count_piper_voices();
            assert_eq!(count, baseline + 1, "Should have 1 more voice after adding file");
        }

        #[tokio::test]
        #[ignore = "requires system binaries"]
        async fn check_piper_detects_installed_binary() {
            let temp_dir = TempDir::new().unwrap();
            let installer = ProviderInstaller::new(temp_dir.path().to_path_buf());

            let status = installer.check_status(&VoiceProviderType::Piper).await;

            // If piper is installed, should be detected
            if status.installed {
                assert!(status.binary_path.is_some());
            }
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

    proptest! {
        /// count_onnx_files should never panic regardless of path
        #[test]
        fn count_onnx_never_panics(path in ".*") {
            let _ = count_onnx_files(Path::new(&path), false);
            let _ = count_onnx_files(Path::new(&path), true);
        }

        /// InstallStatus should always serialize successfully
        #[test]
        fn install_status_always_serializes(
            installed in any::<bool>(),
            voices in 0u32..1000,
        ) {
            let status = InstallStatus {
                provider: VoiceProviderType::Piper,
                installed,
                version: None,
                binary_path: None,
                voices_available: voices,
                install_method: InstallMethod::AppManaged,
                install_instructions: None,
            };

            let result = serde_json::to_string(&status);
            prop_assert!(result.is_ok());
        }
    }
}
