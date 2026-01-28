//! TUI application state and logic.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use ratatui::text::Text;

use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::graph::{DerivedState, compute_derived};
use crate::issue::{Issue, IssueType, Priority, Status};
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::diff_panel::DiffPanelState;
use super::diff_render::DiffRendererType;

/// Information about an agent worktree.
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// worktree name (directory name)
    pub name: String,
    /// full path to worktree
    pub path: PathBuf,
    /// current branch name
    pub branch: Option<String>,
    /// whether the worktree has uncommitted changes
    pub is_dirty: bool,
}

/// Info about a branch for the git graph.
#[derive(Debug, Clone)]
pub struct BranchGraphInfo {
    /// branch name
    pub name: String,
    /// recent commits on this branch
    pub commits: Vec<crate::git::CommitInfo>,
    /// commits ahead of main
    pub ahead_of_main: usize,
    /// is this main/master
    pub is_main: bool,
}

/// Git graph data for visualization.
#[derive(Debug, Clone, Default)]
pub struct GitGraph {
    /// branches to display
    pub branches: Vec<BranchGraphInfo>,
    /// total commits on main branch
    pub main_total_commits: usize,
}

/// Diff information for a worktree.
#[derive(Debug, Clone, Default)]
pub struct WorktreeDiff {
    /// overall stats
    pub stat: crate::git::DiffStat,
    /// per-file changes
    pub files: Vec<crate::git::FileDiff>,
    /// what the diff is against (e.g., "uncommitted" or "main..HEAD")
    pub diff_base: String,
}

/// which view is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// dashboard with stats overview
    Dashboard,
    /// issue list (default)
    #[default]
    Issues,
    /// agent worktrees view
    Agents,
}

/// which panel has focus in agents view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentsFocus {
    /// worktree list (left panel)
    #[default]
    Worktrees,
    /// file changes (right panel)
    Files,
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
    /// selecting type (for new issue)
    Type {
        title: String,
        priority: usize,
        selected: usize,
    },
    /// selecting dependencies (for new issue)
    Deps {
        title: String,
        priority: usize,
        type_idx: usize,
        selected_deps: Vec<String>,
        cursor: usize,
    },
    /// filtering issues
    Filter(String),
}

/// TUI application state.
pub struct App {
    /// current view
    pub view: View,
    /// all issues loaded from disk
    pub issues: HashMap<String, Issue>,
    /// sorted issue ids (priority order)
    pub sorted_issues: Vec<String>,
    /// filtered issues (when filter is active)
    pub filtered_issues: Vec<String>,
    /// currently selected index
    pub selected: usize,
    /// scroll offset
    pub offset: usize,
    /// current agent id
    pub agent_id: String,
    /// current repo name (for filtering worktrees)
    pub repo_name: String,
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
    /// filter query
    pub filter_query: String,
    /// status filter (empty means show all)
    pub status_filter: HashSet<Status>,
    /// file to open in external editor (set by 'e' key, handled by main loop)
    pub editor_file: Option<std::path::PathBuf>,
    /// filter to show only ready issues
    pub ready_filter: bool,
    /// whether to show the details pane
    pub show_details: bool,
    /// whether to show the detail overlay (full-screen view)
    pub show_detail_overlay: bool,
    /// diff panel state (when showing diff overlay)
    pub diff_panel_state: Option<DiffPanelState>,
    /// diff content currently being displayed
    pub diff_content: Option<Text<'static>>,
    /// file path for the diff being displayed
    pub diff_file_path: Option<String>,
    /// raw diff content (for re-rendering with different renderer)
    pub diff_raw_content: Option<String>,
    /// current diff renderer type
    pub diff_renderer: DiffRendererType,
    /// list of agent worktrees
    pub worktrees: Vec<WorktreeInfo>,
    /// selected worktree index in agents view
    pub worktree_selected: usize,
    /// diff info for the selected worktree
    pub worktree_diff: Option<WorktreeDiff>,
    /// selected file index in the worktree diff
    pub worktree_file_selected: usize,
    /// which panel has focus in agents view
    pub agents_focus: AgentsFocus,
    /// git graph data for dashboard
    pub git_graph: Option<GitGraph>,
}

