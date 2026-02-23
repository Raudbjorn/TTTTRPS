//! Settings view — displays LLM provider status, router config, costs, and app config.
//!
//! Supports adding, editing, and deleting LLM providers via a modal form overlay.
//! Press `a` to add, `e` to edit, `d` to delete. Press `r` to refresh data.
//!
//! Providers use four authentication methods:
//! - **ApiKey**: standard API key input (OpenAI, Anthropic, Google, etc.)
//! - **HostOnly**: host URL only (Ollama)
//! - **OAuthPkce**: browser-based PKCE flow (Claude, Gemini)
//! - **DeviceCode**: GitHub Device Code flow (Copilot)

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tokio::sync::mpsc;

use crate::config::AppConfig;
use crate::core::credentials::mask_api_key;
use crate::core::llm::providers::{
    AuthMethod, ProviderConfig, ProviderMeta, PROVIDERS, find_provider_meta,
};
use crate::tui::events::{AppEvent, DeviceFlowUpdateKind};
use crate::tui::services::Services;
use crate::tui::widgets::input_buffer::InputBuffer;

// ── Form field types ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct FormField {
    label: &'static str,
    placeholder: &'static str,
    is_secret: bool,
}

fn fields_for_provider(meta: &ProviderMeta) -> Vec<FormField> {
    let mut fields = Vec::new();
    match meta.auth_method {
        AuthMethod::ApiKey => {
            fields.push(FormField {
                label: "API Key",
                placeholder: meta.key_placeholder,
                is_secret: true,
            });
        }
        AuthMethod::HostOnly => {
            fields.push(FormField {
                label: "Host",
                placeholder: "http://localhost:11434",
                is_secret: false,
            });
        }
        AuthMethod::OAuthPkce | AuthMethod::DeviceCode => {
            // OAuth/Device providers only need a model field
        }
    }
    fields.push(FormField {
        label: "Model",
        placeholder: meta.default_model,
        is_secret: false,
    });
    fields
}

// ── OAuth flow phases ───────────────────────────────────────────────────────

#[derive(Debug)]
enum OAuthPkcePhase {
    /// Browser opened — waiting for user to paste authorization code.
    WaitingForCode { auth_url: String },
    /// Exchanging code for token.
    Exchanging,
    /// Flow completed successfully.
    Success,
    /// Flow failed.
    Error(String),
}

#[derive(Debug)]
enum DeviceCodePhase {
    /// Waiting for user to authorize at verification_uri.
    WaitingForUser {
        user_code: String,
        verification_uri: String,
    },
    /// Got GitHub token, exchanging for Copilot token.
    Completing,
    /// Flow completed successfully.
    Success,
    /// Flow failed.
    Error(String),
}

// ── Modal state ─────────────────────────────────────────────────────────────

#[derive(Debug)]
enum SettingsModal {
    /// Step 1: Picking a provider from the list.
    SelectProvider { selected: usize },
    /// Step 2: Configuring the selected provider (API key / host + model).
    ConfigureProvider {
        meta: &'static ProviderMeta,
        fields: Vec<FormField>,
        focused_field: usize,
        field_values: Vec<String>,
        error: Option<String>,
    },
    /// OAuth PKCE flow modal (Claude/Gemini).
    OAuthPkceFlow {
        provider_id: String,
        display_name: String,
        phase: OAuthPkcePhase,
    },
    /// Device Code flow modal (Copilot).
    DeviceCodeFlow {
        provider_id: String,
        display_name: String,
        phase: DeviceCodePhase,
    },
    /// Confirm deletion.
    ConfirmDelete {
        provider_id: String,
        provider_name: String,
    },
}

// ── Display types ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct ProviderDisplayInfo {
    name: String,
    model: String,
    is_healthy: bool,
    circuit_state: String,
    uptime: f64,
    total_requests: u64,
    success_rate: f64,
    avg_latency_ms: u64,
    total_cost: f64,
}

#[derive(Clone, Debug)]
struct SettingsData {
    providers: Vec<ProviderDisplayInfo>,
    routing_strategy: String,
    request_timeout_secs: u64,
    health_check_interval_secs: u64,
    max_retries: u32,
    enable_fallback: bool,
    stream_chunk_timeout_secs: u64,
    // Cost
    total_cost: f64,
    monthly_cost: f64,
    daily_cost: f64,
    monthly_budget: Option<f64>,
    daily_budget: Option<f64>,
    within_budget: bool,
    // Health overview
    total_providers: usize,
    healthy_count: usize,
    unhealthy_count: usize,
    avg_uptime: f64,
    // App config
    data_dir: String,
    tick_rate_ms: u64,
    mouse_enabled: bool,
    theme: String,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct SettingsState {
    data: Option<SettingsData>,
    lines_cache: Vec<Line<'static>>,
    scroll: usize,
    /// Selected provider row index in the provider table.
    selected_provider: usize,
    loading: bool,
    modal: Option<SettingsModal>,
    /// Input buffer for the active modal form field.
    input: InputBuffer,
    data_rx: mpsc::UnboundedReceiver<SettingsData>,
    data_tx: mpsc::UnboundedSender<SettingsData>,
}

impl SettingsState {
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::unbounded_channel();
        Self {
            data: None,
            lines_cache: Vec::new(),
            scroll: 0,
            selected_provider: 0,
            loading: false,
            modal: None,
            input: InputBuffer::new(),
            data_rx,
            data_tx,
        }
    }

    /// Whether a modal is currently open (blocks global keybindings).
    pub fn has_modal(&self) -> bool {
        self.modal.is_some()
    }

    /// Trigger async data load from backend services.
    pub fn load(&mut self, services: &Services) {
        if self.loading {
            return;
        }
        self.loading = true;

        let llm = services.llm.clone();
        let tx = self.data_tx.clone();

        // Sync data captured before spawn
        let provider_ids = llm.provider_ids();
        let strategy = format!("{:?}", llm.routing_strategy());
        let config = llm.config().clone();

        // App config (sync — re-reads from disk, cheap)
        let app_config = AppConfig::load();
        let data_dir = app_config.data_dir().display().to_string();
        let tick_rate_ms = app_config.tui.tick_rate_ms;
        let mouse_enabled = app_config.tui.mouse_enabled;
        let theme = app_config.tui.theme.clone();

        tokio::spawn(async move {
            let all_stats = llm.get_all_stats().await;
            let all_health = llm.get_all_health().await;
            let cost_summary = llm.get_cost_summary().await;
            let health_summary = llm.get_health_summary().await;

            let mut providers = Vec::with_capacity(provider_ids.len());
            for id in &provider_ids {
                let provider = llm.get_provider(id);
                let stats = all_stats.get(id);
                let health = all_health.get(id);

                providers.push(ProviderDisplayInfo {
                    name: provider
                        .as_ref()
                        .map(|p| p.name().to_string())
                        .unwrap_or_else(|| id.clone()),
                    model: provider
                        .as_ref()
                        .map(|p| p.model().to_string())
                        .unwrap_or_default(),
                    is_healthy: health.map(|h| h.is_healthy).unwrap_or(false),
                    circuit_state: health
                        .map(|h| format!("{:?}", h.circuit_state))
                        .unwrap_or_else(|| "Unknown".to_string()),
                    uptime: health.map(|h| h.uptime_percentage).unwrap_or(0.0),
                    total_requests: stats.map(|s| s.total_requests).unwrap_or(0),
                    success_rate: stats.map(|s| s.success_rate()).unwrap_or(0.0),
                    avg_latency_ms: stats.map(|s| s.avg_latency_ms()).unwrap_or(0),
                    total_cost: stats.map(|s| s.total_cost_usd).unwrap_or(0.0),
                });
            }

            let data = SettingsData {
                providers,
                routing_strategy: strategy,
                request_timeout_secs: config.request_timeout.as_secs(),
                health_check_interval_secs: config.health_check_interval.as_secs(),
                max_retries: config.max_retries,
                enable_fallback: config.enable_fallback,
                stream_chunk_timeout_secs: config.stream_chunk_timeout.as_secs(),
                total_cost: cost_summary.total_cost_usd,
                monthly_cost: cost_summary.monthly_cost,
                daily_cost: cost_summary.daily_cost,
                monthly_budget: cost_summary.monthly_budget,
                daily_budget: cost_summary.daily_budget,
                within_budget: cost_summary.is_within_budget,
                total_providers: health_summary.total_providers,
                healthy_count: health_summary.healthy_providers,
                unhealthy_count: health_summary.unhealthy_providers,
                avg_uptime: health_summary.average_uptime_percentage,
                data_dir,
                tick_rate_ms,
                mouse_enabled,
                theme,
            };

            let _ = tx.send(data);
        });
    }

