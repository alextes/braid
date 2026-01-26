//! brd agent commands - worktree management and AGENTS.md instructions.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::git;
use crate::lock::LockGuard;
use crate::repo::{self, RepoPaths};

use super::{
    claim_issue, commit_and_push_issues_branch, commit_and_push_main, load_all_issues,
    resolve_issue_id,
};

pub fn cmd_agent_init(cli: &Cli, paths: &RepoPaths, name: &str, base: Option<&str>) -> Result<()> {
    // refuse if already in an agent worktree
    let agent_toml = paths.braid_dir().join("agent.toml");
    if agent_toml.exists() {
        return Err(BrdError::Other(
            "already in an agent worktree. run `brd agent init` from main instead.".to_string(),
        ));
    }

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
    let issues_branch = config.as_ref().and_then(|c| c.issues_branch.clone());

    if let Some(ref branch) = issues_branch {
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
            "issues_branch": issues_branch,
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Created agent worktree: {}", name);
        println!("  path:   {}", worktree_path.display());
        println!("  branch: {} (from {})", name, base_branch);
        if let Some(sb) = &issues_branch {
            println!("  sync:   issues on '{}' branch", sb);
        }
        println!();
        println!("To use this agent:");
        println!("  cd {}", worktree_path.display());
        println!("  # start your agent (claude, codex, gemini, etc.)");
        println!("  brd next  # get next issue to work on");
        if issues_branch.is_some() {
            println!("  brd sync  # sync issues with remote");
        }
    }

    Ok(())
}

