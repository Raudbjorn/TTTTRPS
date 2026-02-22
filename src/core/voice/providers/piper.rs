use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::RwLock;
use tokio::process::Command;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn, debug};
use serde_json::Value;

use super::super::types::{Result, SynthesisRequest, Voice, UsageInfo, PiperConfig, VoiceError};
use super::VoiceProvider;

const SYSTEM_VOICES_DIR: &str = "/usr/share/piper-voices";

pub struct PiperProvider {
    models_dir: PathBuf,
    executable: Option<String>,
    config: RwLock<PiperConfig>,
}

impl PiperProvider {
    pub fn new(config: PiperConfig) -> Self {
        let models_dir = config.models_dir.clone().unwrap_or_else(|| {
             dirs::data_local_dir()
                .unwrap_or(PathBuf::from("."))
                .join("ttrpg-assistant/voice/piper")
        });

        // robust discovery of piper executable
        let executable = if Self::check_command("piper") {
            Some("piper".to_string())
        } else if Self::check_command("piper-tts") {
            Some("piper-tts".to_string())
        } else {
            None
        };

        if executable.is_none() {
             warn!("Piper TTS executable not found. Please install 'piper' or 'piper-tts'.");
        }

        Self {
            models_dir,
            executable,
            config: RwLock::new(config),
        }
    }

    /// Update voice adjustment settings (thread-safe)
    pub fn update_settings(&self, length_scale: f32, noise_scale: f32, noise_w: f32, sentence_silence: f32, speaker_id: u32) -> Result<()> {
        let mut config = self.config.write()
            .map_err(|_| VoiceError::NotConfigured("Config lock poisoned".to_string()))?;
        config.length_scale = length_scale;
        config.noise_scale = noise_scale;
        config.noise_w = noise_w;
        config.sentence_silence = sentence_silence;
        config.speaker_id = speaker_id;
        Ok(())
    }

    /// Get current settings (returns a clone for thread safety)
    pub fn settings(&self) -> Result<PiperConfig> {
        self.config.read()
            .map(|c| c.clone())
            .map_err(|_| VoiceError::NotConfigured("Config lock poisoned".to_string()))
    }

    fn check_command(cmd: &str) -> bool {
        std::process::Command::new("which")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    fn get_model_path(&self, voice_id: &str) -> Result<PathBuf> {
        // Voice ID might be the full path or relative path to onnx
        let path = PathBuf::from(voice_id);
        if path.exists() {
            return Ok(path);
        }

        // Check in models dir
        let local_path = self.models_dir.join(voice_id);
        if local_path.exists() {
             return Ok(local_path);
        }

        let system_path = PathBuf::from(SYSTEM_VOICES_DIR).join(voice_id);
        if system_path.exists() {
             return Ok(system_path);
        }

        // Try appending .onnx if not present
        if !voice_id.ends_with(".onnx") {
             let onnx_path = self.models_dir.join(format!("{}.onnx", voice_id));
             if onnx_path.exists() {
                  return Ok(onnx_path);
             }
        }

        Err(VoiceError::InvalidVoiceId(format!("Piper model not found: {}", voice_id)))
    }

    fn parse_voice_from_model(&self, model_path: &Path, config_path: &Path) -> Option<Voice> {
        let content = std::fs::read_to_string(config_path).ok()?;
        let json: Value = serde_json::from_str(&content).ok()?;

        let filename = model_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown");

        // Try to extract quality from path (e.g., /medium/ or /high/)
        let quality = model_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .filter(|q| ["low", "medium", "high", "x_low"].contains(q))
            .map(|q| match q {
                "x_low" => "X-Low",
                "low" => "Low",
                "medium" => "Medium",
                "high" => "High",
                _ => q,
            });

        // Create a nice display name
        let name = if let Some(q) = quality {
            format!("{} ({})", filename, q)
        } else {
            filename.to_string()
        };

        let lang = json["language"]["name_english"]
            .as_str()
            .unwrap_or(json["language"]["code"].as_str().unwrap_or("Unknown"))
            .to_string();

        let description = json["dataset"].as_str().map(|s| s.to_string());

        // Collect labels
        let mut labels = vec![lang.clone()];
        if let Some(q) = quality {
            labels.push(q.to_string());
        }

        Some(Voice {
            id: format!("piper:{}", model_path.to_string_lossy()),
            name,
            provider: "piper".to_string(),
            description,
            preview_url: None,
            labels,
        })
    }

    fn scan_directory(&self, dir: &Path, voices: &mut Vec<Voice>) {
        if !dir.exists() { return; }

        // Use WalkDir for recursive scanning
        let walker = walkdir::WalkDir::new(dir).into_iter();
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "onnx") {
                let config_path = path.with_extension("onnx.json");
                if config_path.exists() {
                     if let Some(voice) = self.parse_voice_from_model(path, &config_path) {
                         if !voices.iter().any(|v| v.id == voice.id) {
                             voices.push(voice);
                         }
                     }
                }
            }
        }
    }
}

