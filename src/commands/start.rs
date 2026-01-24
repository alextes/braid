//! brd start command with auto-sync.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::git;
use crate::graph::get_ready_issues;
use crate::issue::{Issue, IssueType, Status};
use crate::lock::LockGuard;
use crate::repo::{self, RepoPaths};

use super::{issue_to_json, load_all_issues, resolve_issue_id};

/// Claim an issue by setting status to Doing and owner.
/// Returns the issue path where it was saved.
pub fn claim_issue(
    paths: &RepoPaths,
    config: &Config,
    issue: &mut Issue,
    agent_id: &str,
    force: bool,
) -> Result<std::path::PathBuf> {
    if issue.status() == Status::Doing && !force {
        let owner = issue.frontmatter.owner.as_deref().unwrap_or("unknown");
        return Err(BrdError::Other(format!(
            "issue {} is already being worked on by '{}' (use --force to reassign)",
            issue.id(),
            owner
        )));
    }

    issue.frontmatter.status = Status::Doing;
    issue.frontmatter.owner = Some(agent_id.to_string());
    issue.mark_started();

    let issue_path = paths.issues_dir(config).join(format!("{}.md", issue.id()));
    issue.save(&issue_path)?;

    Ok(issue_path)
}

/// Check for done issues that haven't been pushed to main yet.
fn check_unshipped_done_issues(
    paths: &RepoPaths,
    config: &Config,
    cli: &Cli,
) -> Result<Option<Vec<String>>> {
    // Get list of done issues locally
    let issues = load_all_issues(paths, config)?;
    let local_done: Vec<&Issue> = issues
        .values()
        .filter(|i| i.status() == Status::Done)
        .collect();

    if local_done.is_empty() {
        return Ok(None);
    }

    // Check which ones are different from origin/main
    let mut unshipped = Vec::new();
    for issue in local_done {
        let issue_file = format!(".braid/issues/{}.md", issue.id());
        // Check if file differs from origin/main
        let diff_output = git::output(
            &["diff", "origin/main", "--", &issue_file],
            &paths.worktree_root,
        )?;
        if !diff_output.is_empty() {
            unshipped.push(issue.id().to_string());
        }
    }

    if unshipped.is_empty() {
        Ok(None)
    } else {
        if !cli.json {
            eprintln!(
                "warning: {} done issue(s) not yet in main: {}",
                unshipped.len(),
                unshipped.join(", ")
            );
            eprintln!("  consider running `brd agent merge` first");
        }
        Ok(Some(unshipped))
    }
}

/// Fetch and rebase the issues branch in issues_branch mode.
pub fn sync_issues_branch(paths: &RepoPaths, config: &Config, cli: &Cli) -> Result<()> {
    let branch = config
        .issues_branch
        .as_ref()
        .ok_or_else(|| BrdError::Other("issues_branch not configured".to_string()))?;

    let issues_wt = paths.ensure_issues_worktree(branch)?;

    if !git::has_remote(&issues_wt, "origin") {
        if !cli.json {
            eprintln!("(no origin remote, skipping sync)");
        }
        return Ok(());
    }

    if !cli.json {
        eprintln!("syncing with origin/{}...", branch);
    }

    // Fetch
    if !git::run(&["fetch", "origin", branch], &issues_wt)? {
        if !cli.json {
            eprintln!("  (origin/{} not found, skipping rebase)", branch);
        }
        return Ok(());
    }

    // Rebase
    let remote_branch = format!("origin/{}", branch);
    if git::has_remote_branch(&issues_wt, "origin", branch)
        && !git::run(&["rebase", &remote_branch], &issues_wt)?
    {
        let _ = git::run(&["rebase", "--abort"], &issues_wt);
        return Err(BrdError::Other(
            "rebase failed on issues branch - resolve conflicts manually or use --no-sync"
                .to_string(),
        ));
    }

    Ok(())
}

