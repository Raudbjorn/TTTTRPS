use std::io;
use std::time::Duration;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use ttttrps::config::AppConfig;
use ttttrps::tui::app::AppState;
use ttttrps::tui::services::Services;

/// Restore terminal state — called from panic hook and normal exit.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

/// Install a panic hook that restores the terminal before printing the panic.
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));
}

#[tokio::main]
async fn main() {
    // Install panic hook BEFORE entering raw mode
    install_panic_hook();

    // Load configuration
    let config = AppConfig::load();

    // Initialize TUI-safe logging (file only, no stdout)
    let _log_guard = ttttrps::core::logging::init_tui();
    log::info!("TTTTRPS v{} starting", ttttrps::VERSION);

    // Create event channel
    let (event_tx, event_rx) = mpsc::unbounded_channel();

    // Initialize backend services
    let services = match Services::init(&config, event_tx.clone()).await {
        Ok(s) => s,
        Err(e) => {
            // Don't enter raw mode if services fail
            log::error!("Failed to initialize services: {e}");
            eprintln!("Fatal: failed to initialize services: {e}");
            std::process::exit(1);
        }
    };
    log::info!("All services initialized");

    // Setup terminal
    if let Err(e) = enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {e}");
        std::process::exit(1);
    }
    let mut stdout = io::stdout();
    if let Err(e) = execute!(stdout, EnterAlternateScreen) {
        restore_terminal();
        eprintln!("Failed to enter alternate screen: {e}");
        std::process::exit(1);
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            restore_terminal();
            eprintln!("Failed to create terminal: {e}");
            std::process::exit(1);
        }
    };

    // Create app state and run the event loop
    let mut app = AppState::new(event_rx, event_tx, services);
    let tick_rate = Duration::from_millis(config.tui.tick_rate_ms);

    let result = app.run(&mut terminal, tick_rate).await;

    // Restore terminal
    restore_terminal();
    terminal.show_cursor().ok();

    if let Err(e) = result {
        log::error!("Application error: {e}");
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    log::info!("TTTTRPS shut down cleanly");
}
