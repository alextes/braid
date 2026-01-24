//! brd config show - display current configuration.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::repo::RepoPaths;

use super::{get_upstream, has_upstream};

/// Show current configuration.
pub fn cmd_config_show(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let config = Config::load(&paths.config_path())?;

    let auto_sync = config.auto_pull && config.auto_push;

    if cli.json {
        let json = serde_json::json!({
            "issues_branch": config.issues_branch,
            "external_repo": config.issues_repo,
            "auto_sync": auto_sync,
            "auto_pull": config.auto_pull,
            "auto_push": config.auto_push,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    // issues-branch setting
    if let Some(ref branch) = config.issues_branch {
        print!("issues-branch: {}", branch);
        if has_upstream(branch, &paths.worktree_root) {
            let upstream = get_upstream(branch, &paths.worktree_root).unwrap_or_default();
            println!(" (tracking {})", upstream);
        } else {
            println!(" (local only)");
        }
    } else {
        println!("issues-branch: (not set)");
    }

    // external-repo setting
    if let Some(ref external_path) = config.issues_repo {
        print!("external-repo: {}", external_path);
        let resolved = if std::path::Path::new(external_path).is_absolute() {
            std::path::PathBuf::from(external_path)
        } else {
            paths.worktree_root.join(external_path)
        };
        if let Ok(canonical) = resolved.canonicalize() {
            if canonical.to_string_lossy() != *external_path {
                println!(" ({})", canonical.display());
            } else {
                println!();
            }
        } else {
            println!(" (path not found)");
        }
    } else {
        println!("external-repo: (not set)");
    }

    // auto-sync setting
    if auto_sync {
        println!("auto-sync:     enabled");
    } else if !config.auto_pull && !config.auto_push {
        println!("auto-sync:     disabled");
    } else {
        // partial - show individual settings
        println!(
            "auto-sync:     partial (pull={}, push={})",
            config.auto_pull, config.auto_push
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::config::tests::{make_cli, make_paths, setup_git_repo};
    use std::fs;

    fn setup_braid_config(dir: &tempfile::TempDir, content: &str) {
        fs::create_dir_all(dir.path().join(".braid")).unwrap();
        fs::write(dir.path().join(".braid/config.toml"), content).unwrap();
    }

    #[test]
    fn test_config_show_git_native() {
        let dir = setup_git_repo();
        let paths = make_paths(&dir);
        let cli = make_cli();

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        let result = cmd_config_show(&cli, &paths);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[test]
    fn test_config_show_with_issues_branch() {
        let dir = setup_git_repo();
        let paths = make_paths(&dir);
        let cli = make_cli();

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_branch = \"braid-issues\"\n",
        );

        let result = cmd_config_show(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_show_with_external_repo() {
        let dir = setup_git_repo();
        let paths = make_paths(&dir);
        let cli = make_cli();

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_repo = \"../external-issues\"\n",
        );

        let result = cmd_config_show(&cli, &paths);
        assert!(result.is_ok());
    }
}
