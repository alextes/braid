//! git repository discovery.

use std::path::{Path, PathBuf};

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

    /// path to `.braid/issues/`
    pub fn issues_dir(&self) -> PathBuf {
        self.braid_dir().join("issues")
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

/// run `git rev-parse <arg>` and return the result as a PathBuf.
pub fn git_rev_parse(cwd: &std::path::Path, arg: &str) -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg(arg)
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

    #[test]
    fn test_repo_paths_methods() {
        let paths = RepoPaths {
            worktree_root: PathBuf::from("/repo"),
            git_common_dir: PathBuf::from("/repo/.git"),
            brd_common_dir: PathBuf::from("/repo/.git/brd"),
        };

        assert_eq!(paths.braid_dir(), PathBuf::from("/repo/.braid"));
        assert_eq!(paths.issues_dir(), PathBuf::from("/repo/.braid/issues"));
        assert_eq!(
            paths.config_path(),
            PathBuf::from("/repo/.braid/config.toml")
        );
        assert_eq!(paths.lock_path(), PathBuf::from("/repo/.git/brd/lock"));
    }
}
