//! brd init command.

use std::io::{self, BufRead, Write};

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

    // determine issues_branch: from args, interactive prompt, or None (git-native)
    let issues_branch = determine_issues_branch(cli, args, &worktree_root)?;

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
        config.issues_branch = issues_branch.clone();
        config.save(&config_path)?;
    } else if issues_branch.is_some() {
        // update existing config with issues_branch
        let mut config = Config::load(&config_path)?;
        config.issues_branch = issues_branch.clone();
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

    // if issues_branch is set, create the branch and worktree
    if let Some(branch_name) = &issues_branch {
        setup_issues_branch(
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
            "issues_branch": issues_branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Initialized braid in {}", braid_dir.display());
        if let Some(branch) = &issues_branch {
            println!("Local-sync mode enabled: issues will live on '{}'", branch);
        }
        println!();
        println!("next steps:");
        println!("  brd add \"my first task\"     # create an issue");
        println!("  brd agent inject            # add agent instructions to AGENTS.md");
        if issues_branch.is_some() {
            println!("  brd sync                    # sync issues to remote");
        }
    }

    Ok(())
}

/// Determine the issues_branch value based on args and interactive prompts.
fn determine_issues_branch(
    cli: &Cli,
    args: &InitArgs,
    worktree_root: &std::path::Path,
) -> Result<Option<String>> {
    // if explicitly set via --issues-branch, use it
    if args.issues_branch.is_some() {
        return Ok(args.issues_branch.clone());
    }

    // if non-interactive or json mode, use git-native (no issues_branch)
    if args.non_interactive || cli.json {
        return Ok(None);
    }

    // interactive prompt
    println!("Initializing braid in {}...", worktree_root.display());
    println!();
    println!("How will you use braid?");
    println!();
    println!("  1. Solo or remote team (git-native) [recommended]");
    println!("     Issues sync via normal git push/pull. Simple and familiar.");
    println!();
    println!("  2. Multiple local agents (local-sync)");
    println!("     Issues sync instantly via shared worktree. Best for 2+ agents");
    println!("     on the same machine.");
    println!();
    print!("Choice [1]: ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let choice = line.trim();

    match choice {
        "" | "1" => Ok(None), // git-native
        "2" => {
            // prompt for branch name
            print!("Branch name [braid-issues]: ");
            io::stdout().flush()?;

            let mut branch_line = String::new();
            stdin.lock().read_line(&mut branch_line)?;
            let branch = branch_line.trim();

            let branch_name = if branch.is_empty() {
                "braid-issues".to_string()
            } else {
                branch.to_string()
            };

            println!();
            Ok(Some(branch_name))
        }
        _ => {
            eprintln!("Invalid choice '{}', using git-native mode", choice);
            Ok(None)
        }
    }
}

/// Set up sync branch mode by creating the branch and issues worktree.
fn setup_issues_branch(
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use tempfile::tempdir;

    static INIT_TEST_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let prev = std::env::var(key).ok();
            match value {
                Some(val) => unsafe { std::env::set_var(key, val) },
                None => unsafe { std::env::remove_var(key) },
            }
            Self { key, prev }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(val) => unsafe { std::env::set_var(self.key, val) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    struct CwdGuard {
        prev: PathBuf,
    }

    impl CwdGuard {
        fn enter(path: &Path) -> Self {
            let prev = std::env::current_dir().unwrap();
            std::env::set_current_dir(path).unwrap();
            Self { prev }
        }
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.prev).unwrap();
        }
    }

    use crate::git::test::run_ok as git_ok;

    fn with_repo<F: FnOnce(&Path)>(name: &str, f: F) {
        let _lock = INIT_TEST_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join(name);
        std::fs::create_dir_all(&repo_path).unwrap();
        git_ok(&repo_path, &["init"]);
        let _cwd = CwdGuard::enter(&repo_path);
        f(&repo_path);
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
    fn test_init_creates_structure() {
        with_repo("my-repo", |repo_path| {
            let _env = EnvGuard::set("USER", Some("tester"));
            let cli = make_cli(false);
            let args = InitArgs {
                issues_branch: None,
                non_interactive: true,
            };

            cmd_init(&cli, &args).unwrap();

            let braid_dir = repo_path.join(".braid");
            assert!(braid_dir.exists());
            assert!(braid_dir.join("issues").exists());
            assert!(braid_dir.join("config.toml").exists());
            assert!(braid_dir.join(".gitignore").exists());
            assert!(braid_dir.join("agent.toml").exists());

            let config = Config::load(&braid_dir.join("config.toml")).unwrap();
            let expected = Config::with_derived_prefix("my-repo");
            assert_eq!(config.id_prefix, expected.id_prefix);

            let gitignore = std::fs::read_to_string(braid_dir.join(".gitignore")).unwrap();
            assert_eq!(gitignore, "agent.toml\nruntime/\n");

            let agent_toml = std::fs::read_to_string(braid_dir.join("agent.toml")).unwrap();
            assert!(agent_toml.contains("agent_id = \"tester\""));
        });
    }

    #[test]
    fn test_init_missing_user_env_uses_default() {
        with_repo("no-user", |repo_path| {
            let _env = EnvGuard::set("USER", None);
            let cli = make_cli(false);
            let args = InitArgs {
                issues_branch: None,
                non_interactive: true,
            };

            cmd_init(&cli, &args).unwrap();

            let agent_toml = std::fs::read_to_string(repo_path.join(".braid/agent.toml")).unwrap();
            assert!(agent_toml.contains("agent_id = \"default-user\""));
        });
    }

    #[test]
    fn test_init_idempotent_preserves_config_and_agent() {
        with_repo("keep-config", |repo_path| {
            let _env = EnvGuard::set("USER", Some("first"));
            let cli = make_cli(false);
            let args = InitArgs {
                issues_branch: None,
                non_interactive: true,
            };

            cmd_init(&cli, &args).unwrap();

            let braid_dir = repo_path.join(".braid");
            let mut config = Config::load(&braid_dir.join("config.toml")).unwrap();
            config.id_prefix = "keep".to_string();
            config.save(&braid_dir.join("config.toml")).unwrap();
            std::fs::write(braid_dir.join("agent.toml"), "agent_id = \"keep\"\n").unwrap();

            let _env2 = EnvGuard::set("USER", Some("second"));
            cmd_init(&cli, &args).unwrap();

            let config = Config::load(&braid_dir.join("config.toml")).unwrap();
            assert_eq!(config.id_prefix, "keep");
            let agent_toml = std::fs::read_to_string(braid_dir.join("agent.toml")).unwrap();
            assert!(agent_toml.contains("agent_id = \"keep\""));
        });
    }
}
