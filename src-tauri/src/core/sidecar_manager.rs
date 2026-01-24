use sha2::{Sha256, Digest};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const MEILISEARCH_VERSION: &str = "v1.31.0";
const MEILISEARCH_DOWNLOAD_URL: &str = "https://github.com/meilisearch/meilisearch/releases/download";

/// Windows flag to prevent console window from appearing
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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
        // Try to read system meilisearch config first
        let (host, port, master_key) = Self::read_system_config();

        Self {
            host,
            port,
            master_key,
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

    /// Read configuration from /etc/meilisearch.conf (system meilisearch)
    /// Returns (host, port, master_key)
    fn read_system_config() -> (String, u16, String) {
        let default_host = "127.0.0.1".to_string();
        let default_port = 7700u16;
        let default_key = "ttrpg-assistant-dev-key".to_string();

        let config_path = std::path::Path::new("/etc/meilisearch.conf");
        if !config_path.exists() {
            return (default_host, default_port, default_key);
        }

        let content = match std::fs::read_to_string(config_path) {
            Ok(c) => c,
            Err(_) => return (default_host, default_port, default_key),
        };

        let mut host = default_host;
        let mut port = default_port;
        let mut master_key = default_key;
        let mut found_any = false;

        // Parse env-style config
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if line.starts_with("MEILI_MASTER_KEY=") {
                let value = line.trim_start_matches("MEILI_MASTER_KEY=");
                let value = value.trim_matches('"').trim_matches('\'');
                if !value.is_empty() {
                    master_key = value.to_string();
                    found_any = true;
                }
            } else if line.starts_with("MEILI_HTTP_ADDR=") {
                let value = line.trim_start_matches("MEILI_HTTP_ADDR=");
                let value = value.trim_matches('"').trim_matches('\'');
                // Parse host:port format (e.g., "127.0.0.1:7700" or "localhost:7700")
                if let Some((h, p)) = value.split_once(':') {
                    host = h.to_string();
                    if let Ok(parsed_port) = p.parse::<u16>() {
                        port = parsed_port;
                    }
                    found_any = true;
                }
            }
        }

        if found_any {
            log::info!(
                "Using system Meilisearch from /etc/meilisearch.conf: {}:{}",
                host, port
            );
        }

        (host, port, master_key)
    }
}

/// Determines how meilisearch was resolved
#[derive(Debug, Clone, PartialEq)]
pub enum MeilisearchSource {
    /// Already running on configured port
    ExistingInstance,
    /// Found in $PATH
    SystemPath,
    /// Downloaded to cache directory
    Downloaded,
}