#[async_trait]
impl VoiceProvider for PiperProvider {
    fn id(&self) -> &'static str {
        "piper"
    }

    async fn synthesize(&self, request: &SynthesisRequest) -> Result<Vec<u8>> {
        let exe = self.executable.as_ref().ok_or_else(|| {
            VoiceError::NotConfigured("Piper executable not found".to_string())
        })?;

        info!("Piper synthesize called with voice_id: {}", request.voice_id);
        let voice_id = request.voice_id.strip_prefix("piper:").unwrap_or(&request.voice_id);
        info!("After strip_prefix, voice_id: {}", voice_id);
        let model_path = self.get_model_path(voice_id)?;
        info!("Resolved model_path: {:?}", model_path);

        // Use settings from config (thread-safe read, extracted before async)
        let (length_scale, noise_scale, noise_w, sentence_silence, speaker_id) = {
            let config = self.config.read()
                .map_err(|_| VoiceError::NotConfigured("Config lock poisoned".to_string()))?;
            (config.length_scale, config.noise_scale, config.noise_w,
             config.sentence_silence, config.speaker_id)
        };

        let output_file = NamedTempFile::with_suffix(".wav")
            .map_err(VoiceError::IoError)?;
        let output_path = output_file.path().to_path_buf();

        debug!(
            model = ?model_path,
            text_len = request.text.len(),
            length_scale = length_scale,
            noise_scale = noise_scale,
            speaker_id = speaker_id,
            "Synthesizing with Piper CLI"
        );

        let mut cmd = Command::new(exe);
        cmd.arg("--model").arg(&model_path)
           .arg("--output_file").arg(&output_path)
           .arg("--length_scale").arg(length_scale.to_string())
           .arg("--noise_scale").arg(noise_scale.to_string())
           .arg("--noise_w").arg(noise_w.to_string())
           .arg("--sentence_silence").arg(sentence_silence.to_string());

        // Add speaker ID if multi-speaker model
        if speaker_id > 0 {
            cmd.arg("--speaker").arg(speaker_id.to_string());
        }

        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(VoiceError::IoError)?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(request.text.as_bytes()).await.map_err(VoiceError::IoError)?;
        }

        let output = child.wait_with_output().await.map_err(VoiceError::IoError)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            warn!(
                model = ?model_path,
                exit_code = ?output.status.code(),
                stderr = %stderr,
                stdout = %stdout,
                "Piper command failed"
            );
            return Err(VoiceError::ApiError(format!("Piper failed: {}", stderr)));
        }

        let wav_data = tokio::fs::read(&output_path).await.map_err(VoiceError::IoError)?;

        info!(size = wav_data.len(), "Piper synthesis complete");

        Ok(wav_data)
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        let mut voices = Vec::new();

        // Scan standard locations
        let locations = vec![
            self.models_dir.clone(),
            PathBuf::from(SYSTEM_VOICES_DIR),
        ];

        // Perform scanning
        // For CLI providers, blocking FS scan is acceptable given usage pattern
        for dir in locations {
            self.scan_directory(&dir, &mut voices);
        }

        // Sort by name
        voices.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(voices)
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        Ok(UsageInfo::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> PiperConfig {
        PiperConfig {
            models_dir: None,
            length_scale: 1.0,
            noise_scale: 0.667,
            noise_w: 0.8,
            sentence_silence: 0.2,
            speaker_id: 0,
        }
    }

    #[test]
    fn test_settings_returns_result() {
        let provider = PiperProvider::new(default_config());
        let result = provider.settings();
        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.length_scale, 1.0);
    }

    #[test]
    fn test_update_settings_returns_result() {
        let provider = PiperProvider::new(default_config());
        let result = provider.update_settings(1.5, 0.5, 0.6, 0.3, 1);
        assert!(result.is_ok());

        let config = provider.settings().unwrap();
        assert_eq!(config.length_scale, 1.5);
        assert_eq!(config.noise_scale, 0.5);
        assert_eq!(config.noise_w, 0.6);
        assert_eq!(config.sentence_silence, 0.3);
        assert_eq!(config.speaker_id, 1);
    }

    #[test]
    fn test_concurrent_access_is_safe() {
        use std::sync::Arc;
        use std::thread;

        let provider = Arc::new(PiperProvider::new(default_config()));
        let mut handles = vec![];

        // Spawn multiple threads reading settings concurrently
        for _ in 0..10 {
            let p = Arc::clone(&provider);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _ = p.settings();
                }
            }));
        }

        // Spawn threads writing settings concurrently
        for i in 0..5 {
            let p = Arc::clone(&provider);
            handles.push(thread::spawn(move || {
                for j in 0..20 {
                    let _ = p.update_settings(
                        1.0 + (i as f32 * 0.1),
                        0.5,
                        0.6,
                        0.3,
                        (j % 5) as u32,
                    );
                }
            }));
        }

        // All threads should complete without deadlock or panic
        for handle in handles {
            handle.join().expect("Thread should not panic");
        }

        // Final state should be consistent (we can read settings)
        let final_config = provider.settings();
        assert!(final_config.is_ok());
    }

    #[test]
    fn test_provider_id() {
        let provider = PiperProvider::new(default_config());
        assert_eq!(provider.id(), "piper");
    }
}
