//! brd add command.

use crate::cli::{AddArgs, Cli};
use crate::config::Config;
use crate::error::Result;
use crate::issue::{Issue, IssueType, Priority};
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{generate_issue_id, issue_to_json, load_all_issues, resolve_issue_id};

pub fn cmd_add(cli: &Cli, paths: &RepoPaths, args: &AddArgs) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let priority: Priority = args.priority.parse()?;
    let issue_type: Option<IssueType> = args.r#type.as_deref().map(|s| s.parse()).transpose()?;

    // resolve deps to full IDs
    let all_issues = load_all_issues(paths)?;
    let resolved_deps: Vec<String> = args
        .dep
        .iter()
        .map(|d| resolve_issue_id(d, &all_issues))
        .collect::<Result<Vec<_>>>()?;

    // generate ID
    let id = generate_issue_id(&config, &paths.issues_dir())?;

    // create issue
    let mut issue = Issue::new(id.clone(), args.title.clone(), priority, resolved_deps);
    issue.frontmatter.issue_type = issue_type;
    issue.frontmatter.acceptance = args.ac.clone();
    issue.frontmatter.labels = args.label.clone();
    if let Some(ref b) = args.body {
        issue.body = b.clone();
    }

    // save with lock
    let _lock = LockGuard::acquire(&paths.lock_path())?;
    let issue_path = paths.issues_dir().join(format!("{}.md", id));
    issue.save(&issue_path)?;

    // dual-write: also save to local worktree if different from control root
    if paths.worktree_root != paths.control_root {
        let local_issue_path = paths
            .worktree_root
            .join(".braid/issues")
            .join(format!("{}.md", id));
        issue.save(&local_issue_path)?;
    }

    if cli.json {
        let json = issue_to_json(&issue, &all_issues);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Created issue: {}", id);
        println!("  {}", issue_path.display());
    }

    Ok(())
}
