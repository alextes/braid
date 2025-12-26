//! command implementations for the brd CLI.

mod add;
mod agent;
mod completions;
mod dep;
mod doctor;
mod done;
mod init;
mod ls;
mod next;
mod ready;
mod show;
mod start;

pub use add::cmd_add;
pub use agent::cmd_agent_init;
pub use completions::cmd_completions;
pub use dep::{cmd_dep_add, cmd_dep_rm};
pub use doctor::cmd_doctor;
pub use done::cmd_done;
pub use init::cmd_init;
pub use ls::cmd_ls;
pub use next::cmd_next;
pub use ready::cmd_ready;
pub use show::cmd_show;
pub use start::cmd_start;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::graph::compute_derived;
use crate::issue::Issue;
use crate::repo::RepoPaths;
use rand::Rng;

/// load all issues from the issues directory.
pub(crate) fn load_all_issues(paths: &RepoPaths) -> Result<HashMap<String, Issue>> {
    let mut issues = HashMap::new();
    let issues_dir = paths.issues_dir();

    if !issues_dir.exists() {
        return Ok(issues);
    }

    for entry in std::fs::read_dir(&issues_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "md") {
            match Issue::load(&path) {
                Ok(issue) => {
                    issues.insert(issue.id().to_string(), issue);
                }
                Err(e) => {
                    let err_str = e.to_string();
                    eprintln!("warning: failed to load {}: {}", path.display(), e);
                    if err_str.contains("invalid type: map, expected a string") {
                        eprintln!(
                            "  hint: strings containing colons must be quoted, e.g. '- \"foo: bar\"'"
                        );
                    }
                }
            }
        }
    }

    Ok(issues)
}

/// resolve a partial issue ID to a full ID.
pub(crate) fn resolve_issue_id(partial: &str, issues: &HashMap<String, Issue>) -> Result<String> {
    // exact match
    if issues.contains_key(partial) {
        return Ok(partial.to_string());
    }

    // partial match
    let matches: Vec<&str> = issues
        .keys()
        .filter(|id| id.contains(partial) || id.ends_with(partial))
        .map(|s| s.as_str())
        .collect();

    match matches.len() {
        0 => Err(BrdError::IssueNotFound(partial.to_string())),
        1 => Ok(matches[0].to_string()),
        _ => Err(BrdError::AmbiguousId(
            partial.to_string(),
            matches.into_iter().map(String::from).collect(),
        )),
    }
}

/// generate a unique issue ID.
pub(crate) fn generate_issue_id(config: &Config, issues_dir: &Path) -> Result<String> {
    let charset: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();

    for _ in 0..20 {
        let suffix: String = (0..config.id_len)
            .map(|_| {
                let idx = rng.gen_range(0..charset.len());
                charset[idx] as char
            })
            .collect();

        let id = format!("{}-{}", config.id_prefix, suffix);
        let path = issues_dir.join(format!("{}.md", id));

        if !path.exists() {
            return Ok(id);
        }
    }

    Err(BrdError::Other(
        "failed to generate unique ID after 20 attempts".to_string(),
    ))
}

/// convert an issue to JSON format.
pub(crate) fn issue_to_json(
    issue: &Issue,
    all_issues: &HashMap<String, Issue>,
) -> serde_json::Value {
    let derived = compute_derived(issue, all_issues);

    serde_json::json!({
        "id": issue.id(),
        "title": issue.title(),
        "priority": issue.priority().to_string(),
        "status": issue.status().to_string(),
        "deps": issue.deps(),
        "labels": issue.labels(),
        "owner": issue.frontmatter.owner,
        "created_at": issue.frontmatter.created_at.format(&time::format_description::well_known::Rfc3339).unwrap(),
        "updated_at": issue.frontmatter.updated_at.format(&time::format_description::well_known::Rfc3339).unwrap(),
        "acceptance": issue.frontmatter.acceptance,
        "derived": {
            "is_ready": derived.is_ready,
            "open_deps": derived.open_deps,
            "missing_deps": derived.missing_deps,
            "is_blocked": derived.is_blocked
        }
    })
}

/// run `git rev-parse <arg>` and return the result as a PathBuf.
pub(crate) fn git_rev_parse(cwd: &Path, arg: &str) -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg(arg)
        .current_dir(cwd)
        .output()?;

    if !output.status.success() {
        return Err(BrdError::NotGitRepo);
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path_str))
}