/// Fetch and rebase onto origin/main.
/// If `stash` is true, stash uncommitted changes before rebase and restore after.
pub fn sync_with_main(paths: &RepoPaths, cli: &Cli, stash: bool) -> Result<()> {
    // Skip if no origin remote
    if !git::has_remote(&paths.worktree_root, "origin") {
        if !cli.json {
            eprintln!("(no origin remote, skipping sync)");
        }
        return Ok(());
    }

    if !cli.json {
        eprintln!("syncing with origin/main...");
    }

    // Check for uncommitted changes outside .braid
    let has_non_braid_changes = if !git::is_clean(&paths.worktree_root)? {
        let status = git::output(&["status", "--porcelain"], &paths.worktree_root)?;
        status.lines().any(|line| !line.contains(".braid/"))
    } else {
        false
    };

    let mut stashed = false;

    if has_non_braid_changes {
        if stash {
            if !cli.json {
                eprintln!("  stashing uncommitted changes...");
            }
            stashed = git::stash_push(&paths.worktree_root, "brd start: stashing changes")?;
        } else {
            return Err(BrdError::Other(
                "working tree has uncommitted changes outside .braid\n\n\
                 options:\n  \
                 - commit or stash manually first\n  \
                 - use --stash to auto-stash and restore after sync\n  \
                 - use --no-sync to skip sync (trust local state)"
                    .to_string(),
            ));
        }
    }

    // Fetch
    if !git::run(&["fetch", "origin", "main"], &paths.worktree_root)? {
        // Restore stash if fetch fails
        if stashed {
            if !cli.json {
                eprintln!("  restoring stashed changes...");
            }
            let _ = git::stash_pop(&paths.worktree_root);
        }
        // origin exists but main branch doesn't - skip sync
        if !cli.json {
            eprintln!("  (origin/main not found, skipping rebase)");
        }
        return Ok(());
    }

    // Only rebase if origin/main exists
    if git::has_remote_branch(&paths.worktree_root, "origin", "main")
        && !git::run(&["rebase", "origin/main"], &paths.worktree_root)?
    {
        // Abort rebase on failure
        let _ = git::run(&["rebase", "--abort"], &paths.worktree_root);
        // Restore stash even on failure
        if stashed {
            if !cli.json {
                eprintln!("  restoring stashed changes...");
            }
            let _ = git::stash_pop(&paths.worktree_root);
        }
        return Err(BrdError::Other(
            "rebase failed - resolve conflicts manually or use --no-sync".to_string(),
        ));
    }

    // Restore stashed changes after successful sync
    if stashed {
        if !cli.json {
            eprintln!("  restoring stashed changes...");
        }
        if !git::stash_pop(&paths.worktree_root)? {
            return Err(BrdError::Other(
                "sync succeeded but failed to restore stashed changes\n\
                 your changes are in `git stash` - run `git stash pop` to restore"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

/// Commit and push the claim to main with retry logic.
pub fn commit_and_push_main(paths: &RepoPaths, issue_id: &str, cli: &Cli) -> Result<()> {
    commit_and_push_main_with_action(paths, issue_id, "start", cli)
}

/// Commit and push to main with a custom action prefix for the commit message.
pub fn commit_and_push_main_with_action(
    paths: &RepoPaths,
    issue_id: &str,
    action: &str,
    cli: &Cli,
) -> Result<()> {
    // Commit
    if !git::run(&["add", ".braid"], &paths.worktree_root)? {
        return Err(BrdError::Other("failed to stage .braid".to_string()));
    }

    let commit_msg = format!("chore(braid): {} {}", action, issue_id);
    if !git::run(&["commit", "-m", &commit_msg], &paths.worktree_root)? {
        // Nothing to commit is ok
        if !cli.json {
            eprintln!("  (no changes to commit)");
        }
    }

    // Skip push if no origin remote
    if !git::has_remote(&paths.worktree_root, "origin") {
        if !cli.json {
            eprintln!("  (no origin remote, skipping push)");
        }
        return Ok(());
    }

    // Push with retry
    const MAX_RETRIES: u32 = 2;
    for attempt in 0..=MAX_RETRIES {
        if git::run(&["push", "origin", "HEAD:main"], &paths.worktree_root)? {
            return Ok(());
        }

        if attempt < MAX_RETRIES {
            if !cli.json {
                eprintln!(
                    "  push rejected, rebasing and retrying ({}/{})...",
                    attempt + 1,
                    MAX_RETRIES
                );
            }

            // Pull and rebase
            if !git::run(&["fetch", "origin", "main"], &paths.worktree_root)? {
                return Err(BrdError::Other("failed to fetch during retry".to_string()));
            }
            if !git::run(&["rebase", "origin/main"], &paths.worktree_root)? {
                let _ = git::run(&["rebase", "--abort"], &paths.worktree_root);
                return Err(BrdError::Other(
                    "rebase failed during push retry - resolve manually".to_string(),
                ));
            }
        }
    }

    Err(BrdError::Other(format!(
        "push failed after {} retries - another agent may have pushed. \
         run `git pull --rebase origin main` and check if issue is still available",
        MAX_RETRIES
    )))
}

/// Commit and push to sync branch.
pub fn commit_and_push_issues_branch(
    paths: &RepoPaths,
    config: &Config,
    issue_id: &str,
    cli: &Cli,
) -> Result<()> {
    commit_and_push_issues_branch_with_action(paths, config, issue_id, "start", cli)
}

/// Commit and push to sync branch with a custom action prefix for the commit message.
pub fn commit_and_push_issues_branch_with_action(
    paths: &RepoPaths,
    config: &Config,
    issue_id: &str,
    action: &str,
    cli: &Cli,
) -> Result<()> {
    let branch = config
        .issues_branch
        .as_ref()
        .ok_or_else(|| BrdError::Other("issues_branch not configured".to_string()))?;

    let issues_wt = paths.ensure_issues_worktree(branch)?;

    // Commit
    if !git::run(&["add", ".braid"], &issues_wt)? {
        return Err(BrdError::Other(
            "failed to stage .braid in sync worktree".to_string(),
        ));
    }

    let commit_msg = format!("chore(braid): {} {}", action, issue_id);
    if !git::run(&["commit", "-m", &commit_msg], &issues_wt)? && !cli.json {
        eprintln!("  (no changes to commit in sync branch)");
    }

    // Push with retry
    const MAX_RETRIES: u32 = 2;
    for attempt in 0..=MAX_RETRIES {
        if git::run(&["push", "origin", branch], &issues_wt)? {
            return Ok(());
        }

        if attempt < MAX_RETRIES {
            if !cli.json {
                eprintln!(
                    "  push rejected, rebasing sync branch ({}/{})...",
                    attempt + 1,
                    MAX_RETRIES
                );
            }

            if !git::run(&["fetch", "origin", branch], &issues_wt)? {
                return Err(BrdError::Other(
                    "failed to fetch sync branch during retry".to_string(),
                ));
            }
            if !git::run(&["rebase", &format!("origin/{}", branch)], &issues_wt)? {
                let _ = git::run(&["rebase", "--abort"], &issues_wt);
                return Err(BrdError::Other(
                    "rebase failed on sync branch - resolve manually".to_string(),
                ));
            }
        }
    }

    Err(BrdError::Other(format!(
        "push to sync branch failed after {} retries",
        MAX_RETRIES
    )))
}

pub fn cmd_start(
    cli: &Cli,
    paths: &RepoPaths,
    id: Option<&str>,
    force: bool,
    no_sync: bool,
    no_push: bool,
    stash: bool,
) -> Result<()> {
    let config = Config::load(&paths.config_path())?;

    // Step 1: Pre-flight check for unshipped done issues (git-native mode only)
    if !no_sync && config.auto_pull && !config.is_issues_branch_mode() {
        // Try to check, but don't fail if origin/main doesn't exist
        let _ = check_unshipped_done_issues(paths, &config, cli);
    }

    // Step 2: Sync with remote if auto_pull is enabled
    if !no_sync && config.auto_pull {
        if config.is_issues_branch_mode() {
            sync_issues_branch(paths, &config, cli)?;
        } else {
            sync_with_main(paths, cli, stash)?;
        }
    }

    // Step 3: Reload issues and claim
    let _lock = LockGuard::acquire(&paths.lock_path())?;
    let mut issues = load_all_issues(paths, &config)?;

    // Resolve issue id
    let full_id = match id {
        Some(partial) => resolve_issue_id(partial, &issues)?,
        None => {
            let ready = get_ready_issues(&issues);
            ready
                .into_iter()
                .find(|issue| issue.issue_type() != Some(IssueType::Meta))
                .map(|i| i.id().to_string())
                .ok_or_else(|| BrdError::Other("no ready issues".to_string()))?
        }
    };

    let agent_id = repo::get_agent_id(&paths.worktree_root);

    // Warn if agent already has uncompleted work
    if !cli.json {
        let active_issues: Vec<_> = issues
            .values()
            .filter(|i| {
                i.status() == Status::Doing
                    && i.frontmatter.owner.as_deref() == Some(&agent_id)
                    && i.id() != full_id
            })
            .collect();

        if !active_issues.is_empty() {
            eprintln!(
                "warning: you have {} issue(s) still in progress:",
                active_issues.len()
            );
            for issue in &active_issues {
                eprintln!("  - {}: {}", issue.id(), issue.title());
            }
            eprintln!();
        }
    }

    // Verify issue is still available and claim it
    {
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(full_id.clone()))?;

        claim_issue(paths, &config, issue, &agent_id, force)?;
    }

    // Step 4: Commit and push if auto_push is enabled (unless --no-push)
    // If auto_push is false, the file is saved locally and visible to local agents.
    // Use `brd sync` to batch-commit when ready.
    if !no_push && config.auto_push {
        if config.is_issues_branch_mode() {
            commit_and_push_issues_branch(paths, &config, &full_id, cli)?;
        } else {
            commit_and_push_main(paths, &full_id, cli)?;
        }
    }

    // Output
    if cli.json {
        let issue = issues.get(&full_id).unwrap();
        let json = issue_to_json(issue, &issues);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Started: {} (owner: {})", full_id, agent_id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::Priority;
    use crate::test_utils::{TestRepo, test_cli};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_start_sets_status_and_owner() {
        let repo = TestRepo::new().with_agent("tester").build();
        repo.issue("brd-aaaa").create();

        cmd_start(
            &test_cli(),
            &repo.paths,
            Some("brd-aaaa"),
            false,
            true,
            true,
            false,
        )
        .unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.get("brd-aaaa").unwrap();
        assert_eq!(issue.status(), Status::Doing);
        assert_eq!(issue.frontmatter.owner.as_deref(), Some("tester"));
    }

    #[test]
    fn test_start_auto_picks_non_meta() {
        let repo = TestRepo::new().with_agent("tester").build();
        repo.issue("brd-meta")
            .priority(Priority::P0)
            .issue_type(IssueType::Meta)
            .create();
        repo.issue("brd-work").priority(Priority::P1).create();

        cmd_start(&test_cli(), &repo.paths, None, false, true, true, false).unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let work = issues.get("brd-work").unwrap();
        let meta = issues.get("brd-meta").unwrap();
        assert_eq!(work.status(), Status::Doing);
        assert_eq!(work.frontmatter.owner.as_deref(), Some("tester"));
        assert_eq!(meta.status(), Status::Open);
    }

    #[test]
    fn test_start_requires_force_for_doing_issue() {
        let repo = TestRepo::new().with_agent("tester").build();
        repo.issue("brd-aaaa")
            .status(Status::Doing)
            .owner("someone")
            .create();

        let err = cmd_start(
            &test_cli(),
            &repo.paths,
            Some("brd-aaaa"),
            false,
            true,
            true,
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("already being worked on"));

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(
            issues.get("brd-aaaa").unwrap().frontmatter.owner.as_deref(),
            Some("someone")
        );
    }

    #[test]
    fn test_start_force_reassigns_owner() {
        let repo = TestRepo::new().with_agent("tester").build();
        repo.issue("brd-aaaa")
            .status(Status::Doing)
            .owner("someone")
            .create();

        cmd_start(
            &test_cli(),
            &repo.paths,
            Some("brd-aaaa"),
            true,
            true,
            true,
            false,
        )
        .unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(
            issues.get("brd-aaaa").unwrap().frontmatter.owner.as_deref(),
            Some("tester")
        );
    }

    #[test]
    fn test_start_ambiguous_id() {
        let repo = TestRepo::new().with_agent("tester").build();
        repo.issue("brd-aaaa").create();
        repo.issue("brd-aaab").create();

        let err = cmd_start(
            &test_cli(),
            &repo.paths,
            Some("aaa"),
            false,
            true,
            true,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, BrdError::AmbiguousId(_, _)));
    }

    #[test]
    fn test_start_issue_not_found() {
        let repo = TestRepo::new().with_agent("tester").build();
        let err = cmd_start(
            &test_cli(),
            &repo.paths,
            Some("brd-missing"),
            false,
            true,
            true,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }

    // Git-specific tests need actual git repos
    fn create_git_repo() -> (tempfile::TempDir, RepoPaths) {
        let dir = tempdir().unwrap();
        git::test::run_ok(dir.path(), &["init"]);
        git::test::run_ok(dir.path(), &["config", "user.email", "test@test.com"]);
        git::test::run_ok(dir.path(), &["config", "user.name", "test"]);
        git::test::run_ok(dir.path(), &["config", "commit.gpgsign", "false"]);
        fs::write(dir.path().join("README.md"), "test\n").unwrap();
        git::test::run_ok(dir.path(), &["add", "."]);
        git::test::run_ok(dir.path(), &["commit", "-m", "init"]);

        let paths = RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };
        (dir, paths)
    }

    #[test]
    fn test_sync_with_main_no_origin_skips() {
        let (_dir, paths) = create_git_repo();
        let cli = test_cli();

        // No origin remote - should skip sync without error
        let result = sync_with_main(&paths, &cli, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sync_with_main_dirty_without_stash_shows_options() {
        let (_dir, paths) = create_git_repo();
        let cli = test_cli();

        // Add an origin remote
        git::test::run_ok(
            &paths.worktree_root,
            &["remote", "add", "origin", "https://example.com/repo.git"],
        );

        // Create uncommitted changes outside .braid
        fs::write(paths.worktree_root.join("dirty.txt"), "uncommitted").unwrap();

        // Without --stash, should error with helpful message
        let err = sync_with_main(&paths, &cli, false).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("uncommitted changes"));
        assert!(msg.contains("--stash"));
        assert!(msg.contains("--no-sync"));
    }

    #[test]
    fn test_sync_with_main_braid_changes_allowed() {
        let (_dir, paths) = create_git_repo();
        let cli = test_cli();

        // Create .braid directory with uncommitted changes
        fs::create_dir_all(paths.worktree_root.join(".braid")).unwrap();
        fs::write(paths.worktree_root.join(".braid/test.md"), "braid change").unwrap();

        // No origin - should skip but not error on .braid changes
        let result = sync_with_main(&paths, &cli, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sync_with_main_stash_flag_preserves_changes() {
        let (_dir, paths) = create_git_repo();
        let cli = test_cli();

        // Add origin remote (fetch will fail but stash should still work)
        git::test::run_ok(
            &paths.worktree_root,
            &["remote", "add", "origin", "https://example.com/repo.git"],
        );

        // Create uncommitted changes
        let dirty_file = paths.worktree_root.join("dirty.txt");
        fs::write(&dirty_file, "uncommitted content").unwrap();

        // With --stash, should stash, attempt sync, then restore
        // (fetch will fail since origin doesn't exist, but changes should be preserved)
        let result = sync_with_main(&paths, &cli, true);
        assert!(result.is_ok());

        // Changes should still be there after sync attempt
        assert!(dirty_file.exists());
        assert_eq!(
            fs::read_to_string(&dirty_file).unwrap(),
            "uncommitted content"
        );
    }
}
