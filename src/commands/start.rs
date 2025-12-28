//! brd start command.

use crate::cli::Cli;
use crate::error::{BrdError, Result};
use crate::graph::get_ready_issues;
use crate::issue::{IssueType, Status};
use crate::lock::LockGuard;
use crate::repo::{self, RepoPaths};

use super::{issue_to_json, load_all_issues, resolve_issue_id};

pub fn cmd_start(cli: &Cli, paths: &RepoPaths, id: Option<&str>, force: bool) -> Result<()> {
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths)?;

    // resolve issue id: either from argument or pick next ready (skipping meta issues)
    let full_id = match id {
        Some(partial) => resolve_issue_id(partial, &issues)?,
        None => {
            let ready = get_ready_issues(&issues);
            // skip meta issues - they're tracking containers, not actionable work
            ready
                .into_iter()
                .find(|issue| issue.issue_type() != Some(IssueType::Meta))
                .map(|i| i.id().to_string())
                .ok_or_else(|| BrdError::Other("no ready issues".to_string()))?
        }
    };

    let agent_id = repo::get_agent_id(&paths.worktree_root);

    {
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(full_id.clone()))?;

        // check if already being worked on
        if issue.status() == Status::Doing && !force {
            let owner = issue.frontmatter.owner.as_deref().unwrap_or("unknown");
            return Err(BrdError::Other(format!(
                "issue {} is already being worked on by '{}' (use --force to reassign)",
                full_id, owner
            )));
        }

        issue.frontmatter.status = Status::Doing;
        issue.frontmatter.owner = Some(agent_id.clone());
        issue.touch();

        let issue_path = paths.issues_dir().join(format!("{}.md", full_id));
        issue.save(&issue_path)?;
    }

    if cli.json {
        let issue = issues.get(&full_id).unwrap();
        let json = issue_to_json(issue, &issues);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Started: {} (owner: {})", full_id, agent_id);
    }

    Ok(())
}