    /// Poll for async data completion. Call from on_tick.
    pub fn poll(&mut self) {
        if let Ok(data) = self.data_rx.try_recv() {
            self.lines_cache = build_lines(&data);
            self.data = Some(data);
            self.loading = false;
        }
    }

    // ── Modal openers (called from app.rs handle_action) ──────────────

    pub fn open_add_modal(&mut self) {
        self.modal = Some(SettingsModal::SelectProvider { selected: 0 });
    }

    pub fn open_edit_modal(&mut self, provider_id: &str, services: &Services) {
        let Some(meta) = find_provider_meta(provider_id) else {
            return;
        };
        let fields = fields_for_provider(meta);

        // Pre-fill values from existing credential
        let mut field_values: Vec<String> = fields
            .iter()
            .map(|f| {
                if f.label == "Model" {
                    // Get model from router
                    services
                        .llm
                        .get_provider(provider_id)
                        .map(|p| p.model().to_string())
                        .unwrap_or_else(|| meta.default_model.to_string())
                } else {
                    String::new()
                }
            })
            .collect();

        // Fill host for Ollama from existing credential
        if meta.needs_host() {
            if let Ok(cred) = services.credentials.get_llm_credential(provider_id) {
                if let Some(host) = cred.host {
                    if let Some(idx) = fields.iter().position(|f| f.label == "Host") {
                        field_values[idx] = host;
                    }
                }
            }
        }

        self.input.clear();
        if !field_values.is_empty() && !field_values[0].is_empty() {
            // Pre-fill the input buffer with the first field's value
            for c in field_values[0].chars() {
                self.input.insert_char(c);
            }
        }

        self.modal = Some(SettingsModal::ConfigureProvider {
            meta,
            fields,
            focused_field: 0,
            field_values,
            error: None,
        });
    }

    pub fn open_delete_modal(&mut self, provider_id: &str) {
        let display_name = find_provider_meta(provider_id)
            .map(|m| m.display_name)
            .unwrap_or(provider_id);
        self.modal = Some(SettingsModal::ConfirmDelete {
            provider_id: provider_id.to_string(),
            provider_name: display_name.to_string(),
        });
    }

    // ── OAuth event handler (called from app.rs) ──────────────────────

