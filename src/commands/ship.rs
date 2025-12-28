//! brd ship command - push changes to main via rebase + fast-forward.

use std::process::Command;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::repo::RepoPaths;

/// run a git command and return its output.
fn git(paths: &RepoPaths, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("git")
        .args(args)
        .current_dir(&paths.worktree_root)
        .output()
}

/// run a git command, returning Ok if successful or Err with stderr on failure.
fn git_check(paths: &RepoPaths, args: &[&str]) -> Result<String> {
    let output = git(paths, args)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(BrdError::Other(stderr))
    }
}

/// get the current branch name.
fn get_current_branch(paths: &RepoPaths) -> Result<String> {
    git_check(paths, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// check if working tree is clean.
fn is_working_tree_clean(paths: &RepoPaths) -> Result<bool> {
    let output = git_check(paths, &["status", "--porcelain"])?;
    Ok(output.is_empty())
}

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
    if !is_working_tree_clean(paths)? {
        return Err(BrdError::Other(
            "working tree is dirty - commit or stash changes first".to_string(),
        ));
    }

    let branch = get_current_branch(paths)?;

    if !cli.json {
        println!("shipping {} to main...", branch);
    }

    // step 2: fetch origin main
    if !cli.json {
        println!("  fetching origin main...");
    }
    if let Err(e) = git_check(paths, &["fetch", "origin", "main"]) {
        return Err(BrdError::Other(format!(
            "failed to fetch origin main: {}",
            e
        )));
    }

    // step 3: rebase onto origin/main
    if !cli.json {
        println!("  rebasing onto origin/main...");
    }
    let rebase_output = git(paths, &["rebase", "origin/main"])?;
    if !rebase_output.status.success() {
        // rebase failed - abort and tell user
        let _ = git(paths, &["rebase", "--abort"]);
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
    let push_output = git(paths, &["push", "origin", &push_ref])?;
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
    git_check(paths, &["fetch", "origin", "main"])?;
    git_check(paths, &["reset", "--hard", "origin/main"])?;

    // check if sync branch mode is active
    let config = Config::load(&paths.config_path()).ok();
    let sync_branch = config.as_ref().and_then(|c| c.sync_branch.as_ref());

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "branch": branch,
            "action": "shipped",
            "sync_branch": sync_branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("shipped {} to main", branch);
        if let Some(sb) = sync_branch {
            println!();
            println!("note: sync branch mode is active ({})", sb);
            println!("  if you have issue changes, run `brd sync` to push them");
        }
    }

    Ok(())
}
