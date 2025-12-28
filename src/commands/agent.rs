//! brd agent commands.

use std::path::PathBuf;
use std::process::Command;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::lock::LockGuard;
use crate::repo::{self, RepoPaths};

use super::{
    claim_issue, commit_and_push_main, commit_and_push_sync_branch, has_origin, has_origin_main,
    is_clean, load_all_issues, resolve_issue_id,
};

pub fn cmd_agent_init(cli: &Cli, paths: &RepoPaths, name: &str, base: Option<&str>) -> Result<()> {
    // validate agent name (alphanumeric + hyphens)
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(BrdError::Other(format!(
            "invalid agent name '{}': use only alphanumeric, hyphens, underscores",
            name
        )));
    }

    // determine worktree path (~/.braid/worktrees/<repo-name>/<agent-name>)
    let home_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| BrdError::Other("cannot determine home directory".to_string()))?;
    let repo_name = paths
        .worktree_root
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| BrdError::Other("cannot determine repo name".to_string()))?;
    let worktrees_dir = home_dir.join(".braid").join("worktrees").join(repo_name);
    let worktree_path = worktrees_dir.join(name);

    // ensure parent directories exist
    std::fs::create_dir_all(&worktrees_dir)?;

    // check if worktree already exists
    if worktree_path.exists() {
        return Err(BrdError::Other(format!(
            "directory already exists: {}",
            worktree_path.display()
        )));
    }

    // get base branch (default to current branch)
    let base_branch = match base {
        Some(b) => b.to_string(),
        None => {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&paths.worktree_root)
                .output()?;
            if !output.status.success() {
                return Err(BrdError::Other("failed to get current branch".to_string()));
            }
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
    };

    // create worktree with new branch
    let output = std::process::Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            name,
            worktree_path.to_str().unwrap(),
            &base_branch,
        ])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BrdError::Other(format!(
            "failed to create worktree: {}",
            stderr.trim()
        )));
    }

    // create .braid directory in new worktree (for agent.toml)
    let new_braid_dir = worktree_path.join(".braid");
    std::fs::create_dir_all(&new_braid_dir)?;

    // create agent.toml
    let agent_toml_path = new_braid_dir.join("agent.toml");
    let agent_toml_content = format!("agent_id = \"{}\"\n", name);
    std::fs::write(&agent_toml_path, agent_toml_content)?;

    // check for sync branch mode and ensure issues worktree exists
    let config = Config::load(&paths.config_path()).ok();
    let sync_branch = config.as_ref().and_then(|c| c.sync_branch.clone());

    if let Some(ref branch) = sync_branch {
        // ensure the shared issues worktree exists
        if let Err(e) = paths.ensure_issues_worktree(branch) {
            eprintln!("warning: failed to ensure issues worktree: {}", e);
        }
    }

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "agent_id": name,
            "worktree": worktree_path.to_string_lossy(),
            "branch": name,
            "base": base_branch,
            "sync_branch": sync_branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Created agent worktree: {}", name);
        println!("  path:   {}", worktree_path.display());
        println!("  branch: {} (from {})", name, base_branch);
        if let Some(sb) = &sync_branch {
            println!("  sync:   issues on '{}' branch", sb);
        }
        println!();
        println!("To use this agent:");
        println!("  cd {}", worktree_path.display());
        println!("  # start your agent (claude, codex, gemini, etc.)");
        println!("  brd next  # get next issue to work on");
        if sync_branch.is_some() {
            println!("  brd sync  # sync issues with remote");
        }
    }

    Ok(())
}

/// run a git command and return success status.
fn git(args: &[&str], cwd: &std::path::Path) -> std::io::Result<bool> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    Ok(output.status.success())
}

