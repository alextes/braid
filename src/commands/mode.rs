//! brd mode command - show and switch workflow modes.

use std::path::Path;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::git;
use crate::repo::RepoPaths;

/// Check if a branch has an upstream tracking branch.
fn has_upstream(branch: &str, cwd: &Path) -> bool {
    git::run(
        &["rev-parse", "--abbrev-ref", &format!("{}@{{u}}", branch)],
        cwd,
    )
    .unwrap_or(false)
}

/// Get the upstream tracking branch name.
fn get_upstream(branch: &str, cwd: &Path) -> Option<String> {
    git::output(
        &["rev-parse", "--abbrev-ref", &format!("{}@{{u}}", branch)],
        cwd,
    )
    .ok()
    .filter(|s| !s.is_empty())
}

/// Agent worktree info for rebase warnings.
struct AgentWorktree {
    branch: String,
    path: std::path::PathBuf,
}

/// Find agent worktrees that need to rebase on main.
/// Returns worktrees that have .braid/agent.toml and are behind main.
fn find_agent_worktrees_needing_rebase(cwd: &Path) -> Vec<AgentWorktree> {
    let mut result = Vec::new();

    // get worktree list in porcelain format
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(cwd)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return result,
    };

    // parse porcelain output: "worktree <path>\nHEAD <sha>\nbranch refs/heads/<name>\n\n"
    let mut current_path: Option<std::path::PathBuf> = None;
    let mut current_branch: Option<String> = None;

    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(std::path::PathBuf::from(path));
            current_branch = None;
        } else if let Some(branch_ref) = line.strip_prefix("branch ") {
            // branch refs/heads/agent-one -> agent-one
            current_branch = branch_ref.strip_prefix("refs/heads/").map(String::from);
        } else if line.is_empty() {
            // end of entry, check if it's an agent worktree
            if let (Some(path), Some(branch)) = (current_path.take(), current_branch.take()) {
                // check if this is an agent worktree (has .braid/agent.toml)
                let agent_toml = path.join(".braid/agent.toml");
                if agent_toml.exists() {
                    // check if behind main
                    if is_behind_main(&path) {
                        result.push(AgentWorktree { branch, path });
                    }
                }
            }
        }
    }

    result
}

/// Check if a worktree is behind main (main has commits not in this branch).
fn is_behind_main(worktree_path: &Path) -> bool {
    // count commits in main that aren't in HEAD
    let output = std::process::Command::new("git")
        .args(["rev-list", "--count", "HEAD..main"])
        .current_dir(worktree_path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let count_str = String::from_utf8_lossy(&o.stdout);
            count_str.trim().parse::<u32>().unwrap_or(0) > 0
        }
        _ => false,
    }
}

/// Print warning about agent worktrees needing rebase.
fn warn_agent_worktrees(worktrees: &[AgentWorktree]) {
    if worktrees.is_empty() {
        return;
    }

    println!();
    println!(
        "Warning: Found {} agent worktree(s) that need to rebase on main:",
        worktrees.len()
    );
    for wt in worktrees {
        println!("  - {} (at {})", wt.branch, wt.path.display());
    }
    println!();
    println!("Run `git rebase main` in each worktree to pick up the new config.");
}

