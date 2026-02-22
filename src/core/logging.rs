//! Rich Terminal Logging Module
//!
//! Provides Python Rich-like terminal output with:
//! - Beautiful error reporting with source context (miette)
//! - Progress bars for document ingestion (indicatif)
//! - Terminal styling with colors and emoji (console)
//! - Syntax highlighting for code blocks (syntect)
//! - Markdown rendering in terminal (pulldown-cmark)
//! - Automatic terminal capability detection

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use console::{style, Color, Term};
use flate2::write::GzEncoder;
use flate2::Compression;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use miette::Diagnostic;
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use supports_color::Stream;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use thiserror::Error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

// ============================================================================
// Static Resources (Lazy Loaded)
// ============================================================================

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
static TERMINAL_CAPS: OnceLock<TerminalCapabilities> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(|| SyntaxSet::load_defaults_newlines())
}

fn get_theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

fn get_terminal_caps() -> &'static TerminalCapabilities {
    TERMINAL_CAPS.get_or_init(TerminalCapabilities::detect)
}

// ============================================================================
// Terminal Capability Detection
// ============================================================================

/// Terminal color support levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorLevel {
    /// 24-bit TrueColor (16.7M colors)
    TrueColor,
    /// 256-color palette
    Ansi256,
    /// 16 ANSI colors
    Ansi16,
    /// No color support
    NoColor,
}

/// Detected terminal capabilities
#[derive(Debug, Clone)]
pub struct TerminalCapabilities {
    pub color_level: ColorLevel,
    pub supports_unicode: bool,
    pub is_interactive: bool,
    pub width: u16,
}

impl TerminalCapabilities {
    /// Detect terminal capabilities from environment
    pub fn detect() -> Self {
        use is_terminal::IsTerminal;

        let color_level = match supports_color::on(Stream::Stdout) {
            Some(support) if support.has_16m => ColorLevel::TrueColor,
            Some(support) if support.has_256 => ColorLevel::Ansi256,
            Some(support) if support.has_basic => ColorLevel::Ansi16,
            _ => ColorLevel::NoColor,
        };

        let is_interactive = io::stdout().is_terminal();
        let width = Term::stdout().size().1;

        // Unicode support heuristic
        let supports_unicode = std::env::var("TERM")
            .map(|t| !t.contains("dumb"))
            .unwrap_or(true)
            && std::env::var("LANG")
                .map(|l| l.contains("UTF-8") || l.contains("utf8"))
                .unwrap_or(true);

        Self {
            color_level,
            supports_unicode,
            is_interactive,
            width,
        }
    }

    /// Check if colors should be used
    pub fn should_colorize(&self) -> bool {
        self.is_interactive && self.color_level != ColorLevel::NoColor
    }
}

// ============================================================================
// Logging Initialization
// ============================================================================

