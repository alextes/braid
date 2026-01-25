//! brd config command - view and change braid configuration.

mod auto_sync;
mod external_repo;
mod issues_branch;
mod show;

pub use auto_sync::cmd_config_auto_sync;
pub use external_repo::cmd_config_external_repo;
pub use issues_branch::cmd_config_issues_branch;
pub use show::cmd_config_show;

use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::error::Result;
use crate::git;

pub(crate) const ISSUES_SYMLINK_PATTERN: &str = ".braid/issues";

/// Prompt for confirmation. Returns true if user confirms (Y/y or empty).
pub(crate) fn confirm(prompt: &str) -> Result<bool> {
    print!("{} [Y/n]: ", prompt);
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let answer = line.trim().to_lowercase();

    Ok(answer.is_empty() || answer == "y" || answer == "yes")
}

/// Count .md files in a directory.
pub(crate) fn count_issues(dir: &Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                .count()
        })
        .unwrap_or(0)
}

/// Check if a branch has an upstream tracking branch.
pub(crate) fn has_upstream(branch: &str, cwd: &Path) -> bool {
    git::run(
        &["rev-parse", "--abbrev-ref", &format!("{}@{{u}}", branch)],
        cwd,
    )
    .unwrap_or(false)
}

/// Get the upstream tracking branch name.
pub(crate) fn get_upstream(branch: &str, cwd: &Path) -> Option<String> {
    git::output(
        &["rev-parse", "--abbrev-ref", &format!("{}@{{u}}", branch)],
        cwd,
    )
    .ok()
    .filter(|s| !s.is_empty())
}

/// Agent worktree info for rebase warnings.
pub(crate) struct AgentWorktree {
    pub branch: String,
    pub path: std::path::PathBuf,
}

/// Find agent worktrees that need to rebase on main.
/// Returns worktrees that have .braid/agent.toml and are behind main.
pub(crate) fn find_agent_worktrees_needing_rebase(cwd: &Path) -> Vec<AgentWorktree> {
    let mut result = Vec::new();

    // get worktree list in porcelain format
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(cwd)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return result,
    };

    // parse porcelain output: "worktree <path>\nHEAD <sha>\nbranch refs/heads/<name>\n\n"
    let mut current_path: Option<std::path::PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(std::path::PathBuf::from(path));
            current_branch = None;
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            // branch refs/heads/agent-one -> agent-one
            current_branch = branch_ref.strip_prefix("refs/heads/").map(String::from);
        } else if line.is_empty() {
            // end of entry, check if it's an agent worktree
            if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                // check if this is an agent worktree (has .braid/agent.toml)
                let agent_toml = path.join(".braid/agent.toml");
                if agent_toml.exists() {
                    // check if behind main
                    if is_behind_main(&path) {
                        result.push(AgentWorktree { branch, path });
                    }
                }
            }
        }
    }

    result
}

/// Check if a worktree is behind main (main has commits not in this branch).
pub(crate) fn is_behind_main(worktree_path: &Path) -> bool {
    // count commits in main that aren't in HEAD
    let output = std::process::Command::new("git")
        .args(["rev-list", "--count", "HEAD..main"])
        .current_dir(worktree_path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let count_str = String::from_utf8_lossy(&o.stdout);
            count_str.trim().parse::<u32>().unwrap_or(0) > 0
        }
        _ => false,
    }
}

/// Print warning about agent worktrees needing rebase.
pub(crate) fn warn_agent_worktrees(worktrees: &[AgentWorktree]) {
    if worktrees.is_empty() {
        return;
    }

    println!();
    println!(
        "Warning: Found {} agent worktree(s) that need to rebase on main:",
        worktrees.len()
    );
    for wt in worktrees {
        println!("  - {} (at {})", wt.branch, wt.path.display());
    }
    println!();
    println!("Run `git rebase main` in each worktree to pick up the new config.");
}

/// Remove a pattern from .git/info/exclude.
pub(crate) fn remove_from_git_exclude(paths: &crate::repo::RepoPaths, pattern: &str) -> Result<()> {
    let exclude_path = paths.git_common_dir.join("info/exclude");

    if !exclude_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&exclude_path)?;
    let new_content: String = content
        .lines()
        .filter(|line| line.trim() != pattern)
        .collect::<Vec<_>>()
        .join("\n");

    // preserve trailing newline if original had one
    let new_content = if content.ends_with('\n') && !new_content.is_empty() {
        format!("{}\n", new_content)
    } else {
        new_content
    };

    std::fs::write(&exclude_path, new_content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    pub(super) fn setup_git_repo() -> tempfile::TempDir {
        let dir = tempdir().unwrap();

        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // initial commit
        fs::write(dir.path().join(".gitkeep"), "").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // ensure branch is named "main" (CI may default to "master")
        Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        dir
    }

    pub(super) fn make_cli() -> crate::cli::Cli {
        crate::cli::Cli {
            json: false,
            repo: None,
            no_color: true,
            verbose: false,
            command: crate::cli::Command::Doctor,
        }
    }

    pub(super) fn make_paths(dir: &tempfile::TempDir) -> crate::repo::RepoPaths {
        crate::repo::RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        }
    }

    #[test]
    fn test_count_issues_empty_dir() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("issues")).unwrap();
        assert_eq!(count_issues(&dir.path().join("issues")), 0);
    }

    #[test]
    fn test_count_issues_nonexistent_dir() {
        let dir = tempdir().unwrap();
        assert_eq!(count_issues(&dir.path().join("nonexistent")), 0);
    }

    #[test]
    fn test_count_issues_with_md_files() {
        let dir = tempdir().unwrap();
        let issues = dir.path().join("issues");
        fs::create_dir_all(&issues).unwrap();
        fs::write(issues.join("a.md"), "").unwrap();
        fs::write(issues.join("b.md"), "").unwrap();
        fs::write(issues.join("c.txt"), "").unwrap();
        assert_eq!(count_issues(&issues), 2);
    }

    #[test]
    fn test_has_upstream_no_upstream() {
        let dir = setup_git_repo();
        // no upstream configured
        assert!(!has_upstream("main", dir.path()));
    }

    #[test]
    fn test_get_upstream_no_upstream() {
        let dir = setup_git_repo();
        // no upstream configured
        assert!(get_upstream("main", dir.path()).is_none());
    }

    #[test]
    fn test_is_behind_main_same_commit() {
        let dir = setup_git_repo();

        // create a branch at the same commit as main
        Command::new("git")
            .args(["branch", "feature"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // check from main (not behind itself)
        assert!(!is_behind_main(dir.path()));
    }

    #[test]
    fn test_is_behind_main_behind() {
        let dir = setup_git_repo();

        // create a branch
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // go back to main and add a commit
        Command::new("git")
            .args(["checkout", "main"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        fs::write(dir.path().join("new_file"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "new commit on main"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // switch to feature and check if behind
        Command::new("git")
            .args(["checkout", "feature"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        assert!(is_behind_main(dir.path()));
    }
}
