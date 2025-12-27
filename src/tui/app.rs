//! TUI application state and logic.

use std::collections::HashMap;

use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::graph::{DerivedState, compute_derived, get_ready_issues};
use crate::issue::{Issue, Priority, Status};
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

/// which pane is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    Ready,
    All,
}

/// input mode for creating new issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    /// normal mode - no input active
    Normal,
    /// entering issue title
    Title(String),
    /// selecting priority
    Priority { title: String, selected: usize },
}

/// TUI application state.
pub struct App {
    /// all issues loaded from disk
    pub issues: HashMap<String, Issue>,
    /// ready issues (sorted)
    pub ready_issues: Vec<String>,
    /// all issues (sorted)
    pub all_issues: Vec<String>,
    /// currently selected index in ready pane
    pub ready_selected: usize,
    /// currently selected index in all pane
    pub all_selected: usize,
    /// which pane is active
    pub active_pane: ActivePane,
    /// current agent id
    pub agent_id: String,
    /// status message to display
    pub message: Option<String>,
    /// whether to show help
    pub show_help: bool,
    /// current input mode
    pub input_mode: InputMode,
}

impl App {
    /// create a new app by loading issues from disk.
    pub fn new(paths: &RepoPaths) -> Result<Self> {
        let agent_id = get_agent_id(&paths.worktree_root);
        let mut app = Self {
            issues: HashMap::new(),
            ready_issues: Vec::new(),
            all_issues: Vec::new(),
            ready_selected: 0,
            all_selected: 0,
            active_pane: ActivePane::Ready,
            agent_id,
            message: None,
            show_help: false,
            input_mode: InputMode::Normal,
        };
        app.reload_issues(paths)?;
        Ok(app)
    }

    /// reload issues from disk.
    pub fn reload_issues(&mut self, paths: &RepoPaths) -> Result<()> {
        self.issues = load_all_issues(paths)?;

        // build ready list
        let ready = get_ready_issues(&self.issues);
        self.ready_issues = ready.iter().map(|i| i.id().to_string()).collect();

        // build all list (sorted same way)
        let mut all: Vec<&Issue> = self.issues.values().collect();
        all.sort_by(|a, b| a.cmp_by_priority(b));
        self.all_issues = all.iter().map(|i| i.id().to_string()).collect();

        // clamp selections
        if self.ready_selected >= self.ready_issues.len() && !self.ready_issues.is_empty() {
            self.ready_selected = self.ready_issues.len() - 1;
        }
        if self.all_selected >= self.all_issues.len() && !self.all_issues.is_empty() {
            self.all_selected = self.all_issues.len() - 1;
        }

        self.message = Some("refreshed".to_string());
        Ok(())
    }

    /// get the currently selected issue id.
    pub fn selected_issue_id(&self) -> Option<&str> {
        match self.active_pane {
            ActivePane::Ready => self
                .ready_issues
                .get(self.ready_selected)
                .map(|s| s.as_str()),
            ActivePane::All => self.all_issues.get(self.all_selected).map(|s| s.as_str()),
        }
    }

    /// get the currently selected issue.
    pub fn selected_issue(&self) -> Option<&Issue> {
        self.selected_issue_id().and_then(|id| self.issues.get(id))
    }

    /// get derived state for an issue.
    pub fn derived_state(&self, issue: &Issue) -> DerivedState {
        compute_derived(issue, &self.issues)
    }

    /// move selection up.
    pub fn move_up(&mut self) {
        match self.active_pane {
            ActivePane::Ready => {
                if self.ready_selected > 0 {
                    self.ready_selected -= 1;
                }
            }
            ActivePane::All => {
                if self.all_selected > 0 {
                    self.all_selected -= 1;
                }
            }
        }
        self.message = None;
    }

    /// move selection down.
    pub fn move_down(&mut self) {
        match self.active_pane {
            ActivePane::Ready => {
                if self.ready_selected + 1 < self.ready_issues.len() {
                    self.ready_selected += 1;
                }
            }
            ActivePane::All => {
                if self.all_selected + 1 < self.all_issues.len() {
                    self.all_selected += 1;
                }
            }
        }
        self.message = None;
    }

    /// switch active pane.
    pub fn switch_pane(&mut self) {
        self.active_pane = match self.active_pane {
            ActivePane::Ready => ActivePane::All,
            ActivePane::All => ActivePane::Ready,
        };
        self.message = None;
    }

    /// toggle help display.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// start the selected issue.
    pub fn start_selected(&mut self, paths: &RepoPaths) -> Result<()> {
        let Some(issue_id) = self.selected_issue_id().map(|s| s.to_string()) else {
            self.message = Some("no issue selected".to_string());
            return Ok(());
        };

        let _lock = LockGuard::acquire(&paths.lock_path())?;

        let issue = self
            .issues
            .get_mut(&issue_id)
            .ok_or_else(|| BrdError::IssueNotFound(issue_id.clone()))?;

        if issue.status() == Status::Doing {
            let owner = issue.frontmatter.owner.as_deref().unwrap_or("unknown");
            self.message = Some(format!("already being worked on by '{}'", owner));
            return Ok(());
        }

        issue.frontmatter.status = Status::Doing;
        issue.frontmatter.owner = Some(self.agent_id.clone());
        issue.touch();

        // save to control root
        let issue_path = paths.issues_dir().join(format!("{}.md", issue_id));
        issue.save(&issue_path)?;

        // dual-write to local worktree if different
        if paths.worktree_root != paths.control_root {
            let local_path = paths
                .worktree_root
                .join(".braid/issues")
                .join(format!("{}.md", issue_id));
            issue.save(&local_path)?;
        }

        self.message = Some(format!("started {}", issue_id));
        self.reload_issues(paths)?;
        Ok(())
    }

