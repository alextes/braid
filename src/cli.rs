//! CLI command definitions and wiring.

use clap::{Parser, Subcommand};

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

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// initialize braid in the current repository
    Init,

    /// add a new issue
    Add {
        /// issue title
        title: String,

        /// priority (P0-P3, default P2)
        #[arg(long, short, default_value = "P2")]
        priority: String,

        /// add dependency on another issue (can be repeated)
        #[arg(long, short)]
        dep: Vec<String>,

        /// add acceptance criterion (can be repeated)
        #[arg(long)]
        ac: Vec<String>,
    },

    /// list issues
    Ls {
        /// filter by status
        #[arg(long)]
        status: Option<String>,

        /// filter by priority
        #[arg(long)]
        priority: Option<String>,

        /// show only ready issues
        #[arg(long)]
        ready: bool,

        /// show only blocked issues
        #[arg(long)]
        blocked: bool,
    },

    /// show details of an issue
    Show {
        /// issue ID (full or partial)
        id: String,
    },

    /// list ready issues
    Ready {
        /// include issues claimed by other agents
        #[arg(long)]
        include_claimed: bool,
    },

    /// get the next issue to work on
    Next {
        /// claim the issue
        #[arg(long)]
        claim: bool,

        /// include issues claimed by other agents
        #[arg(long)]
        include_claimed: bool,
    },

    /// add or remove dependencies
    Dep {
        #[command(subcommand)]
        action: DepAction,
    },

    /// claim an issue
    Claim {
        /// issue ID
        id: String,
    },

    /// release a claim on an issue
    Release {
        /// issue ID
        id: String,

        /// force release even if claimed by another agent
        #[arg(long)]
        force: bool,
    },

    /// reclaim an issue (steal expired claim)
    Reclaim {
        /// issue ID
        id: String,

        /// force reclaim even if not expired
        #[arg(long)]
        force: bool,
    },

    /// list all claims
    Claims {
        /// show all claims including expired
        #[arg(long)]
        all: bool,
    },

    /// start working on an issue
    Start {
        /// issue ID
        id: String,
    },

    /// mark an issue as done
    Done {
        /// issue ID
        id: String,

        /// force completion even if not claimed by you
        #[arg(long)]
        force: bool,
    },

    /// validate repository state
    Doctor,

    /// generate shell completions
    Completions {
        /// shell to generate completions for
        shell: clap_complete::Shell,
    },
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
