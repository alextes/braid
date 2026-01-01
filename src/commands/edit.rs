//! brd edit command - open an issue in $EDITOR.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::issue::Status;
use crate::repo::{self, RepoPaths};

use super::{load_all_issues, resolve_issue_id};

/// Open an issue in $EDITOR.
pub fn cmd_edit(cli: &Cli, paths: &RepoPaths, id: Option<&str>) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let issues = load_all_issues(paths, &config)?;

    // resolve issue ID
    let full_id = match id {
        Some(partial) => resolve_issue_id(partial, &issues)?,
        None => {
            // find current "doing" issue owned by this agent
            let agent_id = repo::get_agent_id(&paths.worktree_root);
            let doing: Vec<_> = issues
                .values()
                .filter(|i| {
                    i.status() == Status::Doing && i.frontmatter.owner.as_deref() == Some(&agent_id)
                })
                .collect();

            match doing.len() {
                0 => {
                    return Err(BrdError::Other(
                        "no issue in progress. specify an issue ID or run `brd start` first"
                            .to_string(),
                    ));
                }
                1 => doing[0].id().to_string(),
                _ => {
                    let ids: Vec<_> = doing.iter().map(|i| i.id()).collect();
                    return Err(BrdError::Other(format!(
                        "multiple issues in progress: {}. specify which to edit",
                        ids.join(", ")
                    )));
                }
            }
        }
    };

    // get issue path
    let issue_path = paths.issues_dir(&config).join(format!("{}.md", full_id));

    if !issue_path.exists() {
        return Err(BrdError::Other(format!(
            "issue file not found: {}",
            issue_path.display()
        )));
    }

    // get editor from $EDITOR or $VISUAL
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .map_err(|_| {
            BrdError::Other("$EDITOR or $VISUAL not set. set one to use `brd edit`".to_string())
        })?;

    if cli.json {
        let json = serde_json::json!({
            "id": full_id,
            "path": issue_path.to_string_lossy(),
            "editor": editor,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    // open editor
    let status = std::process::Command::new(&editor)
        .arg(&issue_path)
        .status()?;

    if !status.success() {
        return Err(BrdError::Other(format!(
            "editor '{}' exited with status {}",
            editor,
            status.code().unwrap_or(-1)
        )));
    }

    Ok(())
}