impl App {
    /// create a new app by loading issues from disk.
    pub fn new(paths: &RepoPaths) -> Result<Self> {
        let agent_id = get_agent_id(&paths.worktree_root);
        let repo_name = get_repo_name(&paths.worktree_root);
        let config = Config::load(&paths.config_path())?;

        // extract diff renderer preference before moving config
        let diff_renderer = config
            .diff_renderer
            .as_deref()
            .and_then(DiffRendererType::parse)
            .map(|r| {
                if r.is_available() {
                    r
                } else {
                    DiffRendererType::Native
                }
            })
            .unwrap_or_default();

        let mut app = Self {
            view: View::Issues,
            issues: HashMap::new(),
            sorted_issues: Vec::new(),
            filtered_issues: Vec::new(),
            selected: 0,
            offset: 0,
            agent_id,
            repo_name,
            message: None,
            show_help: false,
            input_mode: InputMode::Normal,
            config,
            detail_dep_selected: None,
            filter_query: String::new(),
            status_filter: HashSet::new(),
            editor_file: None,
            ready_filter: false,
            show_details: true,
            show_detail_overlay: false,
            diff_panel_state: None,
            diff_content: None,
            diff_file_path: None,
            diff_raw_content: None,
            diff_renderer,
            worktrees: Vec::new(),
            worktree_selected: 0,
            worktree_diff: None,
            worktree_file_selected: 0,
            agents_focus: AgentsFocus::default(),
            git_graph: None,
        };
        app.reload_issues(paths)?;
        app.reload_worktrees();
        app.load_git_graph();
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

        // build sorted list: done/skip last, then by priority
        let mut all: Vec<&Issue> = self.issues.values().collect();
        all.sort_by(|a, b| {
            let a_resolved = matches!(a.status(), Status::Done | Status::Skip);
            let b_resolved = matches!(b.status(), Status::Done | Status::Skip);
            a_resolved
                .cmp(&b_resolved)
                .then_with(|| a.cmp_by_priority(b))
        });
        self.sorted_issues = all.iter().map(|i| i.id().to_string()).collect();

        // clamp selection
        let len = self.visible_issues().len();
        if self.selected >= len && len > 0 {
            self.selected = len - 1;
        }
        if self.offset >= len {
            self.offset = 0;
        }

        self.reset_dep_selection();
        self.apply_filter();
        if show_message {
            self.message = Some("refreshed".to_string());
        }
        Ok(())
    }

    /// reload worktrees from ~/.braid/worktrees/<repo>/*/.
    pub fn reload_worktrees(&mut self) {
        self.worktrees = discover_worktrees(&self.repo_name);
        // clamp selection
        if self.worktree_selected >= self.worktrees.len() && !self.worktrees.is_empty() {
            self.worktree_selected = self.worktrees.len() - 1;
        }
        // load diff for selected worktree
        self.load_worktree_diff();
        // reload git graph
        self.load_git_graph();
    }

    /// load git graph data from worktrees.
    pub fn load_git_graph(&mut self) {
        self.git_graph = None;

        if self.worktrees.is_empty() {
            return;
        }

        // find main branch worktree
        let main_wt = self
            .worktrees
            .iter()
            .find(|wt| matches!(wt.branch.as_deref(), Some("main" | "master")));

        let Some(main_wt) = main_wt else {
            return;
        };

        let main_branch = main_wt.branch.clone().unwrap_or_else(|| "main".to_string());

        // get total commits on main
        let main_total = crate::git::total_commit_count(&main_wt.path, &main_branch).unwrap_or(0);

        // get recent commits on main (last 8)
        let main_commits =
            crate::git::log_commits(&main_wt.path, &main_branch, 8).unwrap_or_default();

        let mut branches = vec![BranchGraphInfo {
            name: main_branch.clone(),
            commits: main_commits,
            ahead_of_main: 0,
            is_main: true,
        }];

        // get info for each non-main worktree
        for wt in &self.worktrees {
            let Some(ref branch) = wt.branch else {
                continue;
            };
            if branch == "main" || branch == "master" {
                continue;
            }

            // count commits ahead of main
            let ahead = crate::git::commit_count(&wt.path, &main_branch, "HEAD").unwrap_or(0);

            // get last 3 commits on this branch
            let commits = crate::git::log_commits(&wt.path, "HEAD", 3).unwrap_or_default();

            branches.push(BranchGraphInfo {
                name: branch.clone(),
                commits,
                ahead_of_main: ahead,
                is_main: false,
            });
        }

        self.git_graph = Some(GitGraph {
            branches,
            main_total_commits: main_total,
        });
    }

