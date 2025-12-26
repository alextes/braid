//! TUI event handling.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use super::app::App;
use crate::error::Result;
use crate::repo::RepoPaths;

/// handle events. returns true if the app should quit.
pub fn handle_events(app: &mut App, paths: &RepoPaths) -> Result<bool> {
    // poll with timeout to allow for refresh
    if event::poll(Duration::from_millis(100))?
        && let Event::Key(key) = event::read()?
    {
        // handle help mode separately
        if app.show_help {
            if key.code == KeyCode::Char('?') || key.code == KeyCode::Esc {
                app.toggle_help();
            }
            return Ok(false);
        }

        match key.code {
            // quit
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),

            // navigation
            KeyCode::Up | KeyCode::Char('k') => app.move_up(),
            KeyCode::Down | KeyCode::Char('j') => app.move_down(),
            KeyCode::Tab => app.switch_pane(),

            // actions
            KeyCode::Char('s') => {
                if let Err(e) = app.start_selected(paths) {
                    app.message = Some(format!("error: {}", e));
                }
            }
            KeyCode::Char('d') => {
                if let Err(e) = app.done_selected(paths) {
                    app.message = Some(format!("error: {}", e));
                }
            }
            KeyCode::Char('r') => {
                if let Err(e) = app.reload_issues(paths) {
                    app.message = Some(format!("error: {}", e));
                }
            }

            // help
            KeyCode::Char('?') => app.toggle_help(),

            _ => {}
        }
    }

    Ok(false)
}
