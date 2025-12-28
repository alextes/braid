//! brd skip command.

use crate::cli::Cli;
use crate::error::{BrdError, Result};
use crate::issue::Status;
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues, resolve_issue_id};

pub fn cmd_skip(cli: &Cli, paths: &RepoPaths, id: &str) -> Result<()> {
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths)?;
    let full_id = resolve_issue_id(id, &issues)?;

    {
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

        issue.frontmatter.status = Status::Skip;
        issue.frontmatter.owner = None;
        issue.touch();

        let issue_path = paths.issues_dir().join(format!("{}.md", full_id));
        issue.save(&issue_path)?;
    }

    if cli.json {
        let issue = issues.get(&full_id).unwrap();
        let json = issue_to_json(issue, &issues);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Skipped: {}", full_id);
    }

    Ok(())
}
