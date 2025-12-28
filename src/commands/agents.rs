//! brd agent inject/instructions - manage agent instructions block in AGENTS.md.

use std::fs;

use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::repo::RepoPaths;

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
- `brd mode sync-local` — switch to local-sync mode for multi-agent setups"#
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
    let mut local_sync = Config::default();
    local_sync.sync_branch = Some("braid-issues".to_string());

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
    use std::fs;
    use tempfile::tempdir;

    fn create_paths() -> (tempfile::TempDir, RepoPaths) {
        let dir = tempdir().unwrap();
        let braid_dir = dir.path().join(".braid");
        fs::create_dir_all(&braid_dir).unwrap();

        // Create a minimal config
        let config = Config::default();
        config.save(&braid_dir.join("config.toml")).unwrap();

        let paths = RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };
        fs::create_dir_all(&paths.brd_common_dir).unwrap();
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
        let mut config = Config::default();
        config.sync_branch = Some("braid-issues".to_string());
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
        fs::write(&agents_path, generate_block(&config)).unwrap();

        assert_eq!(check_agents_block(&paths), Some(AGENTS_BLOCK_VERSION));
    }

    #[test]
    fn test_cmd_agents_inject_creates_file() {
        let (_dir, paths) = create_paths();
        cmd_agents_inject(&paths).unwrap();

        let content = fs::read_to_string(paths.worktree_root.join("AGENTS.md")).unwrap();
        assert!(content.contains("Instructions for AI agents"));
        assert!(content.contains(BLOCK_START));
        assert!(content.contains(BLOCK_END));
        assert!(content.contains(&format!("v{AGENTS_BLOCK_VERSION}")));
    }

    #[test]
    fn test_cmd_agents_inject_appends_block() {
        let (_dir, paths) = create_paths();
        let agents_path = paths.worktree_root.join("AGENTS.md");
        fs::write(&agents_path, "custom header\n").unwrap();

        cmd_agents_inject(&paths).unwrap();

        let content = fs::read_to_string(&agents_path).unwrap();
        assert!(content.starts_with("custom header"));
        assert!(content.contains(BLOCK_START));
        assert!(content.contains(BLOCK_END));
    }

    #[test]
    fn test_cmd_agents_inject_updates_existing_block() {
        let (_dir, paths) = create_paths();
        let agents_path = paths.worktree_root.join("AGENTS.md");
        let old_block = format!("{BLOCK_START} v1 -->\nold\n{BLOCK_END}");
        fs::write(&agents_path, format!("before\n{old_block}\nafter")).unwrap();

        cmd_agents_inject(&paths).unwrap();

        let content = fs::read_to_string(&agents_path).unwrap();
        assert!(content.contains("before"));
        assert!(content.contains("after"));
        assert!(!content.contains("old\n"));
        assert!(content.contains(&format!("v{AGENTS_BLOCK_VERSION}")));
    }

    #[test]
    fn test_cmd_agents_inject_missing_end_marker() {
        let (_dir, paths) = create_paths();
        let agents_path = paths.worktree_root.join("AGENTS.md");
        fs::write(&agents_path, format!("{BLOCK_START} v1 -->\nno end")).unwrap();

        let err = cmd_agents_inject(&paths).unwrap_err();
        assert!(err.to_string().contains("no end marker"));
    }
}