/// Initialize the logging system.
///
/// This sets up:
/// 1. A stdout logger (pretty formatted with colors).
/// 2. A file logger (JSON formatted) in the app data directory.
/// 3. Redirects standard `log` crate events to `tracing`.
/// 4. Configures miette for beautiful error reporting.
///
/// Returns a `WorkerGuard` which must be kept alive for the duration of the application
/// to ensure buffered logs are flushed on shutdown.
pub fn init() -> WorkerGuard {
    // 1. Create logs directory in app data directory (not in source tree)
    // This prevents the dev file watcher from detecting log changes and triggering rebuilds
    let log_dir = dirs::data_dir()
        .map(|d| d.join("ttrpg-assistant").join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"));

    if !log_dir.exists() {
        if let Err(e) = fs::create_dir_all(&log_dir) {
            eprintln!("Failed to create logs directory: {}", e);
        }
    }

    // 2. Setup file appender (Rolling - Daily)
    // "REALLY generous limits" -> Daily rotation with no size limit (default for tracing-appender rolling)
    let file_appender = tracing_appender::rolling::daily(&log_dir, "ttrpg-assistant.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 3. Define Filters (compression moved to after logging init)
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

    // 5. Setup Layers

    // File Layer: JSON format for easy parsing/ingestion
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .json()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(true)
        .with_filter(env_filter.clone());

    // Stdout Layer: Pretty human-readable format with colors
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(io::stdout)
        .pretty()
        .with_filter(env_filter);

    // 6. Initialize Registry
    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();

    // 7. Redirect standard `log` macros to `tracing`
    if let Err(e) = tracing_log::LogTracer::init() {
        eprintln!("Failed to initialize LogTracer: {}", e);
    }

    // 8. Configure miette for beautiful error reporting
    init_miette();

    // 9. Compress old logs in background (AFTER logging is initialized so log macros work)
    let log_dir_clone = log_dir.clone();
    std::thread::spawn(move || {
        compress_old_logs(log_dir_clone);
    });

    log::info!(
        "Logging initialized. Writing to: {:?} (daily rolling)",
        log_dir.join("ttrpg-assistant.log")
    );

    guard
}

/// Initialize the logging system for TUI mode.
///
/// Identical to [`init()`] but omits the stdout layer to avoid corrupting
/// the terminal while ratatui is in raw/alternate-screen mode.
/// All logs go to the file appender only.
pub fn init_tui() -> WorkerGuard {
    let log_dir = dirs::data_dir()
        .map(|d| d.join("ttrpg-assistant").join("logs"))
        .unwrap_or_else(|| PathBuf::from("logs"));

    if !log_dir.exists() {
        if let Err(e) = fs::create_dir_all(&log_dir) {
            eprintln!("Failed to create logs directory: {}", e);
        }
    }

    let file_appender = tracing_appender::rolling::daily(&log_dir, "ttrpg-assistant.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .json()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(true)
        .with_filter(env_filter);

    // No stdout layer — TUI owns the terminal
    tracing_subscriber::registry()
        .with(file_layer)
        .init();

    if let Err(e) = tracing_log::LogTracer::init() {
        eprintln!("Failed to initialize LogTracer: {}", e);
    }

    init_miette();

    let log_dir_clone = log_dir.clone();
    std::thread::spawn(move || {
        compress_old_logs(log_dir_clone);
    });

    guard
}

/// Compress old log files in the background
fn compress_old_logs(log_dir: PathBuf) {
    let now = chrono::Local::now();
    let today_suffix = now.format("%Y-%m-%d").to_string();

    if let Ok(entries) = fs::read_dir(&log_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Check for files to compress
                let should_compress = if name.starts_with("ttrpg-assistant.log.") {
                    // New rolling format: prefix.YYYY-MM-DD
                    // Compress if it's NOT today's log and NOT already compressed
                    !name.ends_with(&today_suffix) && !name.ends_with(".gz")
                } else if name.starts_with("ttrpg-assistant-") && name.ends_with(".log") {
                    // Old format: ttrpg-assistant-YYYY-MM-DD_...
                    // Always compress these as we've switched to rolling
                    true
                } else {
                    false
                };

                if should_compress {
                    if let Err(e) = compress_file(&path) {
                        // Log to stderr since we might be inside the logging system (avoid recursion if possible, though log crate handles it)
                        // Or just use the initialized logger
                        log::warn!("Failed to compress old log {:?}: {}", path, e);
                    } else {
                        log::info!("Compressed old log: {:?}", path);
                    }
                }
            }
        }
    }
}

fn compress_file(path: &std::path::Path) -> std::io::Result<()> {
    let file = fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);

    // Create .gz path
    let mut gz_path_name = path
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No filename"))?
        .to_os_string();
    gz_path_name.push(".gz");
    let parent_dir = path
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "No parent directory"))?;
    let gz_path = parent_dir.join(gz_path_name);

    // Skip if already exists
    if gz_path.exists() {
        return Ok(());
    }

    let output = fs::File::create(&gz_path)?;
    let mut encoder = GzEncoder::new(output, Compression::default());

    std::io::copy(&mut reader, &mut encoder)?;
    encoder.finish()?;

    // Remove original file
    fs::remove_file(path)?;

    Ok(())
}

