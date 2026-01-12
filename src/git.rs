//! Git command execution helpers.
//!
//! Provides a unified interface for running git commands across the codebase.

use std::path::Path;
use std::process::{Command, Output};

use crate::error::{BrdError, Result};

/// Run a git command and return whether it succeeded.
pub fn run(args: &[&str], cwd: &Path) -> Result<bool> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    Ok(output.status.success())
}

/// Run a git command and return stdout as a string.
pub fn output(args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a git command and return the full output.
pub fn run_full(args: &[&str], cwd: &Path) -> Result<Output> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    Ok(output)
}

/// Check if the working tree is clean (no uncommitted changes).
pub fn is_clean(cwd: &Path) -> Result<bool> {
    let out = output(&["status", "--porcelain"], cwd)?;
    Ok(out.is_empty())
}

/// Get the current branch name.
pub fn current_branch(cwd: &Path) -> Result<String> {
    let branch = output(&["rev-parse", "--abbrev-ref", "HEAD"], cwd)?;
    if branch.is_empty() {
        return Err(BrdError::Other("failed to get current branch".to_string()));
    }
    Ok(branch)
}

/// Check if a remote exists.
pub fn has_remote(cwd: &Path, name: &str) -> bool {
    run(&["remote", "get-url", name], cwd).unwrap_or(false)
}

/// Check if a remote branch exists.
pub fn has_remote_branch(cwd: &Path, remote: &str, branch: &str) -> bool {
    let refspec = format!("{}/{}", remote, branch);
    run(&["rev-parse", "--verify", &refspec], cwd).unwrap_or(false)
}

