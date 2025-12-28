//! brd sync command - sync issues with the sync branch.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::repo::RepoPaths;

/// Run a git command in a directory and return success status.
fn git(args: &[&str], cwd: &std::path::Path) -> Result<bool> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()?;
    Ok(output.status.success())
}

/// Run a git command and capture output.
fn git_output(args: &[&str], cwd: &std::path::Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check if working tree is clean.
fn is_clean(cwd: &std::path::Path) -> Result<bool> {
    let output = git_output(&["status", "--porcelain"], cwd)?;
    Ok(output.is_empty())
}

pub fn cmd_sync(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let config = Config::load(&paths.config_path())?;

    let branch = config.sync_branch.as_ref().ok_or_else(|| {
        BrdError::Other(
            "not in sync branch mode. use `brd init --sync-branch <name>` to enable".to_string(),
        )
    })?;

    // ensure issues worktree exists
    let issues_wt = paths.ensure_issues_worktree(branch)?;

    if !cli.json {
        println!("Syncing issues with remote '{}'...", branch);
    }

    // 1. check for local changes in issues worktree
    let has_local_changes = !is_clean(&issues_wt)?;

    // 2. stash local changes if any
    if has_local_changes {
        if !cli.json {
            println!("  stashing local changes...");
        }
        if !git(
            &["stash", "push", "-m", "brd sync: stashing local changes"],
            &issues_wt,
        )? {
            return Err(BrdError::Other("failed to stash changes".to_string()));
        }
    }

    // 3. fetch and rebase
    if !cli.json {
        println!("  fetching origin/{}...", branch);
    }

    // try to fetch; if remote doesn't exist, that's ok (first sync)
    let fetch_result = git(&["fetch", "origin", branch], &issues_wt);
    let remote_exists = fetch_result.unwrap_or(false);

    if remote_exists {
        if !cli.json {
            println!("  rebasing onto origin/{}...", branch);
        }
        if !git(&["rebase", &format!("origin/{}", branch)], &issues_wt)? {
            // abort rebase and restore
            let _ = git(&["rebase", "--abort"], &issues_wt);
            if has_local_changes {
                let _ = git(&["stash", "pop"], &issues_wt);
            }
            return Err(BrdError::Other(
                "rebase failed - there may be conflicts. resolve manually in the issues worktree"
                    .to_string(),
            ));
        }
    }

    // 4. restore local changes
    if has_local_changes {
        if !cli.json {
            println!("  restoring local changes...");
        }
        if !git(&["stash", "pop"], &issues_wt)? {
            return Err(BrdError::Other(
                "failed to restore local changes from stash".to_string(),
            ));
        }
    }

    // 5. check for any changes to commit
    let has_uncommitted = !is_clean(&issues_wt)?;
    if has_uncommitted {
        if !cli.json {
            println!("  committing issue changes...");
        }
        if !git(&["add", ".braid"], &issues_wt)? {
            return Err(BrdError::Other("failed to stage changes".to_string()));
        }
        if !git(&["commit", "-m", "chore(braid): sync issues"], &issues_wt)? {
            return Err(BrdError::Other("failed to commit changes".to_string()));
        }
    }

    // 6. push to remote
    if !cli.json {
        println!("  pushing to origin/{}...", branch);
    }
    if !git(&["push", "origin", branch], &issues_wt)? {
        // try with --set-upstream if first push
        if !git(&["push", "--set-upstream", "origin", branch], &issues_wt)? {
            return Err(BrdError::Other(format!(
                "failed to push to origin/{}. you may need to pull and retry.",
                branch
            )));
        }
    }

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "branch": branch,
            "issues_worktree": issues_wt.to_string_lossy(),
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Sync complete.");
    }

    Ok(())
}