    /// mark the selected issue as done.
    pub fn done_selected(&mut self, paths: &RepoPaths) -> Result<()> {
        let Some(issue_id) = self.selected_issue_id().map(|s| s.to_string()) else {
            self.message = Some("no issue selected".to_string());
            return Ok(());
        };

        let _lock = LockGuard::acquire(&paths.lock_path())?;

        let issue = self
            .issues
            .get_mut(&issue_id)
            .ok_or_else(|| BrdError::IssueNotFound(issue_id.clone()))?;

        issue.frontmatter.status = Status::Done;
        issue.frontmatter.owner = None;
        issue.touch();

        // save to control root
        let issue_path = paths.issues_dir().join(format!("{}.md", issue_id));
        issue.save(&issue_path)?;

        // dual-write to local worktree if different
        if paths.worktree_root != paths.control_root {
            let local_path = paths
                .worktree_root
                .join(".braid/issues")
                .join(format!("{}.md", issue_id));
            issue.save(&local_path)?;
        }

        self.message = Some(format!("done {}", issue_id));
        self.reload_issues(paths)?;
        Ok(())
    }

    /// start adding a new issue (enter title input mode).
    pub fn start_add_issue(&mut self) {
        self.input_mode = InputMode::Title(String::new());
        self.message = None;
    }

    /// cancel adding an issue.
    pub fn cancel_add_issue(&mut self) {
        self.input_mode = InputMode::Normal;
        self.message = Some("cancelled".to_string());
    }

    /// confirm title and move to priority selection.
    pub fn confirm_title(&mut self) {
        if let InputMode::Title(title) = &self.input_mode {
            if title.trim().is_empty() {
                self.message = Some("title cannot be empty".to_string());
                return;
            }
            self.input_mode = InputMode::Priority {
                title: title.clone(),
                selected: 2, // default to P2
            };
        }
    }

    /// create the issue with the given title and priority.
    pub fn create_issue(&mut self, paths: &RepoPaths) -> Result<()> {
        let (title, priority_idx) = match &self.input_mode {
            InputMode::Priority { title, selected } => (title.clone(), *selected),
            _ => return Ok(()),
        };

        let priority = match priority_idx {
            0 => Priority::P0,
            1 => Priority::P1,
            2 => Priority::P2,
            3 => Priority::P3,
            _ => Priority::P2,
        };

        let config = Config::load(&paths.config_path())?;
        let id = generate_issue_id(&config, &paths.issues_dir())?;
        let issue = Issue::new(id.clone(), title, priority, vec![]);

        let _lock = LockGuard::acquire(&paths.lock_path())?;

        // save to control root
        let issue_path = paths.issues_dir().join(format!("{}.md", id));
        issue.save(&issue_path)?;

        // dual-write to local worktree if different
        if paths.worktree_root != paths.control_root {
            let local_path = paths
                .worktree_root
                .join(".braid/issues")
                .join(format!("{}.md", id));
            issue.save(&local_path)?;
        }

        self.input_mode = InputMode::Normal;
        self.message = Some(format!("created {}", id));
        self.reload_issues(paths)?;
        Ok(())
    }
}

/// generate a unique issue ID.
fn generate_issue_id(config: &Config, issues_dir: &std::path::Path) -> Result<String> {
    use rand::Rng;

    let prefix = &config.id_prefix;
    let mut rng = rand::rng();

    for _ in 0..100 {
        let suffix: String = (0..4)
            .map(|_| {
                let idx = rng.random_range(0..36);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + idx - 10) as char
                }
            })
            .collect();
        let id = format!("{}-{}", prefix, suffix);
        let path = issues_dir.join(format!("{}.md", id));
        if !path.exists() {
            return Ok(id);
        }
    }

    Err(BrdError::Other("failed to generate unique ID".to_string()))
}

/// load all issues from the issues directory.
fn load_all_issues(paths: &RepoPaths) -> Result<HashMap<String, Issue>> {
    let mut issues = HashMap::new();
    let issues_dir = paths.issues_dir();

    if !issues_dir.exists() {
        return Ok(issues);
    }

    for entry in std::fs::read_dir(&issues_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "md") {
            match Issue::load(&path) {
                Ok(issue) => {
                    issues.insert(issue.id().to_string(), issue);
                }
                Err(e) => {
                    // log warning but continue
                    eprintln!("warning: failed to load {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(issues)
}

/// get agent ID from worktree.
fn get_agent_id(worktree_root: &std::path::Path) -> String {
    let agent_toml = worktree_root.join(".braid/agent.toml");
    if let Ok(content) = std::fs::read_to_string(&agent_toml)
        && let Ok(value) = content.parse::<toml::Table>()
        && let Some(toml::Value::String(id)) = value.get("agent_id")
    {
        return id.clone();
    }
    std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
}
