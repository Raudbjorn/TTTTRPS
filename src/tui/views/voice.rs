//! Voice manager — queue status, provider config, voice profiles, audio monitor.
//!
//! Displays synthesis queue stats and voice provider configuration
//! from Services (SynthesisQueue + VoiceManager).
//!
//! Tabs:
//!   Queue    — live job list (pending + processing + recent history)
//!   Config   — provider selector with per-provider config fields
//!   Profiles — voice profile manager (character → voice ID mappings)
//!   Monitor  — audio level sparkline placeholder

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use super::super::theme;
use crate::tui::services::Services;

// ── Constants ────────────────────────────────────────────────────────────────

/// All providers in display order (matches VoiceProviderType enum).
const ALL_PROVIDERS: &[ProviderEntry] = &[
    ProviderEntry { id: "piper", label: "Piper (Local)", is_local: true },
    ProviderEntry { id: "coqui", label: "Coqui TTS Server", is_local: true },
    ProviderEntry { id: "xtts_v2", label: "XTTS-v2 (Coqui)", is_local: true },
    ProviderEntry { id: "gpt_sovits", label: "GPT-SoVITS", is_local: true },
    ProviderEntry { id: "fish_speech", label: "Fish Speech", is_local: true },
    ProviderEntry { id: "ollama", label: "Ollama", is_local: true },
    ProviderEntry { id: "chatterbox", label: "Chatterbox", is_local: true },
    ProviderEntry { id: "dia", label: "Dia", is_local: true },
    ProviderEntry { id: "elevenlabs", label: "ElevenLabs", is_local: false },
    ProviderEntry { id: "openai", label: "OpenAI TTS", is_local: false },
    ProviderEntry { id: "fish_audio", label: "Fish Audio (Cloud)", is_local: false },
];

/// Maximum number of history items shown in the queue tab.
const QUEUE_HISTORY_LIMIT: usize = 10;

// ── Provider table entry ─────────────────────────────────────────────────────

struct ProviderEntry {
    id: &'static str,
    label: &'static str,
    is_local: bool,
}

/// Per-provider config fields rendered in the Config tab.
struct ConfigField {
    label: &'static str,
    value: String,
    is_secret: bool,
}

