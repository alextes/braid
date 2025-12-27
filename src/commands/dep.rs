//! brd dep add/rm commands.

use crate::cli::Cli;
use crate::error::{BrdError, Result};
use crate::graph::would_create_cycle;
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{load_all_issues, resolve_issue_id};

pub fn cmd_dep_add(cli: &Cli, paths: &RepoPaths, child_id: &str, parent_id: &str) -> Result<()> {
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths)?;
    let child_full = resolve_issue_id(child_id, &issues)?;
    let parent_full = resolve_issue_id(parent_id, &issues)?;

    // check not self-dep
    if child_full == parent_full {
        return Err(BrdError::Other("cannot add self-dependency".to_string()));
    }

    // check for cycles
    if let Some(cycle_path) = would_create_cycle(&child_full, &parent_full, &issues) {
        let cycle_str = cycle_path.join(" -> ");
        return Err(BrdError::Other(format!(
            "cannot add dependency: would create cycle: {}",
            cycle_str
        )));
    }

    let child = issues
        .get_mut(&child_full)
        .ok_or_else(|| BrdError::IssueNotFound(child_id.to_string()))?;

    if !child.frontmatter.deps.contains(&parent_full) {
        child.frontmatter.deps.push(parent_full.clone());
        child.touch();
        let issue_path = paths.issues_dir().join(format!("{}.md", child_full));
        child.save(&issue_path)?;
    }

    if cli.json {
        println!(r#"{{"ok": true}}"#);
    } else {
        println!("Added dependency: {} -> {}", child_full, parent_full);
    }

    Ok(())
}

pub fn cmd_dep_rm(cli: &Cli, paths: &RepoPaths, child_id: &str, parent_id: &str) -> Result<()> {
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths)?;
    let child_full = resolve_issue_id(child_id, &issues)?;
    let parent_full = resolve_issue_id(parent_id, &issues)?;

    let child = issues
        .get_mut(&child_full)
        .ok_or_else(|| BrdError::IssueNotFound(child_id.to_string()))?;

    child.frontmatter.deps.retain(|d| d != &parent_full);
    child.touch();
    let issue_path = paths.issues_dir().join(format!("{}.md", child_full));
    child.save(&issue_path)?;

    if cli.json {
        println!(r#"{{"ok": true}}"#);
    } else {
        println!("Removed dependency: {} -> {}", child_full, parent_full);
    }

    Ok(())
}
