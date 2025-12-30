//! brd ship command - push changes to main via rebase + fast-forward.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::git;
use crate::repo::RepoPaths;

/// check if we're in an agent worktree (has .braid/agent.toml).
fn is_agent_worktree(paths: &RepoPaths) -> bool {
    paths.worktree_root.join(".braid/agent.toml").exists()
}

pub fn cmd_ship(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    // step 0: check we're in an agent worktree
    if !is_agent_worktree(paths) {
        return Err(BrdError::Other(
            "not in an agent worktree - brd agent ship only works from agent worktrees".to_string(),
        ));
    }

    // step 1: check for clean working tree
    if !git::is_clean(&paths.worktree_root)? {
        return Err(BrdError::Other(
            "working tree is dirty - commit or stash changes first".to_string(),
        ));
    }

    let branch = git::current_branch(&paths.worktree_root)?;

    if !cli.json {
        println!("shipping {} to main...", branch);
    }

    // step 2: fetch origin main
    if !cli.json {
        println!("  fetching origin main...");
    }
    if !git::run(&["fetch", "origin", "main"], &paths.worktree_root)? {
        return Err(BrdError::Other("failed to fetch origin main".to_string()));
    }

    // step 3: rebase onto origin/main
    if !cli.json {
        println!("  rebasing onto origin/main...");
    }
    let rebase_output = git::run_full(&["rebase", "origin/main"], &paths.worktree_root)?;
    if !rebase_output.status.success() {
        // rebase failed - abort and tell user
        let _ = git::run(&["rebase", "--abort"], &paths.worktree_root);
        let stderr = String::from_utf8_lossy(&rebase_output.stderr);
        return Err(BrdError::Other(format!(
            "rebase failed - resolve conflicts manually:\n{}",
            stderr.trim()
        )));
    }

    // step 4: push to main (fast-forward only)
    if !cli.json {
        println!("  pushing to main...");
    }
    let push_ref = format!("{}:main", branch);
    let push_output = git::run_full(&["push", "origin", &push_ref], &paths.worktree_root)?;
    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        if stderr.contains("non-fast-forward")
            || stderr.contains("rejected")
            || stderr.contains("failed to push")
        {
            return Err(BrdError::Other(
                "push rejected (not fast-forward) - main has moved, run `brd agent ship` again"
                    .to_string(),
            ));
        }
        return Err(BrdError::Other(format!("push failed: {}", stderr.trim())));
    }

    // step 5: reset to origin/main
    if !cli.json {
        println!("  resetting to origin/main...");
    }
    git::run(&["fetch", "origin", "main"], &paths.worktree_root)?;
    git::run(&["reset", "--hard", "origin/main"], &paths.worktree_root)?;

    // check if sync branch mode is active
    let config = Config::load(&paths.config_path()).ok();
    let issues_branch = config.as_ref().and_then(|c| c.issues_branch.as_ref());

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "branch": branch,
            "action": "shipped",
            "issues_branch": issues_branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("shipped {} to main", branch);
        if let Some(sb) = issues_branch {
            println!();
            println!("note: sync branch mode is active ({})", sb);
            println!("  if you have issue changes, run `brd sync` to push them");
        }
    }

    Ok(())
}