    /// load diff info for the currently selected worktree.
    pub fn load_worktree_diff(&mut self) {
        self.worktree_diff = None;
        self.worktree_file_selected = 0;

        let Some(wt) = self.worktrees.get(self.worktree_selected) else {
            return;
        };

        // determine diff strategy based on worktree state
        let (files, stat, diff_base) = if wt.is_dirty {
            // uncommitted changes: diff against HEAD
            let Ok(files) = crate::git::diff_files(&wt.path, Some("HEAD"), None) else {
                return;
            };
            let Ok(stat) = crate::git::diff_stat(&wt.path, Some("HEAD"), None) else {
                return;
            };
            (files, stat, "uncommitted".to_string())
        } else {
            // clean tree: diff against main branch
            let Some(main_branch) = find_main_branch(&wt.path) else {
                return; // can't determine base
            };
            let Ok(files) = crate::git::diff_files(&wt.path, Some(&main_branch), Some("HEAD"))
            else {
                return;
            };
            let Ok(stat) = crate::git::diff_stat(&wt.path, Some(&main_branch), Some("HEAD")) else {
                return;
            };
            (files, stat, format!("{}..HEAD", main_branch))
        };

        self.worktree_diff = Some(WorktreeDiff {
            stat,
            files,
            diff_base,
        });
    }

    /// select next worktree and load its diff.
    pub fn worktree_next(&mut self) {
        if self.worktrees.is_empty() {
            return;
        }
        if self.worktree_selected < self.worktrees.len() - 1 {
            self.worktree_selected += 1;
            self.load_worktree_diff();
        }
    }

    /// select previous worktree and load its diff.
    pub fn worktree_prev(&mut self) {
        if self.worktree_selected > 0 {
            self.worktree_selected -= 1;
            self.load_worktree_diff();
        }
    }

    /// select next file in worktree diff.
    pub fn worktree_file_next(&mut self) {
        if let Some(ref diff) = self.worktree_diff
            && self.worktree_file_selected < diff.files.len().saturating_sub(1)
        {
            self.worktree_file_selected += 1;
        }
    }

    /// select previous file in worktree diff.
    pub fn worktree_file_prev(&mut self) {
        if self.worktree_file_selected > 0 {
            self.worktree_file_selected -= 1;
        }
    }

    /// half-page up in agents view (based on focused panel).
    pub fn agents_half_page_up(&mut self) {
        const HALF_PAGE: usize = 10;
        match self.agents_focus {
            AgentsFocus::Worktrees => {
                self.worktree_selected = self.worktree_selected.saturating_sub(HALF_PAGE);
                self.load_worktree_diff();
            }
            AgentsFocus::Files => {
                self.worktree_file_selected = self.worktree_file_selected.saturating_sub(HALF_PAGE);
            }
        }
    }

    /// half-page down in agents view (based on focused panel).
    pub fn agents_half_page_down(&mut self) {
        const HALF_PAGE: usize = 10;
        match self.agents_focus {
            AgentsFocus::Worktrees => {
                let max = self.worktrees.len().saturating_sub(1);
                self.worktree_selected = (self.worktree_selected + HALF_PAGE).min(max);
                self.load_worktree_diff();
            }
            AgentsFocus::Files => {
                if let Some(ref diff) = self.worktree_diff {
                    let max = diff.files.len().saturating_sub(1);
                    self.worktree_file_selected =
                        (self.worktree_file_selected + HALF_PAGE).min(max);
                }
            }
        }
    }

