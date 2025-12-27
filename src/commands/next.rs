//! brd next command.

use crate::cli::Cli;
use crate::error::Result;
use crate::graph::get_ready_issues;
use crate::issue::IssueType;
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues};

pub fn cmd_next(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let issues = load_all_issues(paths)?;
    let ready = get_ready_issues(&issues);
    // skip meta issues - they're tracking containers, not actionable work
    let next_issue = ready
        .into_iter()
        .find(|issue| issue.issue_type() != Some(IssueType::Meta));

    if cli.json {
        match next_issue {
            Some(issue) => {
                let json = issue_to_json(issue, &issues);
                println!("{}", serde_json::to_string_pretty(&json).unwrap());
            }
            None => println!("null"),
        }
    } else if let Some(issue) = next_issue {
        println!("{}  {}  {}", issue.id(), issue.priority(), issue.title());
    } else {
        println!("No ready issues.");
    }

    Ok(())
}