/// Show current workflow mode.
pub fn cmd_mode_show(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let config = Config::load(&paths.config_path())?;

    if cli.json {
        let json = if let Some(ref external_path) = config.issues_repo {
            serde_json::json!({
                "mode": "external-repo",
                "path": external_path,
            })
        } else if let Some(ref branch) = config.issues_branch {
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

    if let Some(ref external_path) = config.issues_repo {
        println!("Mode: external-repo");
        println!("Path: {}", external_path);

        // resolve and show actual issues location
        let resolved = if std::path::Path::new(external_path).is_absolute() {
            std::path::PathBuf::from(external_path)
        } else {
            paths.worktree_root.join(external_path)
        };

        if let Ok(canonical) = resolved.canonicalize() {
            println!("Resolved: {}", canonical.display());
        }

        println!();
        println!("Issues are tracked in a separate repository.");
        println!("Good for: separation of concerns, privacy, multi-repo coordination.");
    } else if let Some(ref branch) = config.issues_branch {
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

/// Switch to local-sync mode.
pub fn cmd_mode_local_sync(cli: &Cli, paths: &RepoPaths, branch: &str) -> Result<()> {
    let mut config = Config::load(&paths.config_path())?;

    // check if already in sync mode
    if config.issues_branch.is_some() {
        return Err(BrdError::Other(format!(
            "already in sync mode (branch: {}). run `brd mode default` first to switch.",
            config.issues_branch.as_ref().unwrap()
        )));
    }

    // check for uncommitted changes
    if !git::is_clean(&paths.worktree_root)? {
        return Err(BrdError::Other(
            "working tree has uncommitted changes - commit or stash first".to_string(),
        ));
    }

    if !cli.json {
        println!("Switching to local-sync mode...");
    }

    // 1. create sync branch if it doesn't exist
    let branch_exists = git::run(&["rev-parse", "--verify", branch], &paths.worktree_root)?;

    if !branch_exists {
        if !git::run(&["branch", branch], &paths.worktree_root)? {
            return Err(BrdError::Other(format!(
                "failed to create sync branch '{}'",
                branch
            )));
        }
        if !cli.json {
            println!("  created branch '{}'", branch);
        }
    }

    // 2. set up shared issues worktree
    let issues_wt = paths.ensure_issues_worktree(branch)?;
    if !cli.json {
        println!("  issues worktree at {}", issues_wt.display());
    }

    // 3. move existing issues to sync branch worktree
    let local_issues = paths.worktree_root.join(".braid/issues");
    let wt_issues = issues_wt.join(".braid/issues");
    std::fs::create_dir_all(&wt_issues)?;

    let mut moved_count = 0;
    if local_issues.exists() {
        for entry in std::fs::read_dir(&local_issues)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                let dest = wt_issues.join(path.file_name().unwrap());
                std::fs::copy(&path, &dest)?;
                std::fs::remove_file(&path)?;
                moved_count += 1;
            }
        }
    }

    if moved_count > 0 && !cli.json {
        println!("  moved {} issue(s) to sync branch", moved_count);
    }

    // 4. update config
    config.issues_branch = Some(branch.to_string());
    config.save(&paths.config_path())?;

    // 5. commit the changes
    if !git::run(&["add", ".braid"], &paths.worktree_root)? {
        return Err(BrdError::Other(
            "failed to stage .braid changes".to_string(),
        ));
    }

    let commit_msg = format!("chore(braid): switch to local-sync mode ({})", branch);
    // commit might fail if nothing changed, that's ok
    let _ = git::run(&["commit", "-m", &commit_msg], &paths.worktree_root);

    // also commit in the issues worktree
    if !git::run(&["add", ".braid"], &issues_wt)? {
        return Err(BrdError::Other(
            "failed to stage .braid in issues worktree".to_string(),
        ));
    }
    let _ = git::run(
        &["commit", "-m", "chore(braid): initial issues"],
        &issues_wt,
    );

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
            "mode": "local-sync",
            "branch": branch,
            "issues_worktree": issues_wt.to_string_lossy(),
            "moved_issues": moved_count,
            "agent_worktrees_needing_rebase": worktrees_json,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!();
        println!("Switched to local-sync mode.");
        println!("Issues now live on '{}' branch.", branch);

        warn_agent_worktrees(&agent_worktrees);
    }

    Ok(())
}

/// Switch back to git-native mode.
pub fn cmd_mode_default(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let mut config = Config::load(&paths.config_path())?;

    // handle external-repo mode first (simpler - just clear the config)
    if let Some(ref external_path) = config.issues_repo {
        let path = external_path.clone();

        if !cli.json {
            println!("Switching from external-repo to git-native mode...");
        }

        config.issues_repo = None;
        config.save(&paths.config_path())?;

        // commit the config change
        if git::run(&["add", ".braid/config.toml"], &paths.worktree_root)? {
            let commit_msg = format!("chore(braid): switch to git-native mode (from external {})", path);
            let _ = git::run(&["commit", "-m", &commit_msg], &paths.worktree_root);
        }

        let agent_worktrees = find_agent_worktrees_needing_rebase(&paths.worktree_root);

        if cli.json {
            let worktrees_json: Vec<_> = agent_worktrees
                .iter()
                .map(|wt| serde_json::json!({"branch": wt.branch, "path": wt.path.to_string_lossy()}))
                .collect();
            let json = serde_json::json!({
                "ok": true,
                "mode": "git-native",
                "from_external": path,
                "agent_worktrees_needing_rebase": worktrees_json,
            });
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        } else {
            println!();
            println!("Switched to git-native mode.");
            println!("Note: issues remain in external repo at {}", path);
            println!("You'll need to manually copy issues if you want them locally.");
            warn_agent_worktrees(&agent_worktrees);
        }

        return Ok(());
    }

    // check if in sync mode
    let branch = match &config.issues_branch {
        Some(b) => b.clone(),
        None => {
            return Err(BrdError::Other(
                "already in git-native mode".to_string(),
            ));
        }
    };

    // check for uncommitted changes
    if !git::is_clean(&paths.worktree_root)? {
        return Err(BrdError::Other(
            "working tree has uncommitted changes - commit or stash first".to_string(),
        ));
    }

    if !cli.json {
        println!("Switching to git-native mode...");
    }

    // 1. get issues from sync worktree
    let issues_wt = paths.issues_worktree_dir();
    let wt_issues = issues_wt.join(".braid/issues");
    let local_issues = paths.worktree_root.join(".braid/issues");

    // check for uncommitted changes in issues worktree
    if issues_wt.exists() && !git::is_clean(&issues_wt)? {
        return Err(BrdError::Other(
            "issues worktree has uncommitted changes - commit them first with `brd sync`"
                .to_string(),
        ));
    }

    std::fs::create_dir_all(&local_issues)?;

    // 2. copy issues back from sync worktree
    let mut moved_count = 0;
    if wt_issues.exists() {
        for entry in std::fs::read_dir(&wt_issues)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                let dest = local_issues.join(path.file_name().unwrap());
                std::fs::copy(&path, &dest)?;
                moved_count += 1;
            }
        }
    }

    if moved_count > 0 && !cli.json {
        println!("  copied {} issue(s) from sync branch", moved_count);
    }

    // 3. update config (remove issues_branch)
    config.issues_branch = None;
    config.save(&paths.config_path())?;

    // 4. commit the changes
    if !git::run(&["add", ".braid"], &paths.worktree_root)? {
        return Err(BrdError::Other(
            "failed to stage .braid changes".to_string(),
        ));
    }

    let commit_msg = format!("chore(braid): switch to git-native mode (from {})", branch);
    let _ = git::run(&["commit", "-m", &commit_msg], &paths.worktree_root);

    // 5. check for agent worktrees needing rebase
    let agent_worktrees = find_agent_worktrees_needing_rebase(&paths.worktree_root);

    // 6. output results
    if !cli.json {
        println!();
        println!("Switched to git-native mode.");
        println!("Issues now live on main branch.");
        if issues_wt.exists() {
            println!();
            println!(
                "Note: issues worktree still exists at {}",
                issues_wt.display()
            );
            println!(
                "You can remove it with: git worktree remove {}",
                issues_wt.display()
            );
        }

        warn_agent_worktrees(&agent_worktrees);
    }

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
            "mode": "git-native",
            "from_branch": branch,
            "moved_issues": moved_count,
            "agent_worktrees_needing_rebase": worktrees_json,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    Ok(())
}