/// Initialize miette for beautiful error reporting
fn init_miette() {
    let caps = get_terminal_caps();

    miette::set_hook(Box::new(move |_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(caps.color_level == ColorLevel::TrueColor)
                .unicode(caps.supports_unicode)
                .context_lines(3)
                .tab_width(4)
                .break_words(true)
                .color(caps.should_colorize())
                .build(),
        )
    }))
    .ok(); // Ignore if already set
}

// ============================================================================
// Rich Text Styling
// ============================================================================

/// Theme-aware color palette for consistent styling
#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub primary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub muted: Color,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::modern()
    }
}

impl ColorPalette {
    /// Modern color palette (default)
    pub fn modern() -> Self {
        Self {
            primary: Color::Cyan,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Blue,
            muted: Color::Color256(243), // Gray
        }
    }

    /// Solarized dark theme
    pub fn solarized_dark() -> Self {
        Self {
            primary: Color::Color256(33),  // #268BD2
            success: Color::Color256(64),  // #859900
            warning: Color::Color256(136), // #B58900
            error: Color::Color256(160),   // #DC322F
            info: Color::Color256(37),     // #2AA198
            muted: Color::Color256(245),   // Base1
        }
    }

    /// Create styled text for a log level
    pub fn level_style(&self, level: &str) -> String {
        let (color, label) = match level.to_lowercase().as_str() {
            "info" => (self.info, "[INFO]"),
            "warn" | "warning" => (self.warning, "[WARN]"),
            "error" => (self.error, "[ERROR]"),
            "debug" => (self.muted, "[DEBUG]"),
            "trace" => (self.muted, "[TRACE]"),
            "success" | "ok" => (self.success, "[OK]"),
            _ => (self.primary, "[LOG]"),
        };

        format!("{}", style(label).fg(color).bold())
    }
}

/// Builder for styled text segments
#[derive(Default)]
pub struct RichText {
    segments: Vec<String>,
}

impl RichText {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add plain text
    pub fn text(mut self, text: &str) -> Self {
        self.segments.push(text.to_string());
        self
    }

    /// Add bold text
    pub fn bold(mut self, text: &str) -> Self {
        self.segments.push(format!("{}", style(text).bold()));
        self
    }

    /// Add colored text
    pub fn color(mut self, text: &str, color: Color) -> Self {
        self.segments.push(format!("{}", style(text).fg(color)));
        self
    }

    /// Add success styled text
    pub fn success(mut self, text: &str) -> Self {
        self.segments
            .push(format!("{}", style(text).green().bold()));
        self
    }

    /// Add error styled text
    pub fn error(mut self, text: &str) -> Self {
        self.segments.push(format!("{}", style(text).red().bold()));
        self
    }

    /// Add warning styled text
    pub fn warning(mut self, text: &str) -> Self {
        self.segments
            .push(format!("{}", style(text).yellow().bold()));
        self
    }

    /// Add muted/dim text
    pub fn muted(mut self, text: &str) -> Self {
        self.segments.push(format!("{}", style(text).dim()));
        self
    }

    /// Build the final string
    pub fn build(self) -> String {
        self.segments.join("")
    }
}

impl std::fmt::Display for RichText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for seg in &self.segments {
            write!(f, "{}", seg)?;
        }
        Ok(())
    }
}

// ============================================================================
// Emoji Support
// ============================================================================

