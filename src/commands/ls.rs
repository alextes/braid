//! brd ls command.

use std::time::Instant;

use crossterm::style::{Attribute, Color, SetAttribute, SetForegroundColor};
use time::OffsetDateTime;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::graph::compute_derived;
use crate::issue::{Issue, IssueType, Priority, Status};
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues};

/// Format an age duration in short human format (e.g., "5m", "2h", "3d", "1w", "3mo", "1y").
fn format_age(created_at: OffsetDateTime) -> String {
    let now = OffsetDateTime::now_utc();
    let duration = now - created_at;
    let minutes = duration.whole_minutes();

    if minutes < 0 {
        // Future date (shouldn't happen, but handle gracefully)
        "0m".to_string()
    } else if minutes < 60 {
        format!("{}m", minutes.max(1))
    } else if minutes < 60 * 24 {
        format!("{}h", minutes / 60)
    } else if minutes < 60 * 24 * 7 {
        format!("{}d", minutes / (60 * 24))
    } else if minutes < 60 * 24 * 30 {
        format!("{}w", minutes / (60 * 24 * 7))
    } else if minutes < 60 * 24 * 365 {
        format!("{}mo", minutes / (60 * 24 * 30))
    } else {
        format!("{}y", minutes / (60 * 24 * 365))
    }
}

/// Maximum number of done issues to show by default
const DEFAULT_DONE_LIMIT: usize = 10;

