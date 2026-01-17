//! brd config command - view and change braid configuration.

use std::io::{self, BufRead, Write};
use std::path::Path;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::git;
use crate::repo::RepoPaths;

const ISSUES_SYMLINK_PATTERN: &str = ".braid/issues";

/// Prompt for confirmation. Returns true if user confirms (Y/y or empty).
fn confirm(prompt: &str) -> Result<bool> {
    print!("{} [Y/n]: ", prompt);
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let answer = line.trim().to_lowercase();

    Ok(answer.is_empty() || answer == "y" || answer == "yes")
}

/// Count .md files in a directory.
fn count_issues(dir: &Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
                .count()
        })
        .unwrap_or(0)
}

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

/// Set or clear the issues-branch setting.
pub fn cmd_config_issues_branch(
    cli: &Cli,
    paths: &RepoPaths,
    name: Option<&str>,
    clear: bool,
    yes: bool,
) -> Result<()> {
    // Handle clear case
    if clear {
        return clear_issues_branch(cli, paths, yes);
    }

    // Handle set case
    let branch = match name {
        Some(b) => b,
        None => {
            return Err(BrdError::Other(
                "must provide branch name or use --clear".to_string(),
            ));
        }
    };

    let mut config = Config::load(&paths.config_path())?;

    // check if already set to this branch
    if config.issues_branch.as_deref() == Some(branch) {
        if !cli.json {
            println!("issues-branch already set to '{}'", branch);
        }
        return Ok(());
    }

    // check if already set to a different branch
    if let Some(existing) = &config.issues_branch {
        return Err(BrdError::Other(format!(
            "issues-branch already set to '{}'. run `brd config issues-branch --clear` first.",
            existing
        )));
    }

    // check for uncommitted changes
    if !git::is_clean(&paths.worktree_root)? {
        return Err(BrdError::Other(
            "working tree has uncommitted changes - commit or stash first".to_string(),
        ));
    }

    // confirmation prompt (unless -y or --json)
    if !yes && !cli.json {
        let local_issues = paths.worktree_root.join(".braid/issues");
        let issue_count = count_issues(&local_issues);

        println!("Setting issues-branch to '{}'...", branch);
        println!();
        println!("This will:");
        println!("  • Create branch '{}' for issue storage", branch);
        println!(
            "  • Set up shared worktree at {}",
            paths.brd_common_dir.join("issues").display()
        );
        if issue_count > 0 {
            println!(
                "  • Move {} issue(s) from .braid/issues/ to the worktree",
                issue_count
            );
        }
        println!("  • Commit the changes");
        println!();

        if !confirm("Continue?")? {
            println!("Aborted.");
            return Ok(());
        }
        println!();
    }

    if !cli.json {
        println!("Setting issues-branch...");
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

    let commit_msg = format!("chore(braid): set issues-branch to '{}'", branch);
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
            "issues_branch": branch,
            "issues_worktree": issues_wt.to_string_lossy(),
            "moved_issues": moved_count,
            "agent_worktrees_needing_rebase": worktrees_json,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!();
        println!("issues-branch set to '{}'", branch);
        println!("Issues now live on shared worktree.");

        warn_agent_worktrees(&agent_worktrees);
    }

    Ok(())
}

/// Clear the issues-branch setting (move issues back to .braid/issues/).
fn clear_issues_branch(cli: &Cli, paths: &RepoPaths, yes: bool) -> Result<()> {
    let mut config = Config::load(&paths.config_path())?;

    // check if issues_branch is set
    let branch = match &config.issues_branch {
        Some(b) => b.clone(),
        None => {
            if !cli.json {
                println!("issues-branch is not set");
            }
            return Ok(());
        }
    };

    // check for uncommitted changes
    if !git::is_clean(&paths.worktree_root)? {
        return Err(BrdError::Other(
            "working tree has uncommitted changes - commit or stash first".to_string(),
        ));
    }

    // get issues from sync worktree
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

    // confirmation prompt (unless -y or --json)
    if !yes && !cli.json {
        let issue_count = count_issues(&wt_issues);

        println!("Clearing issues-branch setting...");
        println!();
        println!("This will:");
        if issue_count > 0 {
            println!(
                "  • Copy {} issue(s) from worktree to .braid/issues/",
                issue_count
            );
        }
        println!("  • Remove issues-branch from config");
        println!("  • Commit the changes");
        println!(
            "  • Leave worktree at {} (you can remove it manually)",
            issues_wt.display()
        );
        println!();

        if !confirm("Continue?")? {
            println!("Aborted.");
            return Ok(());
        }
        println!();
    }

    if !cli.json {
        println!("Clearing issues-branch...");
    }

    // remove symlink if it exists
    if let Err(e) = remove_issues_symlink(paths)
        && !cli.json
    {
        eprintln!("  warning: could not remove issues symlink: {}", e);
    }

    std::fs::create_dir_all(&local_issues)?;

    // copy issues back from sync worktree
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
        println!("  copied {} issue(s) from worktree", moved_count);
    }

    // update config (remove issues_branch)
    config.issues_branch = None;
    config.save(&paths.config_path())?;

    // commit the changes
    if !git::run(&["add", ".braid"], &paths.worktree_root)? {
        return Err(BrdError::Other(
            "failed to stage .braid changes".to_string(),
        ));
    }

    let commit_msg = format!("chore(braid): clear issues-branch (was '{}')", branch);
    let _ = git::run(&["commit", "-m", &commit_msg], &paths.worktree_root);

    // check for agent worktrees needing rebase
    let agent_worktrees = find_agent_worktrees_needing_rebase(&paths.worktree_root);

    if !cli.json {
        println!();
        println!("issues-branch cleared");
        println!("Issues now live in .braid/issues/");
        if issues_wt.exists() {
            println!();
            println!("Note: worktree still exists at {}", issues_wt.display());
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
            "issues_branch": serde_json::Value::Null,
            "from_branch": branch,
            "moved_issues": moved_count,
            "agent_worktrees_needing_rebase": worktrees_json,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    }

    Ok(())
}

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