    pub fn handle_oauth_event(&mut self, event: &AppEvent, services: &Services) {
        match event {
            AppEvent::OAuthFlowResult { provider_id, result } => {
                match result {
                    Ok(auth_url) => {
                        // PKCE flow started — show the code-paste modal
                        let display_name = find_provider_meta(provider_id)
                            .map(|m| m.display_name.to_string())
                            .unwrap_or_else(|| provider_id.clone());
                        self.input.clear();
                        self.modal = Some(SettingsModal::OAuthPkceFlow {
                            provider_id: provider_id.clone(),
                            display_name,
                            phase: OAuthPkcePhase::WaitingForCode {
                                auth_url: auth_url.clone(),
                            },
                        });
                    }
                    Err(msg) => {
                        let display_name = find_provider_meta(provider_id)
                            .map(|m| m.display_name.to_string())
                            .unwrap_or_else(|| provider_id.clone());
                        self.modal = Some(SettingsModal::OAuthPkceFlow {
                            provider_id: provider_id.clone(),
                            display_name,
                            phase: OAuthPkcePhase::Error(msg.clone()),
                        });
                    }
                }
            }
            AppEvent::DeviceFlowUpdate { provider_id, update } => {
                let display_name = find_provider_meta(provider_id)
                    .map(|m| m.display_name.to_string())
                    .unwrap_or_else(|| provider_id.clone());
                match update {
                    DeviceFlowUpdateKind::Started {
                        user_code,
                        verification_uri,
                    } => {
                        self.modal = Some(SettingsModal::DeviceCodeFlow {
                            provider_id: provider_id.clone(),
                            display_name,
                            phase: DeviceCodePhase::WaitingForUser {
                                user_code: user_code.clone(),
                                verification_uri: verification_uri.clone(),
                            },
                        });
                    }
                    DeviceFlowUpdateKind::Polling => {
                        // Keep current modal (just re-renders the spinner)
                    }
                    DeviceFlowUpdateKind::Completing => {
                        self.modal = Some(SettingsModal::DeviceCodeFlow {
                            provider_id: provider_id.clone(),
                            display_name,
                            phase: DeviceCodePhase::Completing,
                        });
                    }
                    DeviceFlowUpdateKind::Complete => {
                        self.modal = Some(SettingsModal::DeviceCodeFlow {
                            provider_id: provider_id.clone(),
                            display_name,
                            phase: DeviceCodePhase::Success,
                        });
                        self.load(services);
                    }
                    DeviceFlowUpdateKind::Error(msg) => {
                        self.modal = Some(SettingsModal::DeviceCodeFlow {
                            provider_id: provider_id.clone(),
                            display_name,
                            phase: DeviceCodePhase::Error(msg.clone()),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    // ── Input ─────────────────────────────────────────────────────────

    pub fn handle_input(&mut self, event: &Event, services: &Services) -> bool {
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        else {
            return false;
        };

        // If a modal is open, route input there
        if self.modal.is_some() {
            return self.handle_modal_input(*code, *modifiers, services);
        }

        // Normal settings view input
        match (*modifiers, *code) {
            (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                self.scroll_down(1);
                self.advance_provider_selection(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                self.scroll_up(1);
                self.advance_provider_selection(-1);
                true
            }
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                self.scroll = self.lines_cache.len().saturating_sub(1);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                self.scroll = 0;
                true
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.scroll_down(15);
                true
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.scroll_up(15);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('r')) => {
                self.load(services);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('a')) => {
                self.open_add_modal();
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('e')) => {
                if let Some(id) = self.selected_provider_id() {
                    self.open_edit_modal(&id, services);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Char('d')) => {
                if let Some(id) = self.selected_provider_id() {
                    self.open_delete_modal(&id);
                }
                true
            }
            _ => false,
        }
    }

    fn handle_modal_input(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        services: &Services,
    ) -> bool {
        // Take ownership of modal to work with it
        let modal = self.modal.take().unwrap();

        match modal {
            SettingsModal::SelectProvider { selected } => {
                match (modifiers, code) {
                    (KeyModifiers::NONE, KeyCode::Esc) => {
                        // Close modal
                    }
                    (KeyModifiers::NONE, KeyCode::Char('j') | KeyCode::Down) => {
                        let new = (selected + 1).min(PROVIDERS.len() - 1);
                        self.modal = Some(SettingsModal::SelectProvider { selected: new });
                    }
                    (KeyModifiers::NONE, KeyCode::Char('k') | KeyCode::Up) => {
                        let new = selected.saturating_sub(1);
                        self.modal = Some(SettingsModal::SelectProvider { selected: new });
                    }
                    (KeyModifiers::NONE, KeyCode::Enter) => {
                        let meta = &PROVIDERS[selected];
                        match meta.auth_method {
                            AuthMethod::OAuthPkce => {
                                // Save model config then start OAuth flow
                                self.start_oauth_pkce_flow(meta, services);
                            }
                            AuthMethod::DeviceCode => {
                                // Save model config then start device code flow
                                self.start_device_code_flow(meta, services);
                            }
                            AuthMethod::ApiKey | AuthMethod::HostOnly => {
                                // Move to configure step
                                let fields = fields_for_provider(meta);
                                let mut field_values: Vec<String> =
                                    fields.iter().map(|_| String::new()).collect();

                                // Pre-fill default host for Ollama
                                if meta.needs_host() {
                                    if let Some(idx) =
                                        fields.iter().position(|f| f.label == "Host")
                                    {
                                        field_values[idx] =
                                            "http://localhost:11434".to_string();
                                    }
                                }
                                // Pre-fill default model
                                if let Some(idx) =
                                    fields.iter().position(|f| f.label == "Model")
                                {
                                    field_values[idx] = meta.default_model.to_string();
                                }

                                self.input.clear();
                                // Load first field value into input
                                for c in field_values[0].chars() {
                                    self.input.insert_char(c);
                                }

                                self.modal = Some(SettingsModal::ConfigureProvider {
                                    meta,
                                    fields,
                                    focused_field: 0,
                                    field_values,
                                    error: None,
                                });
                            }
                        }
                    }
                    _ => {
                        self.modal = Some(SettingsModal::SelectProvider { selected });
                    }
                }
            }
            SettingsModal::ConfigureProvider {
                meta,
                fields,
                focused_field,
                mut field_values,
                error: _,
            } => {
                match (modifiers, code) {
                    (KeyModifiers::NONE, KeyCode::Esc) => {
                        // Close modal
                    }
                    (KeyModifiers::NONE, KeyCode::Tab)
                    | (KeyModifiers::NONE, KeyCode::Down) => {
                        // Save current input to field values
                        field_values[focused_field] = self.input.take();
                        let next = (focused_field + 1) % fields.len();
                        // Load next field value
                        self.input.clear();
                        for c in field_values[next].chars() {
                            self.input.insert_char(c);
                        }
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field: next,
                            field_values,
                            error: None,
                        });
                    }
                    (KeyModifiers::SHIFT, KeyCode::BackTab) | (KeyModifiers::NONE, KeyCode::Up) => {
                        field_values[focused_field] = self.input.take();
                        let prev = if focused_field == 0 {
                            fields.len() - 1
                        } else {
                            focused_field - 1
                        };
                        self.input.clear();
                        for c in field_values[prev].chars() {
                            self.input.insert_char(c);
                        }
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field: prev,
                            field_values,
                            error: None,
                        });
                    }
                    (KeyModifiers::NONE, KeyCode::Enter) => {
                        // Save current field, then attempt to save provider
                        field_values[focused_field] = self.input.take();
                        match self.save_provider(meta, &fields, &field_values, services) {
                            Ok(()) => {
                                // Success — close modal, refresh
                                self.load(services);
                            }
                            Err(err) => {
                                // Show error, stay in modal
                                // Reload input with current field
                                for c in field_values[focused_field].chars() {
                                    self.input.insert_char(c);
                                }
                                self.modal = Some(SettingsModal::ConfigureProvider {
                                    meta,
                                    fields,
                                    focused_field,
                                    field_values,
                                    error: Some(err),
                                });
                            }
                        }
                    }
                    // Text input
                    (KeyModifiers::NONE, KeyCode::Char(c)) => {
                        self.input.insert_char(c);
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field,
                            field_values,
                            error: None,
                        });
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                        self.input.insert_char(c);
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field,
                            field_values,
                            error: None,
                        });
                    }
                    (KeyModifiers::NONE, KeyCode::Backspace) => {
                        self.input.backspace();
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field,
                            field_values,
                            error: None,
                        });
                    }
                    (KeyModifiers::NONE, KeyCode::Delete) => {
                        self.input.delete();
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field,
                            field_values,
                            error: None,
                        });
                    }
                    (KeyModifiers::NONE, KeyCode::Left) => {
                        self.input.move_left();
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field,
                            field_values,
                            error: None,
                        });
                    }
                    (KeyModifiers::NONE, KeyCode::Right) => {
                        self.input.move_right();
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field,
                            field_values,
                            error: None,
                        });
                    }
                    (KeyModifiers::NONE, KeyCode::Home) => {
                        self.input.move_home();
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field,
                            field_values,
                            error: None,
                        });
                    }
                    (KeyModifiers::NONE, KeyCode::End) => {
                        self.input.move_end();
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field,
                            field_values,
                            error: None,
                        });
                    }
                    _ => {
                        self.modal = Some(SettingsModal::ConfigureProvider {
                            meta,
                            fields,
                            focused_field,
                            field_values,
                            error: None,
                        });
                    }
                }
            }
            SettingsModal::OAuthPkceFlow {
                provider_id,
                display_name,
                phase,
            } => {
                match (modifiers, code) {
                    (KeyModifiers::NONE, KeyCode::Esc) => {
                        // Close modal (cancel flow)
                    }
                    (KeyModifiers::NONE, KeyCode::Enter) => {
                        match &phase {
                            OAuthPkcePhase::WaitingForCode { .. } => {
                                // User pasted code and hit Enter — exchange it
                                let auth_code = self.input.take();
                                if auth_code.is_empty() {
                                    self.modal = Some(SettingsModal::OAuthPkceFlow {
                                        provider_id,
                                        display_name,
                                        phase: OAuthPkcePhase::WaitingForCode {
                                            auth_url: String::new(),
                                        },
                                    });
                                } else {
                                    self.modal = Some(SettingsModal::OAuthPkceFlow {
                                        provider_id: provider_id.clone(),
                                        display_name: display_name.clone(),
                                        phase: OAuthPkcePhase::Exchanging,
                                    });
                                    self.complete_oauth_pkce(
                                        &provider_id,
                                        &auth_code,
                                        services,
                                    );
                                }
                            }
                            OAuthPkcePhase::Success | OAuthPkcePhase::Error(_) => {
                                // Close on Enter after completion
                                self.load(services);
                            }
                            OAuthPkcePhase::Exchanging => {
                                // Still exchanging, ignore
                                self.modal = Some(SettingsModal::OAuthPkceFlow {
                                    provider_id,
                                    display_name,
                                    phase,
                                });
                            }
                        }
                    }
                    // Text input for code paste
                    (KeyModifiers::NONE, KeyCode::Char(c))
                        if matches!(phase, OAuthPkcePhase::WaitingForCode { .. }) =>
                    {
                        self.input.insert_char(c);
                        self.modal = Some(SettingsModal::OAuthPkceFlow {
                            provider_id,
                            display_name,
                            phase,
                        });
                    }
                    (KeyModifiers::SHIFT, KeyCode::Char(c))
                        if matches!(phase, OAuthPkcePhase::WaitingForCode { .. }) =>
                    {
                        self.input.insert_char(c);
                        self.modal = Some(SettingsModal::OAuthPkceFlow {
                            provider_id,
                            display_name,
                            phase,
                        });
                    }
                    (KeyModifiers::NONE, KeyCode::Backspace)
                        if matches!(phase, OAuthPkcePhase::WaitingForCode { .. }) =>
                    {
                        self.input.backspace();
                        self.modal = Some(SettingsModal::OAuthPkceFlow {
                            provider_id,
                            display_name,
                            phase,
                        });
                    }
                    _ => {
                        self.modal = Some(SettingsModal::OAuthPkceFlow {
                            provider_id,
                            display_name,
                            phase,
                        });
                    }
                }
            }
            SettingsModal::DeviceCodeFlow {
                provider_id,
                display_name,
                phase,
            } => {
                match (modifiers, code) {
                    (KeyModifiers::NONE, KeyCode::Esc) => {
                        // Close modal (cancel flow — polling will just stop)
                    }
                    (KeyModifiers::NONE, KeyCode::Enter)
                        if matches!(
                            phase,
                            DeviceCodePhase::Success | DeviceCodePhase::Error(_)
                        ) =>
                    {
                        // Close on Enter after completion
                        self.load(services);
                    }
                    _ => {
                        self.modal = Some(SettingsModal::DeviceCodeFlow {
                            provider_id,
                            display_name,
                            phase,
                        });
                    }
                }
            }
            SettingsModal::ConfirmDelete {
                provider_id,
                provider_name,
            } => {
                match (modifiers, code) {
                    (KeyModifiers::NONE, KeyCode::Char('y') | KeyCode::Char('Y')) => {
                        self.delete_provider(&provider_id, services);
                        self.load(services);
                    }
                    (KeyModifiers::NONE, KeyCode::Char('n'))
                    | (KeyModifiers::NONE, KeyCode::Esc) => {
                        // Close modal
                    }
                    _ => {
                        self.modal = Some(SettingsModal::ConfirmDelete {
                            provider_id,
                            provider_name,
                        });
                    }
                }
            }
        }

        true // Modal always consumes input
    }

    // ── OAuth flow initiators ────────────────────────────────────────

    fn start_oauth_pkce_flow(&mut self, meta: &'static ProviderMeta, services: &Services) {
        let model = meta.default_model.to_string();
        let provider_id = meta.id.to_string();

        // Persist config.toml (OAuth providers don't store secrets in keyring)
        if let Err(e) = services.save_oauth_provider(meta.id, &model) {
            log::error!("Failed to save config for OAuth provider: {e}");
        }

        // Start the OAuth flow
        let event_tx = services.event_tx.clone();
        let pid = provider_id.clone();

        // Show a temporary "starting..." modal
        self.modal = Some(SettingsModal::OAuthPkceFlow {
            provider_id: provider_id.clone(),
            display_name: meta.display_name.to_string(),
            phase: OAuthPkcePhase::Exchanging, // reuse as "starting"
        });

        tokio::spawn(async move {
            use crate::core::llm::providers::{ClaudeProvider, GeminiProvider};

            let result = if pid == "claude" {
                match ClaudeProvider::from_storage_name("auto", model, 8192) {
                    Ok(claude) => match claude.start_oauth_flow().await {
                        Ok(auth_url) => {
                            let _ = open::that(&auth_url);
                            Ok(auth_url)
                        }
                        Err(e) => Err(format!("{e}")),
                    },
                    Err(e) => Err(format!("Failed to create Claude provider: {e}")),
                }
            } else if pid == "gemini" {
                match GeminiProvider::from_storage_name("auto", model, 8192) {
                    Ok(gemini) => match gemini.start_oauth_flow().await {
                        Ok((auth_url, _state)) => {
                            let _ = open::that(&auth_url);
                            Ok(auth_url)
                        }
                        Err(e) => Err(format!("{e}")),
                    },
                    Err(e) => Err(format!("Failed to create Gemini provider: {e}")),
                }
            } else {
                Err(format!("Unknown OAuth provider: {pid}"))
            };

            let _ = event_tx.send(AppEvent::OAuthFlowResult {
                provider_id: pid,
                result,
            });
        });
    }

    fn complete_oauth_pkce(
        &self,
        provider_id: &str,
        auth_code: &str,
        services: &Services,
    ) {
        let pid = provider_id.to_string();
        let code = auth_code.to_string();
        let event_tx = services.event_tx.clone();
        let mut llm = services.llm.clone();

        // Load existing config to get model
        let app_config = AppConfig::load();
        let model = app_config
            .llm
            .providers
            .get(&pid)
            .map(|c| c.model_name())
            .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

        let provider_config = ProviderConfig::from_parts(&pid, "", "", &model);

        tokio::spawn(async move {
            use crate::core::llm::providers::{ClaudeProvider, GeminiProvider};

            let result = if pid == "claude" {
                match ClaudeProvider::from_storage_name("auto", model.clone(), 8192) {
                    Ok(claude) => claude
                        .complete_oauth_flow(&code)
                        .await
                        .map_err(|e| format!("{e}")),
                    Err(e) => Err(format!("Failed to create Claude provider: {e}")),
                }
            } else if pid == "gemini" {
                match GeminiProvider::from_storage_name("auto", model.clone(), 8192) {
                    Ok(gemini) => gemini
                        .complete_oauth_flow(&code, None)
                        .await
                        .map_err(|e| format!("{e}")),
                    Err(e) => Err(format!("Failed to create Gemini provider: {e}")),
                }
            } else {
                Err(format!("Unknown OAuth provider: {pid}"))
            };

            match result {
                Ok(()) => {
                    // Re-create provider with stored tokens and add to router
                    let fresh = provider_config.create_provider();
                    llm.add_provider(fresh).await;
                    let _ = event_tx.send(AppEvent::OAuthFlowResult {
                        provider_id: pid,
                        result: Ok("success".to_string()),
                    });
                }
                Err(e) => {
                    let _ = event_tx.send(AppEvent::OAuthFlowResult {
                        provider_id: pid,
                        result: Err(e),
                    });
                }
            }
        });
    }

    fn start_device_code_flow(&mut self, meta: &'static ProviderMeta, services: &Services) {
        let model = meta.default_model.to_string();
        let provider_id = meta.id.to_string();

        // Persist config.toml (Device Code providers don't store secrets in keyring)
        if let Err(e) = services.save_oauth_provider(meta.id, &model) {
            log::error!("Failed to save config for device code provider: {e}");
        }

        let provider_config = ProviderConfig::from_parts(meta.id, "", "", &model);
        let event_tx = services.event_tx.clone();
        let mut llm = services.llm.clone();
        let pid = provider_id.clone();

        // Show temporary modal
        self.modal = Some(SettingsModal::DeviceCodeFlow {
            provider_id: provider_id.clone(),
            display_name: meta.display_name.to_string(),
            phase: DeviceCodePhase::Completing, // reuse as "starting"
        });

        tokio::spawn(async move {
            use crate::core::llm::providers::CopilotLLMProvider;

            let copilot = match CopilotLLMProvider::from_storage_name("auto", model, 8192) {
                Ok(c) => c,
                Err(e) => {
                    let _ = event_tx.send(AppEvent::DeviceFlowUpdate {
                        provider_id: pid,
                        update: DeviceFlowUpdateKind::Error(format!(
                            "Failed to create Copilot provider: {e}"
                        )),
                    });
                    return;
                }
            };

            // Start device flow
            let pending = match copilot.start_device_flow().await {
                Ok(p) => p,
                Err(e) => {
                    let _ = event_tx.send(AppEvent::DeviceFlowUpdate {
                        provider_id: pid,
                        update: DeviceFlowUpdateKind::Error(format!("{e}")),
                    });
                    return;
                }
            };

            // Open browser
            let _ = open::that(pending.verification_url_with_code());

            // Notify TUI of user code
            let _ = event_tx.send(AppEvent::DeviceFlowUpdate {
                provider_id: pid.clone(),
                update: DeviceFlowUpdateKind::Started {
                    user_code: pending.user_code.clone(),
                    verification_uri: pending.verification_uri.clone(),
                },
            });

            // Poll loop
            let interval_secs = pending.interval.max(5);
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;

                let _ = event_tx.send(AppEvent::DeviceFlowUpdate {
                    provider_id: pid.clone(),
                    update: DeviceFlowUpdateKind::Polling,
                });

                match copilot.poll_for_token(&pending).await {
                    Ok(crate::oauth::copilot::PollResult::Pending) => continue,
                    Ok(crate::oauth::copilot::PollResult::SlowDown) => {
                        // Back off
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                    Ok(crate::oauth::copilot::PollResult::Complete(github_token)) => {
                        let _ = event_tx.send(AppEvent::DeviceFlowUpdate {
                            provider_id: pid.clone(),
                            update: DeviceFlowUpdateKind::Completing,
                        });

                        match copilot.complete_auth(github_token).await {
                            Ok(()) => {
                                // Add provider to router
                                let fresh = provider_config.create_provider();
                                llm.add_provider(fresh).await;
                                let _ = event_tx.send(AppEvent::DeviceFlowUpdate {
                                    provider_id: pid,
                                    update: DeviceFlowUpdateKind::Complete,
                                });
                            }
                            Err(e) => {
                                let _ = event_tx.send(AppEvent::DeviceFlowUpdate {
                                    provider_id: pid,
                                    update: DeviceFlowUpdateKind::Error(format!("{e}")),
                                });
                            }
                        }
                        return;
                    }
                    Err(e) => {
                        let _ = event_tx.send(AppEvent::DeviceFlowUpdate {
                            provider_id: pid,
                            update: DeviceFlowUpdateKind::Error(format!("{e}")),
                        });
                        return;
                    }
                }
            }
        });
    }

    // ── Provider CRUD ─────────────────────────────────────────────────

    fn save_provider(
        &self,
        meta: &ProviderMeta,
        fields: &[FormField],
        values: &[String],
        services: &Services,
    ) -> Result<(), String> {
        // Extract field values by label
        let get_field = |label: &str| -> String {
            fields
                .iter()
                .zip(values.iter())
                .find(|(f, _)| f.label == label)
                .map(|(_, v)| v.trim().to_string())
                .unwrap_or_default()
        };

        let api_key = get_field("API Key");
        let host = get_field("Host");
        let model = get_field("Model");

        // Validation
        if meta.needs_api_key() && api_key.is_empty() {
            return Err("API key is required".to_string());
        }
        if meta.needs_host() && host.is_empty() {
            return Err("Host URL is required".to_string());
        }
        if model.is_empty() {
            return Err("Model name is required".to_string());
        }

        services.save_provider(meta.id, &api_key, &host, &model)
    }

    fn delete_provider(&self, provider_id: &str, services: &Services) {
        services.delete_provider(provider_id);
    }

    // ── Helpers ───────────────────────────────────────────────────────

    fn selected_provider_id(&self) -> Option<String> {
        self.data
            .as_ref()
            .and_then(|d| {
                if d.providers.is_empty() {
                    None
                } else {
                    Some(
                        d.providers[self.selected_provider.min(d.providers.len() - 1)]
                            .name
                            .to_lowercase(),
                    )
                }
            })
    }

    fn advance_provider_selection(&mut self, delta: i32) {
        let count = self
            .data
            .as_ref()
            .map(|d| d.providers.len())
            .unwrap_or(0);
        if count == 0 {
            return;
        }
        if delta > 0 {
            self.selected_provider = (self.selected_provider + delta as usize).min(count - 1);
        } else {
            self.selected_provider = self.selected_provider.saturating_sub((-delta) as usize);
        }
    }

    fn scroll_down(&mut self, n: usize) {
        self.scroll = self
            .scroll
            .saturating_add(n)
            .min(self.lines_cache.len().saturating_sub(1));
    }

    fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    // ── Rendering ─────────────────────────────────────────────────────

    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.loading && self.data.is_none() {
            let loading = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Loading settings...", Style::default().fg(Color::DarkGray)),
                ]),
            ]);
            frame.render_widget(loading, inner);
        } else if self.lines_cache.is_empty() {
            let empty = Paragraph::new(vec![
                Line::raw(""),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "No data loaded. Press r to refresh.",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
            ]);
            frame.render_widget(empty, inner);
        } else {
            let content = Paragraph::new(self.lines_cache.clone())
                .scroll((self.scroll as u16, 0));
            frame.render_widget(content, inner);
        }

        // Render modal overlay if active
        if let Some(ref modal) = self.modal {
            self.render_modal(frame, area, modal);
        }
    }

    fn render_modal(&self, frame: &mut Frame, area: Rect, modal: &SettingsModal) {
        match modal {
            SettingsModal::SelectProvider { selected } => {
                self.render_select_provider(frame, area, *selected);
            }
            SettingsModal::ConfigureProvider {
                meta,
                fields,
                focused_field,
                field_values,
                error,
            } => {
                self.render_configure_provider(
                    frame,
                    area,
                    meta,
                    fields,
                    *focused_field,
                    field_values,
                    error.as_deref(),
                );
            }
            SettingsModal::OAuthPkceFlow {
                display_name,
                phase,
                ..
            } => {
                self.render_oauth_pkce_flow(frame, area, display_name, phase);
            }
            SettingsModal::DeviceCodeFlow {
                display_name,
                phase,
                ..
            } => {
                self.render_device_code_flow(frame, area, display_name, phase);
            }
            SettingsModal::ConfirmDelete {
                provider_name, ..
            } => {
                self.render_confirm_delete(frame, area, provider_name);
            }
        }
    }

    fn render_select_provider(&self, frame: &mut Frame, area: Rect, selected: usize) {
        // Calculate modal size: need room for all providers + header + footer
        let modal_height = (PROVIDERS.len() as u16 + 7).min(area.height - 4);
        let modal_width = 52.min(area.width - 4);
        let modal = centered_fixed(modal_width, modal_height, area);

        let block = Block::default()
            .title(" Add Provider ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let mut lines = vec![
            Line::raw(""),
            Line::from(Span::styled(
                "  Select a provider:",
                Style::default().fg(Color::White),
            )),
            Line::raw(""),
        ];

        for (i, p) in PROVIDERS.iter().enumerate() {
            let marker = if i == selected { "  \u{25b8} " } else { "    " };
            let name_style = if i == selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let tag_color = match p.auth_method {
                AuthMethod::ApiKey => Color::DarkGray,
                AuthMethod::HostOnly => Color::Blue,
                AuthMethod::OAuthPkce => Color::Magenta,
                AuthMethod::DeviceCode => Color::Cyan,
            };
            lines.push(Line::from(vec![
                Span::raw(marker),
                Span::styled(format!("{:<24}", p.display_name), name_style),
                Span::styled(format!("[{}]", p.auth_tag()), Style::default().fg(tag_color)),
            ]));
        }

        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("j/k", Style::default().fg(Color::DarkGray)),
            Span::raw(":navigate  "),
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(":select  "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":cancel"),
        ]));

        frame.render_widget(Clear, modal);
        frame.render_widget(Paragraph::new(lines).block(block), modal);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_configure_provider(
        &self,
        frame: &mut Frame,
        area: Rect,
        meta: &ProviderMeta,
        fields: &[FormField],
        focused_field: usize,
        field_values: &[String],
        error: Option<&str>,
    ) {
        let modal_height = (fields.len() as u16 * 3 + 8).min(area.height - 4);
        let modal_width = 48.min(area.width - 4);
        let modal = centered_fixed(modal_width, modal_height, area);

        let title = format!(" Configure {} ", meta.display_name);
        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(modal);
        let mut lines: Vec<Line<'static>> = vec![Line::raw("")];

        for (i, field) in fields.iter().enumerate() {
            let is_focused = i == focused_field;

            // Label
            let label_style = if is_focused {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(field.label.to_string(), label_style),
            ]));

            // Input field
            let value = if is_focused {
                self.input.text().to_string()
            } else {
                field_values.get(i).cloned().unwrap_or_default()
            };
            let is_empty = value.is_empty();

            let display_value = if field.is_secret && !is_focused && !is_empty {
                mask_api_key(&value)
            } else if is_empty && !is_focused {
                field.placeholder.to_string()
            } else {
                value
            };

            let field_width = (inner.width as usize).saturating_sub(6);
            let truncated = if display_value.len() > field_width {
                format!("...{}", &display_value[display_value.len() - field_width + 3..])
            } else {
                format!("{:<width$}", display_value, width = field_width)
            };

            let marker = if is_focused { "\u{25b8} " } else { "  " };

            let value_style = if is_empty && !is_focused {
                Style::default().fg(Color::DarkGray)
            } else if is_focused {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::Gray)
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {marker}"), Style::default().fg(Color::Yellow)),
                Span::styled("[", Style::default().fg(Color::DarkGray)),
                Span::styled(truncated, value_style),
                Span::styled("]", Style::default().fg(Color::DarkGray)),
            ]));

            lines.push(Line::raw(""));
        }

        // Error message
        if let Some(err) = error {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(err.to_string(), Style::default().fg(Color::Red)),
            ]));
        }

        // Footer
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Tab", Style::default().fg(Color::DarkGray)),
            Span::raw(":next  "),
            Span::styled("Enter", Style::default().fg(Color::DarkGray)),
            Span::raw(":save  "),
            Span::styled("Esc", Style::default().fg(Color::DarkGray)),
            Span::raw(":cancel"),
        ]));

        frame.render_widget(Clear, modal);
        frame.render_widget(Paragraph::new(lines).block(block), modal);

        // Position cursor in the active field
        if let Some(SettingsModal::ConfigureProvider {
            focused_field, ..
        }) = &self.modal
        {
            let cursor_x = inner.x + 4 + self.input.cursor_position() as u16;
            // Each field takes 3 lines (label + input + blank), starting at line 1
            let cursor_y = inner.y + 1 + (*focused_field as u16 * 3) + 1;
            if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    fn render_oauth_pkce_flow(
        &self,
        frame: &mut Frame,
        area: Rect,
        display_name: &str,
        phase: &OAuthPkcePhase,
    ) {
        let modal = centered_fixed(48, 12, area);

        let title = format!(" {} ", display_name);
        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta));

        let mut lines: Vec<Line<'static>> = vec![Line::raw("")];

        match phase {
            OAuthPkcePhase::WaitingForCode { .. } => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Browser opened for authorization",
                        Style::default().fg(Color::White),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Paste authorization code:",
                        Style::default().fg(Color::Cyan),
                    ),
                ]));
                // Input field
                let code_display = self.input.text().to_string();
                let field_width = 36;
                let truncated = if code_display.len() > field_width {
                    format!("...{}", &code_display[code_display.len() - field_width + 3..])
                } else {
                    format!("{:<width$}", code_display, width = field_width)
                };
                lines.push(Line::from(vec![
                    Span::styled("  \u{25b8} ", Style::default().fg(Color::Yellow)),
                    Span::styled("[", Style::default().fg(Color::DarkGray)),
                    Span::styled(truncated, Style::default().fg(Color::White)),
                    Span::styled("]", Style::default().fg(Color::DarkGray)),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Enter", Style::default().fg(Color::DarkGray)),
                    Span::raw(":submit  "),
                    Span::styled("Esc", Style::default().fg(Color::DarkGray)),
                    Span::raw(":cancel"),
                ]));
            }
            OAuthPkcePhase::Exchanging => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Exchanging authorization code...",
                        Style::default().fg(Color::Yellow),
                    ),
                ]));
            }
            OAuthPkcePhase::Success => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "\u{2713} OAuth flow completed successfully!",
                        Style::default().fg(Color::Green).bold(),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Enter", Style::default().fg(Color::DarkGray)),
                    Span::raw(":close"),
                ]));
            }
            OAuthPkcePhase::Error(msg) => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("\u{2717} {msg}"),
                        Style::default().fg(Color::Red),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Enter/Esc", Style::default().fg(Color::DarkGray)),
                    Span::raw(":close"),
                ]));
            }
        }

        let inner = block.inner(modal);
        frame.render_widget(Clear, modal);
        frame.render_widget(Paragraph::new(lines).block(block), modal);

        // Cursor for code input
        if matches!(phase, OAuthPkcePhase::WaitingForCode { .. }) {
            let cursor_x = inner.x + 4 + self.input.cursor_position() as u16;
            let cursor_y = inner.y + 4; // label at 3, input at 4
            if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    fn render_device_code_flow(
        &self,
        frame: &mut Frame,
        area: Rect,
        display_name: &str,
        phase: &DeviceCodePhase,
    ) {
        let modal = centered_fixed(48, 12, area);

        let title = format!(" {} ", display_name);
        let block = Block::default()
            .title(title)
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let mut lines: Vec<Line<'static>> = vec![Line::raw("")];

        match phase {
            DeviceCodePhase::WaitingForUser {
                user_code,
                verification_uri,
            } => {
                lines.push(Line::from(vec![
                    Span::raw("  Visit: "),
                    Span::styled(
                        verification_uri.to_string(),
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  Enter code:  "),
                    Span::styled(
                        user_code.to_string(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Waiting for authorization...",
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Esc", Style::default().fg(Color::DarkGray)),
                    Span::raw(":cancel"),
                ]));
            }
            DeviceCodePhase::Completing => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "Completing authorization...",
                        Style::default().fg(Color::Yellow),
                    ),
                ]));
            }
            DeviceCodePhase::Success => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        "\u{2713} Device code flow completed!",
                        Style::default().fg(Color::Green).bold(),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Enter", Style::default().fg(Color::DarkGray)),
                    Span::raw(":close"),
                ]));
            }
            DeviceCodePhase::Error(msg) => {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("\u{2717} {msg}"),
                        Style::default().fg(Color::Red),
                    ),
                ]));
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("Enter/Esc", Style::default().fg(Color::DarkGray)),
                    Span::raw(":close"),
                ]));
            }
        }

        frame.render_widget(Clear, modal);
        frame.render_widget(Paragraph::new(lines).block(block), modal);
    }

    fn render_confirm_delete(&self, frame: &mut Frame, area: Rect, provider_name: &str) {
        let modal = centered_fixed(46, 8, area);

        let block = Block::default()
            .title(" Delete Provider ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Red));

        let lines = vec![
            Line::raw(""),
            Line::from(vec![
                Span::raw("  Delete "),
                Span::styled(
                    format!("\"{provider_name}\""),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" and remove its"),
            ]),
            Line::from(Span::raw("  credentials from the system keyring?")),
            Line::raw(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled("y", Style::default().fg(Color::Red).bold()),
                Span::raw(":confirm  "),
                Span::styled("n/Esc", Style::default().fg(Color::DarkGray)),
                Span::raw(":cancel"),
            ]),
        ];

        frame.render_widget(Clear, modal);
        frame.render_widget(Paragraph::new(lines).block(block), modal);
    }
}

