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
    // handle diff panel mode
    if app.is_diff_visible() {
        handle_diff_panel_key(app, key);
        return Ok(false);
    }

    // handle help mode separately
    if app.show_help {
        if key.code == KeyCode::Char('?') || key.code == KeyCode::Esc {
            app.toggle_help();
        }
        return Ok(false);
    }

    // handle detail overlay mode
    if app.show_detail_overlay {
        if key.code == KeyCode::Esc
            || key.code == KeyCode::Char('q')
            || key.code == KeyCode::Enter
            || key.code == KeyCode::Tab
        {
            app.hide_detail_overlay();
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
                KeyCode::Enter => app.confirm_priority(),
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
        InputMode::Type {
            title,
            priority,
            selected,
        } => {
            match key.code {
                KeyCode::Esc => app.cancel_add_issue(),
                KeyCode::Enter => app.confirm_type(),
                KeyCode::Up | KeyCode::Char('k') => {
                    if *selected > 0 {
                        app.input_mode = InputMode::Type {
                            title: title.clone(),
                            priority: *priority,
                            selected: selected - 1,
                        };
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *selected < 2 {
                        // 3 options: (none), design, meta
                        app.input_mode = InputMode::Type {
                            title: title.clone(),
                            priority: *priority,
                            selected: selected + 1,
                        };
                    }
                }
                _ => {}
            }
            return Ok(false);
        }
        InputMode::Deps {
            title,
            priority,
            type_idx,
            selected_deps,
            cursor,
        } => {
            let max_cursor = app.sorted_issues.len().saturating_sub(1);
            match key.code {
                KeyCode::Esc => app.cancel_add_issue(),
                KeyCode::Enter => {
                    if let Err(e) = app.create_issue(paths) {
                        app.message = Some(format!("error: {}", e));
                        app.input_mode = InputMode::Normal;
                    }
                }
                KeyCode::Char(' ') => app.toggle_dep(),
                KeyCode::Up | KeyCode::Char('k') => {
                    if *cursor > 0 {
                        app.input_mode = InputMode::Deps {
                            title: title.clone(),
                            priority: *priority,
                            type_idx: *type_idx,
                            selected_deps: selected_deps.clone(),
                            cursor: cursor - 1,
                        };
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *cursor < max_cursor {
                        app.input_mode = InputMode::Deps {
                            title: title.clone(),
                            priority: *priority,
                            type_idx: *type_idx,
                            selected_deps: selected_deps.clone(),
                            cursor: cursor + 1,
                        };
                    }
                }
                _ => {}
            }
            return Ok(false);
        }
        InputMode::Filter(current) => {
            match key.code {
                KeyCode::Esc => app.cancel_filter(),
                KeyCode::Enter => app.confirm_filter(),
                KeyCode::Backspace => {
                    let mut s = current.clone();
                    s.pop();
                    app.filter_query = s.clone();
                    app.apply_filter();
                    app.input_mode = InputMode::Filter(s);
                    app.message = None;
                }
                KeyCode::Char(c) => {
                    let mut s = current.clone();
                    s.push(c);
                    app.filter_query = s.clone();
                    app.apply_filter();
                    app.input_mode = InputMode::Filter(s);
                    app.message = None;
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
        KeyCode::Char('g') => app.move_to_top(),
        KeyCode::Char('G') => app.move_to_bottom(),
        KeyCode::Left | KeyCode::Char('h') => app.move_dep_prev(),
        KeyCode::Right | KeyCode::Char('l') => app.move_dep_next(),

        // actions
        KeyCode::Char('a') | KeyCode::Char('n') => app.start_add_issue(),
        KeyCode::Char('e') => app.open_in_editor(paths),
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
        KeyCode::Enter => {
            if !app.show_details {
                // when details pane is hidden, show full-screen overlay
                app.show_detail_overlay();
            } else {
                // when details pane is visible, open selected dependency
                app.open_selected_dependency();
            }
        }
        KeyCode::Tab => app.toggle_details(),

        // views
        KeyCode::Char('1') => app.view = crate::tui::app::View::Dashboard,
        KeyCode::Char('2') => app.view = crate::tui::app::View::Issues,
        KeyCode::Char('3') => {
            app.reload_worktrees();
            app.view = crate::tui::app::View::Agents;
        }

        // filter
        KeyCode::Char('/') => app.start_filter(),
        KeyCode::Char('R') => app.toggle_ready_filter(),
        KeyCode::Esc => {
            if app.has_filter() {
                app.clear_filter();
            }
        }

        // help
        KeyCode::Char('?') => app.toggle_help(),

        _ => {}
    }

    Ok(false)
}

/// handle key events when diff panel is visible.
fn handle_diff_panel_key(app: &mut App, key: KeyEvent) {
    // get content height for scroll bounds
    let content_height = app
        .diff_content
        .as_ref()
        .map(|t| t.lines.len() as u16)
        .unwrap_or(0);
    // assume a reasonable viewport height (will be adjusted by actual rendering)
    let viewport_height = 40u16;

    match key.code {
        // close diff panel
        KeyCode::Esc | KeyCode::Char('q') => app.close_diff(),

        // scroll down
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(ref mut state) = app.diff_panel_state {
                state.scroll_down(1, content_height, viewport_height);
            }
        }

        // scroll up
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(ref mut state) = app.diff_panel_state {
                state.scroll_up(1);
            }
        }

        // page down
        KeyCode::PageDown | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(ref mut state) = app.diff_panel_state {
                state.page_down(content_height, viewport_height);
            }
        }

        // page up
        KeyCode::PageUp | KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(ref mut state) = app.diff_panel_state {
                state.page_up(viewport_height);
            }
        }

        // go to top
        KeyCode::Char('g') => {
            if let Some(ref mut state) = app.diff_panel_state {
                state.scroll_to_top();
            }
        }

        // go to bottom
        KeyCode::Char('G') => {
            if let Some(ref mut state) = app.diff_panel_state {
                state.scroll_to_bottom(content_height, viewport_height);
            }
        }

        _ => {}
    }
}

#[cfg(test)]
mod tests {
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
        env.add_issue("brd-aaaa", "first", Priority::P1, Status::Open);
        env.add_issue("brd-bbbb", "second", Priority::P2, Status::Open);

        let mut app = env.app();
        app.selected = 10;
        app.reload_issues(&env.paths).expect("reload failed");

        assert_eq!(app.selected, app.sorted_issues.len() - 1);
    }

    #[test]
    fn test_navigation_clears_message() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "first", Priority::P1, Status::Open);
        env.add_issue("brd-bbbb", "second", Priority::P2, Status::Open);

        let mut app = env.app();
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

        // start add flow
        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('a'))).expect("start add failed");
        assert!(matches!(
            app.input_mode,
            InputMode::Title(ref title) if title.is_empty()
        ));
        assert!(app.message.is_none());

        // empty title rejected
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("empty title failed");
        assert_eq!(app.message.as_deref(), Some("title cannot be empty"));
        assert!(matches!(app.input_mode, InputMode::Title(_)));

        // type title
        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('t'))).expect("title char failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('e'))).expect("title char failed");
        assert!(matches!(
            app.input_mode,
            InputMode::Title(ref title) if title == "te"
        ));

        // confirm title → priority selection
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("confirm title failed");
        assert!(
            matches!(app.input_mode, InputMode::Priority { ref title, selected } if title == "te" && selected == 2)
        );

        // navigate priority
        handle_key_event(&mut app, &env.paths, key(KeyCode::Up)).expect("priority up failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Up)).expect("priority up failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Up)).expect("priority up failed");
        assert!(matches!(app.input_mode, InputMode::Priority { selected, .. } if selected == 0));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("priority down failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("priority down failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("priority down failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("priority down failed");
        assert!(matches!(app.input_mode, InputMode::Priority { selected, .. } if selected == 3));

        // confirm priority → type selection
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter))
            .expect("confirm priority failed");
        assert!(
            matches!(app.input_mode, InputMode::Type { priority, selected, .. } if priority == 3 && selected == 0)
        );

        // navigate type (0=none, 1=design, 2=meta)
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("type down failed");
        assert!(matches!(app.input_mode, InputMode::Type { selected, .. } if selected == 1));

        // confirm type → deps selection
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("confirm type failed");
        assert!(matches!(app.input_mode, InputMode::Deps { type_idx, .. } if type_idx == 1));

        // confirm deps (no deps selected) → create issue
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("create issue failed");
        assert!(matches!(app.input_mode, InputMode::Normal));
        assert_eq!(app.message.as_deref(), Some("refreshed"));
        assert_eq!(app.sorted_issues.len(), 1);

        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("move down failed");
        assert!(app.message.is_none());
    }

    #[test]
    fn test_filter_inline_updates_query() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "alpha", Priority::P1, Status::Open);
        env.add_issue("brd-bbbb", "bravo", Priority::P2, Status::Open);

        let mut app = env.app();
        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('/')))
            .expect("start filter failed");
        assert!(matches!(
            app.input_mode,
            InputMode::Filter(ref query) if query.is_empty()
        ));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('a')))
            .expect("filter char failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('l')))
            .expect("filter char failed");

        assert_eq!(app.filter_query, "al");
        assert_eq!(app.visible_issues().len(), 1);
        assert_eq!(
            app.visible_issues().first().map(String::as_str),
            Some("brd-aaaa")
        );

        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("confirm filter failed");
        assert!(matches!(app.input_mode, InputMode::Normal));
        assert!(app.has_filter());
    }

    #[test]
    fn test_filter_escape_clears() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "alpha", Priority::P1, Status::Open);

        let mut app = env.app();
        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('/')))
            .expect("start filter failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('a')))
            .expect("filter char failed");
        handle_key_event(&mut app, &env.paths, key(KeyCode::Esc)).expect("clear filter failed");

        assert!(matches!(app.input_mode, InputMode::Normal));
        assert!(app.filter_query.is_empty());
        assert!(app.status_filter.is_empty());
    }

    #[test]
    fn test_view_switching() {
        let env = TestEnv::new();
        let mut app = env.app();

        assert_eq!(app.view, crate::tui::app::View::Issues);

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('1')))
            .expect("switch to dashboard failed");
        assert_eq!(app.view, crate::tui::app::View::Dashboard);

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('2')))
            .expect("switch to issues failed");
        assert_eq!(app.view, crate::tui::app::View::Issues);
    }

    #[test]
    fn test_open_in_editor_sets_flag() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "issue", Priority::P2, Status::Open);
        let mut app = env.app();

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('e')))
            .expect("open editor failed");
        // The editor_file flag should be set
        assert!(app.editor_file.is_some());
        assert!(app.editor_file.as_ref().unwrap().ends_with("brd-aaaa.md"));
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
        env.add_issue("brd-dep1", "dep one", Priority::P2, Status::Open);
        env.add_issue("brd-dep2", "dep two", Priority::P3, Status::Open);
        env.add_issue_with_deps(
            "brd-main",
            "main issue",
            Priority::P1,
            Status::Open,
            vec!["brd-dep1".to_string(), "brd-dep2".to_string()],
        );

        let mut app = env.app();
        assert_eq!(app.selected_issue_id(), Some("brd-main"));
        assert_eq!(app.detail_dep_selected, Some(0));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Right)).expect("move dep failed");
        assert_eq!(app.detail_dep_selected, Some(1));

        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("open dep failed");
        assert_eq!(app.selected_issue_id(), Some("brd-dep2"));
    }

    #[test]
    fn test_tab_toggles_details_pane() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "issue", Priority::P1, Status::Open);
        let mut app = env.app();

        // default: details shown
        assert!(app.show_details);

        // toggle off
        handle_key_event(&mut app, &env.paths, key(KeyCode::Tab)).expect("tab failed");
        assert!(!app.show_details);

        // toggle on
        handle_key_event(&mut app, &env.paths, key(KeyCode::Tab)).expect("tab failed");
        assert!(app.show_details);
    }

    #[test]
    fn test_enter_shows_overlay_when_details_hidden() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "issue", Priority::P1, Status::Open);
        let mut app = env.app();

        // hide details pane
        app.show_details = false;

        // press enter should show overlay
        handle_key_event(&mut app, &env.paths, key(KeyCode::Enter)).expect("enter failed");
        assert!(app.show_detail_overlay);
    }

    #[test]
    fn test_detail_overlay_closes_on_esc() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "issue", Priority::P1, Status::Open);
        let mut app = env.app();

        app.show_detail_overlay = true;

        handle_key_event(&mut app, &env.paths, key(KeyCode::Esc)).expect("esc failed");
        assert!(!app.show_detail_overlay);
    }

    #[test]
    fn test_detail_overlay_closes_on_tab() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "issue", Priority::P1, Status::Open);
        let mut app = env.app();

        app.show_detail_overlay = true;

        handle_key_event(&mut app, &env.paths, key(KeyCode::Tab)).expect("tab failed");
        assert!(!app.show_detail_overlay);
    }

    #[test]
    fn test_detail_overlay_ignores_navigation() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "first", Priority::P1, Status::Open);
        env.add_issue("brd-bbbb", "second", Priority::P2, Status::Open);
        let mut app = env.app();

        app.show_detail_overlay = true;
        let initial_selected = app.selected;

        // navigation keys should be ignored
        handle_key_event(&mut app, &env.paths, key(KeyCode::Down)).expect("down failed");
        assert!(app.show_detail_overlay);
        assert_eq!(app.selected, initial_selected);

        handle_key_event(&mut app, &env.paths, key(KeyCode::Char('j'))).expect("j failed");
        assert!(app.show_detail_overlay);
        assert_eq!(app.selected, initial_selected);
    }
}