/// Remove the issues symlink if it exists.
pub fn remove_issues_symlink(paths: &RepoPaths) -> Result<()> {
    let symlink_path = paths.worktree_root.join(".braid/issues");

    if symlink_path.is_symlink() {
        std::fs::remove_file(&symlink_path)?;
    }

    // remove from .git/info/exclude
    remove_from_git_exclude(paths, ISSUES_SYMLINK_PATTERN)?;

    Ok(())
}

/// Remove a pattern from .git/info/exclude.
fn remove_from_git_exclude(paths: &RepoPaths, pattern: &str) -> Result<()> {
    let exclude_path = paths.git_common_dir.join("info/exclude");

    if !exclude_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(&exclude_path)?;
    let new_content: String = content
        .lines()
        .filter(|line| line.trim() != pattern)
        .collect::<Vec<_>>()
        .join("\n");

    // preserve trailing newline if original had one
    let new_content = if content.ends_with('\n') && !new_content.is_empty() {
        format!("{}\n", new_content)
    } else {
        new_content
    };

    std::fs::write(&exclude_path, new_content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::tempdir;

    fn setup_git_repo() -> tempfile::TempDir {
        let dir = tempdir().unwrap();

        Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "commit.gpgsign", "false"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // initial commit
        fs::write(dir.path().join(".gitkeep"), "").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        dir
    }

    fn make_cli() -> Cli {
        Cli {
            json: false,
            repo: None,
            no_color: true,
            verbose: false,
            command: crate::cli::Command::Doctor,
        }
    }

    fn make_paths(dir: &tempfile::TempDir) -> RepoPaths {
        RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        }
    }

    fn setup_braid_config(dir: &tempfile::TempDir, content: &str) {
        fs::create_dir_all(dir.path().join(".braid")).unwrap();
        fs::write(dir.path().join(".braid/config.toml"), content).unwrap();
    }

    // count_issues tests
    #[test]
    fn test_count_issues_empty_dir() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("issues")).unwrap();
        assert_eq!(count_issues(&dir.path().join("issues")), 0);
    }

    #[test]
    fn test_count_issues_nonexistent_dir() {
        let dir = tempdir().unwrap();
        assert_eq!(count_issues(&dir.path().join("nonexistent")), 0);
    }

    #[test]
    fn test_count_issues_with_md_files() {
        let dir = tempdir().unwrap();
        let issues_dir = dir.path().join("issues");
        fs::create_dir_all(&issues_dir).unwrap();

        fs::write(issues_dir.join("issue1.md"), "content").unwrap();
        fs::write(issues_dir.join("issue2.md"), "content").unwrap();
        fs::write(issues_dir.join("not-an-issue.txt"), "content").unwrap();

        assert_eq!(count_issues(&issues_dir), 2);
    }

    // has_upstream tests
    #[test]
    fn test_has_upstream_no_upstream() {
        let dir = setup_git_repo();

        Command::new("git")
            .args(["branch", "test-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        assert!(!has_upstream("test-branch", dir.path()));
    }

    // get_upstream tests
    #[test]
    fn test_get_upstream_no_upstream() {
        let dir = setup_git_repo();

        Command::new("git")
            .args(["branch", "test-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        assert!(get_upstream("test-branch", dir.path()).is_none());
    }

    // is_behind_main tests
    #[test]
    fn test_is_behind_main_same_commit() {
        let dir = setup_git_repo();

        // ensure we're on main
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(dir.path())
            .output();

        // create a branch at same commit as main
        Command::new("git")
            .args(["branch", "test-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // checkout the branch
        Command::new("git")
            .args(["checkout", "test-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        assert!(!is_behind_main(dir.path()));
    }

    #[test]
    fn test_is_behind_main_behind() {
        let dir = setup_git_repo();

        // ensure we're on main
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(dir.path())
            .output();

        // create a branch at current commit
        Command::new("git")
            .args(["branch", "test-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // add a commit to main
        fs::write(dir.path().join("new-file.txt"), "content").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "new commit on main"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // checkout the branch (now behind main)
        Command::new("git")
            .args(["checkout", "test-branch"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        assert!(is_behind_main(dir.path()));
    }

    // cmd_config_show tests
    #[test]
    fn test_config_show_git_native() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        let result = cmd_config_show(&cli, &paths);
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_show_with_issues_branch() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

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
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_repo = \"../external\"\n",
        );

        let result = cmd_config_show(&cli, &paths);
        assert!(result.is_ok());
    }

    // cmd_config_issues_branch tests
    #[test]
    fn test_config_issues_branch_already_set() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_branch = \"existing-branch\"\n",
        );

        let result = cmd_config_issues_branch(&cli, &paths, Some("new-branch"), false, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("issues-branch already set")
        );
    }

    #[test]
    fn test_config_issues_branch_uncommitted_changes() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // create uncommitted changes
        fs::write(dir.path().join("uncommitted.txt"), "content").unwrap();

        let result = cmd_config_issues_branch(&cli, &paths, Some("braid-issues"), false, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("uncommitted changes")
        );
    }

    #[test]
    fn test_config_issues_branch_success() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
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

        let result = cmd_config_issues_branch(&cli, &paths, Some("braid-issues"), false, true);
        assert!(result.is_ok());

        // verify config was updated
        let config = Config::load(&paths.config_path()).unwrap();
        assert_eq!(config.issues_branch, Some("braid-issues".to_string()));
    }

    #[test]
    fn test_config_issues_branch_clear_when_not_set() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // clearing when not set should succeed (no-op)
        let result = cmd_config_issues_branch(&cli, &paths, None, true, true);
        assert!(result.is_ok());
    }

    // cmd_config_external_repo tests
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

    // cmd_config_auto_sync tests
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

    // clear_issues_branch tests (via cmd_config_issues_branch --clear)
    #[test]
    fn test_clear_issues_branch_success() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // ensure main branch exists
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(dir.path())
            .output();

        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // commit .braid
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

        // set issues-branch (creates worktree)
        let result = cmd_config_issues_branch(&cli, &paths, Some("braid-issues"), false, true);
        assert!(result.is_ok());

        // verify issues_branch is set
        let config = Config::load(&paths.config_path()).unwrap();
        assert_eq!(config.issues_branch, Some("braid-issues".to_string()));

        // create an issue in the worktree
        let issues_wt = paths.issues_worktree_dir();
        let wt_issues = issues_wt.join(".braid/issues");
        fs::create_dir_all(&wt_issues).unwrap();
        fs::write(
            wt_issues.join("tst-abc1.md"),
            "---\nid: tst-abc1\ntitle: test issue\npriority: P2\nstatus: open\ncreated_at: 2024-01-01T00:00:00Z\nupdated_at: 2024-01-01T00:00:00Z\n---\n",
        )
        .unwrap();

        // commit the issue in the worktree
        Command::new("git")
            .args(["add", "."])
            .current_dir(&issues_wt)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add issue"])
            .current_dir(&issues_wt)
            .output()
            .unwrap();

        // now clear issues-branch
        let result = cmd_config_issues_branch(&cli, &paths, None, true, true);
        assert!(result.is_ok());

        // verify issues_branch is cleared
        let config = Config::load(&paths.config_path()).unwrap();
        assert!(config.issues_branch.is_none());

        // verify issue was copied back to local .braid/issues/
        let local_issues = dir.path().join(".braid/issues");
        assert!(local_issues.join("tst-abc1.md").exists());
    }

    #[test]
    fn test_clear_issues_branch_dirty_issues_worktree() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // ensure main branch exists
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(dir.path())
            .output();

        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // commit .braid
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

        // set issues-branch (creates worktree)
        let result = cmd_config_issues_branch(&cli, &paths, Some("braid-issues"), false, true);
        assert!(result.is_ok());

        // create uncommitted changes in the issues worktree
        let issues_wt = paths.issues_worktree_dir();
        fs::write(issues_wt.join("uncommitted.txt"), "dirty").unwrap();

        // try to clear - should fail
        let result = cmd_config_issues_branch(&cli, &paths, None, true, true);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("issues worktree has uncommitted changes")
        );
    }

    #[test]
    fn test_clear_issues_branch_no_issues() {
        let dir = setup_git_repo();
        let cli = make_cli();
        let paths = make_paths(&dir);

        // ensure main branch exists
        let _ = Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(dir.path())
            .output();

        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        setup_braid_config(
            &dir,
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        );

        // commit .braid
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

        // set issues-branch (creates worktree)
        let result = cmd_config_issues_branch(&cli, &paths, Some("braid-issues"), false, true);
        assert!(result.is_ok());

        // don't create any issues - just clear immediately
        let result = cmd_config_issues_branch(&cli, &paths, None, true, true);
        assert!(result.is_ok());

        // verify issues_branch is cleared
        let config = Config::load(&paths.config_path()).unwrap();
        assert!(config.issues_branch.is_none());
    }

    // cmd_config_external_repo success test
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

    // clear_external_repo tests (via cmd_config_external_repo --clear)
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
