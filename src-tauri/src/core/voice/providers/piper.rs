use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tempfile::NamedTempFile;
use std::io::Write;
use tracing::{debug, info, warn};

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

        // Simple discovery of piper executable
        let executable = if Self::check_command("piper") {
            Some("piper".to_string())
        } else if Self::check_command("piper-tts") {
            Some("piper-tts".to_string())
        } else {
            None
        };

        if executable.is_none() {
             warn!("Piper TTS not found. Please install 'piper' or 'piper-tts'.");
        }

        Self {
            models_dir,
            executable,
        }
    }

    fn check_command(cmd: &str) -> bool {
        std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn get_model_path(&self, voice_id: &str) -> Result<PathBuf> {
        // Voice ID is expected to be the full path or relative path to onnx
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

        // Try appending .onnx
        let onnx_path = self.models_dir.join(format!("{}.onnx", voice_id));
        if onnx_path.exists() {
             return Ok(onnx_path);
        }

        Err(VoiceError::InvalidVoiceId(format!("Piper model not found: {}", voice_id)))
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

        // Create temp file for output because Piper writes to file or stdout.
        // Stdout capture is cleaner if possible, but existing code used file.
        // Let's try stdout capture to avoid disk I/O if possible, or use NamedTempFile if piper requires file.
        // Reading Piper docs... it can output to stdout if --output_file is not correctly specified?
        // Or usually --output_file - implies stdout.

        // Let's use NamedTempFile as it matches the reference implementation provided.
        let output_file = NamedTempFile::with_suffix(".wav")
            .map_err(|e| VoiceError::IoError(e))?;
        let output_path = output_file.path().to_path_buf();

        let mut cmd = Command::new(exe);
        cmd.arg("--model").arg(&model_path)
           .arg("--output_file").arg(&output_path);

        // Settings
        if let Some(settings) = &request.settings {
             // Piper doesn't support stability/similarity directly map to its params mostly.
             // But we can map speed?
             // length_scale
        }

        cmd.stdin(Stdio::piped())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| VoiceError::IoError(e))?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(request.text.as_bytes()).await.map_err(|e| VoiceError::IoError(e))?;
        }

        let output = child.wait_with_output().await.map_err(|e| VoiceError::IoError(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(VoiceError::ApiError(format!("Piper failed: {}", stderr)));
        }

        let wav_data = tokio::fs::read(&output_path).await.map_err(|e| VoiceError::IoError(e))?;
        Ok(wav_data)
    }

    async fn list_voices(&self) -> Result<Vec<Voice>> {
        // Accessing 'audaio' implementation: it scanned directories.
        // We should implement similar scanning logic.
        let mut voices = Vec::new();

        // Scan internal logic helper
        // Since we are inside async trait, we should probably run blocking fs operations in spawn_blocking
        // if we were strict, but listing voices is infrequent.

        // For now, let's just implement a basic scan of models_dir and /usr/share/piper-voices
        let dirs_to_scan = vec![self.models_dir.clone(), PathBuf::from(SYSTEM_VOICES_DIR)];

        for dir in dirs_to_scan {
            if !dir.exists() { continue; }
            // Basic recursive scan
            let walker = walkdir::WalkDir::new(&dir).into_iter();
            for entry in walker.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "onnx") {
                    // Found a model
                    // Try to finding corresponding .json
                    let json_path = path.with_extension("onnx.json");
                    if json_path.exists() {
                        // Parse JSON
                         if let Ok(content) = std::fs::read_to_string(&json_path) {
                             if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                 // Construct Voice object
                                 let name = path.file_stem().unwrap().to_string_lossy().to_string();
                                 let id = path.to_string_lossy().to_string();
                                 // Try to get more info from json
                                 let lang = json["language"]["name_english"].as_str().unwrap_or("Unknown").to_string();

                                 voices.push(Voice {
                                     id,
                                     name,
                                     provider: "piper".to_string(),
                                     description: Some(format!("Language: {}", lang)),
                                     preview_url: None,
                                     labels: vec![lang],
                                 });
                             }
                         }
                    }
                }
            }
        }

        Ok(voices)
    }

    async fn check_usage(&self) -> Result<UsageInfo> {
        Ok(UsageInfo::default())
    }
}
