use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
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
}

impl PiperProvider {
    pub fn new(config: PiperConfig) -> Self {
        let models_dir = config.models_dir.unwrap_or_else(|| {
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
        }
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
            id: model_path.to_string_lossy().to_string(),
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
            if path.extension().map_or(false, |e| e == "onnx") {
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

        let model_path = self.get_model_path(&request.voice_id)?;

        // Piper settings logic mirroring audaio logic
        // default settings
        let length_scale = 1.0;
        let noise_scale = 0.667;
        let noise_w = 0.8;
        let sentence_silence = 0.2;

        // If settings are present in request, we could override them.
        // Currently VoiceSettings has stability/similarity, not directly mapping to length/noise.
        // We'll stick to defaults.

        let output_file = NamedTempFile::with_suffix(".wav")
            .map_err(|e| VoiceError::IoError(e))?;
        let output_path = output_file.path().to_path_buf();

        debug!(
            model = ?model_path,
            text_len = request.text.len(),
            "Synthesizing with Piper CLI"
        );

        let mut cmd = Command::new(exe);
        cmd.arg("--model").arg(&model_path)
           .arg("--output_file").arg(&output_path)
           .arg("--length_scale").arg(length_scale.to_string())
           .arg("--noise_scale").arg(noise_scale.to_string())
           .arg("--noise_w").arg(noise_w.to_string())
           .arg("--sentence_silence").arg(sentence_silence.to_string());

        // Potentially handle speaker ID if we ever parse multi-speaker models correctly into request.voice_id format

        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| VoiceError::IoError(e))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(request.text.as_bytes()).await.map_err(|e| VoiceError::IoError(e))?;
        }

        let output = child.wait_with_output().await.map_err(|e| VoiceError::IoError(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(VoiceError::ApiError(format!("Piper failed: {}", stderr)));
        }

        let wav_data = tokio::fs::read(&output_path).await.map_err(|e| VoiceError::IoError(e))?;

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
