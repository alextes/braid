//! brd show command.

use crate::cli::Cli;
use crate::error::{BrdError, Result};
use crate::graph::compute_derived;
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues, resolve_issue_id};

pub fn cmd_show(cli: &Cli, paths: &RepoPaths, id: &str) -> Result<()> {
    let issues = load_all_issues(paths)?;
    let full_id = resolve_issue_id(id, &issues)?;
    let issue = issues
        .get(&full_id)
        .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

    if cli.json {
        let json = issue_to_json(issue, &issues);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("ID:       {}", issue.id());
        println!("Title:    {}", issue.title());
        println!("Priority: {}", issue.priority());
        println!("Status:   {}", issue.status());

        if !issue.deps().is_empty() {
            println!("Deps:     {}", issue.deps().join(", "));
        }

        if !issue.tags().is_empty() {
            println!("Tags:     {}", issue.tags().join(", "));
        }

        if let Some(owner) = &issue.frontmatter.owner {
            println!("Owner:    {}", owner);
        }

        let derived = compute_derived(issue, &issues);
        if derived.is_ready {
            println!("State:    READY");
        } else if derived.is_blocked {
            println!("State:    BLOCKED");
            if !derived.open_deps.is_empty() {
                println!("  open:   {}", derived.open_deps.join(", "));
            }
            if !derived.missing_deps.is_empty() {
                println!("  missing: {}", derived.missing_deps.join(", "));
            }
        }

        if !issue.frontmatter.acceptance.is_empty() {
            println!("\nAcceptance:");
            for ac in &issue.frontmatter.acceptance {
                println!("  - {}", ac);
            }
        }

        if !issue.body.is_empty() {
            println!("\n{}", issue.body);
        }
    }

    Ok(())
}
