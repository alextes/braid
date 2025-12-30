//! TUI application state and logic.

use std::collections::{HashMap, HashSet};

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

/// which view mode is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Normal,
    Live,
}

/// input mode for creating/editing issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    /// normal mode - no input active
    Normal,
    /// entering issue title (for new issue)
    Title(String),
    /// selecting priority (for new issue)
    Priority { title: String, selected: usize },
    /// selecting which field to edit
    EditSelect { issue_id: String, selected: usize },
    /// editing issue title
    EditTitle { issue_id: String, current: String },
    /// editing issue priority
    EditPriority { issue_id: String, selected: usize },
    /// editing issue status
    EditStatus { issue_id: String, selected: usize },
    /// filtering issues in the all pane
    Filter(String),
}

/// TUI application state.
pub struct App {
    /// all issues loaded from disk
    pub issues: HashMap<String, Issue>,
    /// ready issues (sorted)
    pub ready_issues: Vec<String>,
    /// all issues (sorted)
    pub all_issues: Vec<String>,
    /// filtered all issues (when filter is active)
    pub filtered_all_issues: Vec<String>,
    /// in-progress issues (sorted by age)
    pub in_progress_issues: Vec<String>,
    /// recently completed issues (sorted by updated_at)
    pub recent_done_issues: Vec<String>,
    /// currently selected index in ready pane
    pub ready_selected: usize,
    /// scroll offset for ready list
    pub ready_offset: usize,
    /// currently selected index in all pane
    pub all_selected: usize,
    /// scroll offset for all list
    pub all_offset: usize,
    /// currently selected index in in-progress list
    pub in_progress_selected: usize,
    /// scroll offset for in-progress list
    pub in_progress_offset: usize,
    /// which pane is active
    pub active_pane: ActivePane,
    /// which view is active
    pub view_mode: ViewMode,
    /// current agent id
    pub agent_id: String,
    /// status message to display
    pub message: Option<String>,
    /// whether to show help
    pub show_help: bool,
    /// current input mode
    pub input_mode: InputMode,
    /// current config
    pub config: Config,
    /// selected dependency index in detail pane
    pub detail_dep_selected: Option<usize>,
    /// filter query for all issues pane
    pub all_filter_query: String,
    /// status filter for all issues (empty means show all)
    pub all_status_filter: HashSet<Status>,
}

const RECENT_DONE_LIMIT: usize = 8;

impl App {
    /// create a new app by loading issues from disk.
    pub fn new(paths: &RepoPaths) -> Result<Self> {
        let agent_id = get_agent_id(&paths.worktree_root);
        let config = Config::load(&paths.config_path())?;
        let mut app = Self {
            issues: HashMap::new(),
            ready_issues: Vec::new(),
            all_issues: Vec::new(),
            filtered_all_issues: Vec::new(),
            in_progress_issues: Vec::new(),
            recent_done_issues: Vec::new(),
            ready_selected: 0,
            ready_offset: 0,
            all_selected: 0,
            all_offset: 0,
            in_progress_selected: 0,
            in_progress_offset: 0,
            active_pane: ActivePane::Ready,
            view_mode: ViewMode::Normal,
            agent_id,
            message: None,
            show_help: false,
            input_mode: InputMode::Normal,
            config,
            detail_dep_selected: None,
            all_filter_query: String::new(),
            all_status_filter: HashSet::new(),
        };
        app.reload_issues(paths)?;
        Ok(app)
    }

    /// reload issues from disk.
    pub fn reload_issues(&mut self, paths: &RepoPaths) -> Result<()> {
        self.reload_issues_with_message(paths, true)
    }

