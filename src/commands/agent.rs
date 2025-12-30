//! brd agent commands - worktree management and AGENTS.md instructions.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::lock::LockGuard;
use crate::repo::{self, RepoPaths};

use super::{
    claim_issue, commit_and_push_main, commit_and_push_sync_branch, has_origin, has_origin_main,
    is_clean, load_all_issues, resolve_issue_id,
};

pub fn cmd_agent_init(cli: &Cli, paths: &RepoPaths, name: &str, base: Option<&str>) -> Result<()> {
    // validate agent name (alphanumeric + hyphens)
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(BrdError::Other(format!(
            "invalid agent name '{}': use only alphanumeric, hyphens, underscores",
            name
        )));
    }

    // determine worktree path (~/.braid/worktrees/<repo-name>/<agent-name>)
    let home_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| BrdError::Other("cannot determine home directory".to_string()))?;
    let repo_name = paths
        .worktree_root
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| BrdError::Other("cannot determine repo name".to_string()))?;
    let worktrees_dir = home_dir.join(".braid").join("worktrees").join(repo_name);
    let worktree_path = worktrees_dir.join(name);

    // ensure parent directories exist
    std::fs::create_dir_all(&worktrees_dir)?;

    // check if worktree already exists
    if worktree_path.exists() {
        return Err(BrdError::Other(format!(
            "directory already exists: {}",
            worktree_path.display()
        )));
    }

    // get base branch (default to current branch)
    let base_branch = match base {
        Some(b) => b.to_string(),
        None => {
            let output = std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .current_dir(&paths.worktree_root)
                .output()?;
            if !output.status.success() {
                return Err(BrdError::Other("failed to get current branch".to_string()));
            }
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
    };

    // create worktree with new branch
    let output = std::process::Command::new("git")
        .args([
            "worktree",
            "add",
            "-b",
            name,
            worktree_path.to_str().unwrap(),
            &base_branch,
        ])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BrdError::Other(format!(
            "failed to create worktree: {}",
            stderr.trim()
        )));
    }

    // create .braid directory in new worktree (for agent.toml)
    let new_braid_dir = worktree_path.join(".braid");
    std::fs::create_dir_all(&new_braid_dir)?;

    // create agent.toml
    let agent_toml_path = new_braid_dir.join("agent.toml");
    let agent_toml_content = format!("agent_id = \"{}\"\n", name);
    std::fs::write(&agent_toml_path, agent_toml_content)?;

    // check for sync branch mode and ensure issues worktree exists
    let config = Config::load(&paths.config_path()).ok();
    let sync_branch = config.as_ref().and_then(|c| c.sync_branch.clone());

    if let Some(ref branch) = sync_branch {
        // ensure the shared issues worktree exists
        if let Err(e) = paths.ensure_issues_worktree(branch) {
            eprintln!("warning: failed to ensure issues worktree: {}", e);
        }
    }

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "agent_id": name,
            "worktree": worktree_path.to_string_lossy(),
            "branch": name,
            "base": base_branch,
            "sync_branch": sync_branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Created agent worktree: {}", name);
        println!("  path:   {}", worktree_path.display());
        println!("  branch: {} (from {})", name, base_branch);
        if let Some(sb) = &sync_branch {
            println!("  sync:   issues on '{}' branch", sb);
        }
        println!();
        println!("To use this agent:");
        println!("  cd {}", worktree_path.display());
        println!("  # start your agent (claude, codex, gemini, etc.)");
        println!("  brd next  # get next issue to work on");
        if sync_branch.is_some() {
            println!("  brd sync  # sync issues with remote");
        }
    }

    Ok(())
}

/// run a git command and return success status.
fn git(args: &[&str], cwd: &std::path::Path) -> std::io::Result<bool> {
    let output = Command::new("git").args(args).current_dir(cwd).output()?;
    Ok(output.status.success())
}