/// Maximum number of open issues to show by default
const DEFAULT_OPEN_LIMIT: usize = 15;

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
    let start = Instant::now();
    let config = Config::load(&paths.config_path())?;
    let issues = load_all_issues(paths, &config)?;

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

    // partition into doing, open, and resolved (done/skip) issues
    let mut doing: Vec<&Issue> = Vec::new();
    let mut open: Vec<&Issue> = Vec::new();
    let mut resolved: Vec<&Issue> = Vec::new();

    for issue in filtered {
        match issue.status() {
            Status::Doing => doing.push(issue),
            Status::Open => open.push(issue),
            Status::Done | Status::Skip => resolved.push(issue),
        }
    }

    // sort doing and open by priority, created_at, id
    doing.sort_by(|a, b| a.cmp_by_priority(b));
    open.sort_by(|a, b| a.cmp_by_priority(b));

    // sort resolved issues by updated_at (most recent first), then by id for stability
    resolved.sort_by(|a, b| {
        b.frontmatter
            .updated_at
            .cmp(&a.frontmatter.updated_at)
            .then_with(|| a.id().cmp(b.id()))
    });

    // compute total counts BEFORE truncation
    let total_doing = doing.len();
    let total_open = open.len();
    let total_done = resolved
        .iter()
        .filter(|i| i.status() == Status::Done)
        .count();
    let total_skip = resolved
        .iter()
        .filter(|i| i.status() == Status::Skip)
        .count();

    // track how many are hidden
    let hidden_open;
    let hidden_resolved;

    // limit open and resolved issues unless --all is specified
    if !show_all && open.len() > DEFAULT_OPEN_LIMIT {
        hidden_open = open.len() - DEFAULT_OPEN_LIMIT;
        open.truncate(DEFAULT_OPEN_LIMIT);
    } else {
        hidden_open = 0;
    }

    if !show_all && resolved.len() > DEFAULT_DONE_LIMIT {
        hidden_resolved = resolved.len() - DEFAULT_DONE_LIMIT;
        resolved.truncate(DEFAULT_DONE_LIMIT);
    } else {
        hidden_resolved = 0;
    }

    // combine: doing first, then open, then resolved
    let filtered: Vec<&Issue> = doing.into_iter().chain(open).chain(resolved).collect();

    if cli.json {
        let json: Vec<_> = filtered
            .iter()
            .map(|issue| issue_to_json(issue, &issues))
            .collect();
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        // use pre-computed totals
        let open_count = total_open + total_doing;

        if filtered.is_empty() {
            println!("No issues found.");
        }

        // track position to insert indicator
        let mut printed_count = 0;
        let indicator_after = total_doing + total_open.min(DEFAULT_OPEN_LIMIT);

        for issue in &filtered {
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
            let use_color = !cli.no_color;
            let priority_color = if use_color && !is_resolved {
                match issue.priority() {
                    Priority::P0 => Some(Color::Red),
                    Priority::P1 => Some(Color::Yellow),
                    _ => None,
                }
            } else {
                None
            };

            if use_color {
                if is_resolved {
                    print!("{}", SetAttribute(Attribute::Dim));
                } else {
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

            // age column: padded to 4 chars (max is "99mo")
            let age = format_age(issue.frontmatter.created_at);
            let age_col = format!("{:>4}", age);

            // type column: "design", "meta", or padded empty (8 chars)
            let type_col = match issue.issue_type() {
                Some(IssueType::Design) => "design  ",
                Some(IssueType::Meta) => "meta    ",
                None => "        ",
            };

            // status column: padded to 5 chars (length of "doing")
            let status_col = match issue.status() {
                Status::Open => "open ",
                Status::Doing => "doing",
                Status::Done => "done ",
                Status::Skip => "skip ",
            };

            print!("{}  ", issue.id());
            if let Some(color) = priority_color {
                print!("{}", SetForegroundColor(color));
            }
            print!("{}", issue.priority());
            if priority_color.is_some() {
                print!("{}", SetForegroundColor(Color::Reset));
            }
            print!(
                "  {}  {}{}  {}{}",
                age_col,
                type_col,
                status_col,
                issue.title(),
                deps_info
            );

            if !issue.tags().is_empty() {
                print!(" ");
                for (i, tag) in issue.tags().iter().enumerate() {
                    if i > 0 {
                        print!(" ");
                    }
                    if use_color {
                        let color = if tag == "bug" {
                            Color::Red
                        } else {
                            Color::Cyan
                        };
                        print!(
                            "{}#{}{}",
                            SetForegroundColor(color),
                            tag,
                            SetForegroundColor(Color::Reset)
                        );
                    } else {
                        print!("#{}", tag);
                    }
                }
            }

            if use_color && (is_resolved || is_doing || issue.issue_type().is_some()) {
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

            printed_count += 1;

            // print indicator after last open issue (before resolved)
            if printed_count == indicator_after && hidden_open > 0 {
                if use_color {
                    println!(
                        "{}... +{} more open{}",
                        SetAttribute(Attribute::Dim),
                        hidden_open,
                        SetAttribute(Attribute::Reset)
                    );
                } else {
                    println!("... +{} more open", hidden_open);
                }
            }
        }

        // print indicator after resolved issues
        if hidden_resolved > 0 {
            // use status name if filtering, otherwise generic "resolved"
            let status_name = match status_filter {
                Some(Status::Done) => "done",
                Some(Status::Skip) => "skip",
                _ => "resolved",
            };
            if !cli.no_color {
                println!(
                    "{}... +{} more {} (--all to show all){}",
                    SetAttribute(Attribute::Dim),
                    hidden_resolved,
                    status_name,
                    SetAttribute(Attribute::Reset)
                );
            } else {
                println!(
                    "... +{} more {} (--all to show all)",
                    hidden_resolved, status_name
                );
            }
        }

        let elapsed_ms = start.elapsed().as_millis();

        // build summary line: open (open+doing), plus non-zero resolved counts
        let mut parts = Vec::new();
        parts.push(format!("open: {}", open_count));
        if total_doing > 0 {
            parts.push(format!("doing: {}", total_doing));
        }
        if total_done > 0 {
            parts.push(format!("done: {}", total_done));
        }
        if total_skip > 0 {
            parts.push(format!("skip: {}", total_skip));
        }

        println!("{} | took: {}ms", parts.join(" | "), elapsed_ms);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::Duration;

    #[test]
    fn test_format_age_minutes() {
        let now = OffsetDateTime::now_utc();

        // Just now shows as 1m (minimum)
        assert_eq!(format_age(now), "1m");

        // 30 minutes ago
        assert_eq!(format_age(now - Duration::minutes(30)), "30m");

        // 59 minutes ago
        assert_eq!(format_age(now - Duration::minutes(59)), "59m");
    }

    #[test]
    fn test_format_age_hours() {
        let now = OffsetDateTime::now_utc();

        // 1 hour ago
        assert_eq!(format_age(now - Duration::hours(1)), "1h");

        // 12 hours ago
        assert_eq!(format_age(now - Duration::hours(12)), "12h");

        // 23 hours ago
        assert_eq!(format_age(now - Duration::hours(23)), "23h");
    }

    #[test]
    fn test_format_age_days() {
        let now = OffsetDateTime::now_utc();

        // 1 day ago
        assert_eq!(format_age(now - Duration::days(1)), "1d");

        // 6 days ago
        assert_eq!(format_age(now - Duration::days(6)), "6d");
    }

    #[test]
    fn test_format_age_weeks() {
        let now = OffsetDateTime::now_utc();

        // 7 days = 1 week
        assert_eq!(format_age(now - Duration::days(7)), "1w");

        // 14 days = 2 weeks
        assert_eq!(format_age(now - Duration::days(14)), "2w");

        // 29 days = 4 weeks
        assert_eq!(format_age(now - Duration::days(29)), "4w");
    }

    #[test]
    fn test_format_age_months() {
        let now = OffsetDateTime::now_utc();

        // 30 days = 1 month
        assert_eq!(format_age(now - Duration::days(30)), "1mo");

        // 60 days = 2 months
        assert_eq!(format_age(now - Duration::days(60)), "2mo");

        // 364 days = 12 months
        assert_eq!(format_age(now - Duration::days(364)), "12mo");
    }

    #[test]
    fn test_format_age_years() {
        let now = OffsetDateTime::now_utc();

        // 365 days = 1 year
        assert_eq!(format_age(now - Duration::days(365)), "1y");

        // 730 days = 2 years
        assert_eq!(format_age(now - Duration::days(730)), "2y");
    }

    #[test]
    fn test_format_age_future_date() {
        let now = OffsetDateTime::now_utc();
        // Future date shows as 0m
        assert_eq!(format_age(now + Duration::days(2)), "0m");
    }
}
