//! brd ls command.

use crossterm::style::{Attribute, SetAttribute};

use crate::cli::Cli;
use crate::error::Result;
use crate::graph::compute_derived;
use crate::issue::{Issue, IssueType, Priority, Status};
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues};

/// Maximum number of done issues to show by default
const DEFAULT_DONE_LIMIT: usize = 10;

#[allow(clippy::too_many_arguments)]
pub fn cmd_ls(
    cli: &Cli,
    paths: &RepoPaths,
    status_filter: Option<&str>,
    priority_filter: Option<&str>,
    ready_only: bool,
    blocked_only: bool,
    label_filter: &[String],
    show_all: bool,
) -> Result<()> {
    let issues = load_all_issues(paths)?;

    let status_filter: Option<Status> = status_filter.map(|s| s.parse()).transpose()?;
    let priority_filter: Option<Priority> = priority_filter.map(|p| p.parse()).transpose()?;

    let filtered: Vec<&Issue> = issues
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

    // partition into non-done (todo/doing) and done issues
    let (mut active, mut done): (Vec<&Issue>, Vec<&Issue>) = filtered
        .into_iter()
        .partition(|issue| issue.status() != Status::Done);

    // sort active issues by priority, created_at, id
    active.sort_by(|a, b| a.cmp_by_priority(b));

    // sort done issues by updated_at (most recent first), then by id for stability
    done.sort_by(|a, b| {
        b.frontmatter
            .updated_at
            .cmp(&a.frontmatter.updated_at)
            .then_with(|| a.id().cmp(b.id()))
    });

    // limit done issues unless --all is specified
    if !show_all && done.len() > DEFAULT_DONE_LIMIT {
        done.truncate(DEFAULT_DONE_LIMIT);
    }

    // combine: active first, then done
    let filtered: Vec<&Issue> = active.into_iter().chain(done).collect();

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
            } else if issue.issue_type() == Some(IssueType::Meta) {
                // meta issues show progress as "done/total"
                let total = issue.deps().len();
                let done = total - derived.open_deps.len();
                format!(" ({}/{})", done, total)
            } else {
                format!(
                    " (deps:{} open:{})",
                    issue.deps().len(),
                    derived.open_deps.len()
                )
            };

            // apply styling based on status and type
            let is_done = issue.status() == Status::Done;
            let use_color = !cli.no_color;

            if use_color {
                if is_done {
                    print!("{}", SetAttribute(Attribute::Dim));
                } else {
                    match issue.issue_type() {
                        Some(IssueType::Design) => print!("{}", SetAttribute(Attribute::Italic)),
                        Some(IssueType::Meta) => print!("{}", SetAttribute(Attribute::Bold)),
                        None => {}
                    }
                }
            }

            print!(
                "{}  {}  {}  {}{}",
                issue.id(),
                issue.priority(),
                issue.status(),
                issue.title(),
                deps_info
            );

            if use_color && (is_done || issue.issue_type().is_some()) {
                print!("{}", SetAttribute(Attribute::Reset));
            }
            println!();
        }
    }

    Ok(())
}
