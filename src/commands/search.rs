//! brd search command - prints instructions for searching issues.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::repo::RepoPaths;

pub fn cmd_search(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let issues_dir = paths.issues_dir(&config);

    if cli.json {
        let json = serde_json::json!({
            "issues_dir": issues_dir.to_string_lossy(),
            "hint": "use grep or rg to search issue files"
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("braid issues are plain markdown files. use grep or rg to search:");
        println!();
        println!("  rg <pattern> {}", issues_dir.display());
        println!("  grep -r <pattern> {}", issues_dir.display());
        println!();
        println!("examples:");
        println!(
            "  rg 'authentication' {}   # search for 'authentication'",
            issues_dir.display()
        );
        println!(
            "  rg -l 'P0' {}             # list P0 issues",
            issues_dir.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_repo() -> (tempfile::TempDir, RepoPaths) {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Create .braid directory with config
        let braid_dir = repo_path.join(".braid");
        let issues_dir = braid_dir.join("issues");
        std::fs::create_dir_all(&issues_dir).unwrap();

        std::fs::write(
            braid_dir.join("config.toml"),
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        )
        .unwrap();

        let paths = RepoPaths {
            worktree_root: repo_path.to_path_buf(),
            git_common_dir: repo_path.join(".git"),
            brd_common_dir: repo_path.join(".git/brd"),
        };
        std::fs::create_dir_all(&paths.brd_common_dir).unwrap();

        (dir, paths)
    }

    fn make_cli(json: bool) -> Cli {
        Cli {
            json,
            repo: None,
            no_color: true,
            verbose: false,
            command: crate::cli::Command::Doctor,
        }
    }

    #[test]
    fn test_search_returns_ok() {
        let (_dir, paths) = create_repo();
        let cli = make_cli(false);

        let result = cmd_search(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_json_mode() {
        let (_dir, paths) = create_repo();
        let cli = make_cli(true);

        // Just verify it doesn't error - output goes to stdout
        let result = cmd_search(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_with_sync_branch_mode() {
        let (dir, paths) = create_repo();

        // Update config to use sync branch
        std::fs::write(
            dir.path().join(".braid/config.toml"),
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_branch = \"braid-issues\"\n",
        )
        .unwrap();

        let cli = make_cli(false);
        let result = cmd_search(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_fails_without_config() {
        let dir = tempdir().unwrap();
        let paths = RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };

        let cli = make_cli(false);
        let result = cmd_search(&cli, &paths);
        assert!(result.is_err());
    }
}