    /// reload issues from disk, optionally showing a status message.
    pub fn reload_issues_with_message(
        &mut self,
        paths: &RepoPaths,
        show_message: bool,
    ) -> Result<()> {
        self.issues = load_all_issues(paths, &self.config)?;

        // build ready list
        let ready = get_ready_issues(&self.issues);
        self.ready_issues = ready.iter().map(|i| i.id().to_string()).collect();

        // build all list (sorted same way)
        let mut all: Vec<&Issue> = self.issues.values().collect();
        all.sort_by(|a, b| a.cmp_by_priority(b));
        self.all_issues = all.iter().map(|i| i.id().to_string()).collect();

        // build in-progress list (oldest first)
        let mut in_progress: Vec<&Issue> = self
            .issues
            .values()
            .filter(|issue| issue.status() == Status::Doing)
            .collect();
        in_progress.sort_by(|a, b| a.frontmatter.updated_at.cmp(&b.frontmatter.updated_at));
        self.in_progress_issues = in_progress.iter().map(|i| i.id().to_string()).collect();

        // build recent done list (most recent first)
        let mut recent_done: Vec<&Issue> = self
            .issues
            .values()
            .filter(|issue| matches!(issue.status(), Status::Done | Status::Skip))
            .collect();
        recent_done.sort_by(|a, b| b.frontmatter.updated_at.cmp(&a.frontmatter.updated_at));
        self.recent_done_issues = recent_done
            .iter()
            .take(RECENT_DONE_LIMIT)
            .map(|i| i.id().to_string())
            .collect();

        // clamp selections
        if self.ready_selected >= self.ready_issues.len() && !self.ready_issues.is_empty() {
            self.ready_selected = self.ready_issues.len() - 1;
        }
        if self.all_selected >= self.all_issues.len() && !self.all_issues.is_empty() {
            self.all_selected = self.all_issues.len() - 1;
        }
        if self.in_progress_selected >= self.in_progress_issues.len()
            && !self.in_progress_issues.is_empty()
        {
            self.in_progress_selected = self.in_progress_issues.len() - 1;
        }
        if self.ready_offset >= self.ready_issues.len() {
            self.ready_offset = 0;
        }
        if self.all_offset >= self.all_issues.len() {
            self.all_offset = 0;
        }
        if self.in_progress_offset >= self.in_progress_issues.len() {
            self.in_progress_offset = 0;
        }

        self.reset_dep_selection();
        self.apply_filter();
        if show_message {
            self.message = Some("refreshed".to_string());
        }
        Ok(())
    }

    /// apply the current filter to the all issues list.
    pub fn apply_filter(&mut self) {
        let query = self.all_filter_query.to_lowercase();
        self.filtered_all_issues = self
            .all_issues
            .iter()
            .filter(|id| {
                let Some(issue) = self.issues.get(*id) else {
                    return false;
                };
                // check status filter (empty means show all)
                if !self.all_status_filter.is_empty()
                    && !self.all_status_filter.contains(&issue.status())
                {
                    return false;
                }
                // check query filter
                if !query.is_empty() && !issue.title().to_lowercase().contains(&query) {
                    return false;
                }
                true
            })
            .cloned()
            .collect();

        // clamp selection to filtered list
        // use filtered list length directly since we just populated it
        let visible_len = if self.has_filter() {
            self.filtered_all_issues.len()
        } else {
            self.all_issues.len()
        };
        if self.all_selected >= visible_len && visible_len > 0 {
            self.all_selected = visible_len - 1;
        }
        if self.all_offset >= visible_len {
            self.all_offset = 0;
        }
    }

    /// returns true if a filter is currently active.
    pub fn has_filter(&self) -> bool {
        !self.all_filter_query.is_empty() || !self.all_status_filter.is_empty()
    }

    /// get the visible all issues list (filtered or unfiltered).
    pub fn visible_all_issues(&self) -> &Vec<String> {
        if self.has_filter() {
            &self.filtered_all_issues
        } else {
            &self.all_issues
        }
    }

    /// toggle a status in the filter.
    pub fn toggle_status_filter(&mut self, status: Status) {
        if self.all_status_filter.contains(&status) {
            self.all_status_filter.remove(&status);
        } else {
            self.all_status_filter.insert(status);
        }
        self.apply_filter();
        self.message = None;
    }

    /// clear all filters.
    pub fn clear_filter(&mut self) {
        self.all_filter_query.clear();
        self.all_status_filter.clear();
        self.apply_filter();
        self.message = Some("filter cleared".to_string());
    }

    /// start filter input mode.
    pub fn start_filter(&mut self) {
        self.input_mode = InputMode::Filter(self.all_filter_query.clone());
        self.active_pane = ActivePane::All;
        self.message = None;
    }

    /// clear filter input and return to normal mode.
    pub fn cancel_filter(&mut self) {
        self.clear_filter();
        self.input_mode = InputMode::Normal;
    }

    /// confirm filter input and return to normal mode.
    pub fn confirm_filter(&mut self) {
        if let InputMode::Filter(query) = &self.input_mode {
            self.all_filter_query = query.clone();
            self.apply_filter();
        }
        self.input_mode = InputMode::Normal;
        self.message = None;
    }

