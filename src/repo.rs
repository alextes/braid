//! git repository discovery.

use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::{BrdError, Result};

/// paths discovered from git for a brd repository.
#[derive(Debug, Clone)]
pub struct RepoPaths {
    /// the worktree root (from `git rev-parse --show-toplevel`)
    pub worktree_root: PathBuf,
    /// the git common directory (from `git rev-parse --git-common-dir`)
    pub git_common_dir: PathBuf,
    /// the brd directory inside git common dir (`<git-common-dir>/brd/`)
    pub brd_common_dir: PathBuf,
}

impl RepoPaths {
    /// path to `.braid/` in the current worktree
    pub fn braid_dir(&self) -> PathBuf {
        self.worktree_root.join(".braid")
    }

    /// path to `.braid/issues/` in the current worktree (default mode)
    pub fn local_issues_dir(&self) -> PathBuf {
        self.braid_dir().join("issues")
    }

    /// path to the shared issues worktree directory (sync branch mode)
    pub fn issues_worktree_dir(&self) -> PathBuf {
        self.brd_common_dir.join("issues")
    }

    /// get the issues directory based on config mode.
    /// - default mode: `.braid/issues/` in current worktree
    /// - sync branch mode: `<git-common-dir>/brd/issues/.braid/issues/`
    pub fn issues_dir(&self, config: &Config) -> PathBuf {
        if config.is_sync_branch_mode() {
            self.issues_worktree_dir().join(".braid").join("issues")
        } else {
            self.local_issues_dir()
        }
    }

    /// get the config path based on mode.
    /// - default mode: `.braid/config.toml` in current worktree
    /// - sync branch mode: config from issues worktree
    pub fn resolved_config_path(&self, local_config: &Config) -> PathBuf {
        if local_config.is_sync_branch_mode() {
            self.issues_worktree_dir()
                .join(".braid")
                .join("config.toml")
        } else {
            self.config_path()
        }
    }

    /// ensure the issues worktree exists for sync branch mode.
    /// creates it if it doesn't exist.
    pub fn ensure_issues_worktree(&self, branch: &str) -> Result<PathBuf> {
        let wt_path = self.issues_worktree_dir();

        if wt_path.exists() {
            // worktree already exists, verify it's on the right branch
            let current = git_rev_parse(&wt_path, "--abbrev-ref HEAD")?;
            let current_str = current.to_string_lossy();
            if current_str.trim() != branch {
                return Err(BrdError::Other(format!(
                    "issues worktree exists but is on branch '{}', expected '{}'",
                    current_str.trim(),
                    branch
                )));
            }
            return Ok(wt_path);
        }

        // create the worktree
        let output = std::process::Command::new("git")
            .args(["worktree", "add", "--detach"])
            .arg(&wt_path)
            .arg(branch)
            .current_dir(&self.worktree_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrdError::Other(format!(
                "failed to create issues worktree: {}",
                stderr
            )));
        }

        // checkout the branch (worktree was created detached, now attach to branch)
        let output = std::process::Command::new("git")
            .args(["checkout", branch])
            .current_dir(&wt_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrdError::Other(format!(
                "failed to checkout branch in issues worktree: {}",
                stderr
            )));
        }

        Ok(wt_path)
    }

    /// path to `.braid/config.toml`
    pub fn config_path(&self) -> PathBuf {
        self.braid_dir().join("config.toml")
    }

    /// path to the local lock file (for single-machine coordination)
    pub fn lock_path(&self) -> PathBuf {
        self.brd_common_dir.join("lock")
    }
}

/// get the current agent ID:
/// 1. BRD_AGENT_ID env var
/// 2. .braid/agent.toml in worktree
/// 3. fallback: $USER
pub fn get_agent_id(worktree_root: &Path) -> String {
    // 1. check env var
    if let Ok(id) = std::env::var("BRD_AGENT_ID") {
        return id;
    }

    // 2. check .braid/agent.toml
    let agent_toml = worktree_root.join(".braid/agent.toml");
    if let Ok(content) = std::fs::read_to_string(&agent_toml)
        && let Ok(parsed) = toml::from_str::<toml::Value>(&content)
        && let Some(id) = parsed.get("agent_id").and_then(|v| v.as_str())
    {
        return id.to_string();
    }

    // 3. fallback to $USER
    match std::env::var("USER") {
        Ok(user) => user,
        Err(_) => {
            eprintln!("warning: $USER not set, using 'default-user' as agent_id");
            "default-user".to_string()
        }
    }
}

/// discover repository paths from the current directory or a given path.
pub fn discover(from: Option<&std::path::Path>) -> Result<RepoPaths> {
    let cwd = match from {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir()?,
    };

    // get worktree root
    let worktree_root = git_rev_parse(&cwd, "--show-toplevel")?;

    // get git common dir
    let git_common_dir_str = git_rev_parse(&cwd, "--git-common-dir")?;
    // git-common-dir can be relative, so resolve it
    let git_common_dir = if git_common_dir_str.is_absolute() {
        git_common_dir_str
    } else {
        cwd.join(&git_common_dir_str)
            .canonicalize()
            .unwrap_or(git_common_dir_str)
    };

    let brd_common_dir = git_common_dir.join("brd");

    Ok(RepoPaths {
        worktree_root,
        git_common_dir,
        brd_common_dir,
    })
}

