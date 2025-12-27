//! brd rm command.

use crate::cli::Cli;
use crate::error::{BrdError, Result};
use crate::issue::Status;
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{load_all_issues, resolve_issue_id};

pub fn cmd_rm(cli: &Cli, paths: &RepoPaths, id: &str, force: bool) -> Result<()> {
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let issues = load_all_issues(paths)?;
    let full_id = resolve_issue_id(id, &issues)?;

    let issue = issues
        .get(&full_id)
        .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

    // safety: refuse to delete issues in progress unless forced
    if issue.status() == Status::Doing && !force {
        return Err(BrdError::Other(format!(
            "issue {} is in progress (use --force to delete anyway)",
            full_id
        )));
    }

    // delete from control root
    let issue_path = paths.issues_dir().join(format!("{}.md", full_id));
    std::fs::remove_file(&issue_path)?;

    // dual-write: also delete from local worktree if different from control root
    if paths.worktree_root != paths.control_root {
        let local_issue_path = paths
            .worktree_root
            .join(".braid/issues")
            .join(format!("{}.md", full_id));
        if local_issue_path.exists() {
            std::fs::remove_file(&local_issue_path)?;
        }
    }

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "deleted": full_id,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Deleted: {}", full_id);
    }

    Ok(())
}
