//! brd sync command - sync issues with the sync branch.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::git;
use crate::repo::RepoPaths;

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
        stashed = git::stash_push(&issues_wt, "brd sync: stashing local changes")?;
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
                let _ = git::stash_pop(&issues_wt);
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
        if !git::stash_pop(&issues_wt)? {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    fn setup_git_repo() -> tempfile::TempDir {
        let dir = tempdir().unwrap();

        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // initial commit
        fs::write(dir.path().join(".gitkeep"), "").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        dir
    }

    fn make_cli() -> Cli {
        Cli {
            json: false,
            repo: None,
            no_color: true,
            verbose: false,
            command: crate::cli::Command::Doctor,
        }
    }

    fn make_paths(dir: &tempfile::TempDir) -> RepoPaths {
        RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        }
    }

    #[test]
    fn test_stash_count_empty() {
        let dir = setup_git_repo();
        let count = git::stash_count(dir.path()).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_stash_count_with_stash() {
        let dir = setup_git_repo();

        // create a file and stash it
        fs::write(dir.path().join("unstaged.txt"), "content").unwrap();
        Command::new("git")
            .args(["stash", "push", "--include-untracked"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let count = git::stash_count(dir.path()).unwrap();
        assert_eq!(count, 1);

        // add another stash
        fs::write(dir.path().join("another.txt"), "content").unwrap();
        Command::new("git")
            .args(["stash", "push", "--include-untracked"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let count = git::stash_count(dir.path()).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_has_upstream_no_upstream() {
        let dir = setup_git_repo();

        // create a branch without upstream
        Command::new("git")
            .args(["branch", "test-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let result = has_upstream("test-branch", dir.path()).unwrap();
        assert!(!result);
    }

    #[test]
    fn test_sync_not_in_sync_mode() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // create config without issues_branch
        fs::create_dir_all(dir.path().join(".braid")).unwrap();
        fs::write(
            dir.path().join(".braid/config.toml"),
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        )
        .unwrap();

        let result = cmd_sync(&cli, &paths, false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not in sync branch mode")
        );
    }

    #[test]
    fn test_sync_creates_worktree() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // create brd common dir and config with issues_branch
        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        fs::create_dir_all(dir.path().join(".braid")).unwrap();
        fs::write(
            dir.path().join(".braid/config.toml"),
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_branch = \"braid-issues\"\n",
        ).unwrap();

        // create the issues branch
        Command::new("git")
            .args(["branch", "braid-issues"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // run sync - should create worktree
        let result = cmd_sync(&cli, &paths, false);
        assert!(result.is_ok());

        // verify worktree was created
        assert!(paths.brd_common_dir.join("issues").exists());
    }
}