/// Run git rev-parse with the given arguments.
/// Arguments are split on whitespace to support multi-arg calls like "--abbrev-ref HEAD".
pub fn rev_parse(cwd: &Path, args: &str) -> Result<String> {
    let mut cmd_args = vec!["rev-parse"];
    cmd_args.extend(args.split_whitespace());

    let out = Command::new("git")
        .args(&cmd_args)
        .current_dir(cwd)
        .output()?;

    if !out.status.success() {
        return Err(BrdError::NotGitRepo);
    }

    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Check if a branch exists.
pub fn branch_exists(cwd: &Path, branch: &str) -> bool {
    run(&["rev-parse", "--verify", branch], cwd).unwrap_or(false)
}

/// Count entries in the git stash.
pub fn stash_count(cwd: &Path) -> Result<usize> {
    let out = output(&["stash", "list"], cwd)?;
    if out.is_empty() {
        Ok(0)
    } else {
        Ok(out.lines().count())
    }
}

/// Stash changes with a message. Returns true if a stash was created.
pub fn stash_push(cwd: &Path, message: &str) -> Result<bool> {
    let before = stash_count(cwd)?;
    if !run(
        &["stash", "push", "--include-untracked", "-m", message],
        cwd,
    )? {
        return Err(BrdError::Other("failed to stash changes".to_string()));
    }
    let after = stash_count(cwd)?;
    Ok(after > before)
}

/// Pop the most recent stash. Returns true if successful.
pub fn stash_pop(cwd: &Path) -> Result<bool> {
    run(&["stash", "pop"], cwd)
}

/// Test helpers that panic on failure (for use in tests only).
#[cfg(test)]
pub mod test {
    use super::*;

    /// Run a git command, panicking if it fails.
    pub fn run_ok(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
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

    /// Run a git command and return stdout, panicking if it fails.
    pub fn output(repo: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_repo() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        test::run_ok(dir.path(), &["init"]);
        test::run_ok(dir.path(), &["config", "user.email", "test@test.com"]);
        test::run_ok(dir.path(), &["config", "user.name", "test"]);
        test::run_ok(dir.path(), &["config", "commit.gpgsign", "false"]);
        std::fs::write(dir.path().join("README.md"), "test\n").unwrap();
        test::run_ok(dir.path(), &["add", "."]);
        test::run_ok(dir.path(), &["commit", "-m", "init"]);
        dir
    }

    #[test]
    fn test_run_success() {
        let dir = create_test_repo();
        assert!(run(&["status"], dir.path()).unwrap());
    }

    #[test]
    fn test_run_failure() {
        let dir = create_test_repo();
        assert!(!run(&["checkout", "nonexistent"], dir.path()).unwrap());
    }

    #[test]
    fn test_output() {
        let dir = create_test_repo();
        let out = output(&["rev-parse", "--abbrev-ref", "HEAD"], dir.path()).unwrap();
        assert!(!out.is_empty());
    }

    #[test]
    fn test_is_clean() {
        let dir = create_test_repo();
        assert!(is_clean(dir.path()).unwrap());

        std::fs::write(dir.path().join("dirty.txt"), "dirty").unwrap();
        assert!(!is_clean(dir.path()).unwrap());
    }

    #[test]
    fn test_current_branch() {
        let dir = create_test_repo();
        let branch = current_branch(dir.path()).unwrap();
        assert!(branch == "main" || branch == "master");
    }

    #[test]
    fn test_has_remote() {
        let dir = create_test_repo();
        assert!(!has_remote(dir.path(), "origin"));
    }

    #[test]
    fn test_branch_exists() {
        let dir = create_test_repo();
        let branch = current_branch(dir.path()).unwrap();
        assert!(branch_exists(dir.path(), &branch));
        assert!(!branch_exists(dir.path(), "nonexistent-branch"));
    }

    #[test]
    fn test_rev_parse() {
        let dir = create_test_repo();
        let branch = rev_parse(dir.path(), "--abbrev-ref HEAD").unwrap();
        assert!(branch == "main" || branch == "master");
    }

    #[test]
    fn test_rev_parse_not_repo() {
        let dir = tempdir().unwrap();
        let err = rev_parse(dir.path(), "--show-toplevel").unwrap_err();
        assert!(matches!(err, BrdError::NotGitRepo));
    }

    #[test]
    fn test_stash_count_empty() {
        let dir = create_test_repo();
        assert_eq!(stash_count(dir.path()).unwrap(), 0);
    }

    #[test]
    fn test_stash_push_creates_stash() {
        let dir = create_test_repo();

        // Create uncommitted changes
        std::fs::write(dir.path().join("new_file.txt"), "content").unwrap();

        // Stash should succeed and return true (stash created)
        let created = stash_push(dir.path(), "test stash").unwrap();
        assert!(created);

        // Working tree should be clean now
        assert!(is_clean(dir.path()).unwrap());

        // Stash count should be 1
        assert_eq!(stash_count(dir.path()).unwrap(), 1);
    }

    #[test]
    fn test_stash_push_clean_tree_returns_false() {
        let dir = create_test_repo();

        // No changes to stash - should return false
        let created = stash_push(dir.path(), "empty stash").unwrap();
        assert!(!created);

        // Stash count should still be 0
        assert_eq!(stash_count(dir.path()).unwrap(), 0);
    }

    #[test]
    fn test_stash_pop_restores_changes() {
        let dir = create_test_repo();

        // Create and stash changes
        let file_path = dir.path().join("new_file.txt");
        std::fs::write(&file_path, "content").unwrap();
        stash_push(dir.path(), "test stash").unwrap();

        // File should be gone
        assert!(!file_path.exists());

        // Pop should succeed
        let success = stash_pop(dir.path()).unwrap();
        assert!(success);

        // File should be restored
        assert!(file_path.exists());
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "content");

        // Stash count should be 0
        assert_eq!(stash_count(dir.path()).unwrap(), 0);
    }

    #[test]
    fn test_stash_pop_empty_stash_returns_false() {
        let dir = create_test_repo();

        // Pop with no stash should return false
        let success = stash_pop(dir.path()).unwrap();
        assert!(!success);
    }
}