fn config_fields_for_provider(id: &str, data: &VoiceData) -> Vec<ConfigField> {
    match id {
        "piper" => {
            let cfg = data.piper_config.as_ref();
            vec![
                ConfigField {
                    label: "Models Dir",
                    value: cfg
                        .and_then(|c| c.models_dir.as_ref())
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Length Scale",
                    value: cfg.map(|c| format!("{:.2}", c.length_scale)).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Noise Scale",
                    value: cfg.map(|c| format!("{:.3}", c.noise_scale)).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Noise W",
                    value: cfg.map(|c| format!("{:.2}", c.noise_w)).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Sentence Silence",
                    value: cfg.map(|c| format!("{:.2}s", c.sentence_silence)).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Speaker ID",
                    value: cfg.map(|c| c.speaker_id.to_string()).unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        "coqui" => {
            let cfg = data.coqui_config.as_ref();
            vec![
                ConfigField {
                    label: "Port",
                    value: cfg.map(|c| c.port.to_string()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Model",
                    value: cfg.map(|c| c.model.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Speed",
                    value: cfg.map(|c| format!("{:.1}", c.speed)).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Temperature",
                    value: cfg.map(|c| format!("{:.2}", c.temperature)).unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        "xtts_v2" => {
            let cfg = data.xtts_v2_config.as_ref();
            vec![
                ConfigField {
                    label: "Base URL",
                    value: cfg.map(|c| c.base_url.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Speaker WAV",
                    value: cfg.and_then(|c| c.speaker_wav.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Language",
                    value: cfg.and_then(|c| c.language.clone()).unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        "gpt_sovits" => {
            let cfg = data.gpt_sovits_config.as_ref();
            vec![
                ConfigField {
                    label: "Base URL",
                    value: cfg.map(|c| c.base_url.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Ref Audio",
                    value: cfg.and_then(|c| c.reference_audio.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Language",
                    value: cfg.and_then(|c| c.language.clone()).unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        "fish_speech" => {
            let cfg = data.fish_speech_config.as_ref();
            vec![
                ConfigField {
                    label: "Base URL",
                    value: cfg.map(|c| c.base_url.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Ref Audio",
                    value: cfg.and_then(|c| c.reference_audio.clone()).unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        "ollama" => {
            let cfg = data.ollama_config.as_ref();
            vec![
                ConfigField {
                    label: "Base URL",
                    value: cfg.map(|c| c.base_url.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Model",
                    value: cfg.map(|c| c.model.clone()).unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        "chatterbox" => {
            let cfg = data.chatterbox_config.as_ref();
            vec![
                ConfigField {
                    label: "Base URL",
                    value: cfg.map(|c| c.base_url.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Exaggeration",
                    value: cfg
                        .and_then(|c| c.exaggeration)
                        .map(|v| format!("{:.2}", v))
                        .unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "CFG Weight",
                    value: cfg
                        .and_then(|c| c.cfg_weight)
                        .map(|v| format!("{:.2}", v))
                        .unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        "dia" => {
            let cfg = data.dia_config.as_ref();
            vec![
                ConfigField {
                    label: "Base URL",
                    value: cfg.map(|c| c.base_url.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Voice ID",
                    value: cfg.and_then(|c| c.voice_id.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Dialogue Mode",
                    value: cfg
                        .and_then(|c| c.dialogue_mode)
                        .map(|b| if b { "On" } else { "Off" }.to_string())
                        .unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        "elevenlabs" => {
            let cfg = data.elevenlabs_config.as_ref();
            vec![
                ConfigField {
                    label: "API Key",
                    value: cfg.map(|c| mask_secret(&c.api_key)).unwrap_or_default(),
                    is_secret: true,
                },
                ConfigField {
                    label: "Model",
                    value: cfg.and_then(|c| c.model_id.clone()).unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        "openai" => {
            let cfg = data.openai_config.as_ref();
            vec![
                ConfigField {
                    label: "API Key",
                    value: cfg.map(|c| mask_secret(&c.api_key)).unwrap_or_default(),
                    is_secret: true,
                },
                ConfigField {
                    label: "Model",
                    value: cfg.map(|c| c.model.clone()).unwrap_or_default(),
                    is_secret: false,
                },
                ConfigField {
                    label: "Voice",
                    value: cfg.map(|c| c.voice.clone()).unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        "fish_audio" => {
            let cfg = data.fish_audio_config.as_ref();
            vec![
                ConfigField {
                    label: "API Key",
                    value: cfg.map(|c| mask_secret(&c.api_key)).unwrap_or_default(),
                    is_secret: true,
                },
                ConfigField {
                    label: "Base URL",
                    value: cfg.and_then(|c| c.base_url.clone()).unwrap_or_default(),
                    is_secret: false,
                },
            ]
        }
        _ => vec![],
    }
}

/// Mask a secret string: show first 4 chars + asterisks.
fn mask_secret(s: &str) -> String {
    if s.len() <= 4 {
        "*".repeat(s.len())
    } else {
        format!("{}..{}", &s[..4], "*".repeat(8))
    }
}

// ── Data types ─────────────────────────────────────────────────────────────

use crate::core::voice::types::{
    ChatterboxConfig, CoquiConfig, DiaConfig, ElevenLabsConfig, FishAudioConfig,
    FishSpeechConfig, GptSoVitsConfig, OllamaConfig, OpenAIVoiceConfig, PiperConfig,
    XttsV2Config,
};
use crate::core::voice::queue::{JobStatus, SynthesisJob};

#[allow(dead_code)]
struct VoiceData {
    // Queue stats
    queue_pending: usize,
    queue_processing: usize,
    queue_completed: usize,
    queue_failed: usize,
    // Queue job lists
    pending_jobs: Vec<JobSnapshot>,
    processing_jobs: Vec<JobSnapshot>,
    history_jobs: Vec<JobSnapshot>,
    // Config
    provider: String,
    provider_id: String,
    cache_enabled: bool,
    max_concurrent: usize,
    // Per-provider configs (cloned from VoiceConfig)
    piper_config: Option<PiperConfig>,
    coqui_config: Option<CoquiConfig>,
    xtts_v2_config: Option<XttsV2Config>,
    gpt_sovits_config: Option<GptSoVitsConfig>,
    fish_speech_config: Option<FishSpeechConfig>,
    ollama_config: Option<OllamaConfig>,
    chatterbox_config: Option<ChatterboxConfig>,
    dia_config: Option<DiaConfig>,
    elevenlabs_config: Option<ElevenLabsConfig>,
    openai_config: Option<OpenAIVoiceConfig>,
    fish_audio_config: Option<FishAudioConfig>,
    // Profiles
    profiles: Vec<ProfileSnapshot>,
    profile_count: usize,
    preset_count: usize,
}

/// Lightweight snapshot of a synthesis job for display.
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct JobSnapshot {
    id_short: String,
    text_preview: String,
    status: String,
    priority: String,
    provider: String,
    age_secs: i64,
    progress: f32,
}

impl JobSnapshot {
    fn from_job(job: &SynthesisJob) -> Self {
        let text_preview: String = job.text.chars().take(40).collect();
        let text_preview = if job.text.chars().count() > 40 {
            format!("{}...", text_preview)
        } else {
            text_preview
        };
        let status = match &job.status {
            JobStatus::Pending => "Pending".to_string(),
            JobStatus::Processing => "Processing".to_string(),
            JobStatus::Completed => "Completed".to_string(),
            JobStatus::Failed(e) => format!("Failed: {}", e.chars().take(30).collect::<String>()),
            JobStatus::Canceled => "Canceled".to_string(),
        };
        Self {
            id_short: job.id.chars().take(8).collect(),
            text_preview,
            status,
            priority: format!("{}", job.priority),
            provider: format!("{:?}", job.provider),
            age_secs: job.age_seconds(),
            progress: job.progress.progress,
        }
    }
}

/// Lightweight snapshot of a voice profile for display.
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct ProfileSnapshot {
    id: String,
    name: String,
    provider: String,
    voice_id: String,
    is_preset: bool,
    linked_npcs: usize,
}

// ── Tab ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VoiceTab {
    Queue,
    Config,
    Profiles,
    Monitor,
}

const TAB_ORDER: [VoiceTab; 4] = [
    VoiceTab::Queue,
    VoiceTab::Config,
    VoiceTab::Profiles,
    VoiceTab::Monitor,
];

impl VoiceTab {
    fn label(self) -> &'static str {
        match self {
            Self::Queue => "Queue",
            Self::Config => "Config",
            Self::Profiles => "Profiles",
            Self::Monitor => "Monitor",
        }
    }

    fn next(self) -> Self {
        let idx = TAB_ORDER.iter().position(|t| *t == self).unwrap_or(0);
        TAB_ORDER[(idx + 1) % TAB_ORDER.len()]
    }

    fn prev(self) -> Self {
        let idx = TAB_ORDER.iter().position(|t| *t == self).unwrap_or(0);
        TAB_ORDER[(idx + TAB_ORDER.len() - 1) % TAB_ORDER.len()]
    }
}

// ── State ──────────────────────────────────────────────────────────────────

pub struct VoiceViewState {
    data: Option<VoiceData>,
    loading: bool,
    tab: VoiceTab,
    scroll: usize,
    /// Index of selected provider in Config tab (into ALL_PROVIDERS).
    config_provider_idx: usize,
    /// Index of selected profile in Profiles tab.
    profile_idx: usize,
    /// Sparkline buffer for Monitor tab (placeholder audio levels).
    sparkline_buf: Vec<u8>,
    /// Status message shown temporarily.
    status_msg: Option<String>,
    data_rx: mpsc::UnboundedReceiver<VoiceData>,
    data_tx: mpsc::UnboundedSender<VoiceData>,
}

impl VoiceViewState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            data: None,
            loading: false,
            tab: VoiceTab::Queue,
            scroll: 0,
            config_provider_idx: 0,
            profile_idx: 0,
            sparkline_buf: vec![0; 60],
            status_msg: None,
            data_rx,
            data_tx,
        }
    }

    pub fn load(&mut self, services: &Services) {
        self.loading = true;
        let tx = self.data_tx.clone();
        let voice_queue = services.voice.clone();
        let voice_mgr = services.voice_manager.clone();

        tokio::spawn(async move {
            let stats = voice_queue.stats().await;
            let pending_jobs = voice_queue.list_pending().await;
            let processing_jobs = voice_queue.list_processing().await;
            let history_jobs = voice_queue.list_history(Some(QUEUE_HISTORY_LIMIT)).await;

            let mgr = voice_mgr.read().await;
            let config = mgr.get_config();

            let provider = format!("{:?}", config.provider);
            let provider_id = provider_to_id(&config.provider);
            let cache_enabled = config.cache_dir.is_some();

            // Snapshot profiles (we cannot hold the lock across send)
            let profiles: Vec<ProfileSnapshot> = Vec::new();
            let profile_count = 0;
            let preset_count = 0;

            let data = VoiceData {
                queue_pending: stats.pending_count,
                queue_processing: stats.processing_count,
                queue_completed: stats.completed_count as usize,
                queue_failed: stats.failed_count as usize,
                pending_jobs: pending_jobs.iter().map(JobSnapshot::from_job).collect(),
                processing_jobs: processing_jobs
                    .iter()
                    .map(JobSnapshot::from_job)
                    .collect(),
                history_jobs: history_jobs.iter().map(JobSnapshot::from_job).collect(),
                provider,
                provider_id,
                cache_enabled,
                max_concurrent: stats.utilization as usize,
                // Clone per-provider configs
                piper_config: config.piper.clone(),
                coqui_config: config.coqui.clone(),
                xtts_v2_config: config.xtts_v2.clone(),
                gpt_sovits_config: config.gpt_sovits.clone(),
                fish_speech_config: config.fish_speech.clone(),
                ollama_config: config.ollama.clone(),
                chatterbox_config: config.chatterbox.clone(),
                dia_config: config.dia.clone(),
                elevenlabs_config: config.elevenlabs.clone(),
                openai_config: config.openai.clone(),
                fish_audio_config: config.fish_audio.clone(),
                profiles,
                profile_count,
                preset_count,
            };

            let _ = tx.send(data);
        });
    }

    pub fn poll(&mut self) {
        if let Ok(data) = self.data_rx.try_recv() {
            // Sync config_provider_idx to match loaded provider
            if let Some(idx) = ALL_PROVIDERS
                .iter()
                .position(|p| p.id == data.provider_id)
            {
                self.config_provider_idx = idx;
            }
            self.data = Some(data);
            self.loading = false;
        }
    }

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            ..
        }) = event
        else {
            return false;
        };

        match (*modifiers, *code) {
            // ── Tab navigation ──────────────────────────────────────
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.tab = self.tab.next();
                self.scroll = 0;
                true
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.tab = self.tab.prev();
                self.scroll = 0;
                true
            }
            // ── Refresh ─────────────────────────────────────────────
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }
            // ── Vertical navigation ─────────────────────────────────
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                match self.tab {
                    VoiceTab::Config => {
                        self.config_provider_idx =
                            (self.config_provider_idx + 1).min(ALL_PROVIDERS.len() - 1);
                    }
                    VoiceTab::Profiles => {
                        if let Some(ref data) = self.data {
                            if !data.profiles.is_empty() {
                                self.profile_idx =
                                    (self.profile_idx + 1).min(data.profiles.len() - 1);
                            }
                        }
                    }
                    _ => {
                        self.scroll = self.scroll.saturating_add(1);
                    }
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                match self.tab {
                    VoiceTab::Config => {
                        self.config_provider_idx =
                            self.config_provider_idx.saturating_sub(1);
                    }
                    VoiceTab::Profiles => {
                        self.profile_idx = self.profile_idx.saturating_sub(1);
                    }
                    _ => {
                        self.scroll = self.scroll.saturating_sub(1);
                    }
                }
                true
            }
            // ── Enter: select provider in Config tab ────────────────
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if self.tab == VoiceTab::Config {
                    self.status_msg = Some(format!(
                        "Selected: {} (save with Ctrl+S — not yet wired)",
                        ALL_PROVIDERS
                            .get(self.config_provider_idx)
                            .map(|p| p.label)
                            .unwrap_or("?")
                    ));
                    return true;
                }
                false
            }
            // ── Ctrl+S: save placeholder ────────────────────────────
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                self.status_msg =
                    Some("Save not yet implemented — provider config is read-only".to_string());
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // tabs
                Constraint::Min(0),    // body
                Constraint::Length(1), // status bar
            ])
            .split(area);

        self.render_tabs(frame, chunks[0]);

        match self.tab {
            VoiceTab::Queue => self.render_queue(frame, chunks[1]),
            VoiceTab::Config => self.render_config(frame, chunks[1]),
            VoiceTab::Profiles => self.render_profiles(frame, chunks[1]),
            VoiceTab::Monitor => self.render_monitor(frame, chunks[1]),
        }

        self.render_status_bar(frame, chunks[2]);
    }

    // ── Tab bar ──────────────────────────────────────────────────────────

    fn render_tabs(&self, frame: &mut Frame, area: Rect) {
        let spans: Vec<Span> = TAB_ORDER
            .iter()
            .flat_map(|t| {
                let style = if *t == self.tab {
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT_MUTED)
                };
                vec![
                    Span::styled(format!(" {} ", t.label()), style),
                    Span::raw("\u{2502}"),
                ]
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::TEXT_DIM));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(Paragraph::new(Line::from(spans)), inner);
    }

    // ── Status bar (bottom) ──────────────────────────────────────────────

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let msg = self.status_msg.as_deref().unwrap_or(
            "[Tab] switch  [j/k] navigate  [r] refresh  [Enter] select  [Ctrl+S] save",
        );
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(" {msg}"),
                Style::default().fg(theme::TEXT_DIM),
            ))),
            area,
        );
    }

    // ── Queue tab ────────────────────────────────────────────────────────

    fn render_queue(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Synthesis Queue");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading && self.data.is_none() {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Loading...",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        }

        let Some(ref data) = self.data else {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Press 'r' to load queue status",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        };

        let total =
            data.queue_pending + data.queue_processing + data.queue_completed + data.queue_failed;

        let mut lines: Vec<Line<'static>> = Vec::new();

        // ── Summary ──
        lines.push(Line::raw(""));
        lines.push(section_header("QUEUE SUMMARY"));
        lines.push(Line::raw(""));

        lines.push(stat_line("Pending", data.queue_pending, theme::WARNING));
        lines.push(stat_line("Processing", data.queue_processing, theme::INFO));
        lines.push(stat_line("Completed", data.queue_completed, theme::SUCCESS));
        lines.push(stat_line(
            "Failed",
            data.queue_failed,
            if data.queue_failed > 0 {
                theme::ERROR
            } else {
                theme::TEXT_DIM
            },
        ));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Total:      ",
                Style::default().fg(theme::TEXT_MUTED),
            ),
            Span::styled(
                format!("{total}"),
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Concurrent: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", data.max_concurrent),
                Style::default().fg(theme::TEXT),
            ),
        ]));

        // ── Processing jobs ──
        if !data.processing_jobs.is_empty() {
            lines.push(Line::raw(""));
            lines.push(section_header("PROCESSING"));
            lines.push(Line::raw(""));
            for job in &data.processing_jobs {
                lines.push(job_line(job, theme::INFO));
            }
        }

        // ── Pending jobs ──
        if !data.pending_jobs.is_empty() {
            lines.push(Line::raw(""));
            lines.push(section_header("PENDING"));
            lines.push(Line::raw(""));
            for job in &data.pending_jobs {
                lines.push(job_line(job, theme::WARNING));
            }
        }

        // ── Recent history ──
        if !data.history_jobs.is_empty() {
            lines.push(Line::raw(""));
            lines.push(section_header("RECENT HISTORY"));
            lines.push(Line::raw(""));
            for job in &data.history_jobs {
                let color = if job.status.starts_with("Failed") {
                    theme::ERROR
                } else if job.status == "Completed" {
                    theme::SUCCESS
                } else {
                    theme::TEXT_DIM
                };
                lines.push(job_line(job, color));
            }
        }

        // Apply scroll
        let visible: Vec<Line<'static>> = lines
            .into_iter()
            .skip(self.scroll)
            .collect();

        frame.render_widget(Paragraph::new(visible), inner);
    }

    // ── Config tab ───────────────────────────────────────────────────────

    fn render_config(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(28), Constraint::Percentage(60)])
            .split(area);

        self.render_provider_list(frame, chunks[0]);
        self.render_provider_detail(frame, chunks[1]);
    }

    fn render_provider_list(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Providers");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let active_id = self
            .data
            .as_ref()
            .map(|d| d.provider_id.as_str())
            .unwrap_or("");

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        for (i, entry) in ALL_PROVIDERS.iter().enumerate() {
            let is_selected = i == self.config_provider_idx;
            let is_active = entry.id == active_id;

            let marker = if is_active { "\u{25cf}" } else { "\u{25cb}" };
            let cursor = if is_selected { "\u{25b8} " } else { "  " };
            let locality = if entry.is_local { "local" } else { "cloud" };

            let label_style = if is_selected {
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else if is_active {
                Style::default()
                    .fg(theme::PRIMARY_LIGHT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {cursor}{marker} "),
                    if is_active {
                        Style::default().fg(theme::SUCCESS)
                    } else {
                        Style::default().fg(theme::TEXT_DIM)
                    },
                ),
                Span::styled(format!("{:<20}", entry.label), label_style),
                Span::styled(
                    format!(" ({locality})"),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        }

        frame.render_widget(Paragraph::new(lines), inner);
    }

    fn render_provider_detail(&self, frame: &mut Frame, area: Rect) {
        let entry = &ALL_PROVIDERS[self.config_provider_idx];
        let title = format!("{} Configuration", entry.label);
        let block = theme::block_focused(&title);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(ref data) = self.data else {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Press 'r' to load configuration",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        };

        let fields = config_fields_for_provider(entry.id, data);
        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));

        // Provider type badge
        let kind_label = if entry.is_local {
            "LOCAL"
        } else {
            "CLOUD"
        };
        let kind_color = if entry.is_local {
            theme::INFO
        } else {
            theme::NPC
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(" {kind_label} "),
                Style::default()
                    .fg(theme::BG_BASE)
                    .bg(kind_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                entry.label,
                Style::default()
                    .fg(theme::TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::raw(""));

        if fields.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No configuration required",
                Style::default().fg(theme::TEXT_MUTED),
            )));
        } else {
            // Find the longest label for alignment
            let max_label = fields.iter().map(|f| f.label.len()).max().unwrap_or(0);

            for field in &fields {
                let display_val = if field.value.is_empty() {
                    "(not set)".to_string()
                } else {
                    field.value.clone()
                };

                let val_style = if field.value.is_empty() {
                    Style::default().fg(theme::TEXT_DIM)
                } else if field.is_secret {
                    Style::default().fg(theme::WARNING)
                } else {
                    Style::default().fg(theme::PRIMARY_LIGHT)
                };

                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:>width$}: ", field.label, width = max_label),
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                    Span::styled(display_val, val_style),
                ]));
            }
        }

        // Active status
        let is_active = entry.id == data.provider_id;
        lines.push(Line::raw(""));
        if is_active {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "\u{2713} Active Provider",
                    Style::default()
                        .fg(theme::SUCCESS)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "Press [Enter] to select this provider",
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));
        }

        // Cache info
        lines.push(Line::raw(""));
        lines.push(section_header("CACHE"));
        lines.push(Line::raw(""));
        let cache_color = if data.cache_enabled {
            theme::SUCCESS
        } else {
            theme::TEXT_DIM
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Cache: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                if data.cache_enabled {
                    "Enabled"
                } else {
                    "Disabled"
                },
                Style::default()
                    .fg(cache_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        frame.render_widget(Paragraph::new(lines), inner);
    }

    // ── Profiles tab ─────────────────────────────────────────────────────

    fn render_profiles(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Voice Profiles");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let Some(ref data) = self.data else {
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    " Press 'r' to load profiles",
                    Style::default().fg(theme::TEXT_MUTED),
                ))),
                inner,
            );
            return;
        };

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));
        lines.push(section_header("VOICE PROFILE MANAGER"));
        lines.push(Line::raw(""));

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("User Profiles: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", data.profile_count),
                Style::default().fg(theme::TEXT),
            ),
            Span::raw("    "),
            Span::styled("Presets: ", Style::default().fg(theme::TEXT_MUTED)),
            Span::styled(
                format!("{}", data.preset_count),
                Style::default().fg(theme::TEXT),
            ),
        ]));
        lines.push(Line::raw(""));

        if data.profiles.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No voice profiles configured yet.",
                Style::default().fg(theme::TEXT_DIM),
            )));
            lines.push(Line::raw(""));
            lines.push(Line::from(Span::styled(
                "  Voice profiles map character names to voice IDs.",
                Style::default().fg(theme::TEXT_DIM),
            )));
            lines.push(Line::from(Span::styled(
                "  Profile management will be wired in a future update.",
                Style::default().fg(theme::TEXT_DIM),
            )));
        } else {
            // Table header
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!(
                        "{:<20} {:<14} {:<20} {:>5}",
                        "Name", "Provider", "Voice ID", "NPCs"
                    ),
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::UNDERLINED),
                ),
            ]));
            lines.push(Line::raw(""));

            for (i, profile) in data.profiles.iter().enumerate() {
                let is_selected = i == self.profile_idx;
                let cursor = if is_selected { "\u{25b8} " } else { "  " };
                let preset_tag = if profile.is_preset { " [P]" } else { "" };

                let style = if is_selected {
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT)
                };

                let voice_short: String =
                    profile.voice_id.chars().take(18).collect();

                lines.push(Line::from(vec![
                    Span::styled(cursor.to_string(), style),
                    Span::styled(
                        format!(
                            "{:<20} {:<14} {:<20} {:>5}{}",
                            truncate_str(&profile.name, 18),
                            truncate_str(&profile.provider, 12),
                            voice_short,
                            profile.linked_npcs,
                            preset_tag,
                        ),
                        style,
                    ),
                ]));
            }
        }

        // Placeholder instructions
        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  [j/k] navigate  [a] add  [d] delete  (placeholder — read-only)",
            Style::default().fg(theme::TEXT_DIM),
        )));

        frame.render_widget(Paragraph::new(lines), inner);
    }

    // ── Monitor tab ──────────────────────────────────────────────────────

    fn render_monitor(&self, frame: &mut Frame, area: Rect) {
        let block = theme::block_focused("Audio Monitor");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::raw(""));
        lines.push(section_header("AUDIO LEVEL MONITOR"));
        lines.push(Line::raw(""));

        // Build a sparkline visualization from the buffer
        let bar_chars = [' ', '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}'];
        let sparkline_width = inner.width.saturating_sub(6) as usize;
        let buf_len = self.sparkline_buf.len();
        let start = if buf_len > sparkline_width {
            buf_len - sparkline_width
        } else {
            0
        };

        let spark: String = self.sparkline_buf[start..]
            .iter()
            .map(|&v| {
                let idx = (v as usize).min(bar_chars.len() - 1);
                bar_chars[idx]
            })
            .collect();

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(spark, Style::default().fg(theme::PRIMARY_LIGHT)),
        ]));
        lines.push(Line::raw(""));

        // dB label row
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "-60 dB",
                Style::default().fg(theme::TEXT_DIM),
            ),
            Span::raw(
                " ".repeat(sparkline_width.saturating_sub(16).max(1)),
            ),
            Span::styled("0 dB", Style::default().fg(theme::TEXT_DIM)),
        ]));

        lines.push(Line::raw(""));
        lines.push(section_header("STATUS"));
        lines.push(Line::raw(""));

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Audio Output: ",
                Style::default().fg(theme::TEXT_MUTED),
            ),
            Span::styled(
                "Idle",
                Style::default().fg(theme::TEXT_DIM),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Sample Rate:  ",
                Style::default().fg(theme::TEXT_MUTED),
            ),
            Span::styled(
                "44100 Hz",
                Style::default().fg(theme::TEXT),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "Channels:     ",
                Style::default().fg(theme::TEXT_MUTED),
            ),
            Span::styled("Stereo", Style::default().fg(theme::TEXT)),
        ]));

        lines.push(Line::raw(""));
        lines.push(Line::from(Span::styled(
            "  Audio monitoring will be wired to the playback pipeline in a future update.",
            Style::default().fg(theme::TEXT_DIM),
        )));

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Section header: bold accent line.
fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {title}"),
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Stats row: label + colored value.
fn stat_line(label: &str, value: usize, color: ratatui::style::Color) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<12}", format!("{label}:")),
            Style::default().fg(theme::TEXT_MUTED),
        ),
        Span::styled(format!("{value}"), Style::default().fg(color)),
    ])
}