/// create a feature branch for PR workflow.
/// this command:
/// 1. syncs with main and claims the issue (pushing to main so other agents see it)
/// 2. creates a feature branch for working on the issue
pub fn cmd_agent_branch(cli: &Cli, paths: &RepoPaths, issue_id: &str) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let is_sync_mode = config.is_sync_branch_mode();
    let agent_id = repo::get_agent_id(&paths.worktree_root);

    // Step 1: Check for clean working tree
    if !is_clean(&paths.worktree_root)? {
        return Err(BrdError::Other(
            "working tree has uncommitted changes - commit or stash first".to_string(),
        ));
    }

    // Load issues and resolve the ID
    let _lock = LockGuard::acquire(&paths.lock_path())?;
    let mut issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(issue_id, &issues)?;

    // create branch name: pr/<agent>/<issue-id>
    let branch_name = format!("pr/{}/{}", agent_id, full_id);

    // Check if branch already exists
    if git(
        &["rev-parse", "--verify", &branch_name],
        &paths.worktree_root,
    )
    .unwrap_or(false)
    {
        return Err(BrdError::Other(format!(
            "branch '{}' already exists",
            branch_name
        )));
    }

    if is_sync_mode {
        // Local-sync mode: claim in shared worktree, then create feature branch
        if !cli.json {
            eprintln!("claiming {} in sync branch...", full_id);
        }

        // Claim the issue
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(full_id.clone()))?;
        claim_issue(paths, &config, issue, &agent_id, false)?;

        // Commit in sync branch worktree
        commit_and_push_sync_branch(paths, &config, &full_id, cli)?;

        // Create feature branch from current HEAD
        if !cli.json {
            eprintln!("creating branch {}...", branch_name);
        }
        let output = Command::new("git")
            .args(["checkout", "-b", &branch_name])
            .current_dir(&paths.worktree_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrdError::Other(format!(
                "failed to create branch: {}",
                stderr.trim()
            )));
        }
    } else {
        // Git-native mode: sync with main, claim, push to main, then create feature branch

        // Step 2: Fetch and get current branch
        if !cli.json {
            eprintln!("syncing with origin/main...");
        }

        if has_origin(&paths.worktree_root) {
            let _ = git(&["fetch", "origin", "main"], &paths.worktree_root);
        }

        // Get current branch
        let current_branch = get_current_branch(&paths.worktree_root)?;

        // If not on main, checkout main
        if current_branch != "main" {
            if !cli.json {
                eprintln!("switching to main...");
            }
            if !git(&["checkout", "main"], &paths.worktree_root)? {
                return Err(BrdError::Other("failed to checkout main".to_string()));
            }
        }

        // Rebase on origin/main if it exists
        if has_origin_main(&paths.worktree_root)
            && !git(&["rebase", "origin/main"], &paths.worktree_root)?
        {
            let _ = git(&["rebase", "--abort"], &paths.worktree_root);
            return Err(BrdError::Other(
                "rebase failed - resolve conflicts manually".to_string(),
            ));
        }

        // Reload issues after sync (they may have changed)
        let mut issues = load_all_issues(paths, &config)?;

        // Step 3: Claim the issue
        if !cli.json {
            eprintln!("claiming {}...", full_id);
        }

        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(full_id.clone()))?;
        claim_issue(paths, &config, issue, &agent_id, false)?;

        // Step 4: Commit and push to main
        commit_and_push_main(paths, &full_id, cli)?;

        // Step 5: Create feature branch from main
        if !cli.json {
            eprintln!("creating branch {}...", branch_name);
        }

        let output = Command::new("git")
            .args(["checkout", "-b", &branch_name])
            .current_dir(&paths.worktree_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrdError::Other(format!(
                "failed to create branch: {}",
                stderr.trim()
            )));
        }
    }

    // Output success
    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "branch": branch_name,
            "issue_id": full_id,
            "owner": agent_id,
            "mode": if is_sync_mode { "local-sync" } else { "git-native" },
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("claimed: {} (owner: {})", full_id, agent_id);
        println!("branch:  {}", branch_name);
        println!();
        println!("the claim has been pushed to main - other agents will see it.");
        println!();
        println!("next steps:");
        println!("  # do the work, commit as usual");
        println!("  brd done {}    # mark done", full_id);
        println!("  brd agent pr   # create PR");
    }

    Ok(())
}

