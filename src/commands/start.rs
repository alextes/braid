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
    issue.touch();

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
            eprintln!("  consider running `brd agent ship` first");
        }
        Ok(Some(unshipped))
    }
}

/// Fetch and rebase onto origin/main.
pub fn sync_with_main(paths: &RepoPaths, cli: &Cli) -> Result<()> {
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

    // Check for clean working tree (outside .braid)
    if !git::is_clean(&paths.worktree_root)? {
        // Check if only .braid changes
        let status = git::output(&["status", "--porcelain"], &paths.worktree_root)?;
        let non_braid_changes = status.lines().any(|line| !line.contains(".braid/"));
        if non_braid_changes {
            return Err(BrdError::Other(
                "working tree has uncommitted changes outside .braid - commit or stash first"
                    .to_string(),
            ));
        }
    }

    // Fetch
    if !git::run(&["fetch", "origin", "main"], &paths.worktree_root)? {
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
        return Err(BrdError::Other(
            "rebase failed - resolve conflicts manually or use --no-sync".to_string(),
        ));
    }

    Ok(())
}

/// Commit and push the claim to main with retry logic.
pub fn commit_and_push_main(paths: &RepoPaths, issue_id: &str, cli: &Cli) -> Result<()> {
    // Commit
    if !git::run(&["add", ".braid"], &paths.worktree_root)? {
        return Err(BrdError::Other("failed to stage .braid".to_string()));
    }

    let commit_msg = format!("start: {}", issue_id);
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

    let commit_msg = format!("start: {}", issue_id);
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
) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let is_sync_mode = config.is_issues_branch_mode();

    // Step 1: Pre-flight check for unshipped done issues (git-native mode only)
    if !no_sync && !is_sync_mode {
        // Try to check, but don't fail if origin/main doesn't exist
        let _ = check_unshipped_done_issues(paths, &config, cli);
    }

    // Step 2: Sync with main (git-native mode only)
    if !no_sync && !is_sync_mode {
        sync_with_main(paths, cli)?;
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

    // Step 4: Commit and push (unless --no-push)
    if !no_push {
        if is_sync_mode {
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
    use std::fs;
    use tempfile::tempdir;

    fn create_test_repo() -> (tempfile::TempDir, RepoPaths, Config) {
        let dir = tempdir().unwrap();
        let paths = RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };
        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        fs::create_dir_all(paths.braid_dir().join("issues")).unwrap();
        let config = Config::default();
        config.save(&paths.config_path()).unwrap();
        fs::write(
            paths.braid_dir().join("agent.toml"),
            "agent_id = \"tester\"\n",
        )
        .unwrap();
        (dir, paths, config)
    }

    fn write_issue(
        paths: &RepoPaths,
        config: &Config,
        id: &str,
        priority: crate::issue::Priority,
        status: Status,
        issue_type: Option<IssueType>,
        owner: Option<&str>,
    ) {
        let mut issue = Issue::new(id.to_string(), format!("issue {}", id), priority, vec![]);
        issue.frontmatter.status = status;
        issue.frontmatter.issue_type = issue_type;
        issue.frontmatter.owner = owner.map(|o| o.to_string());
        let issue_path = paths.issues_dir(config).join(format!("{}.md", id));
        issue.save(&issue_path).unwrap();
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
    fn test_start_sets_status_and_owner() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(
            &paths,
            &config,
            "brd-aaaa",
            crate::issue::Priority::P2,
            Status::Todo,
            None,
            None,
        );

        let cli = make_cli();
        cmd_start(&cli, &paths, Some("brd-aaaa"), false, true, true).unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-aaaa").unwrap();
        assert_eq!(issue.status(), Status::Doing);
        assert_eq!(issue.frontmatter.owner.as_deref(), Some("tester"));
    }

    #[test]
    fn test_start_auto_picks_non_meta() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(
            &paths,
            &config,
            "brd-meta",
            crate::issue::Priority::P0,
            Status::Todo,
            Some(IssueType::Meta),
            None,
        );
        write_issue(
            &paths,
            &config,
            "brd-work",
            crate::issue::Priority::P1,
            Status::Todo,
            None,
            None,
        );

        let cli = make_cli();
        cmd_start(&cli, &paths, None, false, true, true).unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let work = issues.get("brd-work").unwrap();
        let meta = issues.get("brd-meta").unwrap();
        assert_eq!(work.status(), Status::Doing);
        assert_eq!(work.frontmatter.owner.as_deref(), Some("tester"));
        assert_eq!(meta.status(), Status::Todo);
    }

    #[test]
    fn test_start_requires_force_for_doing_issue() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(
            &paths,
            &config,
            "brd-aaaa",
            crate::issue::Priority::P2,
            Status::Doing,
            None,
            Some("someone"),
        );

        let cli = make_cli();
        let err = cmd_start(&cli, &paths, Some("brd-aaaa"), false, true, true).unwrap_err();
        assert!(err.to_string().contains("already being worked on"));

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-aaaa").unwrap();
        assert_eq!(issue.frontmatter.owner.as_deref(), Some("someone"));
    }

    #[test]
    fn test_start_force_reassigns_owner() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(
            &paths,
            &config,
            "brd-aaaa",
            crate::issue::Priority::P2,
            Status::Doing,
            None,
            Some("someone"),
        );

        let cli = make_cli();
        cmd_start(&cli, &paths, Some("brd-aaaa"), true, true, true).unwrap();

        let issues = load_all_issues(&paths, &config).unwrap();
        let issue = issues.get("brd-aaaa").unwrap();
        assert_eq!(issue.frontmatter.owner.as_deref(), Some("tester"));
    }

    #[test]
    fn test_start_ambiguous_id() {
        let (_dir, paths, config) = create_test_repo();
        write_issue(
            &paths,
            &config,
            "brd-aaaa",
            crate::issue::Priority::P2,
            Status::Todo,
            None,
            None,
        );
        write_issue(
            &paths,
            &config,
            "brd-aaab",
            crate::issue::Priority::P2,
            Status::Todo,
            None,
            None,
        );

        let cli = make_cli();
        let err = cmd_start(&cli, &paths, Some("aaa"), false, true, true).unwrap_err();
        assert!(matches!(err, BrdError::AmbiguousId(_, _)));
    }

    #[test]
    fn test_start_issue_not_found() {
        let (_dir, paths, _config) = create_test_repo();
        let cli = make_cli();
        let err = cmd_start(&cli, &paths, Some("brd-missing"), false, true, true).unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }
}