/// create a feature branch for PR workflow.
/// this command:
/// 1. syncs with main and claims the issue (pushing to main so other agents see it)
/// 2. creates a feature branch for working on the issue
pub fn cmd_agent_branch(cli: &Cli, paths: &RepoPaths, issue_id: &str) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let is_sync_mode = config.is_sync_branch_mode();
    let agent_id = repo::get_agent_id(&paths.worktree_root);

    // Step 1: Check for clean working tree
    if !is_clean(&paths.worktree_root)? {
        return Err(BrdError::Other(
            "working tree has uncommitted changes - commit or stash first".to_string(),
        ));
    }

    // Load issues and resolve the ID
    let _lock = LockGuard::acquire(&paths.lock_path())?;
    let mut issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(issue_id, &issues)?;

    // create branch name: pr/<agent>/<issue-id>
    let branch_name = format!("pr/{}/{}", agent_id, full_id);

    // Check if branch already exists
    if git(
        &["rev-parse", "--verify", &branch_name],
        &paths.worktree_root,
    )
    .unwrap_or(false)
    {
        return Err(BrdError::Other(format!(
            "branch '{}' already exists",
            branch_name
        )));
    }

    if is_sync_mode {
        // Local-sync mode: claim in shared worktree, then create feature branch
        if !cli.json {
            eprintln!("claiming {} in sync branch...", full_id);
        }

        // Claim the issue
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(full_id.clone()))?;
        claim_issue(paths, &config, issue, &agent_id, false)?;

        // Commit in sync branch worktree
        commit_and_push_sync_branch(paths, &config, &full_id, cli)?;

        // Create feature branch from current HEAD
        if !cli.json {
            eprintln!("creating branch {}...", branch_name);
        }
        let output = Command::new("git")
            .args(["checkout", "-b", &branch_name])
            .current_dir(&paths.worktree_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrdError::Other(format!(
                "failed to create branch: {}",
                stderr.trim()
            )));
        }
    } else {
        // Git-native mode: sync with main, claim, push to main, then create feature branch

        // Step 2: Fetch and get current branch
        if !cli.json {
            eprintln!("syncing with origin/main...");
        }

        if has_origin(&paths.worktree_root) {
            let _ = git(&["fetch", "origin", "main"], &paths.worktree_root);
        }

        // Get current branch
        let current_branch = get_current_branch(&paths.worktree_root)?;

        // If not on main, checkout main
        if current_branch != "main" {
            if !cli.json {
                eprintln!("switching to main...");
            }
            if !git(&["checkout", "main"], &paths.worktree_root)? {
                return Err(BrdError::Other("failed to checkout main".to_string()));
            }
        }

        // Rebase on origin/main if it exists
        if has_origin_main(&paths.worktree_root)
            && !git(&["rebase", "origin/main"], &paths.worktree_root)?
        {
            let _ = git(&["rebase", "--abort"], &paths.worktree_root);
            return Err(BrdError::Other(
                "rebase failed - resolve conflicts manually".to_string(),
            ));
        }

        // Reload issues after sync (they may have changed)
        let mut issues = load_all_issues(paths, &config)?;

        // Step 3: Claim the issue
        if !cli.json {
            eprintln!("claiming {}...", full_id);
        }

        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(full_id.clone()))?;
        claim_issue(paths, &config, issue, &agent_id, false)?;

        // Step 4: Commit and push to main
        commit_and_push_main(paths, &full_id, cli)?;

        // Step 5: Create feature branch from main
        if !cli.json {
            eprintln!("creating branch {}...", branch_name);
        }

        let output = Command::new("git")
            .args(["checkout", "-b", &branch_name])
            .current_dir(&paths.worktree_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrdError::Other(format!(
                "failed to create branch: {}",
                stderr.trim()
            )));
        }
    }

    // Output success
    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "branch": branch_name,
            "issue_id": full_id,
            "owner": agent_id,
            "mode": if is_sync_mode { "local-sync" } else { "git-native" },
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("claimed: {} (owner: {})", full_id, agent_id);
        println!("branch:  {}", branch_name);
        println!();
        println!("the claim has been pushed to main - other agents will see it.");
        println!();
        println!("next steps:");
        println!("  # do the work, commit as usual");
        println!("  brd done {}    # mark done", full_id);
        println!("  brd agent pr   # create PR");
    }

    Ok(())
}

/// get the current branch name.
fn get_current_branch(cwd: &std::path::Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(BrdError::Other("failed to get current branch".to_string()))
    }
}

/// extract issue ID from branch name.
/// supports both formats:
/// - "pr/<agent>/<issue-id>" (new format)
/// - "<agent>/<issue-id>" (legacy format for backwards compatibility)
fn extract_issue_id_from_branch(branch: &str) -> Option<&str> {
    if branch.starts_with("pr/") {
        branch.split('/').nth(2)
    } else {
        branch.split('/').nth(1)
    }
}

