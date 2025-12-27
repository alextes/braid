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
    Init,

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
    },

    /// list ready issues
    Ready,

    /// get the next issue to work on
    Next,

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
    },

    /// mark an issue as done
    Done {
        /// issue ID
        id: String,

        /// force completion even if not claimed by you
        #[arg(long)]
        force: bool,
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
    Tui,

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

    /// manage agent instructions block for AGENTS.md
    Agents {
        #[command(subcommand)]
        action: Option<AgentsAction>,
    },
}

#[derive(Subcommand)]
pub enum AgentsAction {
    /// print the agents block to stdout (default)
    Show,
    /// inject/update the agents block in AGENTS.md
    Inject,
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

    /// push changes to main (rebase + fast-forward push)
    Ship,
}

#[derive(Subcommand)]
pub enum DepAction {
    /// add a dependency (child depends on parent)
    Add {
        /// the issue that will depend on parent
        child: String,
        /// the issue that blocks child
        parent: String,
    },
    /// remove a dependency
    Rm {
        /// the issue to remove dependency from
        child: String,
        /// the dependency to remove
        parent: String,
    },
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

    /// add dependency on another issue (can be repeated)
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
