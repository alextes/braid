//! brd ls command.

use crossterm::style::{Attribute, Color, SetAttribute, SetForegroundColor};

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
    tag_filter: &[String],
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
            if !tag_filter.is_empty() && !tag_filter.iter().all(|tag| issue.tags().contains(tag)) {
                return false;
            }
            true
        })
        .collect();

    // partition into active (todo/doing) and resolved (done/skip) issues
    let (mut active, mut resolved): (Vec<&Issue>, Vec<&Issue>) = filtered
        .into_iter()
        .partition(|issue| !matches!(issue.status(), Status::Done | Status::Skip));

    // sort active issues by priority, created_at, id
    active.sort_by(|a, b| a.cmp_by_priority(b));

    // sort resolved issues by updated_at (most recent first), then by id for stability
    resolved.sort_by(|a, b| {
        b.frontmatter
            .updated_at
            .cmp(&a.frontmatter.updated_at)
            .then_with(|| a.id().cmp(b.id()))
    });

    // limit resolved issues unless --all is specified
    if !show_all && resolved.len() > DEFAULT_DONE_LIMIT {
        resolved.truncate(DEFAULT_DONE_LIMIT);
    }

    // combine: active first, then resolved
    let filtered: Vec<&Issue> = active.into_iter().chain(resolved).collect();

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

            // show owner for doing issues (max 12 chars)
            let owner_info = if issue.status() == Status::Doing {
                issue
                    .frontmatter
                    .owner
                    .as_ref()
                    .map(|o| {
                        let truncated: String = o.chars().take(12).collect();
                        format!(" ({})", truncated)
                    })
                    .unwrap_or_default()
            } else {
                String::new()
            };

            // apply styling based on status, priority, and type
            let is_resolved = matches!(issue.status(), Status::Done | Status::Skip);
            let is_doing = issue.status() == Status::Doing;
            let is_high_priority =
                issue.priority() == Priority::P0 || issue.priority() == Priority::P1;
            let use_color = !cli.no_color;

            if use_color {
                if is_resolved {
                    print!("{}", SetAttribute(Attribute::Dim));
                } else {
                    // priority styling: P0/P1 get bold
                    if is_high_priority {
                        print!("{}", SetAttribute(Attribute::Bold));
                    }
                    // status styling: doing gets underline
                    if is_doing {
                        print!("{}", SetAttribute(Attribute::Underlined));
                    }
                    // type styling
                    match issue.issue_type() {
                        Some(IssueType::Design) => print!("{}", SetAttribute(Attribute::Italic)),
                        Some(IssueType::Meta) => print!("{}", SetAttribute(Attribute::Bold)),
                        None => {}
                    }
                }
            }

            // type column: "design", "meta", or padded empty (8 chars)
            let type_col = match issue.issue_type() {
                Some(IssueType::Design) => "design  ",
                Some(IssueType::Meta) => "meta    ",
                None => "        ",
            };

            // status column: padded to 5 chars (length of "doing")
            let status_col = match issue.status() {
                Status::Todo => "todo ",
                Status::Doing => "doing",
                Status::Done => "done ",
                Status::Skip => "skip ",
            };

            print!(
                "{}  {}  {}{}  {}{}",
                issue.id(),
                issue.priority(),
                type_col,
                status_col,
                issue.title(),
                deps_info
            );

            if use_color
                && (is_resolved || is_doing || is_high_priority || issue.issue_type().is_some())
            {
                print!("{}", SetAttribute(Attribute::Reset));
            }

            // print owner in magenta
            if !owner_info.is_empty() {
                if use_color {
                    print!(
                        "{}{}{}",
                        SetForegroundColor(Color::Magenta),
                        owner_info,
                        SetAttribute(Attribute::Reset)
                    );
                } else {
                    print!("{}", owner_info);
                }
            }
            println!();
        }
    }

    Ok(())
}