pub struct SidecarManager {
    running: Arc<Mutex<bool>>,
    child: Arc<Mutex<Option<Child>>>,
    config: MeilisearchConfig,
    source: Arc<Mutex<Option<MeilisearchSource>>>,
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
            source: Arc::new(Mutex::new(None)),
        }
    }

    pub fn config(&self) -> &MeilisearchConfig {
        &self.config
    }

    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }

    pub async fn source(&self) -> Option<MeilisearchSource> {
        self.source.lock().await.clone()
    }

    /// Check if Meilisearch is healthy (HTTP health endpoint)
    /// Note: /health endpoint doesn't require auth per Meilisearch docs,
    /// but we include it anyway for consistency with protected instances
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.config.url());
        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        // Include auth header for protected instances
        if !self.config.master_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.config.master_key));
        }

        match request.send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Find meilisearch binary in $PATH (checks both with and without .exe on Windows)
    fn find_in_path() -> Option<PathBuf> {
        which::which("meilisearch")
            .or_else(|_| which::which("meilisearch.exe"))
            .ok()
    }

    /// Get the cache directory for downloaded binaries
    fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ttrpg-assistant")
            .join("bin")
    }

    /// Get the expected path for a downloaded meilisearch binary
    fn cached_binary_path() -> PathBuf {
        let filename = if cfg!(windows) {
            format!("meilisearch-{}.exe", MEILISEARCH_VERSION)
        } else {
            format!("meilisearch-{}", MEILISEARCH_VERSION)
        };
        Self::cache_dir().join(filename)
    }

    /// Fetch and parse the SHA256 checksum for a specific file from the release
    async fn fetch_checksum(filename: &str) -> Result<String, String> {
        let checksum_url = format!(
            "{}/{}/meilisearch-{}-sha256.txt",
            MEILISEARCH_DOWNLOAD_URL, MEILISEARCH_VERSION, MEILISEARCH_VERSION
        );

        log::debug!("Fetching checksum from: {}", checksum_url);

        let response = reqwest::get(&checksum_url)
            .await
            .map_err(|e| format!("Failed to fetch checksum file: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to fetch checksum file: HTTP {}",
                response.status()
            ));
        }

        let checksum_content = response
            .text()
            .await
            .map_err(|e| format!("Failed to read checksum file: {}", e))?;

        // Parse the checksum file - format is "checksum  filename" per line
        for line in checksum_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let file = parts[1].trim_start_matches('*'); // Handle BSD-style "*filename"
                if file == filename {
                    return Ok(parts[0].to_lowercase());
                }
            }
        }

        Err(format!(
            "Checksum for '{}' not found in checksum file",
            filename
        ))
    }

    /// Verify SHA256 checksum of downloaded bytes
    fn verify_checksum(bytes: &[u8], expected: &str) -> Result<(), String> {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let result = hasher.finalize();
        let actual = hex::encode(result);

        if actual.to_lowercase() == expected.to_lowercase() {
            log::info!("SHA256 checksum verified: {}", actual);
            Ok(())
        } else {
            Err(format!(
                "SHA256 checksum mismatch: expected {}, got {}",
                expected, actual
            ))
        }
    }

    /// Download meilisearch binary to cache with checksum verification
    async fn download_binary() -> Result<PathBuf, String> {
        let cache_dir = Self::cache_dir();
        let binary_path = Self::cached_binary_path();

        // Already downloaded?
        if binary_path.exists() {
            log::info!("Using cached meilisearch binary: {:?}", binary_path);
            return Ok(binary_path);
        }

        log::info!("Downloading meilisearch {}...", MEILISEARCH_VERSION);

        // Create cache directory
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache dir: {}", e))?;

        // Determine platform-specific download URL
        let (os, arch) = if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
            ("linux", "amd64")
        } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
            ("macos", "amd64")
        } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
            ("macos", "apple-silicon")
        } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
            ("windows", "amd64")
        } else {
            return Err("Unsupported platform for meilisearch download".to_string());
        };

        let filename = if cfg!(target_os = "windows") {
            format!("meilisearch-{}-{}.exe", os, arch)
        } else {
            format!("meilisearch-{}-{}", os, arch)
        };

        // Fetch expected checksum first (fail fast if checksum unavailable)
        let expected_checksum = Self::fetch_checksum(&filename).await?;
        log::debug!("Expected checksum for {}: {}", filename, expected_checksum);

        let url = format!("{}/{}/{}", MEILISEARCH_DOWNLOAD_URL, MEILISEARCH_VERSION, filename);

        // Download binary
        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("Download failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Download failed with status: {}", response.status()));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        // Verify checksum BEFORE writing to disk
        Self::verify_checksum(&bytes, &expected_checksum)?;

        // Write to temp file first, then rename (atomic on Unix, best-effort on Windows)
        // On Windows, keep .exe extension but add .tmp before it
        let temp_path = if cfg!(windows) {
            binary_path.with_extension("tmp.exe")
        } else {
            binary_path.with_extension("tmp")
        };

        std::fs::write(&temp_path, &bytes)
            .map_err(|e| format!("Failed to write binary: {}", e))?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&temp_path)
                .map_err(|e| format!("Failed to get metadata: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&temp_path, perms)
                .map_err(|e| format!("Failed to set permissions: {}", e))?;
        }

        // Rename to final path (atomic on Unix, may fail on Windows if target exists)
        #[cfg(windows)]
        {
            // On Windows, remove target first if it exists (rename doesn't overwrite)
            let _ = std::fs::remove_file(&binary_path);
        }

        std::fs::rename(&temp_path, &binary_path)
            .map_err(|e| format!("Failed to rename binary: {}", e))?;

        log::info!("Downloaded meilisearch to {:?}", binary_path);
        Ok(binary_path)
    }

    /// Resolve the meilisearch binary: check existing, PATH, or download
    async fn resolve_binary(&self) -> Result<(PathBuf, MeilisearchSource), String> {
        // 1. Check if already running on configured port
        if self.health_check().await {
            log::info!(
                "Meilisearch already running at {}",
                self.config.url()
            );
            return Ok((PathBuf::new(), MeilisearchSource::ExistingInstance));
        }

        // 2. Check $PATH
        if let Some(path) = Self::find_in_path() {
            log::info!("Found meilisearch in PATH: {:?}", path);
            return Ok((path, MeilisearchSource::SystemPath));
        }

        // 3. Check cached download
        let cached = Self::cached_binary_path();
        if cached.exists() {
            log::info!("Using cached meilisearch: {:?}", cached);
            return Ok((cached, MeilisearchSource::Downloaded));
        }

        // 4. Download
        let path = Self::download_binary().await?;
        Ok((path, MeilisearchSource::Downloaded))
    }

    pub async fn start(&self) -> Result<(), String> {
        let mut lock = self.running.lock().await;
        if *lock {
            log::info!("Meilisearch sidecar already running.");
            return Ok(());
        }

        let (binary_path, source) = self.resolve_binary().await?;

        // If already running externally, just mark as running
        if source == MeilisearchSource::ExistingInstance {
            *lock = true;
            *self.source.lock().await = Some(source);
            return Ok(());
        }

        // Ensure data directory exists
        std::fs::create_dir_all(&self.config.data_dir)
            .map_err(|e| format!("Failed to create Meilisearch data directory: {}", e))?;

        // Build the command
        let mut cmd = Command::new(&binary_path);

        // Convert data_dir to UTF-8 string - fail fast if path contains non-UTF8 characters
        let db_path = self.config.data_dir
            .to_str()
            .ok_or_else(|| {
                format!(
                    "Meilisearch data directory path contains non-UTF8 characters: {:?}",
                    self.config.data_dir
                )
            })?;

        cmd.args([
            "--http-addr",
            &format!("{}:{}", self.config.host, self.config.port),
            "--master-key",
            &self.config.master_key,
            "--db-path",
            db_path,
            "--env",
            "development",
            "--log-level",
            "WARN",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        // On Windows, prevent console window from appearing
        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

        // Spawn the process
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn meilisearch: {}", e))?;

        let pid = child.id().unwrap_or(0);
        log::info!(
            "Meilisearch started (PID: {}, source: {:?}) at {}",
            pid,
            source,
            self.config.url()
        );

        // Spawn log reader tasks
        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    log::debug!("[MEILI] {}", line);
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if line.contains("error") || line.contains("Error") {
                        log::error!("[MEILI] {}", line);
                    } else {
                        log::debug!("[MEILI] {}", line);
                    }
                }
            });
        }

        *self.child.lock().await = Some(child);
        *self.source.lock().await = Some(source);
        *lock = true;

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let source = self.source.lock().await.clone();

        // Don't kill external instances
        if source == Some(MeilisearchSource::ExistingInstance) {
            log::info!("Not stopping external meilisearch instance");
            *self.running.lock().await = false;
            return Ok(());
        }

        let mut child_lock = self.child.lock().await;
        if let Some(mut child) = child_lock.take() {
            child.kill().await.map_err(|e| e.to_string())?;
            *self.running.lock().await = false;
            log::info!("Meilisearch sidecar stopped");
        }
        Ok(())
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