/// Convert emoji shortcodes to actual emoji
/// e.g., `:rocket:` -> `🚀`
pub fn convert_emoji(text: &str) -> String {
    let caps = get_terminal_caps();
    if !caps.supports_unicode {
        return convert_emoji_fallback(text);
    }

    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == ':' {
            let mut code = String::new();
            let mut found_end = false;

            while let Some(&next_ch) = chars.peek() {
                if next_ch == ':' {
                    chars.next();
                    found_end = true;
                    break;
                }
                if !next_ch.is_alphanumeric() && next_ch != '_' && next_ch != '-' {
                    break;
                }
                code.push(chars.next().unwrap());
            }

            if found_end && !code.is_empty() {
                if let Some(emoji) = emojis::get_by_shortcode(&code) {
                    result.push_str(emoji.as_str());
                    continue;
                }
            }
            // Not a valid emoji, restore the original
            result.push(':');
            result.push_str(&code);
            if found_end {
                result.push(':');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Fallback for terminals without Unicode support
fn convert_emoji_fallback(text: &str) -> String {
    text.replace(":rocket:", "->")
        .replace(":heart:", "<3")
        .replace(":fire:", "!!")
        .replace(":star:", "*")
        .replace(":check:", "[v]")
        .replace(":x:", "[x]")
        .replace(":warning:", "[!]")
        .replace(":info:", "(i)")
}

// ============================================================================
// Syntax Highlighting
// ============================================================================

/// Syntax highlighter for terminal output
pub struct SyntaxHighlighter {
    theme_name: String,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter with the specified theme
    pub fn new(theme_name: &str) -> Result<Self, String> {
        let ts = get_theme_set();
        if !ts.themes.contains_key(theme_name) {
            return Err(format!(
                "Theme '{}' not found. Available: {:?}",
                theme_name,
                ts.themes.keys().collect::<Vec<_>>()
            ));
        }
        Ok(Self {
            theme_name: theme_name.to_string(),
        })
    }

    /// Create with default theme (base16-ocean.dark)
    pub fn default_theme() -> Self {
        Self {
            theme_name: "base16-ocean.dark".to_string(),
        }
    }

    /// Highlight code to terminal with ANSI colors
    pub fn highlight(&self, code: &str, language: &str) -> Result<String, String> {
        let caps = get_terminal_caps();
        if !caps.should_colorize() {
            return Ok(code.to_string());
        }

        let ss = get_syntax_set();
        let ts = get_theme_set();

        let syntax = ss
            .find_syntax_by_extension(language)
            .or_else(|| ss.find_syntax_by_name(language))
            .ok_or_else(|| format!("Language '{}' not supported", language))?;

        let theme = ts
            .themes
            .get(&self.theme_name)
            .ok_or_else(|| "Theme not found".to_string())?;

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut result = String::new();

        for line in LinesWithEndings::from(code) {
            let ranges = highlighter
                .highlight_line(line, ss)
                .map_err(|e| format!("Highlight error: {}", e))?;
            let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
            result.push_str(&escaped);
        }

        // Reset colors at end
        result.push_str("\x1b[0m");
        Ok(result)
    }

    /// Get list of supported language extensions
    pub fn supported_languages() -> Vec<String> {
        get_syntax_set()
            .syntaxes()
            .iter()
            .flat_map(|s| s.file_extensions.iter().cloned())
            .collect()
    }

    /// Get list of available themes
    pub fn available_themes() -> Vec<String> {
        get_theme_set().themes.keys().cloned().collect()
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::default_theme()
    }
}

// ============================================================================
// Markdown Rendering
// ============================================================================

/// Render markdown to terminal with syntax highlighting
pub fn render_markdown(markdown: &str) -> Result<String, String> {
    let caps = get_terminal_caps();
    let highlighter = SyntaxHighlighter::default_theme();
    let parser = Parser::new(markdown);

    let mut output = String::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_buffer = String::new();
    let mut list_depth: usize = 0;

    for event in parser {
        match event {
            // Headers
            Event::Start(Tag::Heading { level, .. }) => {
                let prefix = match level {
                    pulldown_cmark::HeadingLevel::H1 => format!("{} ", style("##").cyan().bold()),
                    pulldown_cmark::HeadingLevel::H2 => format!("{} ", style("###").cyan()),
                    pulldown_cmark::HeadingLevel::H3 => format!("{} ", style("####").cyan()),
                    _ => format!("{} ", style("#####").dim()),
                };
                output.push_str(&prefix);
            }
            Event::End(TagEnd::Heading(_)) => {
                output.push('\n');
            }

            // Code blocks
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                in_code_block = true;
                code_block_lang = lang.to_string();
                code_buffer.clear();
                output.push_str(&format!("{}\n", style("```").dim()));
            }
            Event::End(TagEnd::CodeBlock) => {
                if in_code_block && !code_buffer.is_empty() {
                    if caps.should_colorize() {
                        if let Ok(highlighted) = highlighter.highlight(&code_buffer, &code_block_lang)
                        {
                            output.push_str(&highlighted);
                        } else {
                            output.push_str(&code_buffer);
                        }
                    } else {
                        output.push_str(&code_buffer);
                    }
                }
                output.push_str(&format!("{}\n", style("```").dim()));
                in_code_block = false;
            }

            // Inline code
            Event::Code(code) => {
                output.push_str(&format!("{}", style(format!("`{}`", code)).cyan()));
            }

            // Lists
            Event::Start(Tag::List(_)) => {
                list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                list_depth = list_depth.saturating_sub(1);
            }
            Event::Start(Tag::Item) => {
                let indent = "  ".repeat(list_depth.saturating_sub(1));
                let bullet = if caps.supports_unicode { "•" } else { "*" };
                output.push_str(&format!("{}{} ", indent, style(bullet).cyan()));
            }
            Event::End(TagEnd::Item) => {
                output.push('\n');
            }

            // Emphasis
            Event::Start(Tag::Strong) => {}
            Event::End(TagEnd::Strong) => {}
            Event::Start(Tag::Emphasis) => {}
            Event::End(TagEnd::Emphasis) => {}

            // Text
            Event::Text(text) => {
                if in_code_block {
                    code_buffer.push_str(&text);
                } else {
                    output.push_str(&text);
                }
            }

            // Line breaks
            Event::SoftBreak => {
                if !in_code_block {
                    output.push(' ');
                }
            }
            Event::HardBreak => {
                output.push('\n');
            }

            // Paragraphs
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                output.push_str("\n\n");
            }

            // Links
            Event::Start(Tag::Link { dest_url, .. }) => {
                if caps.color_level == ColorLevel::TrueColor {
                    // OSC 8 hyperlink for TrueColor terminals
                    output.push_str(&format!("\x1b]8;;{}\x1b\\", dest_url));
                }
            }
            Event::End(TagEnd::Link) => {
                if caps.color_level == ColorLevel::TrueColor {
                    output.push_str("\x1b]8;;\x1b\\");
                }
            }

            _ => {}
        }
    }

    Ok(output)
}

// ============================================================================
// Progress Bars (Indicatif Integration)
// ============================================================================

/// Progress bar styles for different operations
pub struct ProgressStyles;

impl ProgressStyles {
    /// Standard progress bar with percentage and ETA
    pub fn default_bar() -> ProgressStyle {
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) | ETA: {eta}",
            )
            .unwrap()
            .progress_chars("=>-")
    }

    /// Progress bar with throughput (for file operations)
    pub fn throughput_bar() -> ProgressStyle {
        ProgressStyle::default_bar()
            .template(
                "[{bar:50.blue/cyan}] {percent}% | {bytes}/{total_bytes} | {bytes_per_sec} | ETA: {eta}",
            )
            .unwrap()
            .progress_chars("=>-")
    }

    /// Spinner for indeterminate operations
    pub fn spinner() -> ProgressStyle {
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap()
    }

    /// Document ingestion pipeline style
    pub fn pipeline_stage(label: &str) -> ProgressStyle {
        ProgressStyle::default_bar()
            .template(&format!(
                "  {} [{{bar:30.green/white}}] {{pos}}/{{len}} | {{per_sec}}",
                style(label).bold()
            ))
            .unwrap()
            .progress_chars("=>-")
    }
}

