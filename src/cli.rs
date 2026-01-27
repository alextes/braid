//! CLI command definitions and wiring.

use clap::{Args, Parser, Subcommand};

/// Parse boolean from env var, accepting "1", "true", "yes" as truthy.
fn parse_bool_env(s: &str) -> Result<bool, String> {
    match s.to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" | "" => Ok(false),
        _ => Err(format!("invalid boolean value: {}", s)),
    }
}

#[derive(Parser)]
#[command(name = "brd")]
#[command(about = "a lightweight, repo-local issue tracker")]
#[command(version)]
pub struct Cli {
    /// output machine-readable JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// run as if invoked from this directory
    #[arg(long, global = true)]
    pub repo: Option<std::path::PathBuf>,

    /// disable ANSI colors
    #[arg(long, global = true)]
    pub no_color: bool,

    /// enable verbose output
    #[arg(short, long, global = true, env = "BRD_VERBOSE", value_parser = parse_bool_env)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// initialize braid in the current repository
    Init(InitArgs),

    /// add a new issue
    Add(AddArgs),

    /// list issues
    Ls {
        /// filter by status
        #[arg(long)]
        status: Option<String>,

        /// filter by priority
        #[arg(long, short)]
        priority: Option<String>,

        /// show only ready issues
        #[arg(long)]
        ready: bool,

        /// show only blocked issues
        #[arg(long)]
        blocked: bool,

        /// filter by tag (can be repeated)
        #[arg(long)]
        tag: Vec<String>,

        /// show all issues (no limit on done issues)
        #[arg(long)]
        all: bool,
    },

    /// show details of an issue
    Show {
        /// issue ID (full or partial)
        id: String,
        /// include full content of dependencies and dependents
        #[arg(long)]
        context: bool,
    },

    /// open an issue in $EDITOR
    Edit {
        /// issue ID (optional - opens current "doing" issue if omitted)
        id: Option<String>,

        /// force interactive mode even without a TTY (for AI agents)
        #[arg(long)]
        force: bool,
    },

    /// quickly update issue fields
    Set {
        /// issue ID (full or partial)
        id: String,

        /// field to update (priority, status, type, owner, title, tag)
        field: String,

        /// new value
        value: String,
    },

    /// list ready issues
    Ready,

    /// show repo status summary
    Status,

    /// add or remove dependencies
    Dep {
        #[command(subcommand)]
        action: DepAction,
    },

    /// start working on an issue (picks next ready issue if no id given)
    Start {
        /// issue ID (optional - picks next ready if omitted)
        id: Option<String>,

        /// force start even if already being worked on
        #[arg(long)]
        force: bool,

        /// skip fetch/rebase, trust local state
        #[arg(long)]
        no_sync: bool,

        /// claim locally but don't commit/push
        #[arg(long)]
        no_push: bool,

        /// stash uncommitted changes before sync, restore after
        #[arg(long)]
        stash: bool,
    },

    /// mark an issue as done
    Done {
        /// issue ID
        id: String,

        /// force completion even if not claimed by you, or close design issue without results
        #[arg(long)]
        force: bool,

        /// issue IDs created as a result of this design issue (required for design issues)
        #[arg(long, short)]
        result: Vec<String>,

        /// skip commit/push even when auto_push is enabled
        #[arg(long)]
        no_push: bool,
    },

    /// mark an issue as skipped (won't do)
    Skip {
        /// issue ID
        id: String,
    },

    /// delete an issue
    Rm {
        /// issue ID
        id: String,

        /// force deletion even if issue is in progress
        #[arg(long)]
        force: bool,
    },

    /// manage agent worktrees
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },

    /// interactive TUI for issue management
    Tui {
        /// force interactive mode even without a TTY
        #[arg(long)]
        force: bool,
    },

    /// migrate issues to current schema version
    Migrate {
        /// dry run - show what would be migrated without changing files
        #[arg(long)]
        dry_run: bool,
    },

    /// validate repository state
    Doctor,

    /// generate shell completions
    Completions {
        /// shell to generate completions for
        shell: clap_complete::Shell,
    },

    /// search issues (prints instructions for using grep/rg)
    Search,

    /// commit .braid changes
    Commit {
        /// custom commit message (auto-generated if omitted)
        #[arg(short, long)]
        message: Option<String>,
    },

    /// sync issues with the sync branch (sync branch mode only)
    Sync {
        /// push and set upstream if needed
        #[arg(long)]
        push: bool,
    },

    /// view or change braid configuration
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
}