/// Format a single job line.
fn job_line(job: &JobSnapshot, color: ratatui::style::Color) -> Line<'static> {
    let progress_pct = (job.progress * 100.0) as u8;
    let age = format_age(job.age_secs);
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("[{}] ", job.id_short),
            Style::default().fg(theme::TEXT_DIM),
        ),
        Span::styled(
            format!("{:<42} ", job.text_preview),
            Style::default().fg(theme::TEXT),
        ),
        Span::styled(
            format!("{:<12} ", job.status),
            Style::default().fg(color),
        ),
        Span::styled(
            format!("{:>3}% ", progress_pct),
            Style::default().fg(theme::TEXT_MUTED),
        ),
        Span::styled(age, Style::default().fg(theme::TEXT_DIM)),
    ])
}

/// Human-readable age from seconds.
fn format_age(secs: i64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

/// Truncate a string to max chars with ellipsis.
fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}\u{2026}")
    }
}

/// Map VoiceProviderType to the string ID used in ALL_PROVIDERS.
fn provider_to_id(
    provider: &crate::core::voice::types::VoiceProviderType,
) -> String {
    use crate::core::voice::types::VoiceProviderType;
    match provider {
        VoiceProviderType::Piper => "piper",
        VoiceProviderType::Coqui => "coqui",
        VoiceProviderType::XttsV2 => "xtts_v2",
        VoiceProviderType::GptSoVits => "gpt_sovits",
        VoiceProviderType::FishSpeech => "fish_speech",
        VoiceProviderType::Ollama => "ollama",
        VoiceProviderType::Chatterbox => "chatterbox",
        VoiceProviderType::Dia => "dia",
        VoiceProviderType::ElevenLabs => "elevenlabs",
        VoiceProviderType::OpenAI => "openai",
        VoiceProviderType::FishAudio => "fish_audio",
        VoiceProviderType::System => "system",
        VoiceProviderType::Disabled => "disabled",
    }
    .to_string()
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = VoiceViewState::new();
        assert!(state.data.is_none());
        assert!(!state.loading);
        assert_eq!(state.tab, VoiceTab::Queue);
        assert_eq!(state.config_provider_idx, 0);
        assert_eq!(state.profile_idx, 0);
        assert_eq!(state.sparkline_buf.len(), 60);
    }

    #[test]
    fn test_tab_cycling_forward() {
        assert_eq!(VoiceTab::Queue.next(), VoiceTab::Config);
        assert_eq!(VoiceTab::Config.next(), VoiceTab::Profiles);
        assert_eq!(VoiceTab::Profiles.next(), VoiceTab::Monitor);
        assert_eq!(VoiceTab::Monitor.next(), VoiceTab::Queue);
    }

    #[test]
    fn test_tab_cycling_backward() {
        assert_eq!(VoiceTab::Queue.prev(), VoiceTab::Monitor);
        assert_eq!(VoiceTab::Monitor.prev(), VoiceTab::Profiles);
        assert_eq!(VoiceTab::Profiles.prev(), VoiceTab::Config);
        assert_eq!(VoiceTab::Config.prev(), VoiceTab::Queue);
    }

    #[test]
    fn test_tab_labels() {
        assert_eq!(VoiceTab::Queue.label(), "Queue");
        assert_eq!(VoiceTab::Config.label(), "Config");
        assert_eq!(VoiceTab::Profiles.label(), "Profiles");
        assert_eq!(VoiceTab::Monitor.label(), "Monitor");
    }

    #[test]
    fn test_mask_secret_short() {
        assert_eq!(mask_secret("ab"), "**");
        assert_eq!(mask_secret("abcd"), "****");
    }

    #[test]
    fn test_mask_secret_long() {
        let masked = mask_secret("sk-1234567890abcdef");
        assert!(masked.starts_with("sk-1"));
        assert!(masked.contains("*"));
        assert!(!masked.contains("567890"));
    }

    #[test]
    fn test_format_age() {
        assert_eq!(format_age(5), "5s");
        assert_eq!(format_age(59), "59s");
        assert_eq!(format_age(60), "1m");
        assert_eq!(format_age(3599), "59m");
        assert_eq!(format_age(3600), "1h");
        assert_eq!(format_age(7200), "2h");
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world this is long", 10), "hello wor\u{2026}");
    }

    #[test]
    fn test_provider_to_id_roundtrip() {
        use crate::core::voice::types::VoiceProviderType;
        let providers = vec![
            VoiceProviderType::Piper,
            VoiceProviderType::Coqui,
            VoiceProviderType::XttsV2,
            VoiceProviderType::GptSoVits,
            VoiceProviderType::FishSpeech,
            VoiceProviderType::Ollama,
            VoiceProviderType::Chatterbox,
            VoiceProviderType::Dia,
            VoiceProviderType::ElevenLabs,
            VoiceProviderType::OpenAI,
            VoiceProviderType::FishAudio,
        ];
        for p in providers {
            let id = provider_to_id(&p);
            // Verify each ID maps to a known ALL_PROVIDERS entry
            assert!(
                ALL_PROVIDERS.iter().any(|e| e.id == id),
                "provider_to_id({:?}) = '{}' not found in ALL_PROVIDERS",
                p,
                id,
            );
        }
    }

    #[test]
    fn test_all_providers_has_11_entries() {
        assert_eq!(ALL_PROVIDERS.len(), 11);
    }

    #[test]
    fn test_job_snapshot_truncates_text() {
        use crate::core::voice::types::VoiceProviderType;
        let job = SynthesisJob::new(
            "This is a very long piece of text that should be truncated in the snapshot display",
            "profile-1",
            VoiceProviderType::Piper,
            "en_US-amy-medium",
        );
        let snap = JobSnapshot::from_job(&job);
        // 40 chars + "..."
        assert!(snap.text_preview.chars().count() <= 43);
        assert!(snap.text_preview.ends_with("..."));
        assert_eq!(snap.id_short.len(), 8);
    }
}