    /// open diff view for the currently selected file in agents view.
    pub fn open_selected_file_diff(&mut self) {
        // extract values we need before mutating self
        let (wt_path, is_dirty, file_path) = {
            let Some(wt) = self.worktrees.get(self.worktree_selected) else {
                return;
            };
            let Some(ref diff) = self.worktree_diff else {
                return;
            };
            let Some(file) = diff.files.get(self.worktree_file_selected) else {
                return;
            };
            (wt.path.clone(), wt.is_dirty, file.path.clone())
        };

        // determine base/head for diff and get content
        let raw_diff = if is_dirty {
            let Ok(diff) = crate::git::diff_content(&wt_path, Some("HEAD"), None, Some(&file_path))
            else {
                return;
            };
            diff
        } else {
            let Some(main) = find_main_branch(&wt_path) else {
                return;
            };
            let Ok(diff) =
                crate::git::diff_content(&wt_path, Some(&main), Some("HEAD"), Some(&file_path))
            else {
                return;
            };
            diff
        };

        self.show_diff_from_raw(&raw_diff, &file_path);
    }

    /// render raw diff content and show in diff panel.
    fn show_diff_from_raw(&mut self, raw_diff: &str, file_path: &str) {
        // store raw diff for re-rendering
        self.diff_raw_content = Some(raw_diff.to_string());
        self.diff_file_path = Some(file_path.to_string());

        // render with current renderer
        let Ok(styled_text) = self.diff_renderer.render(raw_diff, 80) else {
            return;
        };

        self.show_diff(styled_text, file_path.to_string());
    }

    /// re-render current diff with the active renderer.
    pub fn re_render_diff(&mut self) {
        let Some(raw) = self.diff_raw_content.clone() else {
            return;
        };
        // check that we have a file path set (diff is open)
        if self.diff_file_path.is_none() {
            return;
        }

        let Ok(styled_text) = self.diff_renderer.render(&raw, 80) else {
            return;
        };

        // update content but preserve scroll position
        self.diff_content = Some(styled_text);
    }

    /// cycle to next available renderer and re-render.
    pub fn cycle_diff_renderer(&mut self) {
        let start = self.diff_renderer;
        self.diff_renderer = self.diff_renderer.next();

        // skip unavailable renderers, but don't loop forever
        while !self.diff_renderer.is_available() && self.diff_renderer != start {
            self.diff_renderer = self.diff_renderer.next();
        }

        self.re_render_diff();
    }

