//! interactive TUI for braid issue tracker.

mod app;
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

use app::{App, ViewMode};
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
    let refresh_interval = Duration::from_secs(5);
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if handle_events(app, paths)? {
            return Ok(());
        }

        if app.view_mode == ViewMode::Live && last_refresh.elapsed() >= refresh_interval {
            app.reload_issues_with_message(paths, false)?;
            last_refresh = Instant::now();
        }
    }
}