/// Parse "on"/"off" to bool for auto-sync setting.
fn parse_on_off(s: &str) -> Result<bool, String> {
    match s.to_lowercase().as_str() {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        _ => Err(format!("expected 'on' or 'off', got '{}'", s)),
    }
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// set or clear the issues branch (for shared worktree storage)
    IssuesBranch {
        /// branch name to use (omit with --clear to disable)
        name: Option<String>,

        /// clear the issues branch setting
        #[arg(long)]
        clear: bool,

        /// skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// set or clear the external issues repository
    ExternalRepo {
        /// path to external issues repo (omit with --clear to disable)
        path: Option<String>,

        /// clear the external repo setting
        #[arg(long)]
        clear: bool,

        /// skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// enable or disable auto-sync (pull on start, push on done)
    AutoSync {
        /// "on" or "off"
        #[arg(value_parser = parse_on_off, action = clap::ArgAction::Set)]
        enabled: bool,
    },
}

#[derive(Subcommand)]
pub enum AgentAction {
    /// set up a new agent worktree
    Init {
        /// agent name (used for worktree directory and branch)
        name: String,

        /// base branch to create worktree from (default: current branch)
        #[arg(long)]
        base: Option<String>,
    },

    /// create feature branch for PR workflow
    Branch {
        /// issue ID to create branch for
        id: String,
    },

    /// create PR from current branch
    Pr,

    /// merge changes to main (rebase + fast-forward)
    Merge,

    /// inject/update the braid instructions block in AGENTS.md
    Inject {
        /// target file path (default: AGENTS.md)
        #[arg(long, short)]
        file: Option<String>,
    },

    /// print the AGENTS.md instructions block to stdout
    Instructions,

    /// spawn a claude agent to work on an issue
    Spawn {
        /// issue ID to work on
        id: String,

        /// max budget in USD
        #[arg(long, default_value = "1.0")]
        budget: f64,

        /// run in foreground with streaming output
        #[arg(long)]
        foreground: bool,

        /// create/use a worktree for this agent
        #[arg(long)]
        worktree: bool,

        /// claude model to use
        #[arg(long)]
        model: Option<String>,
    },

    /// list running agent sessions
    Ps {
        /// show all sessions including completed
        #[arg(long)]
        all: bool,
    },

    /// view agent session output
    Logs {
        /// session ID (e.g., agent-1)
        session: String,

        /// follow output in real-time
        #[arg(long, short)]
        follow: bool,

        /// show last N events
        #[arg(long)]
        tail: Option<usize>,

        /// show raw JSON events
        #[arg(long)]
        raw: bool,
    },

    /// send a message to a stopped agent session
    Send {
        /// session ID (e.g., agent-1)
        session: String,

        /// message to send
        message: String,
    },

    /// attach interactively to a stopped agent session
    Attach {
        /// session ID (e.g., agent-1)
        session: String,
    },

    /// terminate a running agent
    Kill {
        /// session ID (e.g., agent-1)
        session: String,

        /// use SIGKILL instead of SIGTERM
        #[arg(long)]
        force: bool,
    },

    /// remove stale agent session files
    Clean {
        /// remove all sessions (not just stale ones)
        #[arg(long)]
        all: bool,

        /// skip confirmation prompt
        #[arg(long, short)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum DepAction {
    /// add a dependency (blocked depends on blocker)
    Add {
        /// the issue that will be blocked
        blocked: String,
        /// the issue that blocks it
        blocker: String,
    },
    /// remove a dependency
    Rm {
        /// the issue to remove dependency from
        blocked: String,
        /// the blocker to remove
        blocker: String,
    },
}

/// arguments for the init command.
#[derive(Args)]
pub struct InitArgs {
    /// create a sync branch for issue tracking (issues live on this branch, not main)
    #[arg(long)]
    pub issues_branch: Option<String>,

    /// skip interactive prompts (use git-native mode by default)
    #[arg(short = 'y', long)]
    pub non_interactive: bool,
}

/// arguments for the add command.
#[derive(Args)]
pub struct AddArgs {
    /// issue title
    pub title: String,

    /// priority (P0-P3, default P2)
    #[arg(long, short, default_value = "P2")]
    pub priority: String,

    /// issue type (design, meta)
    #[arg(long, short = 't')]
    pub r#type: Option<String>,

    /// this issue is blocked by DEP (can be repeated)
    #[arg(long, short)]
    pub dep: Vec<String>,

    /// add acceptance criterion (can be repeated)
    #[arg(long)]
    pub ac: Vec<String>,

    /// add tag (can be repeated)
    #[arg(long)]
    pub tag: Vec<String>,

    /// issue description/body
    #[arg(long, short)]
    pub body: Option<String>,
}
