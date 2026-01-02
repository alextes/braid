//! brd commit command.

use std::process::Command;

use crate::cli::Cli;
use crate::error::{BrdError, Result};
use crate::repo::RepoPaths;
use crate::verbose;

/// commit staged and unstaged .braid changes with a generated message.
pub fn cmd_commit(cli: &Cli, paths: &RepoPaths, message: Option<&str>) -> Result<()> {
    let braid_dir = paths.braid_dir();

    // check if .braid directory exists
    if !braid_dir.exists() {
        return Err(BrdError::Other("no .braid directory found".to_string()));
    }

    // stage all .braid changes
    verbose!(cli, "staging .braid changes");
    let add_output = Command::new("git")
        .args(["add", ".braid"])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        return Err(BrdError::Other(format!("git add failed: {}", stderr)));
    }

    // check if there are staged changes in .braid
    let diff_output = Command::new("git")
        .args(["diff", "--cached", "--quiet", ".braid"])
        .current_dir(&paths.worktree_root)
        .output()?;

    if diff_output.status.success() {
        // no staged changes
        if cli.json {
            println!(r#"{{"ok": true, "message": "nothing to commit"}}"#);
        } else {
            println!("nothing to commit");
        }
        return Ok(());
    }

    // generate commit message if not provided
    let commit_msg = match message {
        Some(msg) => msg.to_string(),
        None => generate_commit_message(&paths.worktree_root)?,
    };

    verbose!(cli, "committing with message: {}", commit_msg);

    // commit only .braid changes (not other staged files)
    let commit_output = Command::new("git")
        .args(["commit", "-m", &commit_msg, "--", ".braid"])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        return Err(BrdError::Other(format!("git commit failed: {}", stderr)));
    }

    if cli.json {
        println!(
            r#"{{"ok": true, "message": "committed", "commit_message": {}}}"#,
            serde_json::to_string(&commit_msg).unwrap()
        );
    } else {
        println!("committed: {}", commit_msg);
    }

    Ok(())
}

/// generate a commit message based on the staged .braid changes.
fn generate_commit_message(repo_root: &std::path::Path) -> Result<String> {
    // get the list of changed files
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-status", ".braid"])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        return Ok("chore(braid): update issues".to_string());
    }

    let changes = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = changes.lines().collect();

    if lines.is_empty() {
        return Ok("chore(braid): update issues".to_string());
    }

    // count different types of changes
    let mut added = 0;
    let mut modified = 0;
    let mut deleted = 0;
    let mut issue_ids: Vec<String> = Vec::new();

    for line in &lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let status = parts[0];
            let path = parts[1];

            // extract issue ID from path like .braid/issues/brd-xxxx.md
            if let Some(filename) = path.strip_prefix(".braid/issues/")
                && let Some(id) = filename.strip_suffix(".md")
            {
                issue_ids.push(id.to_string());
            }

            match status {
                "A" => added += 1,
                "M" => modified += 1,
                "D" => deleted += 1,
                _ => {}
            }
        }
    }

    // build commit message
    let mut parts = Vec::new();
    if added > 0 {
        parts.push(format!("add {}", added));
    }
    if modified > 0 {
        parts.push(format!("update {}", modified));
    }
    if deleted > 0 {
        parts.push(format!("remove {}", deleted));
    }

    if parts.is_empty() {
        return Ok("chore(braid): update issues".to_string());
    }

    let action = parts.join(", ");

    // include issue IDs if small number
    let msg = if issue_ids.len() == 1 {
        format!("chore(braid): {} issue ({})", action, issue_ids[0])
    } else if issue_ids.len() <= 3 {
        format!("chore(braid): {} issues ({})", action, issue_ids.join(", "))
    } else {
        format!("chore(braid): {} issues", action)
    };

    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn setup_git_repo() -> tempfile::TempDir {
        let dir = tempdir().unwrap();

        // init git repo
        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // configure git user for commits
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
        // disable gpg signing for tests
        Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // create .braid/issues directory
        fs::create_dir_all(dir.path().join(".braid/issues")).unwrap();

        // initial commit so we have a HEAD
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

        dir
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

    fn make_paths(dir: &tempfile::TempDir) -> RepoPaths {
        RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        }
    }

    #[test]
    fn test_commit_no_braid_dir() {
        let dir = tempdir().unwrap();
        let cli = make_cli();
        let paths = RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };

        let result = cmd_commit(&cli, &paths, None);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("no .braid directory")
        );
    }

    #[test]
    fn test_commit_nothing_to_commit() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // no changes to .braid, should succeed with "nothing to commit"
        let result = cmd_commit(&cli, &paths, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_commit_with_new_issue() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // create a new issue file
        fs::write(
            dir.path().join(".braid/issues/brd-test.md"),
            "---\ntitle: test\n---\n",
        )
        .unwrap();

        let result = cmd_commit(&cli, &paths, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_commit_with_custom_message() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // create a new issue file
        fs::write(
            dir.path().join(".braid/issues/brd-custom.md"),
            "---\ntitle: custom\n---\n",
        )
        .unwrap();

        let result = cmd_commit(&cli, &paths, Some("custom commit message"));
        assert!(result.is_ok());

        // verify commit message
        let log = Command::new("git")
            .args(["log", "-1", "--format=%s"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let msg = String::from_utf8_lossy(&log.stdout);
        assert!(msg.contains("custom commit message"));
    }

    #[test]
    fn test_generate_commit_message_single_add() {
        let dir = setup_git_repo();

        // create and stage a new issue
        fs::write(
            dir.path().join(".braid/issues/brd-aaaa.md"),
            "---\ntitle: test\n---\n",
        )
        .unwrap();
        Command::new("git")
            .args(["add", ".braid"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let msg = generate_commit_message(dir.path()).unwrap();
        assert!(msg.contains("add 1"));
        assert!(msg.contains("brd-aaaa"));
    }

    #[test]
    fn test_generate_commit_message_multiple_changes() {
        let dir = setup_git_repo();

        // create initial issues and commit them
        fs::write(
            dir.path().join(".braid/issues/brd-0001.md"),
            "---\ntitle: one\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join(".braid/issues/brd-0002.md"),
            "---\ntitle: two\n---\n",
        )
        .unwrap();
        Command::new("git")
            .args(["add", ".braid"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add issues"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // now modify one, delete one, add one
        fs::write(
            dir.path().join(".braid/issues/brd-0001.md"),
            "---\ntitle: one modified\n---\n",
        )
        .unwrap();
        fs::remove_file(dir.path().join(".braid/issues/brd-0002.md")).unwrap();
        fs::write(
            dir.path().join(".braid/issues/brd-0003.md"),
            "---\ntitle: three\n---\n",
        )
        .unwrap();

        Command::new("git")
            .args(["add", ".braid"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let msg = generate_commit_message(dir.path()).unwrap();
        assert!(msg.contains("add 1"));
        assert!(msg.contains("update 1"));
        assert!(msg.contains("remove 1"));
    }
}
