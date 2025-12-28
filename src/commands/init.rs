//! brd init command.

use crate::cli::{Cli, InitArgs};
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::repo::git_rev_parse;

pub fn cmd_init(cli: &Cli, args: &InitArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // discover git info
    let worktree_root = git_rev_parse(&cwd, "--show-toplevel")?;
    let git_common_dir_str = git_rev_parse(&cwd, "--git-common-dir")?;
    let git_common_dir = if git_common_dir_str.is_absolute() {
        git_common_dir_str
    } else {
        cwd.join(&git_common_dir_str)
            .canonicalize()
            .unwrap_or(git_common_dir_str)
    };

    let braid_dir = worktree_root.join(".braid");
    let issues_dir = braid_dir.join("issues");
    let config_path = braid_dir.join("config.toml");
    let gitignore_path = braid_dir.join(".gitignore");

    let brd_common_dir = git_common_dir.join("brd");

    // create directories
    std::fs::create_dir_all(&issues_dir)?;
    std::fs::create_dir_all(&brd_common_dir)?;

    // create config if missing
    let repo_name = worktree_root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("brd");

    if !config_path.exists() {
        let mut config = Config::with_derived_prefix(repo_name);
        config.sync_branch = args.sync_branch.clone();
        config.save(&config_path)?;
    } else if args.sync_branch.is_some() {
        // update existing config with sync_branch
        let mut config = Config::load(&config_path)?;
        config.sync_branch = args.sync_branch.clone();
        config.save(&config_path)?;
    }

    // create/update .gitignore
    let gitignore_content = "agent.toml\nruntime/\n";
    std::fs::write(&gitignore_path, gitignore_content)?;

    // create agent.toml if missing (with $USER as default agent_id)
    let agent_toml_path = braid_dir.join("agent.toml");
    if !agent_toml_path.exists() {
        let user = match std::env::var("USER") {
            Ok(u) => u,
            Err(_) => {
                eprintln!("warning: $USER not set, using 'default-user' as agent_id");
                "default-user".to_string()
            }
        };
        let agent_toml_content = format!("agent_id = \"{}\"\n", user);
        std::fs::write(&agent_toml_path, agent_toml_content)?;
    }

    // if sync_branch is set, create the branch and worktree
    if let Some(branch_name) = &args.sync_branch {
        setup_sync_branch(
            &worktree_root,
            &brd_common_dir,
            branch_name,
            &braid_dir,
            cli.json,
        )?;
    }

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "braid_dir": braid_dir.to_string_lossy(),
            "worktree": worktree_root.to_string_lossy(),
            "sync_branch": args.sync_branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Initialized braid in {}", braid_dir.display());
        if let Some(branch) = &args.sync_branch {
            println!("Sync branch mode enabled: issues will live on '{}'", branch);
        }
        println!();
        println!("next steps:");
        println!("  brd add \"my first task\"     # create an issue");
        println!("  brd agents inject           # add agent instructions to AGENTS.md");
        if args.sync_branch.is_some() {
            println!("  brd sync                    # sync issues to remote");
        }
    }

    Ok(())
}

/// Set up sync branch mode by creating the branch and issues worktree.
fn setup_sync_branch(
    worktree_root: &std::path::Path,
    brd_common_dir: &std::path::Path,
    branch_name: &str,
    braid_dir: &std::path::Path,
    json: bool,
) -> Result<()> {
    let issues_wt_path = brd_common_dir.join("issues");

    // 1. check if branch exists, create if not
    let branch_exists = std::process::Command::new("git")
        .args(["rev-parse", "--verify", branch_name])
        .current_dir(worktree_root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !branch_exists {
        // create the branch from current HEAD
        let output = std::process::Command::new("git")
            .args(["branch", branch_name])
            .current_dir(worktree_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrdError::Other(format!(
                "failed to create sync branch '{}': {}",
                branch_name, stderr
            )));
        }

        if !json {
            println!("Created sync branch '{}'", branch_name);
        }
    }

    // 2. create issues worktree if it doesn't exist
    if !issues_wt_path.exists() {
        let output = std::process::Command::new("git")
            .args(["worktree", "add"])
            .arg(&issues_wt_path)
            .arg(branch_name)
            .current_dir(worktree_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrdError::Other(format!(
                "failed to create issues worktree: {}",
                stderr
            )));
        }

        if !json {
            println!("Created issues worktree at {}", issues_wt_path.display());
        }

        // 3. copy existing issues to the worktree if any exist
        let local_issues_dir = braid_dir.join("issues");
        let wt_braid_dir = issues_wt_path.join(".braid");
        let wt_issues_dir = wt_braid_dir.join("issues");

        std::fs::create_dir_all(&wt_issues_dir)?;

        if local_issues_dir.exists() {
            for entry in std::fs::read_dir(&local_issues_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "md") {
                    let dest = wt_issues_dir.join(path.file_name().unwrap());
                    std::fs::copy(&path, &dest)?;
                }
            }
        }

        // 4. copy config to worktree
        let config_src = braid_dir.join("config.toml");
        let config_dest = wt_braid_dir.join("config.toml");
        if config_src.exists() {
            std::fs::copy(&config_src, &config_dest)?;
        }
    }

    Ok(())
}
