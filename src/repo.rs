//! git repository discovery and control root resolution.

use std::path::PathBuf;

use crate::error::{BrdError, Result};

/// paths discovered from git for a brd repository.
#[derive(Debug, Clone)]
pub struct RepoPaths {
    /// the worktree root (from `git rev-parse --show-toplevel`)
    pub worktree_root: PathBuf,
    /// the git common directory (from `git rev-parse --git-common-dir`)
    pub git_common_dir: PathBuf,
    /// the control root where `.braid/` lives
    pub control_root: PathBuf,
    /// the brd directory inside git common dir (`<git-common-dir>/brd/`)
    pub brd_common_dir: PathBuf,
}

impl RepoPaths {
    /// path to `.braid/` in the control root
    pub fn braid_dir(&self) -> PathBuf {
        self.control_root.join(".braid")
    }

    /// path to `.braid/issues/`
    pub fn issues_dir(&self) -> PathBuf {
        self.braid_dir().join("issues")
    }

    /// path to `.braid/config.toml`
    pub fn config_path(&self) -> PathBuf {
        self.braid_dir().join("config.toml")
    }

    /// path to the global lock file
    pub fn lock_path(&self) -> PathBuf {
        self.brd_common_dir.join("lock")
    }

    /// path to the control_root file in git common dir
    pub fn control_root_file(&self) -> PathBuf {
        self.brd_common_dir.join("control_root")
    }

    /// path to claims directory
    pub fn claims_dir(&self) -> PathBuf {
        self.brd_common_dir.join("claims")
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

    // resolve control root (per spec section 5.3)
    let control_root = resolve_control_root(&worktree_root, &brd_common_dir)?;

    Ok(RepoPaths {
        worktree_root,
        git_common_dir,
        control_root,
        brd_common_dir,
    })
}

/// run `git rev-parse <arg>` and return the result as a PathBuf.
fn git_rev_parse(cwd: &std::path::Path, arg: &str) -> Result<PathBuf> {
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

/// resolve the control root per spec section 5.3:
/// 1. BRD_CONTROL_ROOT env var
/// 2. <git-common-dir>/brd/control_root file
/// 3. fallback to current worktree root (with warning)
fn resolve_control_root(worktree_root: &PathBuf, brd_common_dir: &PathBuf) -> Result<PathBuf> {
    // 1. check env var
    if let Ok(env_root) = std::env::var("BRD_CONTROL_ROOT") {
        let path = PathBuf::from(&env_root);
        let resolved = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(path)
        };
        return Ok(resolved.canonicalize().unwrap_or(resolved));
    }

    // 2. check control_root file
    let control_root_file = brd_common_dir.join("control_root");
    if control_root_file.exists() {
        let content = std::fs::read_to_string(&control_root_file)?;
        let path = PathBuf::from(content.trim());
        return Ok(path);
    }

    // 3. fallback to current worktree root
    // note: in a real implementation, we'd emit a warning here
    Ok(worktree_root.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_paths_methods() {
        let paths = RepoPaths {
            worktree_root: PathBuf::from("/repo"),
            git_common_dir: PathBuf::from("/repo/.git"),
            control_root: PathBuf::from("/repo"),
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
