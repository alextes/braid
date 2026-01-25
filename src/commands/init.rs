//! brd init command.

use std::io::{self, BufRead, Write};

use crate::cli::{Cli, InitArgs};
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::is_interactive;
use crate::repo::git_rev_parse;

/// Workflow configuration determined from init prompts.
struct WorkflowConfig {
    issues_branch: Option<String>,
    auto_pull: bool,
    auto_push: bool,
}

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

    // check if already initialized
    if config_path.exists() {
        if !cli.json {
            eprintln!("hint: use `brd mode` to change configuration");
            eprintln!("hint: use `brd doctor` to check repo health");
        }
        return Err(BrdError::AlreadyInitialized);
    }

    // determine workflow config: from args, interactive prompt, or defaults
    let workflow = determine_workflow_config(cli, args, &worktree_root)?;

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
        config.issues_branch = workflow.issues_branch.clone();
        config.auto_pull = workflow.auto_pull;
        config.auto_push = workflow.auto_push;
        config.save(&config_path)?;
    } else {
        // update existing config with workflow settings
        let mut config = Config::load(&config_path)?;
        config.issues_branch = workflow.issues_branch.clone();
        config.auto_pull = workflow.auto_pull;
        config.auto_push = workflow.auto_push;
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
    if let Some(branch_name) = &workflow.issues_branch {
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
            "issues_branch": workflow.issues_branch,
            "auto_pull": workflow.auto_pull,
            "auto_push": workflow.auto_push,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("braid initialized");
        println!();

        // mode description
        let mode_desc = match &workflow.issues_branch {
            Some(branch) => format!("issues branch ({})", branch),
            None => "issues in code branches".to_string(),
        };
        println!("  mode:       {}", mode_desc);

        // auto-sync description
        let sync_desc = if workflow.auto_pull && workflow.auto_push {
            "enabled (pull on start, push on done)"
        } else {
            "disabled (use `brd sync` manually)"
        };
        println!("  auto-sync:  {}", sync_desc);

        println!("  location:   .braid/");
        println!();
        println!("next steps:");
        println!("  brd add \"my first task\"");
        println!("  brd agent inject");
    }

    Ok(())
}

/// Determine workflow configuration based on args and interactive prompts.
///
/// Uses a 2-question flow:
/// Q1: Issues branch? (recommended: yes) -> sets issues_branch
/// Q2: Auto-sync? (contextual recommendation) -> sets auto_pull and auto_push
fn determine_workflow_config(
    cli: &Cli,
    args: &InitArgs,
    _worktree_root: &std::path::Path,
) -> Result<WorkflowConfig> {
    // if explicitly set via --issues-branch, use it with default auto_sync
    if let Some(branch) = args.issues_branch.clone() {
        return Ok(WorkflowConfig {
            issues_branch: Some(branch),
            auto_pull: true,
            auto_push: true,
        });
    }

    // if non-interactive, json mode, or no TTY, use defaults (issues_branch, auto-sync on)
    if args.non_interactive || cli.json || !is_interactive() {
        return Ok(WorkflowConfig {
            issues_branch: Some("braid-issues".to_string()),
            auto_pull: true,
            auto_push: true,
        });
    }

    // interactive prompts
    println!("initializing braid...");
    println!();

    let stdin = io::stdin();

    // Q1: Issues branch?
    println!("store issues on a separate branch?");
    println!("  Y: prevents merge conflicts, cleaner git history");
    println!("  n: issues travel with code branches");
    print!("[Y/n]: ");
    io::stdout().flush()?;

    let mut q1_line = String::new();
    stdin.lock().read_line(&mut q1_line)?;
    let q1_choice = q1_line.trim().to_lowercase();

    let issues_branch = match q1_choice.as_str() {
        "n" | "no" => None,
        "" | "y" | "yes" => Some("braid-issues".to_string()),
        _ => {
            eprintln!("invalid choice '{}', using separate branch", q1_choice);
            Some("braid-issues".to_string())
        }
    };

    println!();

    // Q2: Auto-sync?
    println!("auto-sync with remote?");
    println!("  Y: pull on start, push on done - keeps collaborators in sync");
    println!("  n: manual sync only (use `brd sync`)");
    print!("[Y/n]: ");
    io::stdout().flush()?;

    let mut q2_line = String::new();
    stdin.lock().read_line(&mut q2_line)?;
    let q2_choice = q2_line.trim().to_lowercase();

    let (auto_pull, auto_push) = match q2_choice.as_str() {
        "n" | "no" => (false, false),
        "" | "y" | "yes" => (true, true),
        _ => {
            eprintln!("invalid choice '{}', using auto-sync", q2_choice);
            (true, true)
        }
    };

    println!();

    Ok(WorkflowConfig {
        issues_branch,
        auto_pull,
        auto_push,
    })
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
        // check if repo has any commits (HEAD must exist to create a branch)
        let has_commits = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(worktree_root)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !has_commits {
            return Err(BrdError::Other(
                "cannot set up local-sync mode in a repo with no commits. \
                 make an initial commit first, then run 'brd init --issues-branch <name>'."
                    .to_string(),
            ));
        }

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
            println!("creating issues branch '{}'...", branch_name);
        }
    }

    // 2. create issues worktree if it doesn't exist
    if !issues_wt_path.exists() {
        if !json {
            println!("creating issues worktree...");
        }

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
        git_ok(&repo_path, &["config", "user.email", "test@test.com"]);
        git_ok(&repo_path, &["config", "user.name", "Test User"]);
        // Create initial commit so issues_branch worktree can be created
        std::fs::write(repo_path.join("README.md"), "# Test\n").unwrap();
        git_ok(&repo_path, &["add", "."]);
        git_ok(&repo_path, &["commit", "-m", "initial"]);
        let _cwd = CwdGuard::enter(&repo_path);
        f(&repo_path);
    }

    fn with_empty_repo<F: FnOnce(&Path)>(name: &str, f: F) {
        let _lock = INIT_TEST_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join(name);
        std::fs::create_dir_all(&repo_path).unwrap();
        git_ok(&repo_path, &["init"]);
        git_ok(&repo_path, &["config", "user.email", "test@test.com"]);
        git_ok(&repo_path, &["config", "user.name", "Test User"]);
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
    fn test_init_errors_if_already_initialized() {
        with_repo("already-init", |_repo_path| {
            let _env = EnvGuard::set("USER", Some("first"));
            let cli = make_cli(false);
            let args = InitArgs {
                issues_branch: None,
                non_interactive: true,
            };

            // first init succeeds
            cmd_init(&cli, &args).unwrap();

            // second init fails with already_initialized
            let result = cmd_init(&cli, &args);
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert_eq!(err.code_str(), "already_initialized");
        });
    }

    #[test]
    fn test_init_local_sync_fails_without_commits() {
        // fresh repo with no commits should fail gracefully when local-sync mode requested
        with_empty_repo("no-commits", |_repo_path| {
            let _env = EnvGuard::set("USER", Some("tester"));
            let cli = make_cli(false);
            let args = InitArgs {
                issues_branch: Some("braid-issues".to_string()),
                non_interactive: true,
            };

            let result = cmd_init(&cli, &args);
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("no commits"),
                "expected 'no commits' in error, got: {}",
                err
            );
        });
    }
}