/// create a feature branch for PR workflow.
/// this command:
/// 1. syncs with main and claims the issue (pushing to main so other agents see it)
/// 2. creates a feature branch for working on the issue
pub fn cmd_agent_branch(cli: &Cli, paths: &RepoPaths, issue_id: &str) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let is_sync_mode = config.is_issues_branch_mode();
    let agent_id = repo::get_agent_id(&paths.worktree_root);

    // Step 1: Check for clean working tree
    if !git::is_clean(&paths.worktree_root)? {
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
    if git::branch_exists(&paths.worktree_root, &branch_name) {
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
        commit_and_push_issues_branch(paths, &config, &full_id, cli)?;

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

        if git::has_remote(&paths.worktree_root, "origin") {
            let _ = git::run(&["fetch", "origin", "main"], &paths.worktree_root);
        }

        // Get current branch
        let current_branch = git::current_branch(&paths.worktree_root)?;

        // If not on main, checkout main
        if current_branch != "main" {
            if !cli.json {
                eprintln!("switching to main...");
            }
            if !git::run(&["checkout", "main"], &paths.worktree_root)? {
                return Err(BrdError::Other("failed to checkout main".to_string()));
            }
        }

        // Rebase on origin/main if it exists
        if git::has_remote_branch(&paths.worktree_root, "origin", "main")
            && !git::run(&["rebase", "origin/main"], &paths.worktree_root)?
        {
            let _ = git::run(&["rebase", "--abort"], &paths.worktree_root);
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
    let branch = git::current_branch(&paths.worktree_root)?;

    // check if on main
    if branch == "main" {
        return Err(BrdError::Other(
            "PRs require a feature branch - use `brd agent init <name>` to create one".to_string(),
        ));
    }

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
pub const AGENTS_BLOCK_VERSION: u32 = 7;

const BLOCK_START: &str = "<!-- braid:agents:start";
const BLOCK_END: &str = "<!-- braid:agents:end -->";

/// Files to check for the braid instruction block, in priority order.
/// The first file found containing the block will be used.
pub const INSTRUCTION_FILES: &[&str] = &["AGENTS.md", "CLAUDE.md", "CLAUDE.local.md"];

/// generate the static part of the agents block (mode-independent)
fn generate_static_block() -> String {
    r#"## braid workflow

this repo uses braid (`brd`) for issue tracking. issues live in `.braid/issues/` as markdown files.

basic flow:
1. `brd start` — claim the next ready issue
2. do the work, commit as usual
3. `brd done <id>` — mark the issue complete
4. ship your work:
   - in a worktree: `brd agent merge` (rebase + ff-merge to main)
   - on main: just `git push` (you're already there)

useful commands:
- `brd ls` — list all issues
- `brd ready` — show issues with no unresolved dependencies
- `brd show <id>` — view issue details (shows deps and dependents)
- `brd show <id> --context` — view issue with full content of related issues
- `brd config` — show current workflow configuration

**tip:** before starting work, use `brd show <id> --context` to see the issue plus all its dependencies and dependents in one view.

## working on main vs in a worktree

**quick check — am i in a worktree?**

```bash
cat .braid/agent.toml 2>/dev/null && echo "yes, worktree" || echo "no, main"
```

**if you're in a worktree (feature branch):**
- `brd start` handles syncing automatically
- use `brd agent merge` to ship (rebase + ff-merge to main)
- if you see schema mismatch errors, rebase onto latest main

**if you're on main:**
- `brd start` syncs and claims
- after `brd done`, just `git push` your code commits
- no `brd agent merge` needed — you're already on main

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

/// generate the dynamic sync section based on config
fn generate_sync_section(config: &Config) -> String {
    if let Some(ref branch) = config.issues_branch {
        format!(
            r#"## syncing issues (issues-branch mode)

this repo uses **issues-branch** — issues live on the `{branch}` branch in a shared worktree.

**how it works:**
- all local agents see issue changes instantly (shared filesystem)
- `brd start` and `brd done` write to the shared worktree automatically
- no manual commits needed for issue state changes

**remote sync:**
- run `brd sync` to push issue changes to the remote
- run `brd sync` to pull others' issue changes

**changing settings:**
- `brd config` — show current config
- `brd config issues-branch --clear` — disable issues-branch"#
        )
    } else {
        r#"## syncing issues (issues with code)

this repo stores issues **with code** — issues live in `.braid/issues/` and sync via git.

**how it works:**
- `brd start` auto-syncs: fetches, rebases, claims, commits, and pushes
- `brd done` marks complete and auto-pushes (if auto_push enabled)
- issue changes flow through your normal git workflow

**in a worktree (feature branch):**
```bash
brd done <id>        # marks done, auto-pushes issue state
brd agent merge      # ship code to main (rebase + ff-merge)
```

**on main:**
```bash
brd done <id>        # marks done, auto-pushes issue state
git push             # push your code commits
```

**changing settings:**
- `brd config` — show current config
- `brd config issues-branch <name>` — enable issues-branch for multi-agent setups"#
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

/// Mode indicator for AGENTS.md block
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentsBlockMode {
    GitNative,
    LocalSync,
}

impl std::fmt::Display for AgentsBlockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentsBlockMode::GitNative => write!(f, "git-native"),
            AgentsBlockMode::LocalSync => write!(f, "local-sync"),
        }
    }
}

/// extract mode from an existing agents block by checking for mode-specific headers
pub fn extract_mode(content: &str) -> Option<AgentsBlockMode> {
    // check for mode-specific section headers within the block
    // support both new and old header formats for compatibility
    if content.contains("## syncing issues (issues-branch mode)")
        || content.contains("## syncing issues (local-sync mode)")
    {
        Some(AgentsBlockMode::LocalSync)
    } else if content.contains("## syncing issues (issues with code)")
        || content.contains("## syncing issues (git-native mode)")
    {
        Some(AgentsBlockMode::GitNative)
    } else {
        None
    }
}

/// Check instruction files for a braid block and return the file name and version.
/// Checks files in INSTRUCTION_FILES order: AGENTS.md, CLAUDE.md, CLAUDE.local.md.
pub fn check_agents_block(paths: &RepoPaths) -> Option<(&'static str, u32)> {
    for &file in INSTRUCTION_FILES {
        let file_path = paths.worktree_root.join(file);
        if !file_path.exists() {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&file_path)
            && let Some(version) = extract_version(&content)
        {
            return Some((file, version));
        }
    }
    None
}

/// print the agents block to stdout
pub fn cmd_agents_show() -> Result<()> {
    // Show both modes for reference
    let git_native = Config::default();
    let local_sync = Config {
        issues_branch: Some("braid-issues".to_string()),
        ..Default::default()
    };

    println!("=== git-native mode ===\n");
    println!("{}", generate_block(&git_native));
    println!("\n\n=== local-sync mode ===\n");
    println!("{}", generate_block(&local_sync));
    Ok(())
}

/// result of injecting the agents block
pub enum InjectResult {
    /// created a new file
    Created,
    /// added block to existing file
    Added,
    /// updated existing block
    Updated,
}

/// inject or update the agents block in a file (core logic, no printing)
pub fn inject_agents_block(
    worktree_root: &std::path::Path,
    config: &Config,
    file_name: &str,
) -> Result<InjectResult> {
    let agents_path = worktree_root.join(file_name);
    let block = generate_block(config);

    if agents_path.exists() {
        let content = fs::read_to_string(&agents_path)?;

        if let Some(start_idx) = content.find(BLOCK_START) {
            // update existing block
            if let Some(end_marker_start) = content[start_idx..].find(BLOCK_END) {
                let end_idx = start_idx + end_marker_start + BLOCK_END.len();
                let new_content =
                    format!("{}{}{}", &content[..start_idx], block, &content[end_idx..]);
                fs::write(&agents_path, new_content)?;
                Ok(InjectResult::Updated)
            } else {
                Err(BrdError::Other(format!(
                    "found start marker but no end marker in {}",
                    file_name
                )))
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
            Ok(InjectResult::Added)
        }
    } else {
        // create new file
        fs::write(
            &agents_path,
            format!("# Instructions for AI agents\n\n{}\n", block),
        )?;
        Ok(InjectResult::Created)
    }
}

/// inject or update the agents block in AGENTS.md (or custom file)
pub fn cmd_agents_inject(paths: &RepoPaths, file: Option<&str>) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let file_name = file.unwrap_or("AGENTS.md");

    let mode_name = if config.issues_branch.is_some() {
        "local-sync"
    } else {
        "git-native"
    };

    let result = inject_agents_block(&paths.worktree_root, &config, file_name)?;

    match result {
        InjectResult::Created => println!(
            "created {} with braid agents block (v{}, {})",
            file_name, AGENTS_BLOCK_VERSION, mode_name
        ),
        InjectResult::Added => println!(
            "added braid agents block to {} (v{}, {})",
            file_name, AGENTS_BLOCK_VERSION, mode_name
        ),
        InjectResult::Updated => println!(
            "updated braid agents block in {} (v{}, {})",
            file_name, AGENTS_BLOCK_VERSION, mode_name
        ),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
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

    use crate::git::test::{output as git_output, run_ok as git_ok};

    fn create_repo() -> (tempfile::TempDir, PathBuf, RepoPaths, String) {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join("repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        git_ok(&repo_path, &["init"]);
        git_ok(&repo_path, &["config", "user.email", "test@test.com"]);
        git_ok(&repo_path, &["config", "user.name", "test user"]);
        git_ok(&repo_path, &["config", "commit.gpgsign", "false"]);
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

    use crate::test_utils::test_cli;

    #[test]
    fn test_agent_init_creates_worktree_and_branch() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        cmd_agent_init(&test_cli(), &paths, "agent-one", Some(&base_branch)).unwrap();

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

        let cli = test_cli();
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

        let cli = test_cli();
        let err = cmd_agent_init(&cli, &paths, "agent-one", Some(&base_branch)).unwrap_err();
        assert!(err.to_string().contains("directory already exists"));
    }

    #[test]
    fn test_agent_init_rejects_from_existing_worktree() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        // simulate being in an agent worktree by creating agent.toml
        let braid_dir = repo_path.join(".braid");
        std::fs::create_dir_all(&braid_dir).unwrap();
        std::fs::write(braid_dir.join("agent.toml"), "agent_id = \"existing\"\n").unwrap();

        let cli = test_cli();
        let err = cmd_agent_init(&cli, &paths, "new-agent", Some(&base_branch)).unwrap_err();
        assert!(
            err.to_string().contains("already in an agent worktree"),
            "expected 'already in an agent worktree', got: {}",
            err
        );
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
        assert!(block.contains("## syncing issues (issues with code)"));
        assert!(block.contains("brd agent merge"));
        assert!(!block.contains("## syncing issues (issues-branch mode)"));
    }

    #[test]
    fn test_generate_block_local_sync() {
        let config = Config {
            issues_branch: Some("braid-issues".to_string()),
            ..Default::default()
        };
        let block = generate_block(&config);
        assert!(block.contains("## syncing issues (issues-branch mode)"));
        assert!(block.contains("braid-issues"));
        assert!(block.contains("brd sync"));
        assert!(!block.contains("## syncing issues (issues with code)"));
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
    fn test_extract_mode_git_native() {
        let config = Config::default();
        let block = generate_block(&config);
        assert_eq!(extract_mode(&block), Some(AgentsBlockMode::GitNative));
    }

    #[test]
    fn test_extract_mode_local_sync() {
        let config = Config {
            issues_branch: Some("braid-issues".to_string()),
            ..Default::default()
        };
        let block = generate_block(&config);
        assert_eq!(extract_mode(&block), Some(AgentsBlockMode::LocalSync));
    }

    #[test]
    fn test_extract_mode_missing_block() {
        let content = "no agents block here";
        assert_eq!(extract_mode(content), None);
    }

    #[test]
    fn test_agents_block_mode_display() {
        assert_eq!(format!("{}", AgentsBlockMode::GitNative), "git-native");
        assert_eq!(format!("{}", AgentsBlockMode::LocalSync), "local-sync");
    }

    #[test]
    fn test_check_agents_block_reads_version() {
        let (_dir, paths) = create_paths();
        let agents_path = paths.worktree_root.join("AGENTS.md");
        let config = Config::default();
        std::fs::write(&agents_path, generate_block(&config)).unwrap();

        assert_eq!(
            check_agents_block(&paths),
            Some(("AGENTS.md", AGENTS_BLOCK_VERSION))
        );
    }

    #[test]
    fn test_check_agents_block_checks_claude_md() {
        let (_dir, paths) = create_paths();
        let claude_path = paths.worktree_root.join("CLAUDE.md");
        let config = Config::default();
        std::fs::write(&claude_path, generate_block(&config)).unwrap();

        assert_eq!(
            check_agents_block(&paths),
            Some(("CLAUDE.md", AGENTS_BLOCK_VERSION))
        );
    }

    #[test]
    fn test_check_agents_block_checks_claude_local_md() {
        let (_dir, paths) = create_paths();
        let claude_local_path = paths.worktree_root.join("CLAUDE.local.md");
        let config = Config::default();
        std::fs::write(&claude_local_path, generate_block(&config)).unwrap();

        assert_eq!(
            check_agents_block(&paths),
            Some(("CLAUDE.local.md", AGENTS_BLOCK_VERSION))
        );
    }

    #[test]
    fn test_check_agents_block_priority_order() {
        let (_dir, paths) = create_paths();
        let config = Config::default();
        let block = generate_block(&config);

        // Write to both CLAUDE.md and CLAUDE.local.md
        std::fs::write(paths.worktree_root.join("CLAUDE.md"), &block).unwrap();
        std::fs::write(paths.worktree_root.join("CLAUDE.local.md"), &block).unwrap();

        // AGENTS.md has highest priority, but doesn't exist, so CLAUDE.md should be found
        assert_eq!(
            check_agents_block(&paths),
            Some(("CLAUDE.md", AGENTS_BLOCK_VERSION))
        );

        // Now create AGENTS.md - it should take priority
        std::fs::write(paths.worktree_root.join("AGENTS.md"), &block).unwrap();
        assert_eq!(
            check_agents_block(&paths),
            Some(("AGENTS.md", AGENTS_BLOCK_VERSION))
        );
    }

    #[test]
    fn test_check_agents_block_not_found() {
        let (_dir, paths) = create_paths();
        // No instruction files exist
        assert_eq!(check_agents_block(&paths), None);
    }

    #[test]
    fn test_cmd_agents_inject_creates_file() {
        let (_dir, paths) = create_paths();
        cmd_agents_inject(&paths, None).unwrap();

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

        cmd_agents_inject(&paths, None).unwrap();

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

        cmd_agents_inject(&paths, None).unwrap();

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

        let err = cmd_agents_inject(&paths, None).unwrap_err();
        assert!(err.to_string().contains("no end marker"));
    }

    #[test]
    fn test_cmd_agents_inject_custom_file() {
        let (_dir, paths) = create_paths();
        cmd_agents_inject(&paths, Some("CLAUDE.md")).unwrap();

        // Custom file should be created
        let content = std::fs::read_to_string(paths.worktree_root.join("CLAUDE.md")).unwrap();
        assert!(content.contains("Instructions for AI agents"));
        assert!(content.contains(BLOCK_START));
        assert!(content.contains(BLOCK_END));

        // AGENTS.md should not exist
        assert!(!paths.worktree_root.join("AGENTS.md").exists());
    }

    #[test]
    fn test_cmd_agents_inject_custom_file_in_subdir() {
        let (_dir, paths) = create_paths();
        std::fs::create_dir_all(paths.worktree_root.join(".github")).unwrap();

        cmd_agents_inject(&paths, Some(".github/AGENTS.md")).unwrap();

        let content =
            std::fs::read_to_string(paths.worktree_root.join(".github/AGENTS.md")).unwrap();
        assert!(content.contains(BLOCK_START));
    }

    // ========================================================================
    // extract_issue_id_from_branch tests
    // ========================================================================

    #[test]
    fn test_extract_issue_id_new_format() {
        assert_eq!(
            extract_issue_id_from_branch("pr/agent-one/brd-1234"),
            Some("brd-1234")
        );
    }

    #[test]
    fn test_extract_issue_id_legacy_format() {
        assert_eq!(
            extract_issue_id_from_branch("agent-one/brd-1234"),
            Some("brd-1234")
        );
    }

    #[test]
    fn test_extract_issue_id_no_slash() {
        assert_eq!(extract_issue_id_from_branch("main"), None);
    }

    #[test]
    fn test_extract_issue_id_single_slash() {
        // legacy format with single slash
        assert_eq!(
            extract_issue_id_from_branch("feature/something"),
            Some("something")
        );
    }

    // ========================================================================
    // cmd_agent_init additional tests
    // ========================================================================

    #[test]
    fn test_agent_init_uses_current_branch_as_default_base() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, repo_path, paths, _base_branch) = create_repo();

        // Create and checkout a different branch
        git_ok(&repo_path, &["checkout", "-b", "develop"]);

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let cli = test_cli();
        // Don't specify base - should use current branch (develop)
        cmd_agent_init(&cli, &paths, "agent-dev", None).unwrap();

        let worktree_path = home_dir.path().join(".braid/worktrees/repo/agent-dev");

        // Verify worktree was created
        assert!(worktree_path.exists());

        // The agent branch should exist
        let branch_list = git_output(&repo_path, &["branch", "--list", "agent-dev"]);
        assert!(branch_list.contains("agent-dev"));
    }

    #[test]
    fn test_agent_init_json_output() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, _repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let cli = Cli {
            json: true,
            repo: None,
            no_color: true,
            verbose: false,
            command: crate::cli::Command::Doctor,
        };

        // This should succeed and produce JSON output (we can't easily capture stdout in unit tests)
        let result = cmd_agent_init(&cli, &paths, "agent-json", Some(&base_branch));
        assert!(result.is_ok());
    }

    #[test]
    fn test_agent_init_with_underscore_in_name() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, _repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let cli = test_cli();
        let result = cmd_agent_init(&cli, &paths, "agent_with_underscores", Some(&base_branch));
        assert!(result.is_ok());

        let worktree_path = home_dir
            .path()
            .join(".braid/worktrees/repo/agent_with_underscores");
        assert!(worktree_path.exists());
    }

    #[test]
    fn test_agent_init_rejects_special_chars() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, _repo_path, paths, base_branch) = create_repo();

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let cli = test_cli();

        // Test various invalid names
        let invalid_names = ["agent@one", "agent/one", "agent.one", "agent:one"];
        for name in invalid_names {
            let err = cmd_agent_init(&cli, &paths, name, Some(&base_branch)).unwrap_err();
            assert!(
                err.to_string().contains("invalid agent name"),
                "expected error for name '{}', got: {}",
                name,
                err
            );
        }
    }

    #[test]
    fn test_agent_init_with_sync_branch_mode() {
        let _lock = AGENT_INIT_TEST_LOCK.lock().unwrap();
        let (_dir, repo_path, paths, base_branch) = create_repo();

        // Set up sync branch mode
        let braid_dir = repo_path.join(".braid");
        std::fs::create_dir_all(&braid_dir).unwrap();
        std::fs::write(
            braid_dir.join("config.toml"),
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\nissues_branch = \"braid-issues\"\n",
        )
        .unwrap();

        // Create the issues branch
        git_ok(&repo_path, &["branch", "braid-issues"]);

        let home_dir = tempdir().unwrap();
        let _env = EnvGuard::set("HOME", home_dir.path().to_str().unwrap());

        let cli = test_cli();
        let result = cmd_agent_init(&cli, &paths, "sync-agent", Some(&base_branch));
        assert!(result.is_ok());

        // Verify issues worktree was created
        assert!(paths.brd_common_dir.join("issues").exists());
    }

    // ========================================================================
    // cmd_agent_branch tests
    // ========================================================================

    fn create_repo_with_issue() -> (tempfile::TempDir, PathBuf, RepoPaths) {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().join("repo");
        std::fs::create_dir_all(&repo_path).unwrap();

        git_ok(&repo_path, &["init"]);
        git_ok(&repo_path, &["config", "user.email", "test@test.com"]);
        git_ok(&repo_path, &["config", "user.name", "test user"]);
        git_ok(&repo_path, &["config", "commit.gpgsign", "false"]);

        // Create .braid structure with config and an issue
        let braid_dir = repo_path.join(".braid");
        let issues_dir = braid_dir.join("issues");
        std::fs::create_dir_all(&issues_dir).unwrap();

        std::fs::write(
            braid_dir.join("config.toml"),
            "schema_version = 6\nid_prefix = \"tst\"\nid_len = 4\n",
        )
        .unwrap();

        // Create agent.toml so get_agent_id works
        std::fs::write(braid_dir.join("agent.toml"), "agent_id = \"test-agent\"\n").unwrap();

        // Create a test issue (no type field = regular issue)
        std::fs::write(
            issues_dir.join("tst-0001.md"),
            "---\nschema_version: 6\nid: tst-0001\ntitle: test issue\npriority: P2\nstatus: open\ndeps: []\ncreated_at: 2024-01-01T00:00:00Z\nupdated_at: 2024-01-01T00:00:00Z\n---\n",
        )
        .unwrap();

        std::fs::write(repo_path.join("README.md"), "test\n").unwrap();
        git_ok(&repo_path, &["add", "."]);
        git_ok(&repo_path, &["commit", "-m", "init"]);

        // Ensure branch is named "main" (some systems default to "master")
        let current = git_output(&repo_path, &["rev-parse", "--abbrev-ref", "HEAD"]);
        if current != "main" {
            git_ok(&repo_path, &["branch", "-m", &current, "main"]);
        }

        let paths = RepoPaths {
            worktree_root: repo_path.clone(),
            git_common_dir: repo_path.join(".git"),
            brd_common_dir: repo_path.join(".git/brd"),
        };
        std::fs::create_dir_all(&paths.brd_common_dir).unwrap();

        (dir, repo_path, paths)
    }

    #[test]
    fn test_agent_branch_rejects_dirty_worktree() {
        let (_dir, repo_path, paths) = create_repo_with_issue();
        let cli = test_cli();

        // Create uncommitted changes
        std::fs::write(repo_path.join("dirty.txt"), "uncommitted").unwrap();

        let err = cmd_agent_branch(&cli, &paths, "tst-0001").unwrap_err();
        assert!(err.to_string().contains("uncommitted changes"));
    }

    #[test]
    fn test_agent_branch_rejects_nonexistent_issue() {
        let (_dir, _repo_path, paths) = create_repo_with_issue();
        let cli = test_cli();

        let err = cmd_agent_branch(&cli, &paths, "tst-9999").unwrap_err();
        assert!(
            err.to_string().contains("not found") || err.to_string().contains("no issue matching"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_agent_branch_creates_branch_git_native() {
        let (_dir, repo_path, paths) = create_repo_with_issue();
        let cli = test_cli();

        let result = cmd_agent_branch(&cli, &paths, "tst-0001");
        assert!(result.is_ok(), "cmd_agent_branch failed: {:?}", result);

        // Check branch was created
        let current_branch = git_output(&repo_path, &["rev-parse", "--abbrev-ref", "HEAD"]);
        assert_eq!(current_branch, "pr/test-agent/tst-0001");
    }

    #[test]
    fn test_agent_branch_rejects_duplicate_branch() {
        let (_dir, repo_path, paths) = create_repo_with_issue();
        let cli = test_cli();

        // Create the branch first
        git_ok(&repo_path, &["branch", "pr/test-agent/tst-0001"]);

        let err = cmd_agent_branch(&cli, &paths, "tst-0001").unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    // ========================================================================
    // cmd_agent_pr tests
    // ========================================================================

    #[test]
    fn test_agent_pr_rejects_main_branch() {
        let (_dir, repo_path, paths) = create_repo_with_issue();
        let cli = test_cli();

        // Ensure we're on main (the default branch might be "master" on some systems)
        let current = git_output(&repo_path, &["rev-parse", "--abbrev-ref", "HEAD"]);
        if current != "main" {
            git_ok(&repo_path, &["branch", "-m", &current, "main"]);
        }

        let err = cmd_agent_pr(&cli, &paths).unwrap_err();
        assert!(
            err.to_string().contains("feature branch"),
            "expected 'feature branch' error, got: {}",
            err
        );
    }

    #[test]
    fn test_agent_pr_rejects_invalid_branch_format() {
        let (_dir, repo_path, paths) = create_repo_with_issue();
        let cli = test_cli();

        // Create a branch that doesn't match the expected format
        git_ok(&repo_path, &["checkout", "-b", "random-branch"]);

        let err = cmd_agent_pr(&cli, &paths).unwrap_err();
        assert!(err.to_string().contains("doesn't match expected format"));
    }

    #[test]
    fn test_agent_pr_extracts_issue_from_branch() {
        let (_dir, repo_path, paths) = create_repo_with_issue();
        let cli = test_cli();

        // Create proper feature branch
        git_ok(&repo_path, &["checkout", "-b", "pr/test-agent/tst-0001"]);

        // This will fail because gh CLI isn't available, but we can check it gets past branch validation
        let err = cmd_agent_pr(&cli, &paths).unwrap_err();
        // Error should be about gh CLI or pushing, not about branch format
        assert!(
            !err.to_string().contains("doesn't match expected format"),
            "unexpected error about branch format: {}",
            err
        );
    }
}
