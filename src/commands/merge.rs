//! brd agent merge command - merge changes to main via rebase + fast-forward.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::git;
use crate::repo::RepoPaths;

/// check if we're in an agent worktree (has .braid/agent.toml).
fn is_agent_worktree(paths: &RepoPaths) -> bool {
    paths.worktree_root.join(".braid/agent.toml").exists()
}

pub fn cmd_merge(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    // step 0: check we're in an agent worktree
    if !is_agent_worktree(paths) {
        return Err(BrdError::Other(
            "not in an agent worktree - brd agent merge only works from agent worktrees"
                .to_string(),
        ));
    }

    // step 1: check for clean working tree
    if !git::is_clean(&paths.worktree_root)? {
        return Err(BrdError::Other(
            "working tree is dirty - commit or stash changes first".to_string(),
        ));
    }

    let branch = git::current_branch(&paths.worktree_root)?;

    // check if already on main
    if branch == "main" {
        if cli.json {
            let json = serde_json::json!({
                "ok": false,
                "error": "already_on_main",
                "message": "already on main - use git push directly"
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        } else {
            eprintln!("already on main - use `git push` directly");
        }
        return Ok(());
    }

    if !cli.json {
        println!("merging {} to main...", branch);
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
                "push rejected (not fast-forward) - main has moved, run `brd agent merge` again"
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
            "action": "merged",
            "issues_branch": issues_branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("merged {} to main", branch);
        if let Some(sb) = issues_branch {
            println!();
            println!("note: sync branch mode is active ({})", sb);
            println!("  if you have issue changes, run `brd sync` to push them");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

    fn git_ok(path: &std::path::Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(path)
            .output()
            .expect("git command failed");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn create_repo() -> (tempfile::TempDir, RepoPaths) {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        git_ok(repo_path, &["init"]);
        git_ok(repo_path, &["config", "user.email", "test@test.com"]);
        git_ok(repo_path, &["config", "user.name", "test user"]);
        git_ok(repo_path, &["config", "commit.gpgsign", "false"]);
        git_ok(repo_path, &["checkout", "-b", "main"]);

        std::fs::write(repo_path.join("README.md"), "test\n").unwrap();
        git_ok(repo_path, &["add", "."]);
        git_ok(repo_path, &["commit", "-m", "init"]);

        // Create .braid directory
        let braid_dir = repo_path.join(".braid");
        std::fs::create_dir_all(&braid_dir).unwrap();
        std::fs::write(
            braid_dir.join("config.toml"),
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        )
        .unwrap();

        let paths = RepoPaths {
            worktree_root: repo_path.to_path_buf(),
            git_common_dir: repo_path.join(".git"),
            brd_common_dir: repo_path.join(".git/brd"),
        };
        std::fs::create_dir_all(&paths.brd_common_dir).unwrap();

        (dir, paths)
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

    #[test]
    fn test_is_agent_worktree_true() {
        let (dir, paths) = create_repo();
        let braid_dir = dir.path().join(".braid");
        std::fs::write(braid_dir.join("agent.toml"), "agent_id = \"test\"\n").unwrap();

        assert!(is_agent_worktree(&paths));
    }

    #[test]
    fn test_is_agent_worktree_false() {
        let (_dir, paths) = create_repo();
        // No agent.toml created
        assert!(!is_agent_worktree(&paths));
    }

    #[test]
    fn test_merge_rejects_non_agent_worktree() {
        let (_dir, paths) = create_repo();
        let cli = make_cli();

        // No agent.toml, should reject
        let err = cmd_merge(&cli, &paths).unwrap_err();
        assert!(err.to_string().contains("not in an agent worktree"));
    }

    #[test]
    fn test_merge_rejects_dirty_worktree() {
        let (dir, paths) = create_repo();
        let cli = make_cli();

        // Create agent.toml
        std::fs::write(
            dir.path().join(".braid/agent.toml"),
            "agent_id = \"test\"\n",
        )
        .unwrap();

        // Create uncommitted changes
        std::fs::write(dir.path().join("dirty.txt"), "dirty\n").unwrap();

        let err = cmd_merge(&cli, &paths).unwrap_err();
        assert!(err.to_string().contains("working tree is dirty"));
    }

    #[test]
    fn test_merge_on_main_branch() {
        let (dir, paths) = create_repo();
        let cli = make_cli();

        // Create agent.toml and commit it
        std::fs::write(
            dir.path().join(".braid/agent.toml"),
            "agent_id = \"test\"\n",
        )
        .unwrap();
        git_ok(dir.path(), &["add", "."]);
        git_ok(dir.path(), &["commit", "-m", "add agent.toml"]);

        // We're on main branch, should return Ok but print message
        let result = cmd_merge(&cli, &paths);
        assert!(result.is_ok(), "expected Ok, got: {:?}", result);
    }
}