    /// get the currently selected issue id.
    pub fn selected_issue_id(&self) -> Option<&str> {
        if self.view_mode == ViewMode::Live {
            return self
                .in_progress_issues
                .get(self.in_progress_selected)
                .map(|s| s.as_str());
        }
        match self.active_pane {
            ActivePane::Ready => self
                .ready_issues
                .get(self.ready_selected)
                .map(|s| s.as_str()),
            ActivePane::All => self
                .visible_all_issues()
                .get(self.all_selected)
                .map(|s| s.as_str()),
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
        if self.view_mode == ViewMode::Live {
            if self.in_progress_selected > 0 {
                self.in_progress_selected -= 1;
            }
            self.reset_dep_selection();
            self.message = None;
            return;
        }
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
        self.reset_dep_selection();
        self.message = None;
    }

    /// move selection down.
    pub fn move_down(&mut self) {
        if self.view_mode == ViewMode::Live {
            if self.in_progress_selected + 1 < self.in_progress_issues.len() {
                self.in_progress_selected += 1;
            }
            self.reset_dep_selection();
            self.message = None;
            return;
        }
        match self.active_pane {
            ActivePane::Ready => {
                if self.ready_selected + 1 < self.ready_issues.len() {
                    self.ready_selected += 1;
                }
            }
            ActivePane::All => {
                if self.all_selected + 1 < self.visible_all_issues().len() {
                    self.all_selected += 1;
                }
            }
        }
        self.reset_dep_selection();
        self.message = None;
    }

    /// switch active pane.
    pub fn switch_pane(&mut self) {
        if self.view_mode == ViewMode::Live {
            return;
        }
        self.active_pane = match self.active_pane {
            ActivePane::Ready => ActivePane::All,
            ActivePane::All => ActivePane::Ready,
        };
        self.reset_dep_selection();
        self.message = None;
    }

    /// toggle live view mode.
    pub fn toggle_live_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Normal => ViewMode::Live,
            ViewMode::Live => ViewMode::Normal,
        };
        self.reset_dep_selection();
        self.message = None;
    }

    /// toggle help display.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// move selection to previous dependency.
    pub fn move_dep_prev(&mut self) {
        let Some(issue) = self.selected_issue() else {
            self.message = Some("no issue selected".to_string());
            return;
        };
        if issue.deps().is_empty() {
            self.message = Some("no dependencies".to_string());
            return;
        }
        let current = self.detail_dep_selected.unwrap_or(0);
        let next = current.saturating_sub(1);
        self.detail_dep_selected = Some(next);
        self.message = None;
    }

    /// move selection to next dependency.
    pub fn move_dep_next(&mut self) {
        let Some(issue) = self.selected_issue() else {
            self.message = Some("no issue selected".to_string());
            return;
        };
        if issue.deps().is_empty() {
            self.message = Some("no dependencies".to_string());
            return;
        }
        let current = self.detail_dep_selected.unwrap_or(0);
        let next = if current + 1 < issue.deps().len() {
            current + 1
        } else {
            current
        };
        self.detail_dep_selected = Some(next);
        self.message = None;
    }

    /// open the selected dependency in the list view.
    pub fn open_selected_dependency(&mut self) {
        let dep_id = {
            let Some(issue) = self.selected_issue() else {
                self.message = Some("no issue selected".to_string());
                return;
            };
            if issue.deps().is_empty() {
                self.message = Some("no dependencies".to_string());
                return;
            }
            let Some(dep_idx) = self.detail_dep_selected else {
                self.message = Some("no dependency selected".to_string());
                return;
            };
            let Some(dep_id) = issue.deps().get(dep_idx) else {
                self.message = Some("dependency not found".to_string());
                return;
            };
            dep_id.to_string()
        };

        if self.select_issue_by_id(&dep_id) {
            if self.view_mode == ViewMode::Live {
                self.view_mode = ViewMode::Normal;
            }
            self.message = None;
        } else {
            self.message = Some("dependency issue missing".to_string());
        }
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

        // save issue
        let issue_path = paths
            .issues_dir(&self.config)
            .join(format!("{}.md", issue_id));
        issue.save(&issue_path)?;

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

        // save issue
        let issue_path = paths
            .issues_dir(&self.config)
            .join(format!("{}.md", issue_id));
        issue.save(&issue_path)?;

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

        let issues_dir = paths.issues_dir(&self.config);
        let id = generate_issue_id(&self.config, &issues_dir)?;
        let issue = Issue::new(id.clone(), title, priority, vec![]);

        let _lock = LockGuard::acquire(&paths.lock_path())?;

        // save issue
        let issue_path = issues_dir.join(format!("{}.md", id));
        issue.save(&issue_path)?;

        self.input_mode = InputMode::Normal;
        self.message = Some(format!("created {}", id));
        self.reload_issues(paths)?;
        Ok(())
    }

    /// start editing the selected issue.
    pub fn start_edit_issue(&mut self) {
        let Some(issue_id) = self.selected_issue_id().map(|s| s.to_string()) else {
            self.message = Some("no issue selected".to_string());
            return;
        };
        self.input_mode = InputMode::EditSelect {
            issue_id,
            selected: 0,
        };
        self.message = None;
    }

    /// cancel editing.
    pub fn cancel_edit(&mut self) {
        self.input_mode = InputMode::Normal;
        self.message = Some("cancelled".to_string());
    }

    /// confirm field selection and enter field edit mode.
    pub fn confirm_edit_field(&mut self) {
        let (issue_id, selected) = match &self.input_mode {
            InputMode::EditSelect { issue_id, selected } => (issue_id.clone(), *selected),
            _ => return,
        };

        let Some(issue) = self.issues.get(&issue_id) else {
            self.message = Some("issue not found".to_string());
            self.input_mode = InputMode::Normal;
            return;
        };

        match selected {
            0 => {
                // edit title
                self.input_mode = InputMode::EditTitle {
                    issue_id,
                    current: issue.title().to_string(),
                };
            }
            1 => {
                // edit priority
                let priority_idx = match issue.priority() {
                    Priority::P0 => 0,
                    Priority::P1 => 1,
                    Priority::P2 => 2,
                    Priority::P3 => 3,
                };
                self.input_mode = InputMode::EditPriority {
                    issue_id,
                    selected: priority_idx,
                };
            }
            2 => {
                // edit status
                let status_idx = match issue.status() {
                    Status::Todo => 0,
                    Status::Doing => 1,
                    Status::Done => 2,
                    Status::Skip => 3,
                };
                self.input_mode = InputMode::EditStatus {
                    issue_id,
                    selected: status_idx,
                };
            }
            _ => {}
        }
    }

    /// save the edited issue.
    pub fn save_edit(&mut self, paths: &RepoPaths) -> Result<()> {
        let (issue_id, new_title, new_priority, new_status) = match &self.input_mode {
            InputMode::EditTitle { issue_id, current } => {
                if current.trim().is_empty() {
                    self.message = Some("title cannot be empty".to_string());
                    return Ok(());
                }
                (issue_id.clone(), Some(current.clone()), None, None)
            }
            InputMode::EditPriority { issue_id, selected } => {
                let priority = match selected {
                    0 => Priority::P0,
                    1 => Priority::P1,
                    2 => Priority::P2,
                    3 => Priority::P3,
                    _ => Priority::P2,
                };
                (issue_id.clone(), None, Some(priority), None)
            }
            InputMode::EditStatus { issue_id, selected } => {
                let status = match selected {
                    0 => Status::Todo,
                    1 => Status::Doing,
                    2 => Status::Done,
                    3 => Status::Skip,
                    _ => Status::Todo,
                };
                (issue_id.clone(), None, None, Some(status))
            }
            _ => return Ok(()),
        };

        let _lock = LockGuard::acquire(&paths.lock_path())?;

        let issue = self
            .issues
            .get_mut(&issue_id)
            .ok_or_else(|| BrdError::IssueNotFound(issue_id.clone()))?;

        // apply changes
        if let Some(title) = new_title {
            issue.frontmatter.title = title;
        }
        if let Some(priority) = new_priority {
            issue.frontmatter.priority = priority;
        }
        if let Some(status) = new_status {
            issue.frontmatter.status = status;
            // clear owner if marking as done or todo
            if status != Status::Doing {
                issue.frontmatter.owner = None;
            }
        }
        issue.touch();

        // save issue
        let issue_path = paths
            .issues_dir(&self.config)
            .join(format!("{}.md", issue_id));
        issue.save(&issue_path)?;

        self.input_mode = InputMode::Normal;
        self.message = Some(format!("saved {}", issue_id));
        self.reload_issues(paths)?;
        Ok(())
    }

    fn reset_dep_selection(&mut self) {
        let deps_len = self
            .selected_issue()
            .map(|issue| issue.deps().len())
            .unwrap_or(0);
        if deps_len == 0 {
            self.detail_dep_selected = None;
        } else {
            self.detail_dep_selected = Some(0);
        }
    }

    fn select_issue_by_id(&mut self, issue_id: &str) -> bool {
        if let Some(index) = self.all_issues.iter().position(|id| id == issue_id) {
            self.all_selected = index;
            self.active_pane = ActivePane::All;
            self.reset_dep_selection();
            return true;
        }
        false
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
fn load_all_issues(paths: &RepoPaths, config: &Config) -> Result<HashMap<String, Issue>> {
    let mut issues = HashMap::new();
    let issues_dir = paths.issues_dir(config);

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