/// create a PR from the current branch using gh cli.
pub fn cmd_agent_pr(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let config = Config::load(&paths.config_path())?;

    // get current branch
    let branch = get_current_branch(&paths.worktree_root)?;

    // extract issue ID from branch name
    let issue_id = extract_issue_id_from_branch(&branch).ok_or_else(|| {
        BrdError::Other(format!(
            "branch '{}' doesn't match expected format 'pr/<agent>/<issue-id>'",
            branch
        ))
    })?;

    // load the issue
    let issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(issue_id, &issues)?;
    let issue = issues
        .get(&full_id)
        .ok_or_else(|| BrdError::Other(format!("issue '{}' not found", full_id)))?;

    // generate PR title and body
    let title = format!("feat: {} ({})", issue.title(), full_id);
    let body = if issue.body.is_empty() {
        format!("Closes: {}", full_id)
    } else {
        format!("{}\n\nCloses: {}", issue.body.trim(), full_id)
    };

    // push the branch first
    if !cli.json {
        eprintln!("pushing branch to origin...");
    }
    let push_output = Command::new("git")
        .args(["push", "-u", "origin", &branch])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        // ignore "already up to date" type messages
        if !stderr.contains("Everything up-to-date") && !stderr.contains("set up to track") {
            return Err(BrdError::Other(format!(
                "failed to push branch: {}",
                stderr.trim()
            )));
        }
    }

    // create PR using gh cli
    if !cli.json {
        eprintln!("creating PR...");
    }
    let pr_output = Command::new("gh")
        .args([
            "pr", "create", "--title", &title, "--body", &body, "--base", "main",
        ])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !pr_output.status.success() {
        let stderr = String::from_utf8_lossy(&pr_output.stderr);
        return Err(BrdError::Other(format!(
            "failed to create PR: {}",
            stderr.trim()
        )));
    }

    let pr_url = String::from_utf8_lossy(&pr_output.stdout)
        .trim()
        .to_string();

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "pr_url": pr_url,
            "title": title,
            "issue_id": full_id,
            "branch": branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("created PR: {}", pr_url);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use tempfile::tempdir;

    static AGENT_INIT_TEST_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            unsafe { std::env::set_var(key, value) };
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(val) => unsafe { std::env::set_var(self.key, val) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    fn git_ok(repo: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_output(repo: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn create_repo() -> (tempfile::TempDir, PathBuf, RepoPaths, String) {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join("repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        git_ok(&repo_path, &["init"]);
        git_ok(&repo_path, &["config", "user.email", "test@test.com"]);
        git_ok(&repo_path, &["config", "user.name", "test user"]);
        std::fs::write(repo_path.join("README.md"), "test\n").unwrap();
        git_ok(&repo_path, &["add", "."]);
        git_ok(&repo_path, &["commit", "-m", "init"]);

        let base_branch = git_output(&repo_path, &["rev-parse", "--abbrev-ref", "HEAD"]);

        let paths = RepoPaths {
            worktree_root: repo_path.clone(),
            git_common_dir: repo_path.join(".git"),
            brd_common_dir: repo_path.join(".git/brd"),
        };
        std::fs::create_dir_all(&paths.brd_common_dir).unwrap();

        (dir, repo_path, paths, base_branch)
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
    fn test_agent_init_creates_worktree_and_branch() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let cli = make_cli();
        cmd_agent_init(&cli, &paths, "agent-one", Some(&base_branch)).unwrap();

        let worktree_path = home_dir
            .path()
            .join(".braid")
            .join("worktrees")
            .join("repo")
            .join("agent-one");
        assert!(worktree_path.exists());
        assert!(worktree_path.join(".braid/agent.toml").exists());

        let agent_toml = std::fs::read_to_string(worktree_path.join(".braid/agent.toml")).unwrap();
        assert!(agent_toml.contains("agent_id = \"agent-one\""));

        let branch_list = git_output(&repo_path, &["branch", "--list", "agent-one"]);
        assert!(branch_list.contains("agent-one"));
    }

    #[test]
    fn test_agent_init_rejects_invalid_name() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, _repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let cli = make_cli();
        let err = cmd_agent_init(&cli, &paths, "bad name", Some(&base_branch)).unwrap_err();
        assert!(err.to_string().contains("invalid agent name"));
    }

    #[test]
    fn test_agent_init_rejects_duplicate_name() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, _repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let worktree_path = home_dir
            .path()
            .join(".braid")
            .join("worktrees")
            .join("repo")
            .join("agent-one");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let cli = make_cli();
        let err = cmd_agent_init(&cli, &paths, "agent-one", Some(&base_branch)).unwrap_err();
        assert!(err.to_string().contains("directory already exists"));
    }
}
