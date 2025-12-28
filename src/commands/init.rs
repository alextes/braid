//! brd init command.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::Result;
use crate::repo::git_rev_parse;

pub fn cmd_init(cli: &Cli) -> Result<()> {
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
    if !config_path.exists() {
        let repo_name = worktree_root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("brd");
        let config = Config::with_derived_prefix(repo_name);
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

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "braid_dir": braid_dir.to_string_lossy(),
            "worktree": worktree_root.to_string_lossy(),
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Initialized braid in {}", braid_dir.display());
        println!();
        println!("next steps:");
        println!("  brd add \"my first task\"     # create an issue");
        println!("  brd agents inject           # add agent instructions to AGENTS.md");
    }

    Ok(())
}
