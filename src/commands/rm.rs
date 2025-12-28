//! brd rm command.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::issue::Status;
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{load_all_issues, resolve_issue_id};

pub fn cmd_rm(cli: &Cli, paths: &RepoPaths, id: &str, force: bool) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let issues = load_all_issues(paths, &config)?;
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

    // delete issue file
    let issue_path = paths.issues_dir(&config).join(format!("{}.md", full_id));
    std::fs::remove_file(&issue_path)?;

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
