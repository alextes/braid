//! interactive TUI for braid issue tracker.

mod app;
pub mod diff_panel;
pub mod diff_render;
mod event;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

use crate::error::Result;
use crate::repo::RepoPaths;

use app::App;
use event::handle_events;

/// run the TUI application.
pub fn run(paths: &RepoPaths) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app state
    let mut app = App::new(paths)?;

    // main loop
    let result = run_loop(&mut terminal, &mut app, paths);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    paths: &RepoPaths,
) -> Result<()> {
    let mut last_refresh = Instant::now();
    let refresh_interval = Duration::from_secs(2);
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if handle_events(app, paths)? {
            return Ok(());
        }

        // Handle external editor request
        if let Some(file_path) = app.editor_file.take() {
            open_in_editor(terminal, &file_path)?;
            app.reload_issues_with_message(paths, false)?;
            last_refresh = Instant::now();
        }

        // Auto-refresh every 2 seconds
        if last_refresh.elapsed() >= refresh_interval {
            app.reload_issues_with_message(paths, false)?;
            last_refresh = Instant::now();
        }
    }
}

fn open_in_editor(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    file_path: &std::path::Path,
) -> Result<()> {
    use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
    use std::process::Command;

    // Get editor from environment
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    // Leave alternate screen and disable raw mode
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    // Run editor
    let status = Command::new(&editor).arg(file_path).status();

    // Restore terminal
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    terminal.clear()?;

    if let Err(e) = status {
        return Err(crate::error::BrdError::Other(format!(
            "failed to open editor: {}",
            e
        )));
    }

    Ok(())
}
