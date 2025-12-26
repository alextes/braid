//! brd ls command.

use crate::cli::Cli;
use crate::error::Result;
use crate::graph::compute_derived;
use crate::issue::{Issue, Priority, Status};
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues};

pub fn cmd_ls(
    cli: &Cli,
    paths: &RepoPaths,
    status_filter: Option<&str>,
    priority_filter: Option<&str>,
    ready_only: bool,
    blocked_only: bool,
    label_filter: &[String],
) -> Result<()> {
    let issues = load_all_issues(paths)?;

    let status_filter: Option<Status> = status_filter.map(|s| s.parse()).transpose()?;
    let priority_filter: Option<Priority> = priority_filter.map(|p| p.parse()).transpose()?;

    let mut filtered: Vec<&Issue> = issues
        .values()
        .filter(|issue| {
            if let Some(s) = status_filter
                && issue.status() != s
            {
                return false;
            }
            if let Some(p) = priority_filter
                && issue.priority() != p
            {
                return false;
            }
            if ready_only {
                let derived = compute_derived(issue, &issues);
                if !derived.is_ready {
                    return false;
                }
            }
            if blocked_only {
                let derived = compute_derived(issue, &issues);
                if !derived.is_blocked {
                    return false;
                }
            }
            if !label_filter.is_empty()
                && !label_filter
                    .iter()
                    .all(|label| issue.labels().contains(label))
            {
                return false;
            }
            true
        })
        .collect();

    // sort by priority, created_at, id
    filtered.sort_by(|a, b| {
        a.priority()
            .cmp(&b.priority())
            .then_with(|| a.frontmatter.created_at.cmp(&b.frontmatter.created_at))
            .then_with(|| a.id().cmp(b.id()))
    });

    if cli.json {
        let json: Vec<_> = filtered
            .iter()
            .map(|issue| issue_to_json(issue, &issues))
            .collect();
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else if filtered.is_empty() {
        println!("No issues found.");
    } else {
        for issue in filtered {
            let derived = compute_derived(issue, &issues);
            let deps_info = if issue.deps().is_empty() {
                String::new()
            } else {
                format!(
                    " (deps:{} open:{})",
                    issue.deps().len(),
                    derived.open_deps.len()
                )
            };
            println!(
                "{}  {}  {}  {}{}",
                issue.id(),
                issue.priority(),
                issue.status(),
                issue.title(),
                deps_info
            );
        }
    }

    Ok(())
}