// ── Centered rect with fixed dimensions ─────────────────────────────────────

fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

// ── Line builders ───────────────────────────────────────────────────────────

fn build_lines(data: &SettingsData) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(60);

    // ── LLM Providers ──
    lines.extend(section_header("LLM Providers"));

    if data.providers.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                "No providers configured",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::raw("  Press "),
            Span::styled("a", Style::default().fg(Color::Cyan).bold()),
            Span::raw(" to add a provider"),
        ]));
    } else {
        // Table header
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(
                    "  {:<14} {:<20} {:>6} {:>9} {:>6} {:>5} {:>7} {:>8}",
                    "Name", "Model", "Health", "Circuit", "Reqs", "OK%", "Latency", "Cost"
                ),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        for (i, p) in data.providers.iter().enumerate() {
            let health_icon = if p.is_healthy { "\u{2713}" } else { "\u{2717}" };
            let health_color = if p.is_healthy {
                Color::Green
            } else {
                Color::Red
            };

            let name_display = if p.name.len() > 12 {
                format!("{}...", &p.name[..9])
            } else {
                p.name.clone()
            };

            let model_display = if p.model.len() > 18 {
                format!("{}...", &p.model[..15])
            } else {
                p.model.clone()
            };

            // Selection marker — always present but only visible for selected row
            let _ = i; // used for selection marker in future
            let marker = "  ";

            lines.push(Line::from(vec![
                Span::raw(format!("  {marker}")),
                Span::styled(
                    format!("{:<14}", name_display),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(format!("{:<20} ", model_display)),
                Span::styled(
                    format!("{:>4}", health_icon),
                    Style::default().fg(health_color),
                ),
                Span::raw(format!(
                    " {:>9} {:>6} {:>4.0}% {:>6}ms ${:>7.4}",
                    p.circuit_state,
                    p.total_requests,
                    p.success_rate * 100.0,
                    p.avg_latency_ms,
                    p.total_cost,
                )),
            ]));

            // Per-provider uptime on second line
            if p.uptime > 0.0 {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<14}", ""),
                        Style::default(),
                    ),
                    Span::styled(
                        format!("  uptime: {:.1}%", p.uptime),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
        }
    }

    // Provider management hints
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("a", Style::default().fg(Color::DarkGray)),
        Span::raw(":add  "),
        Span::styled("e", Style::default().fg(Color::DarkGray)),
        Span::raw(":edit  "),
        Span::styled("d", Style::default().fg(Color::DarkGray)),
        Span::raw(":delete  "),
        Span::styled("r", Style::default().fg(Color::DarkGray)),
        Span::raw(":refresh"),
    ]));

    // ── Health Overview ──
    lines.extend(section_header("Health Overview"));
    lines.push(kv_row(
        "Total Providers",
        &data.total_providers.to_string(),
    ));
    lines.push(kv_row(
        "Healthy",
        &format!(
            "{}{}",
            data.healthy_count,
            if data.unhealthy_count > 0 {
                format!(" ({} unhealthy)", data.unhealthy_count)
            } else {
                String::new()
            }
        ),
    ));
    lines.push(kv_row(
        "Average Uptime",
        &format!("{:.1}%", data.avg_uptime),
    ));

    // ── Router Configuration ──
    lines.extend(section_header("Router Configuration"));
    lines.push(kv_row("Routing Strategy", &data.routing_strategy));
    lines.push(kv_row(
        "Request Timeout",
        &format!("{}s", data.request_timeout_secs),
    ));
    lines.push(kv_row(
        "Stream Timeout",
        &format!("{}s", data.stream_chunk_timeout_secs),
    ));
    lines.push(kv_row(
        "Health Check",
        &format!("every {}s", data.health_check_interval_secs),
    ));
    lines.push(kv_row("Max Retries", &data.max_retries.to_string()));
    lines.push(kv_row(
        "Fallback",
        if data.enable_fallback {
            "enabled"
        } else {
            "disabled"
        },
    ));

    // ── Cost Summary ──
    lines.extend(section_header("Cost Summary"));
    let budget_icon = if data.within_budget { "\u{2713}" } else { "\u{2717}" };
    let budget_color = if data.within_budget {
        Color::Green
    } else {
        Color::Red
    };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<22}", "Budget Status"),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!(
                "{} {}",
                budget_icon,
                if data.within_budget {
                    "Within budget"
                } else {
                    "OVER BUDGET"
                }
            ),
            Style::default().fg(budget_color),
        ),
    ]));
    lines.push(kv_row(
        "Total Cost",
        &format!("${:.4}", data.total_cost),
    ));
    lines.push(kv_row(
        "Monthly Cost",
        &format!("${:.4}", data.monthly_cost),
    ));
    lines.push(kv_row(
        "Daily Cost",
        &format!("${:.4}", data.daily_cost),
    ));
    lines.push(kv_row(
        "Monthly Budget",
        &data
            .monthly_budget
            .map(|b| format!("${:.2}", b))
            .unwrap_or_else(|| "none".to_string()),
    ));
    lines.push(kv_row(
        "Daily Budget",
        &data
            .daily_budget
            .map(|b| format!("${:.2}", b))
            .unwrap_or_else(|| "none".to_string()),
    ));

    // ── Application ──
    lines.extend(section_header("Application"));
    lines.push(kv_row(
        "Config File",
        &dirs::config_dir()
            .map(|d| d.join("ttttrps/config.toml").display().to_string())
            .unwrap_or_else(|| "config.toml".to_string()),
    ));
    lines.push(kv_row("Data Directory", &data.data_dir));
    lines.push(kv_row(
        "Tick Rate",
        &format!("{}ms", data.tick_rate_ms),
    ));
    lines.push(kv_row(
        "Mouse",
        if data.mouse_enabled {
            "enabled"
        } else {
            "disabled"
        },
    ));
    lines.push(kv_row("Theme", &data.theme));

    // ── Footer ──
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("j/k", Style::default().fg(Color::DarkGray)),
        Span::raw(":scroll "),
        Span::styled("G/g", Style::default().fg(Color::DarkGray)),
        Span::raw(":bottom/top "),
        Span::styled("r", Style::default().fg(Color::DarkGray)),
        Span::raw(":refresh"),
    ]));
    lines.push(Line::raw(""));

    lines
}

