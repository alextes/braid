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
            if let Some(filename) = path.strip_prefix(".braid/issues/") {
                if let Some(id) = filename.strip_suffix(".md") {
                    issue_ids.push(id.to_string());
                }
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