/// get the current branch name.
fn get_current_branch(cwd: &std::path::Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(cwd)
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(BrdError::Other("failed to get current branch".to_string()))
    }
}

/// extract issue ID from branch name.
/// supports both formats:
/// - "pr/<agent>/<issue-id>" (new format)
/// - "<agent>/<issue-id>" (legacy format for backwards compatibility)
fn extract_issue_id_from_branch(branch: &str) -> Option<&str> {
    if branch.starts_with("pr/") {
        branch.split('/').nth(2)
    } else {
        branch.split('/').nth(1)
    }
}

/// create a PR from the current branch using gh cli.
pub fn cmd_agent_pr(cli: &Cli, paths: &RepoPaths) -> Result<()> {
    let config = Config::load(&paths.config_path())?;

    // get current branch
    let branch = get_current_branch(&paths.worktree_root)?;

    // extract issue ID from branch name
    let issue_id = extract_issue_id_from_branch(&branch).ok_or_else(|| {
        BrdError::Other(format!(
            "branch '{}' doesn't match expected format 'pr/<agent>/<issue-id>'",
            branch
        ))
    })?;

    // load the issue
    let issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(issue_id, &issues)?;
    let issue = issues
        .get(&full_id)
        .ok_or_else(|| BrdError::Other(format!("issue '{}' not found", full_id)))?;

    // generate PR title and body
    let title = format!("feat: {} ({})", issue.title(), full_id);
    let body = if issue.body.is_empty() {
        format!("Closes: {}", full_id)
    } else {
        format!("{}\n\nCloses: {}", issue.body.trim(), full_id)
    };

    // push the branch first
    if !cli.json {
        eprintln!("pushing branch to origin...");
    }
    let push_output = Command::new("git")
        .args(["push", "-u", "origin", &branch])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !push_output.status.success() {
        let stderr = String::from_utf8_lossy(&push_output.stderr);
        // ignore "already up to date" type messages
        if !stderr.contains("Everything up-to-date") && !stderr.contains("set up to track") {
            return Err(BrdError::Other(format!(
                "failed to push branch: {}",
                stderr.trim()
            )));
        }
    }

    // create PR using gh cli
    if !cli.json {
        eprintln!("creating PR...");
    }
    let pr_output = Command::new("gh")
        .args([
            "pr", "create", "--title", &title, "--body", &body, "--base", "main",
        ])
        .current_dir(&paths.worktree_root)
        .output()?;

    if !pr_output.status.success() {
        let stderr = String::from_utf8_lossy(&pr_output.stderr);
        return Err(BrdError::Other(format!(
            "failed to create PR: {}",
            stderr.trim()
        )));
    }

    let pr_url = String::from_utf8_lossy(&pr_output.stdout)
        .trim()
        .to_string();

    if cli.json {
        let json = serde_json::json!({
            "ok": true,
            "pr_url": pr_url,
            "title": title,
            "issue_id": full_id,
            "branch": branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("created PR: {}", pr_url);
    }

    Ok(())
}

// ============================================================================
// AGENTS.md instructions management
// ============================================================================

/// current version of the agents block
pub const AGENTS_BLOCK_VERSION: u32 = 4;

const BLOCK_START: &str = "<!-- braid:agents:start";
const BLOCK_END: &str = "<!-- braid:agents:end -->";

/// generate the static part of the agents block (mode-independent)
fn generate_static_block() -> String {
    r#"## braid workflow

this repo uses braid (`brd`) for issue tracking. issues live in `.braid/issues/` as markdown files.

basic flow:
1. `brd start` — claim the next ready issue (auto-syncs, commits, and pushes)
2. do the work, commit as usual
3. `brd done <id>` — mark the issue complete
4. `brd agent ship` — push your work to main

useful commands:
- `brd ls` — list all issues
- `brd ready` — show issues with no unresolved dependencies
- `brd show <id>` — view issue details
- `brd mode` — show current workflow mode

## working in agent worktrees

**quick check — am i in a worktree?**

```bash
cat .braid/agent.toml 2>/dev/null && echo "yes, worktree" || echo "no, main"
```

if you're in a worktree:
- `brd start` handles syncing automatically
- use `brd agent ship` to merge your work to main (rebase + fast-forward push)
- if you see schema mismatch errors, rebase onto latest main

## design and meta issues

**design issues** (`type: design`) require human collaboration:
- don't close autonomously — discuss with human first
- research options, write up trade-offs in the issue body
- produce output before closing (implementation issues or a plan)
- only mark done after human approves

**meta issues** (`type: meta`) are tracking issues:
- group related work under a parent issue
- show progress as "done/total" in `brd ls`
- typically not picked up directly — work on the child issues instead"#
        .to_string()
}

/// generate the dynamic sync section based on mode
fn generate_sync_section(config: &Config) -> String {
    if let Some(ref branch) = config.sync_branch {
        format!(
            r#"## syncing issues (local-sync mode)

this repo uses **local-sync mode** — issues live on the `{branch}` branch in a shared worktree.

**how it works:**
- all local agents see issue changes instantly (shared filesystem)
- `brd start` and `brd done` write to the shared worktree automatically
- no manual commits needed for issue state changes

**remote sync:**
- run `brd sync` to push issue changes to the remote
- run `brd sync` to pull others' issue changes

**switching modes:**
- `brd mode` — show current mode
- `brd mode default` — switch back to git-native mode"#
        )
    } else {
        r#"## syncing issues (git-native mode)

this repo uses **git-native mode** — issues live alongside code and sync via git.

**how it works:**
- `brd start` auto-syncs: fetches, rebases, claims, commits, and pushes
- issue changes flow through your normal git workflow
- merge to main or create PRs to share issue state

**after marking an issue done:**
```bash
brd done <id>
git add .braid && git commit -m "done: <id>"
brd agent ship  # or create a PR
```

**switching modes:**
- `brd mode` — show current mode
- `brd mode local-sync` — switch to local-sync mode for multi-agent setups"#
            .to_string()
    }
}

/// generate the complete agents block content
pub fn generate_block(config: &Config) -> String {
    let static_block = generate_static_block();
    let sync_section = generate_sync_section(config);

    format!(
        "{BLOCK_START} v{AGENTS_BLOCK_VERSION} -->\n{static_block}\n\n{sync_section}\n{BLOCK_END}"
    )
}

/// extract version from an existing agents block
pub fn extract_version(content: &str) -> Option<u32> {
    let start_idx = content.find(BLOCK_START)?;
    let version_start = start_idx + BLOCK_START.len();
    let line_end = content[version_start..].find('\n')?;
    let version_str = content[version_start..version_start + line_end].trim();

    // parse "v1 -->" or similar
    version_str
        .strip_prefix('v')
        .and_then(|s| s.trim_end_matches("-->").trim().parse().ok())
}

/// check if AGENTS.md contains a braid block and return its version
pub fn check_agents_block(paths: &RepoPaths) -> Option<u32> {
    let agents_path = paths.worktree_root.join("AGENTS.md");
    if !agents_path.exists() {
        return None;
    }
    let content = fs::read_to_string(&agents_path).ok()?;
    extract_version(&content)
}

/// print the agents block to stdout
pub fn cmd_agents_show() -> Result<()> {
    // Show both modes for reference
    let git_native = Config::default();
    let local_sync = Config {
        sync_branch: Some("braid-issues".to_string()),
        ..Default::default()
    };

    println!("=== git-native mode ===\n");
    println!("{}", generate_block(&git_native));
    println!("\n\n=== local-sync mode ===\n");
    println!("{}", generate_block(&local_sync));
    Ok(())
}

/// inject or update the agents block in AGENTS.md
pub fn cmd_agents_inject(paths: &RepoPaths) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let agents_path = paths.worktree_root.join("AGENTS.md");
    let block = generate_block(&config);

    let mode_name = if config.sync_branch.is_some() {
        "local-sync"
    } else {
        "git-native"
    };

    if agents_path.exists() {
        let content = fs::read_to_string(&agents_path)?;

        if let Some(start_idx) = content.find(BLOCK_START) {
            // update existing block
            if let Some(end_marker_start) = content[start_idx..].find(BLOCK_END) {
                let end_idx = start_idx + end_marker_start + BLOCK_END.len();
                let new_content =
                    format!("{}{}{}", &content[..start_idx], block, &content[end_idx..]);
                fs::write(&agents_path, new_content)?;
                println!(
                    "updated braid agents block in AGENTS.md (v{}, {})",
                    AGENTS_BLOCK_VERSION, mode_name
                );
            } else {
                return Err(BrdError::Other(
                    "found start marker but no end marker in AGENTS.md".into(),
                ));
            }
        } else {
            // append to existing file
            let mut content = content;
            if !content.ends_with('\n') {
                content.push('\n');
            }
            content.push('\n');
            content.push_str(&block);
            content.push('\n');
            fs::write(&agents_path, content)?;
            println!(
                "added braid agents block to AGENTS.md (v{}, {})",
                AGENTS_BLOCK_VERSION, mode_name
            );
        }
    } else {
        // create new file
        fs::write(
            &agents_path,
            format!("# Instructions for AI agents\n\n{}\n", block),
        )?;
        println!(
            "created AGENTS.md with braid agents block (v{}, {})",
            AGENTS_BLOCK_VERSION, mode_name
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use tempfile::tempdir;

    static AGENT_INIT_TEST_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            unsafe { std::env::set_var(key, value) };
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

    fn git_ok(repo: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_output(repo: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn create_repo() -> (tempfile::TempDir, PathBuf, RepoPaths, String) {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join("repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        git_ok(&repo_path, &["init"]);
        git_ok(&repo_path, &["config", "user.email", "test@test.com"]);
        git_ok(&repo_path, &["config", "user.name", "test user"]);
        std::fs::write(repo_path.join("README.md"), "test\n").unwrap();
        git_ok(&repo_path, &["add", "."]);
        git_ok(&repo_path, &["commit", "-m", "init"]);

        let base_branch = git_output(&repo_path, &["rev-parse", "--abbrev-ref", "HEAD"]);

        let paths = RepoPaths {
            worktree_root: repo_path.clone(),
            git_common_dir: repo_path.join(".git"),
            brd_common_dir: repo_path.join(".git/brd"),
        };
        std::fs::create_dir_all(&paths.brd_common_dir).unwrap();

        (dir, repo_path, paths, base_branch)
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

    #[test]
    fn test_agent_init_creates_worktree_and_branch() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let cli = make_cli();
        cmd_agent_init(&cli, &paths, "agent-one", Some(&base_branch)).unwrap();

        let worktree_path = home_dir
            .path()
            .join(".braid")
            .join("worktrees")
            .join("repo")
            .join("agent-one");
        assert!(worktree_path.exists());
        assert!(worktree_path.join(".braid/agent.toml").exists());

        let agent_toml = std::fs::read_to_string(worktree_path.join(".braid/agent.toml")).unwrap();
        assert!(agent_toml.contains("agent_id = \"agent-one\""));

        let branch_list = git_output(&repo_path, &["branch", "--list", "agent-one"]);
        assert!(branch_list.contains("agent-one"));
    }

    #[test]
    fn test_agent_init_rejects_invalid_name() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, _repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let cli = make_cli();
        let err = cmd_agent_init(&cli, &paths, "bad name", Some(&base_branch)).unwrap_err();
        assert!(err.to_string().contains("invalid agent name"));
    }

    #[test]
    fn test_agent_init_rejects_duplicate_name() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, _repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let worktree_path = home_dir
            .path()
            .join(".braid")
            .join("worktrees")
            .join("repo")
            .join("agent-one");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let cli = make_cli();
        let err = cmd_agent_init(&cli, &paths, "agent-one", Some(&base_branch)).unwrap_err();
        assert!(err.to_string().contains("directory already exists"));
    }

    // ========================================================================
    // AGENTS.md tests
    // ========================================================================

    fn create_paths() -> (tempfile::TempDir, RepoPaths) {
        let dir = tempdir().unwrap();
        let braid_dir = dir.path().join(".braid");
        std::fs::create_dir_all(&braid_dir).unwrap();

        // Create a minimal config
        let config = Config::default();
        config.save(&braid_dir.join("config.toml")).unwrap();

        let paths = RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };
        std::fs::create_dir_all(&paths.brd_common_dir).unwrap();
        (dir, paths)
    }

    #[test]
    fn test_generate_block_git_native() {
        let config = Config::default();
        let block = generate_block(&config);
        assert!(block.contains("## syncing issues (git-native mode)"));
        assert!(block.contains("brd agent ship"));
        assert!(!block.contains("## syncing issues (local-sync mode)"));
    }

    #[test]
    fn test_generate_block_local_sync() {
        let config = Config {
            sync_branch: Some("braid-issues".to_string()),
            ..Default::default()
        };
        let block = generate_block(&config);
        assert!(block.contains("## syncing issues (local-sync mode)"));
        assert!(block.contains("braid-issues"));
        assert!(block.contains("brd sync"));
        assert!(!block.contains("## syncing issues (git-native mode)"));
    }

    #[test]
    fn test_extract_version_from_block() {
        let config = Config::default();
        let block = generate_block(&config);
        let content = format!("header\n{block}\nfooter");
        assert_eq!(extract_version(&content), Some(AGENTS_BLOCK_VERSION));
    }

    #[test]
    fn test_extract_version_missing_block() {
        let content = "no agents block here";
        assert_eq!(extract_version(content), None);
    }

    #[test]
    fn test_check_agents_block_reads_version() {
        let (_dir, paths) = create_paths();
        let agents_path = paths.worktree_root.join("AGENTS.md");
        let config = Config::default();
        std::fs::write(&agents_path, generate_block(&config)).unwrap();

        assert_eq!(check_agents_block(&paths), Some(AGENTS_BLOCK_VERSION));
    }

    #[test]
    fn test_cmd_agents_inject_creates_file() {
        let (_dir, paths) = create_paths();
        cmd_agents_inject(&paths).unwrap();

        let content = std::fs::read_to_string(paths.worktree_root.join("AGENTS.md")).unwrap();
        assert!(content.contains("Instructions for AI agents"));
        assert!(content.contains(BLOCK_START));
        assert!(content.contains(BLOCK_END));
        assert!(content.contains(&format!("v{AGENTS_BLOCK_VERSION}")));
    }

    #[test]
    fn test_cmd_agents_inject_appends_block() {
        let (_dir, paths) = create_paths();
        let agents_path = paths.worktree_root.join("AGENTS.md");
        std::fs::write(&agents_path, "custom header\n").unwrap();

        cmd_agents_inject(&paths).unwrap();

        let content = std::fs::read_to_string(&agents_path).unwrap();
        assert!(content.starts_with("custom header"));
        assert!(content.contains(BLOCK_START));
        assert!(content.contains(BLOCK_END));
    }

    #[test]
    fn test_cmd_agents_inject_updates_existing_block() {
        let (_dir, paths) = create_paths();
        let agents_path = paths.worktree_root.join("AGENTS.md");
        let old_block = format!("{BLOCK_START} v1 -->\nold\n{BLOCK_END}");
        std::fs::write(&agents_path, format!("before\n{old_block}\nafter")).unwrap();

        cmd_agents_inject(&paths).unwrap();

        let content = std::fs::read_to_string(&agents_path).unwrap();
        assert!(content.contains("before"));
        assert!(content.contains("after"));
        assert!(!content.contains("old\n"));
        assert!(content.contains(&format!("v{AGENTS_BLOCK_VERSION}")));
    }

    #[test]
    fn test_cmd_agents_inject_missing_end_marker() {
        let (_dir, paths) = create_paths();
        let agents_path = paths.worktree_root.join("AGENTS.md");
        std::fs::write(&agents_path, format!("{BLOCK_START} v1 -->\nno end")).unwrap();

        let err = cmd_agents_inject(&paths).unwrap_err();
        assert!(err.to_string().contains("no end marker"));
    }
}
