//! brd config external-repo - set or clear the external-repo setting.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::git;
use crate::repo::RepoPaths;

use super::{confirm, count_issues, find_agent_worktrees_needing_rebase, warn_agent_worktrees};

/// Set or clear the external-repo setting.
pub fn cmd_config_external_repo(
    cli: &Cli,
    paths: &RepoPaths,
    path: Option<&str>,
    clear: bool,
    yes: bool,
) -> Result<()> {
    // Handle clear case
    if clear {
        return clear_external_repo(cli, paths, yes);
    }

    // Handle set case
    let external_path = match path {
        Some(p) => p,
        None => {
            return Err(BrdError::Other(
                "must provide path or use --clear".to_string(),
            ));
        }
    };

    use crate::repo::discover;

    let mut config = Config::load(&paths.config_path())?;

    // check if already set to this path
    if config.issues_repo.as_deref() == Some(external_path) {
        if !cli.json {
            println!("external-repo already set to '{}'", external_path);
        }
        return Ok(());
    }

    // check if already set to a different path
    if let Some(existing) = &config.issues_repo {
        return Err(BrdError::Other(format!(
            "external-repo already set to '{}'. run `brd config external-repo --clear` first.",
            existing
        )));
    }

    // check if issues_branch is set
    if config.issues_branch.is_some() {
        return Err(BrdError::Other(
            "issues-branch is set. run `brd config issues-branch --clear` first.".to_string(),
        ));
    }

    // resolve the external path
    let resolved = if std::path::Path::new(external_path).is_absolute() {
        std::path::PathBuf::from(external_path)
    } else {
        paths.worktree_root.join(external_path)
    };

    // verify external repo exists
    let canonical = resolved.canonicalize().map_err(|_| {
        BrdError::Other(format!(
            "external repo path does not exist: {}",
            resolved.display()
        ))
    })?;

    // verify it's a braid repo (has .braid/config.toml)
    let external_paths = discover(Some(&canonical)).map_err(|_| {
        BrdError::Other(format!(
            "path is not a git repository: {}",
            canonical.display()
        ))
    })?;

    let external_config_path = external_paths.config_path();
    if !external_config_path.exists() {
        return Err(BrdError::Other(format!(
            "external repo is not initialized with braid. run `brd init` in {}",
            canonical.display()
        )));
    }

    // load external config to verify it's valid and count issues
    let external_config = Config::load(&external_config_path)
        .map_err(|e| BrdError::Other(format!("failed to load external repo config: {}", e)))?;

    // confirmation prompt (unless -y or --json)
    if !yes && !cli.json {
        let external_issues_dir = external_paths.issues_dir(&external_config);
        let issue_count = count_issues(&external_issues_dir);

        println!("Setting external-repo to '{}'...", external_path);
        println!();
        println!("This will:");
        println!(
            "  • Point this repo to use issues from {}",
            canonical.display()
        );
        if issue_count > 0 {
            println!("  • {} issue(s) available in external repo", issue_count);
        }
        println!("  • Local .braid/issues/ will be ignored");
        println!("  • Commit the config change");
        println!();

        if !confirm("Continue?")? {
            println!("Aborted.");
            return Ok(());
        }
        println!();
    }

    if !cli.json {
        println!("Setting external-repo...");
    }

    // update config
    config.issues_repo = Some(external_path.to_string());
    config.save(&paths.config_path())?;

    // commit the config change
    if !git::run(&["add", ".braid/config.toml"], &paths.worktree_root)? {
        return Err(BrdError::Other("failed to stage config change".to_string()));
    }

    let commit_msg = format!("chore(braid): set external-repo to '{}'", external_path);
    let _ = git::run(&["commit", "-m", &commit_msg], &paths.worktree_root);

    // check for agent worktrees needing rebase
    let agent_worktrees = find_agent_worktrees_needing_rebase(&paths.worktree_root);

    if cli.json {
        let worktrees_json: Vec<_> = agent_worktrees
            .iter()
            .map(|wt| {
                serde_json::json!({
                    "branch": wt.branch,
                    "path": wt.path.to_string_lossy(),
                })
            })
            .collect();

        let json = serde_json::json!({
            "ok": true,
            "external_repo": external_path,
            "resolved": canonical.to_string_lossy(),
            "agent_worktrees_needing_rebase": worktrees_json,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!();
        println!("external-repo set to '{}'", external_path);
        println!("Issues now tracked in: {}", canonical.display());

        warn_agent_worktrees(&agent_worktrees);
    }

    Ok(())
}

/// Clear the external-repo setting.
fn clear_external_repo(cli: &Cli, paths: &RepoPaths, yes: bool) -> Result<()> {
    let mut config = Config::load(&paths.config_path())?;

    // check if external_repo is set
    let external_path = match &config.issues_repo {
        Some(p) => p.clone(),
        None => {
            if !cli.json {
                println!("external-repo is not set");
            }
            return Ok(());
        }
    };

    // confirmation prompt (unless -y or --json)
    if !yes && !cli.json {
        println!("Clearing external-repo setting...");
        println!();
        println!("This will:");
        println!("  • Remove external repo reference from config");
        println!(
            "  • Issues will remain in external repo at {}",
            external_path
        );
        println!("  • You'll need to manually copy issues if you want them locally");
        println!("  • Commit the config change");
        println!();

        if !confirm("Continue?")? {
            println!("Aborted.");
            return Ok(());
        }
        println!();
    }

    if !cli.json {
        println!("Clearing external-repo...");
    }

    // update config
    config.issues_repo = None;
    config.save(&paths.config_path())?;

    // commit the config change
    if git::run(&["add", ".braid/config.toml"], &paths.worktree_root)? {
        let commit_msg = format!(
            "chore(braid): clear external-repo (was '{}')",
            external_path
        );
        let _ = git::run(&["commit", "-m", &commit_msg], &paths.worktree_root);
    }

    let agent_worktrees = find_agent_worktrees_needing_rebase(&paths.worktree_root);

    if cli.json {
        let worktrees_json: Vec<_> = agent_worktrees
            .iter()
            .map(|wt| {
                serde_json::json!({
                    "branch": wt.branch,
                    "path": wt.path.to_string_lossy(),
                })
            })
            .collect();

        let json = serde_json::json!({
            "ok": true,
            "external_repo": serde_json::Value::Null,
            "from_path": external_path,
            "agent_worktrees_needing_rebase": worktrees_json,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!();
        println!("external-repo cleared");
        println!("Note: issues remain in external repo at {}", external_path);
        println!("You'll need to manually copy issues if you want them locally.");

        warn_agent_worktrees(&agent_worktrees);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::config::tests::{make_cli, make_paths, setup_git_repo};
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    fn setup_braid_config(dir: &tempfile::TempDir, content: &str) {
        fs::create_dir_all(dir.path().join(".braid")).unwrap();
        fs::write(dir.path().join(".braid/config.toml"), content).unwrap();
    }

    #[test]
    fn test_config_external_repo_issues_branch_set() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_branch = \"issues\"\n",
        );

        let result = cmd_config_external_repo(&cli, &paths, Some("../external"), false, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("issues-branch is set")
        );
    }

    #[test]
    fn test_config_external_repo_already_set() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_repo = \"../existing\"\n",
        );

        let result = cmd_config_external_repo(&cli, &paths, Some("../new-external"), false, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("external-repo already set")
        );
    }

    #[test]
    fn test_config_external_repo_nonexistent_path() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        let result = cmd_config_external_repo(&cli, &paths, Some("/nonexistent/path"), false, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_config_external_repo_clear_when_not_set() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // clearing when not set should succeed (no-op)
        let result = cmd_config_external_repo(&cli, &paths, None, true, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_external_repo_success() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // create external repo
        let external_dir = tempdir().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(external_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(external_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(external_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(external_dir.path())
            .output()
            .unwrap();

        // initialize braid in external repo
        fs::create_dir_all(external_dir.path().join(".braid/issues")).unwrap();
        fs::write(
            external_dir.path().join(".braid/config.toml"),
            "schema_version = 6\nid_prefix = \"ext\"\nid_len = 4\n",
        )
        .unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(external_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init braid"])
            .current_dir(external_dir.path())
            .output()
            .unwrap();

        // set up main repo
        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );
        Command::new("git")
            .args(["add", ".braid"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add braid config"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // set external-repo
        let external_path = external_dir.path().to_string_lossy().to_string();
        let result = cmd_config_external_repo(&cli, &paths, Some(&external_path), false, true);
        assert!(result.is_ok());

        // verify config was updated
        let config = Config::load(&paths.config_path()).unwrap();
        assert!(config.issues_repo.is_some());
    }

    #[test]
    fn test_clear_external_repo_success() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // set up config with external_repo set
        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_repo = \"../external\"\n",
        );
        Command::new("git")
            .args(["add", ".braid"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add braid config"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // clear external-repo
        let result = cmd_config_external_repo(&cli, &paths, None, true, true);
        assert!(result.is_ok());

        // verify config was updated
        let config = Config::load(&paths.config_path()).unwrap();
        assert!(config.issues_repo.is_none());
    }
}
