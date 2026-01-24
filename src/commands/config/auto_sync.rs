//! brd config auto-sync - enable or disable auto-sync.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::git;
use crate::repo::RepoPaths;

/// Set auto-sync (auto_pull and auto_push) on or off.
pub fn cmd_config_auto_sync(cli: &Cli, paths: &RepoPaths, enabled: bool) -> Result<()> {
    let mut config = Config::load(&paths.config_path())?;

    let already_set = config.auto_pull == enabled && config.auto_push == enabled;
    if already_set {
        if !cli.json {
            let status = if enabled { "enabled" } else { "disabled" };
            println!("auto-sync already {}", status);
        }
        return Ok(());
    }

    config.auto_pull = enabled;
    config.auto_push = enabled;
    config.save(&paths.config_path())?;

    // commit the config change
    if git::run(&["add", ".braid/config.toml"], &paths.worktree_root)? {
        let status = if enabled { "enabled" } else { "disabled" };
        let commit_msg = format!("chore(braid): set auto-sync to {}", status);
        let _ = git::run(&["commit", "-m", &commit_msg], &paths.worktree_root);
    }

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "auto_sync": enabled,
            "auto_pull": enabled,
            "auto_push": enabled,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        let status = if enabled { "enabled" } else { "disabled" };
        println!("auto-sync {}", status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::config::tests::{make_cli, make_paths, setup_git_repo};
    use std::fs;
    use std::process::Command;

    fn setup_braid_config(dir: &tempfile::TempDir, content: &str) {
        fs::create_dir_all(dir.path().join(".braid")).unwrap();
        fs::write(dir.path().join(".braid/config.toml"), content).unwrap();
    }

    #[test]
    fn test_config_auto_sync_enable() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nauto_pull = false\nauto_push = false\n",
        );

        // commit the .braid directory
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

        let result = cmd_config_auto_sync(&cli, &paths, true);
        assert!(result.is_ok());

        // verify config was updated
        let config = Config::load(&paths.config_path()).unwrap();
        assert!(config.auto_pull);
        assert!(config.auto_push);
    }

    #[test]
    fn test_config_auto_sync_disable() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nauto_pull = true\nauto_push = true\n",
        );

        // commit the .braid directory
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

        let result = cmd_config_auto_sync(&cli, &paths, false);
        assert!(result.is_ok());

        // verify config was updated
        let config = Config::load(&paths.config_path()).unwrap();
        assert!(!config.auto_pull);
        assert!(!config.auto_push);
    }
}
