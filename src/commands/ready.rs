//! brd ready command.

use crossterm::style::{Attribute, SetAttribute};

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
            let is_high_priority =
                issue.priority() == Priority::P0 || issue.priority() == Priority::P1;
            let use_color = !cli.no_color;

            if use_color {
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

            print!("{}  {}  {}", issue.id(), issue.priority(), issue.title());

            if use_color && (is_doing || is_high_priority || issue.issue_type().is_some()) {
                print!("{}", SetAttribute(Attribute::Reset));
            }
            println!();
        }
    }

    Ok(())
}
