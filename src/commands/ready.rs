//! brd ready command.

use crossterm::style::{Attribute, Color, SetAttribute, SetForegroundColor};

use crate::cli::Cli;
use crate::error::Result;
use crate::graph::get_ready_issues;
use crate::issue::{IssueType, Priority, Status};
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
            let is_doing = issue.status() == Status::Doing;
            let use_color = !cli.no_color;
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

            // type column: "design", "meta", or padded empty (8 chars)
            let type_col = match issue.issue_type() {
                Some(IssueType::Design) => "design  ",
                Some(IssueType::Meta) => "meta    ",
                None => "        ",
            };

            print!(
                "{}  ",
                issue.id()
            );
            if let Some(color) = priority_color {
                print!("{}", SetForegroundColor(color));
            }
            print!("{}", issue.priority());
            if priority_color.is_some() {
                print!("{}", SetForegroundColor(Color::Reset));
            }
            print!("  {}{}", type_col, issue.title());

            if use_color && (is_doing || issue.issue_type().is_some()) {
                print!("{}", SetAttribute(Attribute::Reset));
            }
            println!();
        }
    }

    Ok(())
}