/// Manager for multiple progress bars
pub struct ProgressManager {
    multi: MultiProgress,
    overall: Option<ProgressBar>,
}

impl ProgressManager {
    /// Create a new progress manager
    pub fn new() -> Self {
        let caps = get_terminal_caps();
        let multi = if caps.is_interactive {
            MultiProgress::new()
        } else {
            let mp = MultiProgress::new();
            mp.set_draw_target(ProgressDrawTarget::hidden());
            mp
        };

        Self {
            multi,
            overall: None,
        }
    }

    /// Set up overall progress tracking
    pub fn set_total(&mut self, total: u64, message: &str) {
        let pb = self.multi.add(ProgressBar::new(total));
        pb.set_style(ProgressStyles::default_bar());
        pb.set_message(message.to_string());
        self.overall = Some(pb);
    }

    /// Add a sub-task progress bar
    pub fn add_task(&self, name: &str, total: u64) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new(total));
        pb.set_style(ProgressStyles::pipeline_stage(name));
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    }

    /// Add a spinner for indeterminate work
    pub fn add_spinner(&self, message: &str) -> ProgressBar {
        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(ProgressStyles::spinner());
        pb.set_message(message.to_string());
        pb.enable_steady_tick(Duration::from_millis(100));
        pb
    }

    /// Increment overall progress
    pub fn inc_overall(&self) {
        if let Some(pb) = &self.overall {
            pb.inc(1);
        }
    }

    /// Log while suspending progress bars
    pub fn log<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        self.multi.suspend(f);
    }

    /// Finish all progress tracking
    pub fn finish(&self, message: &str) {
        if let Some(pb) = &self.overall {
            pb.finish_with_message(message.to_string());
        }
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Diagnostic Error Types (miette integration)
// ============================================================================

/// TTRPG-specific diagnostic error
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(code("TTRPG::ERROR"))]
pub struct AppError {
    message: String,