fn section_header(title: &str) -> Vec<Line<'static>> {
    vec![
        Line::raw(""),
        Line::from(Span::styled(
            format!("  {title}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("  {}", "\u{2500}".repeat(50)),
            Style::default().fg(Color::DarkGray),
        )),
    ]
}

fn kv_row(key: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{:<22}", key),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(value.to_string()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_state_new() {
        let state = SettingsState::new();
        assert!(state.data.is_none());
        assert!(state.lines_cache.is_empty());
        assert_eq!(state.scroll, 0);
        assert!(!state.loading);
        assert!(!state.has_modal());
    }

    #[test]
    fn test_provider_meta_list_completeness() {
        assert_eq!(PROVIDERS.len(), 13);
        let ids: Vec<&str> = PROVIDERS.iter().map(|p| p.id).collect();
        assert!(ids.contains(&"ollama"));
        assert!(ids.contains(&"openai"));
        assert!(ids.contains(&"anthropic"));
        assert!(ids.contains(&"google"));
        assert!(ids.contains(&"claude"));
        assert!(ids.contains(&"gemini"));
        assert!(ids.contains(&"copilot"));
        assert!(ids.contains(&"openrouter"));
        assert!(ids.contains(&"mistral"));
        assert!(ids.contains(&"groq"));
        assert!(ids.contains(&"together"));
        assert!(ids.contains(&"cohere"));
        assert!(ids.contains(&"deepseek"));
    }

    #[test]
    fn test_provider_meta_unique_ids() {
        let ids: Vec<&str> = PROVIDERS.iter().map(|p| p.id).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn test_auth_method_distribution() {
        let api_key_count = PROVIDERS.iter().filter(|p| p.auth_method == AuthMethod::ApiKey).count();
        let host_only_count = PROVIDERS.iter().filter(|p| p.auth_method == AuthMethod::HostOnly).count();
        let oauth_count = PROVIDERS.iter().filter(|p| p.auth_method == AuthMethod::OAuthPkce).count();
        let device_count = PROVIDERS.iter().filter(|p| p.auth_method == AuthMethod::DeviceCode).count();

        assert_eq!(api_key_count, 9); // openai, anthropic, google, openrouter, mistral, groq, together, cohere, deepseek
        assert_eq!(host_only_count, 1); // ollama
        assert_eq!(oauth_count, 2); // claude, gemini
        assert_eq!(device_count, 1); // copilot
    }

    #[test]
    fn test_fields_for_api_key_provider() {
        let meta = find_provider_meta("openai").unwrap();
        let fields = fields_for_provider(meta);
        assert_eq!(fields.len(), 2); // API Key + Model
        assert_eq!(fields[0].label, "API Key");
        assert!(fields[0].is_secret);
        assert_eq!(fields[1].label, "Model");
        assert!(!fields[1].is_secret);
    }

    #[test]
    fn test_fields_for_ollama() {
        let meta = find_provider_meta("ollama").unwrap();
        let fields = fields_for_provider(meta);
        assert_eq!(fields.len(), 2); // Host + Model
        assert_eq!(fields[0].label, "Host");
        assert!(!fields[0].is_secret);
        assert_eq!(fields[1].label, "Model");
    }

    #[test]
    fn test_fields_for_oauth_provider() {
        let meta = find_provider_meta("claude").unwrap();
        let fields = fields_for_provider(meta);
        assert_eq!(fields.len(), 1); // Model only
        assert_eq!(fields[0].label, "Model");
    }

    #[test]
    fn test_fields_for_device_code_provider() {
        let meta = find_provider_meta("copilot").unwrap();
        let fields = fields_for_provider(meta);
        assert_eq!(fields.len(), 1); // Model only
        assert_eq!(fields[0].label, "Model");
    }

    // build_provider_config tests moved to core::llm::providers::tests::test_from_parts_*

    #[test]
    fn test_centered_fixed() {
        let area = Rect::new(0, 0, 100, 50);
        let modal = centered_fixed(48, 20, area);
        assert_eq!(modal.width, 48);
        assert_eq!(modal.height, 20);
        assert_eq!(modal.x, 26); // (100 - 48) / 2
        assert_eq!(modal.y, 15); // (50 - 20) / 2
    }

    #[test]
    fn test_centered_fixed_small_area() {
        let area = Rect::new(0, 0, 30, 10);
        let modal = centered_fixed(48, 20, area);
        // Should clamp to available area
        assert_eq!(modal.width, 30);
        assert_eq!(modal.height, 10);
    }

    #[test]
    fn test_build_lines_no_providers() {
        let data = SettingsData {
            providers: vec![],
            routing_strategy: "Priority".to_string(),
            request_timeout_secs: 120,
            health_check_interval_secs: 60,
            max_retries: 1,
            enable_fallback: true,
            stream_chunk_timeout_secs: 30,
            total_cost: 0.0,
            monthly_cost: 0.0,
            daily_cost: 0.0,
            monthly_budget: None,
            daily_budget: None,
            within_budget: true,
            total_providers: 0,
            healthy_count: 0,
            unhealthy_count: 0,
            avg_uptime: 0.0,
            data_dir: "/tmp/test".to_string(),
            tick_rate_ms: 50,
            mouse_enabled: false,
            theme: "default".to_string(),
        };
        let lines = build_lines(&data);
        assert!(!lines.is_empty());
        let text: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("LLM Providers"));
        assert!(text.contains("No providers configured"));
        assert!(text.contains("add a provider"));
        assert!(text.contains("Router Configuration"));
        assert!(text.contains("Cost Summary"));
        assert!(text.contains("Application"));
    }

    #[test]
    fn test_build_lines_with_providers() {
        let data = SettingsData {
            providers: vec![
                ProviderDisplayInfo {
                    name: "Claude".to_string(),
                    model: "claude-3-5-sonnet".to_string(),
                    is_healthy: true,
                    circuit_state: "Closed".to_string(),
                    uptime: 99.5,
                    total_requests: 42,
                    success_rate: 0.95,
                    avg_latency_ms: 1200,
                    total_cost: 0.0123,
                },
                ProviderDisplayInfo {
                    name: "Ollama".to_string(),
                    model: "llama3".to_string(),
                    is_healthy: false,
                    circuit_state: "Open".to_string(),
                    uptime: 50.0,
                    total_requests: 10,
                    success_rate: 0.5,
                    avg_latency_ms: 500,
                    total_cost: 0.0,
                },
            ],
            routing_strategy: "Priority".to_string(),
            request_timeout_secs: 120,
            health_check_interval_secs: 60,
            max_retries: 1,
            enable_fallback: true,
            stream_chunk_timeout_secs: 30,
            total_cost: 0.0123,
            monthly_cost: 0.0123,
            daily_cost: 0.005,
            monthly_budget: Some(10.0),
            daily_budget: None,
            within_budget: true,
            total_providers: 2,
            healthy_count: 1,
            unhealthy_count: 1,
            avg_uptime: 74.75,
            data_dir: "/tmp/test".to_string(),
            tick_rate_ms: 50,
            mouse_enabled: false,
            theme: "default".to_string(),
        };
        let lines = build_lines(&data);
        let text: String = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Claude"));
        assert!(text.contains("Ollama"));
        assert!(text.contains("Priority"));
        assert!(text.contains("$10.00"));
    }

    #[test]
    fn test_scroll_bounds() {
        let mut state = SettingsState::new();
        // Empty lines — scroll should stay at 0
        state.scroll_down(10);
        assert_eq!(state.scroll, 0);
        state.scroll_up(10);
        assert_eq!(state.scroll, 0);

        // Simulate some cached lines
        state.lines_cache = vec![Line::raw(""); 30];
        state.scroll_down(5);
        assert_eq!(state.scroll, 5);
        state.scroll_down(100);
        assert_eq!(state.scroll, 29); // clamped to len-1
        state.scroll_up(100);
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn test_section_header() {
        let lines = section_header("Test Section");
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_kv_row() {
        let line = kv_row("Key", "Value");
        let text: String = line
            .spans
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(text.contains("Key"));
        assert!(text.contains("Value"));
    }

    #[test]
    fn test_open_add_modal() {
        let mut state = SettingsState::new();
        assert!(!state.has_modal());
        state.open_add_modal();
        assert!(state.has_modal());
    }

    #[test]
    fn test_open_delete_modal() {
        let mut state = SettingsState::new();
        state.open_delete_modal("openai");
        assert!(state.has_modal());
        if let Some(SettingsModal::ConfirmDelete {
            provider_id,
            provider_name,
        }) = &state.modal
        {
            assert_eq!(provider_id, "openai");
            assert_eq!(provider_name, "OpenAI");
        } else {
            panic!("Expected ConfirmDelete modal");
        }
    }

    #[test]
    fn test_find_provider_meta() {
        assert!(find_provider_meta("openai").is_some());
        assert!(find_provider_meta("anthropic").is_some());
        assert!(find_provider_meta("google").is_some());
        assert!(find_provider_meta("claude").is_some());
        assert!(find_provider_meta("gemini").is_some());
        assert!(find_provider_meta("copilot").is_some());
        assert!(find_provider_meta("nonexistent").is_none());
    }

    #[test]
    fn test_auth_tags() {
        assert_eq!(find_provider_meta("openai").unwrap().auth_tag(), "key");
        assert_eq!(find_provider_meta("ollama").unwrap().auth_tag(), "local");
        assert_eq!(find_provider_meta("claude").unwrap().auth_tag(), "OAuth");
        assert_eq!(find_provider_meta("copilot").unwrap().auth_tag(), "device");
    }
}
