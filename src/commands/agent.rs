//! brd agent commands.

use std::path::PathBuf;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::repo::RepoPaths;

pub fn cmd_agent_init(cli: &Cli, paths: &RepoPaths, name: &str, base: Option<&str>) -> Result<()> {
    // validate agent name (alphanumeric + hyphens)
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(BrdError::Other(format!(
            "invalid agent name '{}': use only alphanumeric, hyphens, underscores",
            name
        )));
    }

    // determine worktree path (~/.braid/worktrees/<repo-name>/<agent-name>)
    let home_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| BrdError::Other("cannot determine home directory".to_string()))?;
    let repo_name = paths
        .worktree_root
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| BrdError::Other("cannot determine repo name".to_string()))?;
    let worktrees_dir = home_dir.join(".braid").join("worktrees").join(repo_name);
    let worktree_path = worktrees_dir.join(name);

    // ensure parent directories exist
    std::fs::create_dir_all(&worktrees_dir)?;

    // check if worktree already exists
    if worktree_path.exists() {
        return Err(BrdError::Other(format!(
            "directory already exists: {}",
            worktree_path.display()
        )));
    }

    // get base branch (default to current branch)
    let base_branch = match base {
        Some(b) => b.to_string(),
        None => {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&paths.worktree_root)
                .output()?;
            if !output.status.success() {
                return Err(BrdError::Other("failed to get current branch".to_string()));
            }
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
    };

    // create worktree with new branch
    let output = std::process::Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            name,
            worktree_path.to_str().unwrap(),
            &base_branch,
        ])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BrdError::Other(format!(
            "failed to create worktree: {}",
            stderr.trim()
        )));
    }

    // create .braid directory in new worktree (for agent.toml)
    let new_braid_dir = worktree_path.join(".braid");
    std::fs::create_dir_all(&new_braid_dir)?;

    // create agent.toml
    let agent_toml_path = new_braid_dir.join("agent.toml");
    let agent_toml_content = format!("agent_id = \"{}\"\n", name);
    std::fs::write(&agent_toml_path, agent_toml_content)?;

    // check for sync branch mode and ensure issues worktree exists
    let config = Config::load(&paths.config_path()).ok();
    let sync_branch = config.as_ref().and_then(|c| c.sync_branch.clone());

    if let Some(ref branch) = sync_branch {
        // ensure the shared issues worktree exists
        if let Err(e) = paths.ensure_issues_worktree(branch) {
            eprintln!("warning: failed to ensure issues worktree: {}", e);
        }
    }

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "agent_id": name,
            "worktree": worktree_path.to_string_lossy(),
            "branch": name,
            "base": base_branch,
            "sync_branch": sync_branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Created agent worktree: {}", name);
        println!("  path:   {}", worktree_path.display());
        println!("  branch: {} (from {})", name, base_branch);
        if let Some(sb) = &sync_branch {
            println!("  sync:   issues on '{}' branch", sb);
        }
        println!();
        println!("To use this agent:");
        println!("  cd {}", worktree_path.display());
        println!("  # start your agent (claude, codex, gemini, etc.)");
        println!("  brd next  # get next issue to work on");
        if sync_branch.is_some() {
            println!("  brd sync  # sync issues with remote");
        }
    }

    Ok(())
}
