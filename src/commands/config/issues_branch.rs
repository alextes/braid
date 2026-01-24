//! brd config issues-branch - set or clear the issues-branch setting.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::git;
use crate::repo::RepoPaths;

use super::{
    ISSUES_SYMLINK_PATTERN, confirm, count_issues, find_agent_worktrees_needing_rebase,
    remove_from_git_exclude, warn_agent_worktrees,
};

/// Set or clear the issues-branch setting.
pub fn cmd_config_issues_branch(
    cli: &Cli,
    paths: &RepoPaths,
    name: Option<&str>,
    clear: bool,
    yes: bool,
) -> Result<()> {
    // Handle clear case
    if clear {
        return clear_issues_branch(cli, paths, yes);
    }

    // Handle set case
    let branch = match name {
        Some(b) => b,
        None => {
            return Err(BrdError::Other(
                "must provide branch name or use --clear".to_string(),
            ));
        }
    };

    let mut config = Config::load(&paths.config_path())?;

    // check if already set to this branch
    if config.issues_branch.as_deref() == Some(branch) {
        if !cli.json {
            println!("issues-branch already set to '{}'", branch);
        }
        return Ok(());
    }

    // check if already set to a different branch
    if let Some(existing) = &config.issues_branch {
        return Err(BrdError::Other(format!(
            "issues-branch already set to '{}'. run `brd config issues-branch --clear` first.",
            existing
        )));
    }

    // check for uncommitted changes
    if !git::is_clean(&paths.worktree_root)? {
        return Err(BrdError::Other(
            "working tree has uncommitted changes - commit or stash first".to_string(),
        ));
    }

    // confirmation prompt (unless -y or --json)
    if !yes && !cli.json {
        let local_issues = paths.worktree_root.join(".braid/issues");
        let issue_count = count_issues(&local_issues);

        println!("Setting issues-branch to '{}'...", branch);
        println!();
        println!("This will:");
        println!("  • Create branch '{}' for issue storage", branch);
        println!(
            "  • Set up shared worktree at {}",
            paths.brd_common_dir.join("issues").display()
        );
        if issue_count > 0 {
            println!(
                "  • Move {} issue(s) from .braid/issues/ to the worktree",
                issue_count
            );
        }
        println!("  • Commit the changes");
        println!();

        if !confirm("Continue?")? {
            println!("Aborted.");
            return Ok(());
        }
        println!();
    }

    if !cli.json {
        println!("Setting issues-branch...");
    }

    // 1. create sync branch if it doesn't exist
    let branch_exists = git::run(&["rev-parse", "--verify", branch], &paths.worktree_root)?;

    if !branch_exists {
        if !git::run(&["branch", branch], &paths.worktree_root)? {
            return Err(BrdError::Other(format!(
                "failed to create sync branch '{}'",
                branch
            )));
        }
        if !cli.json {
            println!("  created branch '{}'", branch);
        }
    }

    // 2. set up shared issues worktree
    let issues_wt = paths.ensure_issues_worktree(branch)?;
    if !cli.json {
        println!("  issues worktree at {}", issues_wt.display());
    }

    // 3. move existing issues to sync branch worktree
    let local_issues = paths.worktree_root.join(".braid/issues");
    let wt_issues = issues_wt.join(".braid/issues");
    std::fs::create_dir_all(&wt_issues)?;

    let mut moved_count = 0;
    if local_issues.exists() {
        for entry in std::fs::read_dir(&local_issues)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                let dest = wt_issues.join(path.file_name().unwrap());
                std::fs::copy(&path, &dest)?;
                std::fs::remove_file(&path)?;
                moved_count += 1;
            }
        }
    }

    if moved_count > 0 && !cli.json {
        println!("  moved {} issue(s) to sync branch", moved_count);
    }

    // 4. update config
    config.issues_branch = Some(branch.to_string());
    config.save(&paths.config_path())?;

    // 5. commit the changes
    if !git::run(&["add", ".braid"], &paths.worktree_root)? {
        return Err(BrdError::Other(
            "failed to stage .braid changes".to_string(),
        ));
    }

    let commit_msg = format!("chore(braid): set issues-branch to '{}'", branch);
    // commit might fail if nothing changed, that's ok
    let _ = git::run(&["commit", "-m", &commit_msg], &paths.worktree_root);

    // also commit in the issues worktree
    if !git::run(&["add", ".braid"], &issues_wt)? {
        return Err(BrdError::Other(
            "failed to stage .braid in issues worktree".to_string(),
        ));
    }
    let _ = git::run(
        &["commit", "-m", "chore(braid): initial issues"],
        &issues_wt,
    );

    // check for agent worktrees needing rebase
    let agent_worktrees = find_agent_worktrees_needing_rebase(&paths.worktree_root);

    if cli.json {
        let worktrees_json: Vec<_> = agent_worktrees
            .iter()
            .map(|wt| {
                serde_json::json!({
                    "branch": wt.branch,
                    "path": wt.path.to_string_lossy(),
                })
            })
            .collect();

        let json = serde_json::json!({
            "ok": true,
            "issues_branch": branch,
            "issues_worktree": issues_wt.to_string_lossy(),
            "moved_issues": moved_count,
            "agent_worktrees_needing_rebase": worktrees_json,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!();
        println!("issues-branch set to '{}'", branch);
        println!("Issues now live on shared worktree.");

        warn_agent_worktrees(&agent_worktrees);
    }

    Ok(())
}

/// Clear the issues-branch setting (move issues back to .braid/issues/).
fn clear_issues_branch(cli: &Cli, paths: &RepoPaths, yes: bool) -> Result<()> {
    let mut config = Config::load(&paths.config_path())?;

    // check if issues_branch is set
    let branch = match &config.issues_branch {
        Some(b) => b.clone(),
        None => {
            if !cli.json {
                println!("issues-branch is not set");
            }
            return Ok(());
        }
    };

    // check for uncommitted changes
    if !git::is_clean(&paths.worktree_root)? {
        return Err(BrdError::Other(
            "working tree has uncommitted changes - commit or stash first".to_string(),
        ));
    }

    // get issues from sync worktree
    let issues_wt = paths.issues_worktree_dir();
    let wt_issues = issues_wt.join(".braid/issues");
    let local_issues = paths.worktree_root.join(".braid/issues");

    // check for uncommitted changes in issues worktree
    if issues_wt.exists() && !git::is_clean(&issues_wt)? {
        return Err(BrdError::Other(
            "issues worktree has uncommitted changes - commit them first with `brd sync`"
                .to_string(),
        ));
    }

    // confirmation prompt (unless -y or --json)
    if !yes && !cli.json {
        let issue_count = count_issues(&wt_issues);

        println!("Clearing issues-branch setting...");
        println!();
        println!("This will:");
        if issue_count > 0 {
            println!(
                "  • Copy {} issue(s) from worktree to .braid/issues/",
                issue_count
            );
        }
        println!("  • Remove issues-branch from config");
        println!("  • Commit the changes");
        println!(
            "  • Leave worktree at {} (you can remove it manually)",
            issues_wt.display()
        );
        println!();

        if !confirm("Continue?")? {
            println!("Aborted.");
            return Ok(());
        }
        println!();
    }

    if !cli.json {
        println!("Clearing issues-branch...");
    }

    // remove symlink if it exists
    if let Err(e) = remove_issues_symlink(paths)
        && !cli.json
    {
        eprintln!("  warning: could not remove issues symlink: {}", e);
    }

    std::fs::create_dir_all(&local_issues)?;

    // copy issues back from sync worktree
    let mut moved_count = 0;
    if wt_issues.exists() {
        for entry in std::fs::read_dir(&wt_issues)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                let dest = local_issues.join(path.file_name().unwrap());
                std::fs::copy(&path, &dest)?;
                moved_count += 1;
            }
        }
    }

    if moved_count > 0 && !cli.json {
        println!("  copied {} issue(s) from worktree", moved_count);
    }

    // update config (remove issues_branch)
    config.issues_branch = None;
    config.save(&paths.config_path())?;

    // commit the changes
    if !git::run(&["add", ".braid"], &paths.worktree_root)? {
        return Err(BrdError::Other(
            "failed to stage .braid changes".to_string(),
        ));
    }

    let commit_msg = format!("chore(braid): clear issues-branch (was '{}')", branch);
    let _ = git::run(&["commit", "-m", &commit_msg], &paths.worktree_root);

    // check for agent worktrees needing rebase
    let agent_worktrees = find_agent_worktrees_needing_rebase(&paths.worktree_root);

    if !cli.json {
        println!();
        println!("issues-branch cleared");
        println!("Issues now live in .braid/issues/");
        if issues_wt.exists() {
            println!();
            println!("Note: worktree still exists at {}", issues_wt.display());
            println!(
                "You can remove it with: git worktree remove {}",
                issues_wt.display()
            );
        }

        warn_agent_worktrees(&agent_worktrees);
    }

    if cli.json {
        let worktrees_json: Vec<_> = agent_worktrees
            .iter()
            .map(|wt| {
                serde_json::json!({
                    "branch": wt.branch,
                    "path": wt.path.to_string_lossy(),
                })
            })
            .collect();

        let json = serde_json::json!({
            "ok": true,
            "issues_branch": serde_json::Value::Null,
            "from_branch": branch,
            "moved_issues": moved_count,
            "agent_worktrees_needing_rebase": worktrees_json,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    Ok(())
}

/// Remove the issues symlink if it exists.
pub fn remove_issues_symlink(paths: &RepoPaths) -> Result<()> {
    let symlink_path = paths.worktree_root.join(".braid/issues");

    if symlink_path.is_symlink() {
        std::fs::remove_file(&symlink_path)?;
    }

    // remove from .git/info/exclude
    remove_from_git_exclude(paths, ISSUES_SYMLINK_PATTERN)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::config::tests::{make_cli, make_paths, setup_git_repo};
    use std::fs;
    use std::process::Command;

    fn setup_braid_config(dir: &tempfile::TempDir, content: &str) {
        fs::create_dir_all(dir.path().join(".braid")).unwrap();
        fs::write(dir.path().join(".braid/config.toml"), content).unwrap();
    }

    #[test]
    fn test_config_issues_branch_already_set() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_branch = \"existing-branch\"\n",
        );

        let result = cmd_config_issues_branch(&cli, &paths, Some("new-branch"), false, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("issues-branch already set")
        );
    }

    #[test]
    fn test_config_issues_branch_uncommitted_changes() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // create uncommitted changes
        fs::write(dir.path().join("uncommitted.txt"), "content").unwrap();

        let result = cmd_config_issues_branch(&cli, &paths, Some("braid-issues"), false, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("uncommitted changes")
        );
    }

    #[test]
    fn test_config_issues_branch_success() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // commit the .braid directory
        Command::new("git")
            .args(["add", ".braid"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add braid config"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let result = cmd_config_issues_branch(&cli, &paths, Some("braid-issues"), false, true);
        assert!(result.is_ok());

        // verify config was updated
        let config = Config::load(&paths.config_path()).unwrap();
        assert_eq!(config.issues_branch, Some("braid-issues".to_string()));
    }

    #[test]
    fn test_config_issues_branch_clear_when_not_set() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // clearing when not set should succeed (no-op)
        let result = cmd_config_issues_branch(&cli, &paths, None, true, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_clear_issues_branch_success() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // ensure main branch exists
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(dir.path())
            .output();

        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // commit .braid
        Command::new("git")
            .args(["add", ".braid"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add braid config"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // set issues-branch (creates worktree)
        let result = cmd_config_issues_branch(&cli, &paths, Some("braid-issues"), false, true);
        assert!(result.is_ok());

        // verify issues_branch is set
        let config = Config::load(&paths.config_path()).unwrap();
        assert_eq!(config.issues_branch, Some("braid-issues".to_string()));

        // create an issue in the worktree
        let issues_wt = paths.issues_worktree_dir();
        let wt_issues = issues_wt.join(".braid/issues");
        fs::create_dir_all(&wt_issues).unwrap();
        fs::write(
            wt_issues.join("tst-abc1.md"),
            "---\nid: tst-abc1\ntitle: test issue\npriority: P2\nstatus: open\ncreated_at: 2024-01-01T00:00:00Z\nupdated_at: 2024-01-01T00:00:00Z\n---\n",
        )
        .unwrap();

        // commit the issue in the worktree
        Command::new("git")
            .args(["add", "."])
            .current_dir(&issues_wt)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add issue"])
            .current_dir(&issues_wt)
            .output()
            .unwrap();

        // now clear issues-branch
        let result = cmd_config_issues_branch(&cli, &paths, None, true, true);
        assert!(result.is_ok());

        // verify issues_branch is cleared
        let config = Config::load(&paths.config_path()).unwrap();
        assert!(config.issues_branch.is_none());

        // verify issue was copied back to local .braid/issues/
        let local_issues = dir.path().join(".braid/issues");
        assert!(local_issues.join("tst-abc1.md").exists());
    }

    #[test]
    fn test_clear_issues_branch_dirty_issues_worktree() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // ensure main branch exists
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(dir.path())
            .output();

        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // commit .braid
        Command::new("git")
            .args(["add", ".braid"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add braid config"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // set issues-branch (creates worktree)
        let result = cmd_config_issues_branch(&cli, &paths, Some("braid-issues"), false, true);
        assert!(result.is_ok());

        // create uncommitted changes in the issues worktree
        let issues_wt = paths.issues_worktree_dir();
        fs::write(issues_wt.join("uncommitted.txt"), "dirty").unwrap();

        // try to clear - should fail
        let result = cmd_config_issues_branch(&cli, &paths, None, true, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("issues worktree has uncommitted changes")
        );
    }

    #[test]
    fn test_clear_issues_branch_no_issues() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // ensure main branch exists
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(dir.path())
            .output();

        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // commit .braid
        Command::new("git")
            .args(["add", ".braid"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add braid config"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // set issues-branch (creates worktree)
        let result = cmd_config_issues_branch(&cli, &paths, Some("braid-issues"), false, true);
        assert!(result.is_ok());

        // don't create any issues - just clear immediately
        let result = cmd_config_issues_branch(&cli, &paths, None, true, true);
        assert!(result.is_ok());

        // verify issues_branch is cleared
        let config = Config::load(&paths.config_path()).unwrap();
        assert!(config.issues_branch.is_none());
    }
}