/// run `git rev-parse <args>` and return the result as a PathBuf.
pub fn git_rev_parse(cwd: &std::path::Path, args: &str) -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .args(args.split_whitespace())
        .current_dir(cwd)
        .output()?;

    if !output.status.success() {
        return Err(BrdError::NotGitRepo);
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path_str))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

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

    fn create_git_repo() -> (tempfile::TempDir, RepoPaths, String) {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();
        git_ok(repo_path, &["init"]);
        git_ok(repo_path, &["config", "user.email", "test@test.com"]);
        git_ok(repo_path, &["config", "user.name", "test user"]);
        git_ok(repo_path, &["config", "commit.gpgsign", "false"]);
        std::fs::write(repo_path.join("README.md"), "test\n").unwrap();
        git_ok(repo_path, &["add", "."]);
        git_ok(repo_path, &["commit", "-m", "init"]);

        let branch = git_output(repo_path, &["rev-parse", "--abbrev-ref", "HEAD"]);

        let paths = RepoPaths {
            worktree_root: repo_path.to_path_buf(),
            git_common_dir: repo_path.join(".git"),
            brd_common_dir: repo_path.join(".git/brd"),
        };
        std::fs::create_dir_all(&paths.brd_common_dir).unwrap();

        (dir, paths, branch)
    }

    #[test]
    fn test_repo_paths_methods() {
        let paths = RepoPaths {
            worktree_root: PathBuf::from("/repo"),
            git_common_dir: PathBuf::from("/repo/.git"),
            brd_common_dir: PathBuf::from("/repo/.git/brd"),
        };

        assert_eq!(paths.braid_dir(), PathBuf::from("/repo/.braid"));
        assert_eq!(
            paths.local_issues_dir(),
            PathBuf::from("/repo/.braid/issues")
        );
        assert_eq!(
            paths.config_path(),
            PathBuf::from("/repo/.braid/config.toml")
        );
        assert_eq!(paths.lock_path(), PathBuf::from("/repo/.git/brd/lock"));
    }

    #[test]
    fn test_issues_dir_default_mode() {
        let paths = RepoPaths {
            worktree_root: PathBuf::from("/repo"),
            git_common_dir: PathBuf::from("/repo/.git"),
            brd_common_dir: PathBuf::from("/repo/.git/brd"),
        };
        let config = Config::default();

        assert_eq!(
            paths.issues_dir(&config),
            PathBuf::from("/repo/.braid/issues")
        );
    }

    #[test]
    fn test_issues_dir_sync_branch_mode() {
        let paths = RepoPaths {
            worktree_root: PathBuf::from("/repo"),
            git_common_dir: PathBuf::from("/repo/.git"),
            brd_common_dir: PathBuf::from("/repo/.git/brd"),
        };
        let config = Config {
            sync_branch: Some("braid-issues".to_string()),
            ..Default::default()
        };

        assert_eq!(
            paths.issues_dir(&config),
            PathBuf::from("/repo/.git/brd/issues/.braid/issues")
        );
    }

    #[test]
    fn test_issues_worktree_dir() {
        let paths = RepoPaths {
            worktree_root: PathBuf::from("/repo"),
            git_common_dir: PathBuf::from("/repo/.git"),
            brd_common_dir: PathBuf::from("/repo/.git/brd"),
        };

        assert_eq!(
            paths.issues_worktree_dir(),
            PathBuf::from("/repo/.git/brd/issues")
        );
    }

    #[test]
    fn test_resolved_config_path_default_mode() {
        let paths = RepoPaths {
            worktree_root: PathBuf::from("/repo"),
            git_common_dir: PathBuf::from("/repo/.git"),
            brd_common_dir: PathBuf::from("/repo/.git/brd"),
        };
        let config = Config::default();

        assert_eq!(paths.resolved_config_path(&config), paths.config_path());
    }

    #[test]
    fn test_resolved_config_path_sync_mode() {
        let paths = RepoPaths {
            worktree_root: PathBuf::from("/repo"),
            git_common_dir: PathBuf::from("/repo/.git"),
            brd_common_dir: PathBuf::from("/repo/.git/brd"),
        };
        let config = Config {
            sync_branch: Some("braid-issues".to_string()),
            ..Default::default()
        };

        assert_eq!(
            paths.resolved_config_path(&config),
            PathBuf::from("/repo/.git/brd/issues/.braid/config.toml")
        );
    }

    #[test]
    fn test_git_rev_parse_not_repo() {
        let dir = tempdir().unwrap();
        let err = git_rev_parse(dir.path(), "--show-toplevel").unwrap_err();
        assert!(matches!(err, BrdError::NotGitRepo));
    }

    #[test]
    fn test_git_rev_parse_multiple_args() {
        // regression test: git_rev_parse must split args correctly
        // "--abbrev-ref HEAD" should become two args, not one
        let (_dir, paths, branch) = create_git_repo();
        let result = git_rev_parse(&paths.worktree_root, "--abbrev-ref HEAD").unwrap();
        assert_eq!(result.to_string_lossy(), branch);
    }

    #[test]
    fn test_ensure_issues_worktree_creates_and_attaches_branch() {
        let (_dir, paths, _branch) = create_git_repo();
        git_ok(&paths.worktree_root, &["branch", "braid-issues"]);

        let issues_wt = paths.ensure_issues_worktree("braid-issues").unwrap();
        assert!(issues_wt.exists());

        let current = git_output(&issues_wt, &["rev-parse", "--abbrev-ref", "HEAD"]);
        assert_eq!(current, "braid-issues");
    }

    #[test]
    fn test_ensure_issues_worktree_rejects_wrong_branch() {
        let (_dir, paths, _branch) = create_git_repo();
        git_ok(&paths.worktree_root, &["branch", "other"]);

        let _issues_wt = paths.ensure_issues_worktree("other").unwrap();
        let err = paths.ensure_issues_worktree("braid-issues").unwrap_err();
        assert!(
            err.to_string()
                .contains("issues worktree exists but is on branch")
        );
    }
}
