use std::io;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let _log_guard = ttttrps::core::logging::init();
    log::info!("TTTTRPS v{} starting", ttttrps::VERSION);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_app(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    loop {
        terminal.draw(|frame| {
            let area = frame.area();

            let chunks = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);

            // Header
            let header = Paragraph::new(Line::from(vec![
                Span::styled(
                    " TTTTRPS ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("- AI-Powered TTRPG Assistant (TUI)"),
            ]))
            .block(Block::default().borders(Borders::ALL));
            frame.render_widget(header, chunks[0]);

            // Main content
            let content = Paragraph::new(vec![
                Line::raw(""),
                Line::raw("  Welcome to TTTTRPS - your AI-powered Game Master companion."),
                Line::raw(""),
                Line::raw("  This is the TUI skeleton. Core systems are ready:"),
                Line::raw("    - LLM routing (Claude, OpenAI, Ollama, Gemini)"),
                Line::raw("    - Campaign & session management"),
                Line::raw("    - Document ingestion & RAG search"),
                Line::raw("    - NPC generation & personality system"),
                Line::raw("    - Voice synthesis queue"),
                Line::raw(""),
                Line::raw("  TUI panels coming soon: chat, library, campaign, combat tracker"),
            ])
            .block(
                Block::default()
                    .title(" Main ")
                    .borders(Borders::ALL),
            );
            frame.render_widget(content, chunks[1]);

            // Footer
            let footer = Paragraph::new(Line::from(vec![
                Span::styled(" q ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw("Quit  "),
                Span::styled(" ? ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("Help"),
            ]))
            .block(Block::default().borders(Borders::ALL));
            frame.render_widget(footer, chunks[2]);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    return Ok(());
                }
            }
        }
    }
}
