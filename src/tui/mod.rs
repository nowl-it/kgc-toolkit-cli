//! Terminal User Interface module

pub mod app;
pub mod components;
pub mod event;
pub mod ui;
pub mod theme;
pub mod keybindings;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;
use thiserror::Error;

pub use app::App;
pub use theme::Theme;

#[derive(Error, Debug)]
pub enum TuiError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Terminal error: {0}")]
    Terminal(String),
}

/// Run the TUI application
pub async fn run() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();
    app.set_status("Checking tools status...");
    event::spawn_refresh_tools(app.message_tx.clone());

    // Run app
    let result = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> anyhow::Result<()> {
    loop {
        // Auto-fetch versions when entering Download tab
        if app.active_tab != app.previous_tab {
            if app.active_tab == app::Tab::Download && app.available_versions.is_empty() {
                app.set_status("Fetching versions...");
                event::spawn_fetch_versions(app.message_tx.clone());
            }
            if app.active_tab == app::Tab::Convert && app.available_xapks.is_empty() {
                let source_dir = app
                    .output_folder
                    .clone()
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
                app.set_status(format!("Scanning XAPK files in {}...", source_dir.display()));
                event::spawn_scan_xapks(app.message_tx.clone(), source_dir);
            }
            if app.active_tab == app::Tab::Xml && app.xml_bundles.is_empty() && app.xml_files.is_empty() {
                app.set_status(format!("Scanning XML folder {}...", app.xml_output_folder.display()));
                event::spawn_scan_xml_files(app.message_tx.clone(), app.xml_output_folder.clone());
            }
            if app.active_tab == app::Tab::Xml && app.xml_remote_options.is_empty() {
                app.set_status("Loading remote XML options...");
                event::spawn_load_xml_remote_options(app.message_tx.clone());
            }
            app.previous_tab = app.active_tab;
        }

        // Process async messages
        app.process_messages();
        
        terminal.draw(|frame| ui::draw(frame, app))?;

        if let Some(event) = event::poll_event()? {
            if !event::handle_event(app, event)? {
                break;
            }
        }
        
        // Small delay to avoid busy waiting
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    Ok(())
}
