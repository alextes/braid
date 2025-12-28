//! brd ready command.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::time::Instant;

use crossterm::style::{Attribute, Color, SetAttribute, SetForegroundColor};

use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::graph::get_ready_issues;
use crate::issue::{Issue, IssueType, Priority, Status};
use crate::repo::RepoPaths;

use super::{issue_to_json, load_all_issues};

fn format_ready_output(
    ready: &[&Issue],
    issues: &HashMap<String, Issue>,
    json: bool,
    use_color: bool,
    elapsed_ms: u128,
) -> String {
    if json {
        let json: Vec<_> = ready
            .iter()
            .map(|issue| issue_to_json(issue, issues))
            .collect();
        let mut output = serde_json::to_string_pretty(&json).unwrap();
        output.push('\n');
        return output;
    }

    let mut output = String::new();
    let open_count = ready.len();

    if ready.is_empty() {
        let _ = writeln!(output, "No ready issues.");
    }

    for issue in ready {
        let is_doing = issue.status() == Status::Doing;
        let priority_color = if use_color {
            match issue.priority() {
                Priority::P0 => Some(Color::Red),
                Priority::P1 => Some(Color::Yellow),
                _ => None,
            }
        } else {
            None
        };

        if use_color {
            if is_doing {
                let _ = write!(output, "{}", SetAttribute(Attribute::Underlined));
            }
            match issue.issue_type() {
                Some(IssueType::Design) => {
                    let _ = write!(output, "{}", SetAttribute(Attribute::Italic));
                }
                Some(IssueType::Meta) => {
                    let _ = write!(output, "{}", SetAttribute(Attribute::Bold));
                }
                None => {}
            }
        }

        let type_col = match issue.issue_type() {
            Some(IssueType::Design) => "design  ",
            Some(IssueType::Meta) => "meta    ",
            None => "        ",
        };

        let _ = write!(output, "{}  ", issue.id());
        if let Some(color) = priority_color {
            let _ = write!(output, "{}", SetForegroundColor(color));
        }
        let _ = write!(output, "{}", issue.priority());
        if priority_color.is_some() {
            let _ = write!(output, "{}", SetForegroundColor(Color::Reset));
        }
        let _ = write!(output, "  {}{}", type_col, issue.title());

        if use_color && (is_doing || issue.issue_type().is_some()) {
            let _ = write!(output, "{}", SetAttribute(Attribute::Reset));
        }
        let _ = writeln!(output);
    }

    let _ = writeln!(output, "open: {} | time: {}ms", open_count, elapsed_ms);

    output
}

pub fn cmd_ready(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let start = Instant::now();
    let config = Config::load(&paths.config_path())?;
    let issues = load_all_issues(paths, &config)?;
    let ready = get_ready_issues(&issues);

    let elapsed_ms = start.elapsed().as_millis();
    let output = format_ready_output(&ready, &issues, cli.json, !cli.no_color, elapsed_ms);
    print!("{output}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issue::{IssueType, Priority, Status};
    use time::{Duration, OffsetDateTime};

    fn make_issue(id: &str, priority: Priority, status: Status, deps: Vec<&str>) -> Issue {
        let mut issue = Issue::new(
            id.to_string(),
            format!("issue {id}"),
            priority,
            deps.into_iter().map(String::from).collect(),
        );
        issue.frontmatter.status = status;
        issue
    }

    #[test]
    fn test_format_ready_output_text_filters_ready() {
        let mut issues = HashMap::new();
        let ready_issue = make_issue("brd-ready", Priority::P2, Status::Todo, vec![]);
        let blocked_issue =
            make_issue("brd-blocked", Priority::P1, Status::Todo, vec!["brd-ready"]);
        let mut done_issue = make_issue("brd-done", Priority::P0, Status::Done, vec![]);
        done_issue.frontmatter.issue_type = Some(IssueType::Meta);

        issues.insert(ready_issue.id().to_string(), ready_issue.clone());
        issues.insert(blocked_issue.id().to_string(), blocked_issue);
        issues.insert(done_issue.id().to_string(), done_issue);

        let ready = get_ready_issues(&issues);
        let output = format_ready_output(&ready, &issues, false, false, 7);

        assert!(output.contains("brd-ready"));
        assert!(!output.contains("brd-blocked"));
        assert!(!output.contains("brd-done"));
        assert!(output.contains("open: 1 | time: 7ms"));
    }

    #[test]
    fn test_format_ready_output_json_ordering() {
        let now = OffsetDateTime::now_utc();
        let mut issues = HashMap::new();

        let mut issue_p2 = make_issue("brd-p2", Priority::P2, Status::Todo, vec![]);
        issue_p2.frontmatter.created_at = now - Duration::days(2);
        issue_p2.frontmatter.updated_at = issue_p2.frontmatter.created_at;

        let mut issue_p0 = make_issue("brd-p0", Priority::P0, Status::Todo, vec![]);
        issue_p0.frontmatter.created_at = now - Duration::days(1);
        issue_p0.frontmatter.updated_at = issue_p0.frontmatter.created_at;

        issues.insert(issue_p2.id().to_string(), issue_p2);
        issues.insert(issue_p0.id().to_string(), issue_p0);

        let ready = get_ready_issues(&issues);
        let output = format_ready_output(&ready, &issues, true, false, 0);
        let json: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

        assert_eq!(json[0]["id"], "brd-p0");
        assert_eq!(json[1]["id"], "brd-p2");
    }

    #[test]
    fn test_format_ready_output_no_ready_issues() {
        let mut issues = HashMap::new();
        let blocked_issue = make_issue("brd-blocked", Priority::P1, Status::Todo, vec!["brd-miss"]);
        issues.insert(blocked_issue.id().to_string(), blocked_issue);

        let ready = get_ready_issues(&issues);
        let output = format_ready_output(&ready, &issues, false, false, 0);

        assert!(output.contains("No ready issues."));
        assert!(output.contains("open: 0 | time: 0ms"));
    }
}
