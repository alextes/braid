//! TUI event handling.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};

use super::app::{App, InputMode};
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

        // handle input modes
        match &app.input_mode {
            InputMode::Title(current) => {
                match key.code {
                    KeyCode::Esc => app.cancel_add_issue(),
                    KeyCode::Enter => app.confirm_title(),
                    KeyCode::Backspace => {
                        let mut s = current.clone();
                        s.pop();
                        app.input_mode = InputMode::Title(s);
                    }
                    KeyCode::Char(c) => {
                        let mut s = current.clone();
                        s.push(c);
                        app.input_mode = InputMode::Title(s);
                    }
                    _ => {}
                }
                return Ok(false);
            }
            InputMode::Priority { title, selected } => {
                match key.code {
                    KeyCode::Esc => app.cancel_add_issue(),
                    KeyCode::Enter => {
                        if let Err(e) = app.create_issue(paths) {
                            app.message = Some(format!("error: {}", e));
                            app.input_mode = InputMode::Normal;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if *selected > 0 {
                            app.input_mode = InputMode::Priority {
                                title: title.clone(),
                                selected: selected - 1,
                            };
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if *selected < 3 {
                            app.input_mode = InputMode::Priority {
                                title: title.clone(),
                                selected: selected + 1,
                            };
                        }
                    }
                    _ => {}
                }
                return Ok(false);
            }
            InputMode::EditSelect { issue_id, selected } => {
                match key.code {
                    KeyCode::Esc => app.cancel_edit(),
                    KeyCode::Enter => app.confirm_edit_field(),
                    KeyCode::Up | KeyCode::Char('k') => {
                        if *selected > 0 {
                            app.input_mode = InputMode::EditSelect {
                                issue_id: issue_id.clone(),
                                selected: selected - 1,
                            };
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if *selected < 2 {
                            app.input_mode = InputMode::EditSelect {
                                issue_id: issue_id.clone(),
                                selected: selected + 1,
                            };
                        }
                    }
                    _ => {}
                }
                return Ok(false);
            }
            InputMode::EditTitle { issue_id, current } => {
                match key.code {
                    KeyCode::Esc => app.cancel_edit(),
                    KeyCode::Enter => {
                        if let Err(e) = app.save_edit(paths) {
                            app.message = Some(format!("error: {}", e));
                            app.input_mode = InputMode::Normal;
                        }
                    }
                    KeyCode::Backspace => {
                        let mut s = current.clone();
                        s.pop();
                        app.input_mode = InputMode::EditTitle {
                            issue_id: issue_id.clone(),
                            current: s,
                        };
                    }
                    KeyCode::Char(c) => {
                        let mut s = current.clone();
                        s.push(c);
                        app.input_mode = InputMode::EditTitle {
                            issue_id: issue_id.clone(),
                            current: s,
                        };
                    }
                    _ => {}
                }
                return Ok(false);
            }
            InputMode::EditPriority { issue_id, selected } => {
                match key.code {
                    KeyCode::Esc => app.cancel_edit(),
                    KeyCode::Enter => {
                        if let Err(e) = app.save_edit(paths) {
                            app.message = Some(format!("error: {}", e));
                            app.input_mode = InputMode::Normal;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if *selected > 0 {
                            app.input_mode = InputMode::EditPriority {
                                issue_id: issue_id.clone(),
                                selected: selected - 1,
                            };
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if *selected < 3 {
                            app.input_mode = InputMode::EditPriority {
                                issue_id: issue_id.clone(),
                                selected: selected + 1,
                            };
                        }
                    }
                    _ => {}
                }
                return Ok(false);
            }
            InputMode::EditStatus { issue_id, selected } => {
                match key.code {
                    KeyCode::Esc => app.cancel_edit(),
                    KeyCode::Enter => {
                        if let Err(e) = app.save_edit(paths) {
                            app.message = Some(format!("error: {}", e));
                            app.input_mode = InputMode::Normal;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if *selected > 0 {
                            app.input_mode = InputMode::EditStatus {
                                issue_id: issue_id.clone(),
                                selected: selected - 1,
                            };
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if *selected < 2 {
                            app.input_mode = InputMode::EditStatus {
                                issue_id: issue_id.clone(),
                                selected: selected + 1,
                            };
                        }
                    }
                    _ => {}
                }
                return Ok(false);
            }
            InputMode::Normal => {}
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
            KeyCode::Char('a') | KeyCode::Char('n') => app.start_add_issue(),
            KeyCode::Char('e') => app.start_edit_issue(),
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
