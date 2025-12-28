//! brd mode command - show current workflow mode.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::repo::RepoPaths;

/// Check if a branch has an upstream tracking branch.
fn has_upstream(branch: &str, cwd: &std::path::Path) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", &format!("{}@{{u}}", branch)])
        .current_dir(cwd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the upstream tracking branch name.
fn get_upstream(branch: &str, cwd: &std::path::Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", &format!("{}@{{u}}", branch)])
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

pub fn cmd_mode(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let config = Config::load(&paths.config_path())?;

    if cli.json {
        let json = if let Some(ref branch) = config.sync_branch {
            let upstream = get_upstream(branch, &paths.worktree_root);
            serde_json::json!({
                "mode": "local-sync",
                "branch": branch,
                "upstream": upstream,
            })
        } else {
            serde_json::json!({
                "mode": "git-native",
            })
        };
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
        return Ok(());
    }

    if let Some(ref branch) = config.sync_branch {
        println!("Mode: local-sync");
        println!("Branch: {}", branch);

        if has_upstream(branch, &paths.worktree_root) {
            let upstream = get_upstream(branch, &paths.worktree_root).unwrap_or_default();
            println!("Remote: {} (tracking)", upstream);
        } else {
            println!("Remote: (none - local only)");
        }

        println!();
        println!("Issues sync via shared worktree. All local agents see changes instantly.");
        if has_upstream(branch, &paths.worktree_root) {
            println!("Remote sync: run `brd sync` to push/pull.");
        } else {
            println!("To enable remote sync: `brd sync --push`");
        }
    } else {
        println!("Mode: git-native (default)");
        println!();
        println!("Issues sync via git - merge to main, rebase to get updates.");
        println!("Good for: solo work, small teams, remote agents.");
    }

    Ok(())
}