    /// apply the current filter to the issues list.
    pub fn apply_filter(&mut self) {
        let query = self.filter_query.to_lowercase();
        self.filtered_issues = self
            .sorted_issues
            .iter()
            .filter(|id| {
                let Some(issue) = self.issues.get(*id) else {
                    return false;
                };
                // check ready filter
                if self.ready_filter {
                    let derived = compute_derived(issue, &self.issues);
                    if !derived.is_ready {
                        return false;
                    }
                }
                // check status filter (empty means show all)
                if !self.status_filter.is_empty() && !self.status_filter.contains(&issue.status()) {
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
        let visible_len = self.visible_issues().len();
        if self.selected >= visible_len && visible_len > 0 {
            self.selected = visible_len - 1;
        }
        if self.offset >= visible_len {
            self.offset = 0;
        }
    }

    /// returns true if a filter is currently active.
    pub fn has_filter(&self) -> bool {
        !self.filter_query.is_empty() || !self.status_filter.is_empty() || self.ready_filter
    }

    /// get the visible issues list (filtered or unfiltered).
    pub fn visible_issues(&self) -> &Vec<String> {
        if self.has_filter() {
            &self.filtered_issues
        } else {
            &self.sorted_issues
        }
    }

    /// clear all filters.
    pub fn clear_filter(&mut self) {
        self.filter_query.clear();
        self.status_filter.clear();
        self.ready_filter = false;
        self.apply_filter();
        self.message = Some("filter cleared".to_string());
    }

    /// toggle the ready filter.
    pub fn toggle_ready_filter(&mut self) {
        self.ready_filter = !self.ready_filter;
        self.apply_filter();
        if self.ready_filter {
            self.message = Some("showing ready issues".to_string());
        } else {
            self.message = Some("showing all issues".to_string());
        }
    }

    /// show diff panel with the given content and file path.
    pub fn show_diff(&mut self, content: Text<'static>, file_path: String) {
        self.diff_panel_state = Some(DiffPanelState::new());
        self.diff_content = Some(content);
        self.diff_file_path = Some(file_path);
    }

    /// close the diff panel.
    pub fn close_diff(&mut self) {
        self.diff_panel_state = None;
        self.diff_content = None;
        self.diff_file_path = None;
        self.diff_raw_content = None;
    }

    /// check if diff panel is currently visible.
    pub fn is_diff_visible(&self) -> bool {
        self.diff_panel_state.is_some()
    }

    /// start filter input mode.
    pub fn start_filter(&mut self) {
        self.input_mode = InputMode::Filter(self.filter_query.clone());
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
            self.filter_query = query.clone();
            self.apply_filter();
        }
        self.input_mode = InputMode::Normal;
        self.message = None;
    }

    /// get the currently selected issue id.
    pub fn selected_issue_id(&self) -> Option<&str> {
        self.visible_issues().get(self.selected).map(|s| s.as_str())
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
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.reset_dep_selection();
        self.message = None;
    }

    /// move selection down.
    pub fn move_down(&mut self) {
        if self.selected + 1 < self.visible_issues().len() {
            self.selected += 1;
        }
        self.reset_dep_selection();
        self.message = None;
    }

    /// move to top of list.
    pub fn move_to_top(&mut self) {
        self.selected = 0;
        self.reset_dep_selection();
        self.message = None;
    }

    /// move to bottom of list.
    pub fn move_to_bottom(&mut self) {
        let len = self.visible_issues().len();
        if len > 0 {
            self.selected = len - 1;
        }
        self.reset_dep_selection();
        self.message = None;
    }

    /// toggle help display.
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// toggle details pane visibility.
    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
        self.message = Some(if self.show_details {
            "details pane shown".to_string()
        } else {
            "details pane hidden".to_string()
        });
    }

    /// show the detail overlay (full-screen detail view).
    pub fn show_detail_overlay(&mut self) {
        self.show_detail_overlay = true;
    }

    /// hide the detail overlay.
    pub fn hide_detail_overlay(&mut self) {
        self.show_detail_overlay = false;
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
        issue.mark_started();

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
        issue.mark_completed();

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

    /// confirm priority and move to type selection.
    pub fn confirm_priority(&mut self) {
        if let InputMode::Priority { title, selected } = &self.input_mode {
            self.input_mode = InputMode::Type {
                title: title.clone(),
                priority: *selected,
                selected: 0, // default to (none)
            };
        }
    }

    /// confirm type and move to deps selection.
    pub fn confirm_type(&mut self) {
        if let InputMode::Type {
            title,
            priority,
            selected,
        } = &self.input_mode
        {
            self.input_mode = InputMode::Deps {
                title: title.clone(),
                priority: *priority,
                type_idx: *selected,
                selected_deps: Vec::new(),
                cursor: 0,
            };
        }
    }

    /// toggle dependency selection at cursor.
    pub fn toggle_dep(&mut self) {
        if let InputMode::Deps {
            selected_deps,
            cursor,
            ..
        } = &mut self.input_mode
            && *cursor < self.sorted_issues.len()
        {
            let issue_id = self.sorted_issues[*cursor].clone();
            if let Some(pos) = selected_deps.iter().position(|d| d == &issue_id) {
                selected_deps.remove(pos);
            } else {
                selected_deps.push(issue_id);
            }
        }
    }

    /// create the issue with all collected data.
    pub fn create_issue(&mut self, paths: &RepoPaths) -> Result<()> {
        let (title, priority_idx, type_idx, deps) = match &self.input_mode {
            InputMode::Deps {
                title,
                priority,
                type_idx,
                selected_deps,
                ..
            } => (title.clone(), *priority, *type_idx, selected_deps.clone()),
            _ => return Ok(()),
        };

        let priority = match priority_idx {
            0 => Priority::P0,
            1 => Priority::P1,
            2 => Priority::P2,
            3 => Priority::P3,
            _ => Priority::P2,
        };

        let issue_type = match type_idx {
            1 => Some(IssueType::Design),
            2 => Some(IssueType::Meta),
            _ => None, // 0 = (none)
        };

        let issues_dir = paths.issues_dir(&self.config);
        let id = generate_issue_id(&self.config, &issues_dir)?;
        let mut issue = Issue::new(id.clone(), title, priority, deps);
        issue.frontmatter.issue_type = issue_type;

        let _lock = LockGuard::acquire(&paths.lock_path())?;

        // save issue
        let issue_path = issues_dir.join(format!("{}.md", id));
        issue.save(&issue_path)?;

        self.input_mode = InputMode::Normal;
        self.message = Some(format!("created {}", id));
        self.reload_issues(paths)?;
        Ok(())
    }

    /// request opening the selected issue in $EDITOR.
    pub fn open_in_editor(&mut self, paths: &RepoPaths) {
        let Some(issue_id) = self.selected_issue_id() else {
            self.message = Some("no issue selected".to_string());
            return;
        };
        let issue_path = paths
            .issues_dir(&self.config)
            .join(format!("{}.md", issue_id));
        if issue_path.exists() {
            self.editor_file = Some(issue_path);
        } else {
            self.message = Some("issue file not found".to_string());
        }
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
        if let Some(index) = self.visible_issues().iter().position(|id| id == issue_id) {
            self.selected = index;
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

/// get repo name from git remote origin URL.
/// falls back to the directory name if no remote is configured.
fn get_repo_name(worktree_root: &std::path::Path) -> String {
    // try to get from git remote origin URL
    if let Ok(output) = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(worktree_root)
        .output()
        && output.status.success()
    {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // extract repo name from URL (handles both HTTPS and SSH formats)
        // e.g., "https://github.com/user/repo.git" -> "repo"
        // e.g., "git@github.com:user/repo.git" -> "repo"
        if let Some(name) = url
            .rsplit('/')
            .next()
            .or_else(|| url.rsplit(':').next())
            .map(|s| s.trim_end_matches(".git"))
            && !name.is_empty()
        {
            return name.to_string();
        }
    }

    // fall back to directory name
    worktree_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// discover all worktrees for the current repo using `git worktree list`.
/// includes both the main worktree and agent worktrees.
fn discover_worktrees(_repo_name: &str) -> Vec<WorktreeInfo> {
    let mut worktrees = Vec::new();

    // run git worktree list --porcelain from current directory
    let Ok(output) = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()
    else {
        return worktrees;
    };

    if !output.status.success() {
        return worktrees;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // parse porcelain format: blocks separated by blank lines
    // each block has: worktree <path>, HEAD <sha>, branch refs/heads/<name>
    let mut current_path: Option<PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in stdout.lines() {
        if line.starts_with("worktree ") {
            current_path = Some(PathBuf::from(line.strip_prefix("worktree ").unwrap()));
        } else if line.starts_with("branch refs/heads/") {
            current_branch = line.strip_prefix("branch refs/heads/").map(String::from);
        } else if line.is_empty() {
            // end of block - process this worktree
            if let Some(path) = current_path.take() {
                let branch = current_branch.take();

                // skip issues worktree (ends with /brd/issues or has braid-issues branch)
                let path_str = path.to_string_lossy();
                if path_str.ends_with("/brd/issues")
                    || path_str.ends_with("/.git/brd/issues")
                    || branch.as_deref() == Some("braid-issues")
                {
                    continue;
                }

                // determine display name
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                // check if dirty
                let is_dirty = crate::git::is_clean(&path).map(|c| !c).unwrap_or(false);

                worktrees.push(WorktreeInfo {
                    name,
                    path,
                    branch,
                    is_dirty,
                });
            }
        }
    }

    // handle last block if file doesn't end with blank line
    if let Some(path) = current_path {
        let branch = current_branch;
        let path_str = path.to_string_lossy();
        if !path_str.ends_with("/brd/issues")
            && !path_str.ends_with("/.git/brd/issues")
            && branch.as_deref() != Some("braid-issues")
        {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let is_dirty = crate::git::is_clean(&path).map(|c| !c).unwrap_or(false);

            worktrees.push(WorktreeInfo {
                name,
                path,
                branch,
                is_dirty,
            });
        }
    }

    // sort: main/master branch first, then alphabetically by name
    worktrees.sort_by(|a, b| {
        let a_is_main = matches!(a.branch.as_deref(), Some("main" | "master"));
        let b_is_main = matches!(b.branch.as_deref(), Some("main" | "master"));
        b_is_main.cmp(&a_is_main).then_with(|| a.name.cmp(&b.name))
    });

    worktrees
}

/// find the main branch name for a repo (main or master).
fn find_main_branch(path: &std::path::Path) -> Option<String> {
    // check for common main branch names
    for name in ["main", "master"] {
        if crate::git::branch_exists(path, name) {
            return Some(name.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
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

        fn app(&self) -> App {
            App::new(&self.paths).expect("failed to create app")
        }
    }

    #[test]
    fn test_apply_filter_matches_title() {
        let env = TestEnv::new();
        env.add_issue(
            "brd-aaaa",
            "fix authentication bug",
            Priority::P1,
            Status::Open,
        );
        env.add_issue(
            "brd-bbbb",
            "add logging feature",
            Priority::P2,
            Status::Open,
        );
        env.add_issue(
            "brd-cccc",
            "authentication refactor",
            Priority::P3,
            Status::Open,
        );

        let mut app = env.app();
        app.filter_query = "auth".to_string();
        app.apply_filter();

        assert_eq!(app.filtered_issues.len(), 2);
        assert!(app.filtered_issues.contains(&"brd-aaaa".to_string()));
        assert!(app.filtered_issues.contains(&"brd-cccc".to_string()));
    }

    #[test]
    fn test_apply_filter_case_insensitive() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "Fix BUG in Parser", Priority::P1, Status::Open);

        let mut app = env.app();
        app.filter_query = "bug".to_string();
        app.apply_filter();

        assert_eq!(app.filtered_issues.len(), 1);
    }

    #[test]
    fn test_move_up_at_top_stays_at_top() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "first", Priority::P1, Status::Open);
        env.add_issue("brd-bbbb", "second", Priority::P2, Status::Open);

        let mut app = env.app();
        app.selected = 0;

        app.move_up();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_move_down_at_bottom_stays_at_bottom() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "first", Priority::P1, Status::Open);
        env.add_issue("brd-bbbb", "second", Priority::P2, Status::Open);

        let mut app = env.app();
        app.selected = 1; // at bottom

        app.move_down();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn test_confirm_title_empty_rejected() {
        let env = TestEnv::new();
        let mut app = env.app();

        app.input_mode = InputMode::Title("".to_string());
        app.confirm_title();

        // should still be in Title mode
        assert!(matches!(app.input_mode, InputMode::Title(_)));
        assert_eq!(app.message.as_deref(), Some("title cannot be empty"));
    }

    #[test]
    fn test_confirm_title_whitespace_rejected() {
        let env = TestEnv::new();
        let mut app = env.app();

        app.input_mode = InputMode::Title("   ".to_string());
        app.confirm_title();

        assert!(matches!(app.input_mode, InputMode::Title(_)));
        assert_eq!(app.message.as_deref(), Some("title cannot be empty"));
    }

    #[test]
    fn test_confirm_title_moves_to_priority() {
        let env = TestEnv::new();
        let mut app = env.app();

        app.input_mode = InputMode::Title("my issue".to_string());
        app.confirm_title();

        assert!(matches!(
            app.input_mode,
            InputMode::Priority { ref title, selected } if title == "my issue" && selected == 2
        ));
    }

    #[test]
    fn test_confirm_priority_moves_to_type() {
        let env = TestEnv::new();
        let mut app = env.app();

        app.input_mode = InputMode::Priority {
            title: "my issue".to_string(),
            selected: 1,
        };
        app.confirm_priority();

        assert!(matches!(
            app.input_mode,
            InputMode::Type { ref title, priority, selected }
            if title == "my issue" && priority == 1 && selected == 0
        ));
    }

    #[test]
    fn test_confirm_type_moves_to_deps() {
        let env = TestEnv::new();
        let mut app = env.app();

        app.input_mode = InputMode::Type {
            title: "my issue".to_string(),
            priority: 2,
            selected: 1, // design
        };
        app.confirm_type();

        assert!(matches!(
            app.input_mode,
            InputMode::Deps { ref title, priority, type_idx, ref selected_deps, cursor }
            if title == "my issue" && priority == 2 && type_idx == 1 && selected_deps.is_empty() && cursor == 0
        ));
    }

    #[test]
    fn test_toggle_dep_adds_and_removes() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "dep issue", Priority::P1, Status::Open);

        let mut app = env.app();
        app.input_mode = InputMode::Deps {
            title: "new issue".to_string(),
            priority: 2,
            type_idx: 0,
            selected_deps: vec![],
            cursor: 0,
        };

        // toggle on
        app.toggle_dep();
        if let InputMode::Deps { selected_deps, .. } = &app.input_mode {
            assert_eq!(selected_deps.len(), 1);
            assert!(selected_deps.contains(&"brd-aaaa".to_string()));
        } else {
            panic!("expected Deps mode");
        }

        // toggle off
        app.toggle_dep();
        if let InputMode::Deps { selected_deps, .. } = &app.input_mode {
            assert!(selected_deps.is_empty());
        } else {
            panic!("expected Deps mode");
        }
    }

    #[test]
    fn test_clear_filter_resets_state() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "issue", Priority::P1, Status::Open);

        let mut app = env.app();
        app.filter_query = "test".to_string();
        app.status_filter.insert(Status::Done);
        app.apply_filter();

        // filtered list should be empty (no match)
        assert!(app.filtered_issues.is_empty());

        app.clear_filter();

        assert!(app.filter_query.is_empty());
        assert!(app.status_filter.is_empty());
        assert!(!app.has_filter());
    }

    #[test]
    fn test_visible_issues_uses_filter_when_active() {
        let env = TestEnv::new();
        env.add_issue("brd-aaaa", "alpha", Priority::P1, Status::Open);
        env.add_issue("brd-bbbb", "beta", Priority::P2, Status::Open);

        let mut app = env.app();

        // no filter - returns all issues
        assert_eq!(app.visible_issues().len(), 2);

        // with filter - returns filtered
        app.filter_query = "alpha".to_string();
        app.apply_filter();
        assert_eq!(app.visible_issues().len(), 1);
        assert_eq!(app.visible_issues()[0], "brd-aaaa");
    }

    #[test]
    fn test_toggle_details() {
        let env = TestEnv::new();
        let mut app = env.app();

        // default: details pane is shown
        assert!(app.show_details);

        // toggle off
        app.toggle_details();
        assert!(!app.show_details);
        assert_eq!(app.message.as_deref(), Some("details pane hidden"));

        // toggle on
        app.toggle_details();
        assert!(app.show_details);
        assert_eq!(app.message.as_deref(), Some("details pane shown"));
    }

    #[test]
    fn test_detail_overlay() {
        let env = TestEnv::new();
        let mut app = env.app();

        // default: overlay is hidden
        assert!(!app.show_detail_overlay);

        // show overlay
        app.show_detail_overlay();
        assert!(app.show_detail_overlay);

        // hide overlay
        app.hide_detail_overlay();
        assert!(!app.show_detail_overlay);
    }

    #[test]
    fn test_has_filter() {
        let env = TestEnv::new();
        let mut app = env.app();

        assert!(!app.has_filter());

        app.filter_query = "test".to_string();
        assert!(app.has_filter());

        app.filter_query.clear();
        app.status_filter.insert(Status::Done);
        assert!(app.has_filter());
    }
}
