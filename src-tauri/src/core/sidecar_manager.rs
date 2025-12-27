use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::Mutex;
use tauri::{async_runtime, AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{CommandEvent, CommandChild};

/// Configuration for Meilisearch sidecar
#[derive(Debug, Clone)]
pub struct MeilisearchConfig {
    pub host: String,
    pub port: u16,
    pub master_key: String,
    pub data_dir: PathBuf,
}

impl Default for MeilisearchConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 7700,
            master_key: "ttrpg-assistant-dev-key".to_string(),
            data_dir: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("ttrpg-assistant")
                .join("meilisearch"),
        }
    }
}

impl MeilisearchConfig {
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

pub struct SidecarManager {
    running: Arc<Mutex<bool>>,
    child: Arc<Mutex<Option<CommandChild>>>,
    config: MeilisearchConfig,
}

impl SidecarManager {
    pub fn new() -> Self {
        Self::with_config(MeilisearchConfig::default())
    }

    pub fn with_config(config: MeilisearchConfig) -> Self {
        Self {
            running: Arc::new(Mutex::new(false)),
            child: Arc::new(Mutex::new(None)),
            config,
        }
    }

    pub fn config(&self) -> &MeilisearchConfig {
        &self.config
    }

    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }

    pub fn start(&self, app_handle: AppHandle) {
        let running = self.running.clone();
        let child_handle = self.child.clone();
        let config = self.config.clone();

        async_runtime::spawn(async move {
            let mut lock = running.lock().await;
            if *lock {
                log::info!("Meilisearch sidecar already running.");
                return;
            }

            // Ensure data directory exists
            if let Err(e) = std::fs::create_dir_all(&config.data_dir) {
                log::error!("Failed to create Meilisearch data directory: {}", e);
                return;
            }

            // Build sidecar command with arguments
            let sidecar_command = match app_handle.shell().sidecar("meilisearch") {
                Ok(cmd) => cmd
                    .args([
                        "--http-addr", &format!("{}:{}", config.host, config.port),
                        "--master-key", &config.master_key,
                        "--db-path", config.data_dir.to_str().unwrap_or("./meilisearch_data"),
                        "--env", "development",
                        "--log-level", "WARN",
                    ]),
                Err(e) => {
                    log::error!("Failed to create meilisearch sidecar command: {}", e);
                    return;
                }
            };

            let (mut rx, child) = match sidecar_command.spawn() {
                Ok(res) => res,
                Err(e) => {
                    log::error!("Failed to spawn meilisearch sidecar: {}", e);
                    return;
                }
            };

            let pid = child.pid();
            *child_handle.lock().await = Some(child);
            *lock = true;
            drop(lock); // Release lock before entering loop

            log::info!("Meilisearch sidecar started (PID: {}) at {}", pid, config.url());

            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        log::debug!("[MEILI] {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Stderr(line) => {
                        let msg = String::from_utf8_lossy(&line);
                        // Filter out noisy startup messages
                        if msg.contains("error") || msg.contains("Error") {
                            log::error!("[MEILI] {}", msg);
                        } else {
                            log::debug!("[MEILI] {}", msg);
                        }
                    }
                    CommandEvent::Terminated(payload) => {
                        log::warn!("Meilisearch terminated: {:?}", payload);
                        let mut lock = running.lock().await;
                        *lock = false;
                        *child_handle.lock().await = None;
                        break;
                    }
                    _ => {}
                }
            }
        });
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut child_lock = self.child.lock().await;
        if let Some(child) = child_lock.take() {
            child.kill().map_err(|e| e.to_string())?;
            *self.running.lock().await = false;
            log::info!("Meilisearch sidecar stopped");
        }
        Ok(())
    }

    /// Check if Meilisearch is healthy (HTTP health endpoint)
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.config.url());
        match reqwest::get(&url).await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Wait for Meilisearch to become healthy
    pub async fn wait_for_ready(&self, timeout_secs: u64) -> bool {
        let start = std::time::Instant::now();
        let duration = std::time::Duration::from_secs(timeout_secs);

        while start.elapsed() < duration {
            if self.health_check().await {
                return true;
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        false
    }
}

impl Default for SidecarManager {
    fn default() -> Self {
        Self::new()
    }
}