/// Switch to external-repo mode.
pub fn cmd_mode_external_repo(cli: &Cli, paths: &RepoPaths, external_path: &str) -> Result<()> {
    use crate::repo::discover;

    let mut config = Config::load(&paths.config_path())?;

    // check if already in a non-default mode
    if config.issues_branch.is_some() {
        return Err(BrdError::Other(
            "currently in local-sync mode. run `brd mode default` first to switch.".to_string(),
        ));
    }
    if config.issues_repo.is_some() {
        return Err(BrdError::Other(format!(
            "already in external-repo mode (path: {}). run `brd mode default` first to switch.",
            config.issues_repo.as_ref().unwrap()
        )));
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

    // load external config to verify it's valid
    Config::load(&external_config_path).map_err(|e| {
        BrdError::Other(format!(
            "failed to load external repo config: {}",
            e
        ))
    })?;

    if !cli.json {
        println!("Switching to external-repo mode...");
    }

    // update config
    config.issues_repo = Some(external_path.to_string());
    config.save(&paths.config_path())?;

    // commit the config change
    if !git::run(&["add", ".braid/config.toml"], &paths.worktree_root)? {
        return Err(BrdError::Other(
            "failed to stage config change".to_string(),
        ));
    }

    let commit_msg = format!("chore(braid): switch to external-repo mode ({})", external_path);
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
            "mode": "external-repo",
            "path": external_path,
            "resolved": canonical.to_string_lossy(),
            "agent_worktrees_needing_rebase": worktrees_json,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!();
        println!("Switched to external-repo mode.");
        println!("Issues now tracked in: {}", canonical.display());

        warn_agent_worktrees(&agent_worktrees);
    }

    Ok(())
}
