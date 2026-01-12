//! command implementations for the brd CLI.

mod add;
mod agent;
mod commit;
mod completions;
mod config;
mod dep;
mod doctor;
mod done;
mod edit;
mod init;
mod ls;
mod merge;
mod migrate;
mod ready;
mod rm;
mod search;
mod set;
mod show;
mod skip;
mod start;
mod status;
mod sync;
mod tui;

pub use add::cmd_add;
pub use agent::{
    // AGENTS.md instructions
    AGENTS_BLOCK_VERSION,
    AgentsBlockMode,
    INSTRUCTION_FILES,
    check_agents_block,
    cmd_agent_branch,
    cmd_agent_init,
    cmd_agent_pr,
    cmd_agents_inject,
    cmd_agents_show,
    extract_mode,
    generate_block,
};
pub use commit::cmd_commit;
pub use completions::cmd_completions;
pub use config::{
    cmd_config_auto_sync, cmd_config_external_repo, cmd_config_issues_branch, cmd_config_show,
};
pub use dep::{cmd_dep_add, cmd_dep_rm};
pub use doctor::cmd_doctor;
pub use done::cmd_done;
pub use edit::cmd_edit;
pub use init::cmd_init;
pub use ls::cmd_ls;
pub use merge::cmd_merge;
pub use migrate::cmd_migrate;
pub use ready::cmd_ready;
pub use rm::cmd_rm;
pub use search::cmd_search;
pub use set::cmd_set;
pub use show::cmd_show;
pub use skip::cmd_skip;
pub use start::{
    claim_issue, cmd_start, commit_and_push_issues_branch, commit_and_push_main, sync_with_main,
};
pub use status::cmd_status;
pub use sync::cmd_sync;
pub use tui::cmd_tui;

use std::collections::HashMap;

use crate::config::Config;
use crate::error::Result;
use crate::graph::{compute_derived, get_dependents};
use crate::issue::Issue;
use crate::repo::RepoPaths;

// re-export functions from issue for use by command modules
pub(crate) use crate::issue::{generate_issue_id, resolve_issue_id};

/// load all issues from the issues directory.
pub(crate) fn load_all_issues(
    paths: &RepoPaths,
    config: &Config,
) -> Result<HashMap<String, Issue>> {
    let mut issues = HashMap::new();
    let issues_dir = paths.issues_dir(config);

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

/// convert an issue to JSON format.
pub(crate) fn issue_to_json(
    issue: &Issue,
    all_issues: &HashMap<String, Issue>,
) -> serde_json::Value {
    let derived = compute_derived(issue, all_issues);
    let dependents = get_dependents(issue.id(), all_issues);

    serde_json::json!({
        "id": issue.id(),
        "title": issue.title(),
        "priority": issue.priority().to_string(),
        "status": issue.status().to_string(),
        "type": issue.frontmatter.issue_type.as_ref().map(|t| t.to_string()),
        "deps": issue.deps(),
        "dependents": dependents,
        "tags": issue.tags(),
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
