//! brd sync command - sync issues with the sync branch.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::git;
use crate::repo::RepoPaths;

fn stash_count(cwd: &std::path::Path) -> Result<usize> {
    let output = git::output(&["stash", "list"], cwd)?;
    if output.is_empty() {
        Ok(0)
    } else {
        Ok(output.lines().count())
    }
}

fn has_upstream(branch: &str, cwd: &std::path::Path) -> Result<bool> {
    git::run(
        &["rev-parse", "--abbrev-ref", &format!("{branch}@{{u}}")],
        cwd,
    )
}

pub fn cmd_sync(cli: &Cli, paths: &RepoPaths, push: bool) -> Result<()> {
    let config = Config::load(&paths.config_path())?;

    let branch = config.issues_branch.as_ref().ok_or_else(|| {
        BrdError::Other(
            "not in sync branch mode. use `brd init --sync-branch <name>` to enable".to_string(),
        )
    })?;

    // ensure issues worktree exists
    let issues_wt = paths.ensure_issues_worktree(branch)?;

    let has_upstream = has_upstream(branch, &issues_wt)?;
    let should_push = has_upstream || push;

    if !cli.json {
        if should_push {
            println!("Syncing issues with remote '{}'...", branch);
        } else {
            println!("syncing issues locally on '{}'...", branch);
        }
    }

    // 1. check for local changes in issues worktree
    let has_local_changes = !git::is_clean(&issues_wt)?;
    let mut stashed = false;

    // 2. stash local changes if any
    if has_local_changes {
        if !cli.json {
            println!("  stashing local changes...");
        }
        let stash_before = stash_count(&issues_wt)?;
        if !git::run(
            &[
                "stash",
                "push",
                "--include-untracked",
                "-m",
                "brd sync: stashing local changes",
            ],
            &issues_wt,
        )? {
            return Err(BrdError::Other("failed to stash changes".to_string()));
        }
        let stash_after = stash_count(&issues_wt)?;
        stashed = stash_after > stash_before;
    }

    // 3. fetch and rebase
    let remote_exists = if has_upstream {
        if !cli.json {
            println!("  fetching origin/{}...", branch);
        }
        // try to fetch; if remote doesn't exist, that's ok (first sync)
        git::run(&["fetch", "origin", branch], &issues_wt)?
    } else {
        false
    };

    if remote_exists {
        if !cli.json {
            println!("  rebasing onto origin/{}...", branch);
        }
        if !git::run(&["rebase", &format!("origin/{}", branch)], &issues_wt)? {
            // abort rebase and restore
            let _ = git::run(&["rebase", "--abort"], &issues_wt);
            if stashed {
                let _ = git::run(&["stash", "pop"], &issues_wt);
            }
            return Err(BrdError::Other(
                "rebase failed - there may be conflicts. resolve manually in the issues worktree"
                    .to_string(),
            ));
        }
    }

    // 4. restore local changes
    if stashed {
        if !cli.json {
            println!("  restoring local changes...");
        }
        if !git::run(&["stash", "pop"], &issues_wt)? {
            return Err(BrdError::Other(
                "failed to restore local changes from stash".to_string(),
            ));
        }
    }

    // 5. check for any changes to commit
    let has_uncommitted = !git::is_clean(&issues_wt)?;
    if has_uncommitted {
        if !cli.json {
            println!("  committing issue changes...");
        }
        if !git::run(&["add", ".braid"], &issues_wt)? {
            return Err(BrdError::Other("failed to stage changes".to_string()));
        }
        if !git::run(&["commit", "-m", "chore(braid): sync issues"], &issues_wt)? {
            return Err(BrdError::Other("failed to commit changes".to_string()));
        }
    }

    // 6. push to remote
    if should_push {
        if !cli.json {
            println!("  pushing to origin/{}...", branch);
        }
        if !git::run(&["push", "origin", branch], &issues_wt)? {
            // try with --set-upstream if first push
            if !git::run(&["push", "--set-upstream", "origin", branch], &issues_wt)? {
                return Err(BrdError::Other(format!(
                    "failed to push to origin/{}. you may need to pull and retry.",
                    branch
                )));
            }
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
