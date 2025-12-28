//! TUI event handling.

use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, InputMode};
use crate::error::Result;
use crate::repo::RepoPaths;

/// handle events. returns true if the app should quit.
pub fn handle_events(app: &mut App, paths: &RepoPaths) -> Result<bool> {
    // poll with timeout to allow for refresh
    if event::poll(Duration::from_millis(100))?
        && let Event::Key(key) = event::read()?
    {
        return handle_key_event(app, paths, key);
    }

    Ok(false)
}

fn handle_key_event(app: &mut App, paths: &RepoPaths, key: KeyEvent) -> Result<bool> {
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
                    if *selected < 3 {
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
        KeyCode::Left | KeyCode::Char('h') => app.move_dep_prev(),
        KeyCode::Right | KeyCode::Char('l') => app.move_dep_next(),

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
        KeyCode::Enter => app.open_selected_dependency(),

        // help
        KeyCode::Char('?') => app.toggle_help(),

        _ => {}
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::super::app::ActivePane;
    use super::*;
    use crate::config::Config;
    use crate::issue::{Issue, Priority, Status};
    use crate::repo::RepoPaths;
    use std::fs;
    use tempfile::TempDir;

    struct TestEnv {
        _dir: TempDir,
        paths: RepoPaths,
        config: Config,
    }

    impl TestEnv {
        fn new() -> Self {
            let dir = tempfile::tempdir().expect("failed to create temp dir");
            let worktree_root = dir.path().to_path_buf();
            let git_common_dir = worktree_root.join(".git");
            let brd_common_dir = git_common_dir.join("brd");
            fs::create_dir_all(&brd_common_dir).expect("failed to create brd dir");
            fs::create_dir_all(worktree_root.join(".braid/issues"))
                .expect("failed to create issues dir");

            let config = Config::default();
            let config_path = worktree_root.join(".braid/config.toml");
            config.save(&config_path).expect("failed to write config");

            Self {
                _dir: dir,
                paths: RepoPaths {
                    worktree_root,
                    git_common_dir,
                    brd_common_dir,
                },
                config,
            }
        }

        fn add_issue(&self, id: &str, title: &str, priority: Priority, status: Status) {
            let mut issue = Issue::new(id.to_string(), title.to_string(), priority, vec![]);
            issue.frontmatter.status = status;
            let issue_path = self
                .paths
                .issues_dir(&self.config)
                .join(format!("{}.md", id));
            issue.save(&issue_path).expect("failed to save issue");
        }

        fn add_issue_with_deps(
            &self,
            id: &str,
            title: &str,
            priority: Priority,
            status: Status,
            deps: Vec<String>,
        ) {
            let mut issue = Issue::new(id.to_string(), title.to_string(), priority, deps);
            issue.frontmatter.status = status;
            let issue_path = self
                .paths
                .issues_dir(&self.config)
                .join(format!("{}.md", id));
            issue.save(&issue_path).expect("failed to save issue");
        }

        fn app(&self) -> App {
            App::new(&self.paths).expect("failed to create app")
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_reload_clamps_selection() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "first", Priority::P1, Status::Todo);
        env.add_issue("brd-bbbb", "second", Priority::P2, Status::Todo);

        let mut app = env.app();
        app.ready_selected = 10;
        app.all_selected = 10;
        app.reload_issues(&env.paths).expect("reload failed");

        assert_eq!(app.ready_selected, app.ready_issues.len() - 1);
        assert_eq!(app.all_selected, app.all_issues.len() - 1);
    }

    #[test]
    fn test_pane_switch_clears_message() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "first", Priority::P1, Status::Todo);
        env.add_issue("brd-bbbb", "second", Priority::P2, Status::Todo);

        let mut app = env.app();
        app.message = Some("note".to_string());
        handle_key_event(&mut app, &env.paths, key(KeyCode::Tab)).expect("tab failed");
        assert_eq!(app.active_pane, ActivePane::All);
        assert!(app.message.is_none());

        app.message = Some("note".to_string());
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("down failed");
        assert!(app.message.is_none());
    }

    #[test]
    fn test_help_mode_ignores_other_keys() {
        let env = TestEnv::new();
        let mut app = env.app();
        app.show_help = true;
        app.message = Some("note".to_string());

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('a')))
            .expect("help ignored key failed");
        assert!(app.show_help);
        assert!(matches!(app.input_mode, InputMode::Normal));
        assert_eq!(app.message.as_deref(), Some("note"));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('?'))).expect("help close failed");
        assert!(!app.show_help);
    }

    #[test]
    fn test_add_issue_flow() {
        let env = TestEnv::new();
        let mut app = env.app();

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('a'))).expect("start add failed");
        assert!(matches!(
            app.input_mode,
            InputMode::Title(ref title) if title.is_empty()
        ));
        assert!(app.message.is_none());

        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("empty title failed");
        assert_eq!(app.message.as_deref(), Some("title cannot be empty"));
        assert!(matches!(app.input_mode, InputMode::Title(_)));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('t'))).expect("title char failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('e'))).expect("title char failed");
        assert!(matches!(
            app.input_mode,
            InputMode::Title(ref title) if title == "te"
        ));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("confirm title failed");
        assert!(
            matches!(app.input_mode, InputMode::Priority { ref title, selected } if title == "te" && selected == 2)
        );

        handle_key_event(&mut app, &env.paths, key(KeyCode::Up)).expect("priority up failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Up)).expect("priority up failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Up)).expect("priority up failed");
        assert!(matches!(app.input_mode, InputMode::Priority { selected, .. } if selected == 0));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("priority down failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("priority down failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("priority down failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("priority down failed");
        assert!(matches!(app.input_mode, InputMode::Priority { selected, .. } if selected == 3));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("create issue failed");
        assert!(matches!(app.input_mode, InputMode::Normal));
        assert_eq!(app.message.as_deref(), Some("refreshed"));
        assert_eq!(app.all_issues.len(), 1);

        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("move down failed");
        assert!(app.message.is_none());
    }

    #[test]
    fn test_edit_title_flow() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "old", Priority::P2, Status::Todo);
        let mut app = env.app();

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('e'))).expect("start edit failed");
        assert!(matches!(app.input_mode, InputMode::EditSelect { selected, .. } if selected == 0));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("confirm field failed");
        assert!(
            matches!(app.input_mode, InputMode::EditTitle { ref current, .. } if current == "old")
        );

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('x'))).expect("edit char failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("save edit failed");
        assert!(matches!(app.input_mode, InputMode::Normal));
        assert_eq!(app.message.as_deref(), Some("refreshed"));
        assert_eq!(app.issues.get("brd-aaaa").unwrap().title(), "oldx");
    }

    #[test]
    fn test_edit_priority_flow() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "issue", Priority::P2, Status::Todo);
        let mut app = env.app();

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('e'))).expect("start edit failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("select priority failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter))
            .expect("confirm priority failed");
        assert!(
            matches!(app.input_mode, InputMode::EditPriority { selected, .. } if selected == 2)
        );

        handle_key_event(&mut app, &env.paths, key(KeyCode::Up)).expect("priority up failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("save priority failed");
        assert!(matches!(app.input_mode, InputMode::Normal));
        assert_eq!(app.issues.get("brd-aaaa").unwrap().priority(), Priority::P1);
    }

    #[test]
    fn test_edit_status_flow() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "issue", Priority::P2, Status::Todo);
        let mut app = env.app();

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('e'))).expect("start edit failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("select status failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("select status failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("confirm status failed");
        assert!(matches!(app.input_mode, InputMode::EditStatus { selected, .. } if selected == 0));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("status down failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("status down failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("save status failed");
        assert!(matches!(app.input_mode, InputMode::Normal));
        assert_eq!(app.issues.get("brd-aaaa").unwrap().status(), Status::Done);
    }

    #[test]
    fn test_quit_keys() {
        let env = TestEnv::new();
        let mut app = env.app();

        let quit =
            handle_key_event(&mut app, &env.paths, key(KeyCode::Char('q'))).expect("quit failed");
        assert!(quit);

        let quit = handle_key_event(
            &mut app,
            &env.paths,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        )
        .expect("ctrl-c failed");
        assert!(quit);
    }

    #[test]
    fn test_dependency_navigation_and_open() {
        let env = TestEnv::new();
        env.add_issue("brd-dep1", "dep one", Priority::P2, Status::Todo);
        env.add_issue("brd-dep2", "dep two", Priority::P3, Status::Todo);
        env.add_issue_with_deps(
            "brd-main",
            "main issue",
            Priority::P1,
            Status::Todo,
            vec!["brd-dep1".to_string(), "brd-dep2".to_string()],
        );

        let mut app = env.app();
        handle_key_event(&mut app, &env.paths, key(KeyCode::Tab))
            .expect("switch pane failed");
        assert_eq!(app.selected_issue_id(), Some("brd-main"));
        assert_eq!(app.detail_dep_selected, Some(0));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Right)).expect("move dep failed");
        assert_eq!(app.detail_dep_selected, Some(1));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("open dep failed");
        assert_eq!(app.active_pane, ActivePane::All);
        assert_eq!(app.selected_issue_id(), Some("brd-dep2"));
    }
}
