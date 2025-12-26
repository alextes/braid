//! brd ready command.

use crate::cli::Cli;
use crate::error::Result;
use crate::graph::get_ready_issues;
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues};

pub fn cmd_ready(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let issues = load_all_issues(paths)?;
    let ready = get_ready_issues(&issues);

    if cli.json {
        let json: Vec<_> = ready
            .iter()
            .map(|issue| issue_to_json(issue, &issues))
            .collect();
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else if ready.is_empty() {
        println!("No ready issues.");
    } else {
        for issue in ready {
            println!("{}  {}  {}", issue.id(), issue.priority(), issue.title());
        }
    }

    Ok(())
}