    #[source_code]
    source_code: Option<String>,

    #[label("error occurs here")]
    span: Option<miette::SourceSpan>,

    #[help]
    help_text: Option<String>,
}

impl AppError {
    /// Create a simple error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source_code: None,
            span: None,
            help_text: None,
        }
    }

    /// Add source context
    pub fn with_source(mut self, source: impl Into<String>, offset: usize, length: usize) -> Self {
        self.source_code = Some(source.into());
        self.span = Some(miette::SourceSpan::new(offset.into(), length.into()));
        self
    }

    /// Add help text
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help_text = Some(help.into());
        self
    }
}

/// Document extraction error with context
#[derive(Debug, Error, Diagnostic)]
#[error("Failed to extract content from {filename}: {reason}")]
#[diagnostic(code("TTRPG::EXTRACTION_ERROR"), help("Check the file format and try again"))]
pub struct ExtractionError {
    pub filename: String,
    pub reason: String,
}

impl ExtractionError {
    pub fn new(filename: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            filename: filename.into(),
            reason: reason.into(),
        }
    }
}

/// LLM provider error
#[derive(Debug, Error, Diagnostic)]
#[error("LLM error ({provider}): {message}")]
#[diagnostic(code("TTRPG::LLM_ERROR"))]
pub struct LlmError {
    pub provider: String,
    pub message: String,

    #[help]
    pub recovery_hint: String,
}

impl LlmError {
    pub fn new(provider: impl Into<String>, message: impl Into<String>) -> Self {
        let provider = provider.into();
        Self {
            recovery_hint: format!("Try again or switch to a different {} model", provider),
            provider,
            message: message.into(),
        }
    }
}

/// Search/Meilisearch error
#[derive(Debug, Error, Diagnostic)]
#[error("Search error: {reason}")]
#[diagnostic(
    code("TTRPG::SEARCH_ERROR"),
    help("Verify Meilisearch is running and the query is valid")
)]
pub struct SearchError {
    pub reason: String,
}

