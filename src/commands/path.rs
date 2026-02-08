//! brd path command - print the file path of an issue.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::repo::RepoPaths;

use super::{load_all_issues, resolve_issue_id};

/// Print the absolute file path of an issue.
pub fn cmd_path(cli: &Cli, paths: &RepoPaths, id: &str) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(id, &issues)?;

    // verify the issue exists
    if !issues.contains_key(&full_id) {
        return Err(BrdError::IssueNotFound(id.to_string()));
    }

    let issue_path = paths.issues_dir(&config).join(format!("{}.md", full_id));

    // canonicalize to absolute path, fall back to the computed path if it doesn't exist yet
    let absolute_path = issue_path
        .canonicalize()
        .unwrap_or_else(|_| issue_path.clone());

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "id": full_id,
            "path": absolute_path.to_string_lossy(),
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("{}", absolute_path.display());
    }

    Ok(())
}
