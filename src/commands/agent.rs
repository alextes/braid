//! brd agent commands.

use std::path::PathBuf;
use std::process::Command;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::repo::{self, RepoPaths};

use super::{load_all_issues, resolve_issue_id};

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
pub fn cmd_agent_branch(cli: &Cli, paths: &RepoPaths, issue_id: &str) -> Result<()> {
    let config = Config::load(&paths.config_path())?;

    // get agent name
    let agent_id = repo::get_agent_id(&paths.worktree_root);

    // load issues and resolve the ID
    let issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(issue_id, &issues)?;

    // create branch name: <agent>/<issue-id>
    let branch_name = format!("{}/{}", agent_id, full_id);

    // check if branch already exists
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

    // fetch latest main
    if !cli.json {
        eprintln!("fetching origin/main...");
    }
    let _ = git(&["fetch", "origin", "main"], &paths.worktree_root);

    // create branch from origin/main (or main if no remote)
    let base = if git(
        &["rev-parse", "--verify", "origin/main"],
        &paths.worktree_root,
    )
    .unwrap_or(false)
    {
        "origin/main"
    } else {
        "main"
    };

    if !cli.json {
        eprintln!("creating branch {} from {}...", branch_name, base);
    }

    let output = Command::new("git")
        .args(["checkout", "-b", &branch_name, base])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BrdError::Other(format!(
            "failed to create branch: {}",
            stderr.trim()
        )));
    }

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "branch": branch_name,
            "issue_id": full_id,
            "base": base,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("created branch: {}", branch_name);
        println!();
        println!("next steps:");
        println!("  brd start {}   # claim the issue", full_id);
        println!("  # do the work, commit as usual");
        println!("  brd done {}    # mark done", full_id);
        println!("  brd agent pr   # create PR");
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