impl SearchError {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

// ============================================================================
// Console Output Utilities
// ============================================================================

/// Print a styled panel with title and content
pub fn print_panel(title: &str, content: &str) {
    let caps = get_terminal_caps();
    let width = (caps.width as usize).min(80).max(20); // Ensure minimum width

    let border_char = if caps.supports_unicode { "─" } else { "-" };
    let corner_tl = if caps.supports_unicode { "╭" } else { "+" };
    let corner_tr = if caps.supports_unicode { "╮" } else { "+" };
    let corner_bl = if caps.supports_unicode { "╰" } else { "+" };
    let corner_br = if caps.supports_unicode { "╯" } else { "+" };
    let side = if caps.supports_unicode { "│" } else { "|" };

    let title_display = format!(" {} ", title);
    // Use saturating arithmetic to prevent underflow
    let border_len = width
        .saturating_sub(title_display.len())
        .saturating_sub(2)
        .max(1);
    let top = format!(
        "{}{}{}{}",
        style(corner_tl).cyan(),
        style(&title_display).cyan().bold(),
        style(border_char.repeat(border_len)).cyan(),
        style(corner_tr).cyan()
    );

    let bottom_border_len = width.saturating_sub(2).max(1);
    let bottom = format!(
        "{}{}{}",
        style(corner_bl).cyan(),
        style(border_char.repeat(bottom_border_len)).cyan(),
        style(corner_br).cyan()
    );

    println!("{}", top);
    let content_width = width.saturating_sub(4).max(1);
    for line in content.lines() {
        let padded = format!("{:width$}", line, width = content_width);
        println!("{} {} {}", style(side).cyan(), padded, style(side).cyan());
    }
    println!("{}", bottom);
}

/// Print a success message
pub fn print_success(message: &str) {
    let prefix = convert_emoji(":check:");
    println!("{} {}", style(prefix).green(), style(message).green());
}

/// Print an error message
pub fn print_error(message: &str) {
    let prefix = convert_emoji(":x:");
    println!("{} {}", style(prefix).red(), style(message).red().bold());
}

/// Print a warning message
pub fn print_warning(message: &str) {
    let prefix = convert_emoji(":warning:");
    println!(
        "{} {}",
        style(prefix).yellow(),
        style(message).yellow().bold()
    );
}

/// Print an info message
pub fn print_info(message: &str) {
    let prefix = convert_emoji(":info:");
    println!("{} {}", style(prefix).blue(), style(message).blue());
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_caps_detection() {
        let caps = TerminalCapabilities::detect();
        // Just verify it doesn't panic
        assert!(caps.width > 0);
    }

    #[test]
    fn test_rich_text_builder() {
        let text = RichText::new()
            .text("Hello ")
            .bold("World")
            .text("!")
            .build();
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn test_emoji_conversion() {
        let result = convert_emoji("Launch :rocket: now!");
        // In test env, may or may not have unicode support
        assert!(result.contains("Launch") && result.contains("now!"));
    }

    #[test]
    fn test_syntax_highlighter_languages() {
        let langs = SyntaxHighlighter::supported_languages();
        assert!(langs.contains(&"rs".to_string()));
        assert!(langs.contains(&"json".to_string()));
    }

    #[test]
    fn test_syntax_highlighter_themes() {
        let themes = SyntaxHighlighter::available_themes();
        assert!(themes.contains(&"base16-ocean.dark".to_string()));
    }

    #[test]
    fn test_app_error() {
        let err = AppError::new("Something went wrong")
            .with_source("let x = broken;", 8, 6)
            .with_help("Check the syntax");

        assert_eq!(err.message, "Something went wrong");
        assert!(err.source_code.is_some());
        assert!(err.help_text.is_some());
    }

    #[test]
    fn test_color_palette() {
        let palette = ColorPalette::default();
        let info = palette.level_style("info");
        assert!(info.contains("INFO"));
    }
}
