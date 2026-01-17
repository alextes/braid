//! brd dep add/rm commands.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::graph::would_create_cycle;
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::{load_all_issues, resolve_issue_id};

pub fn cmd_dep_add(cli: &Cli, paths: &RepoPaths, blocked_id: &str, blocker_id: &str) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths, &config)?;
    let blocked_full = resolve_issue_id(blocked_id, &issues)?;
    let blocker_full = resolve_issue_id(blocker_id, &issues)?;

    // check not self-dep
    if blocked_full == blocker_full {
        return Err(BrdError::Other("cannot add self-dependency".to_string()));
    }

    // check for cycles
    if let Some(cycle_path) = would_create_cycle(&blocked_full, &blocker_full, &issues) {
        let cycle_str = cycle_path.join(" -> ");
        return Err(BrdError::Other(format!(
            "cannot add dependency: would create cycle: {}",
            cycle_str
        )));
    }

    let blocked = issues
        .get_mut(&blocked_full)
        .ok_or_else(|| BrdError::IssueNotFound(blocked_id.to_string()))?;

    if !blocked.frontmatter.deps.contains(&blocker_full) {
        blocked.frontmatter.deps.push(blocker_full.clone());
        blocked.touch();
        let issue_path = paths
            .issues_dir(&config)
            .join(format!("{}.md", blocked_full));
        blocked.save(&issue_path)?;
    }

    if cli.json {
        println!(r#"{{"ok": true}}"#);
    } else {
        println!(
            "added dependency: {} blocked by {}",
            blocked_full, blocker_full
        );
    }

    Ok(())
}

pub fn cmd_dep_rm(cli: &Cli, paths: &RepoPaths, blocked_id: &str, blocker_id: &str) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths, &config)?;
    let blocked_full = resolve_issue_id(blocked_id, &issues)?;
    let blocker_full = resolve_issue_id(blocker_id, &issues)?;

    let blocked = issues
        .get_mut(&blocked_full)
        .ok_or_else(|| BrdError::IssueNotFound(blocked_id.to_string()))?;

    blocked.frontmatter.deps.retain(|d| d != &blocker_full);
    blocked.touch();
    let issue_path = paths
        .issues_dir(&config)
        .join(format!("{}.md", blocked_full));
    blocked.save(&issue_path)?;

    if cli.json {
        println!(r#"{{"ok": true}}"#);
    } else {
        println!(
            "removed dependency: {} no longer blocked by {}",
            blocked_full, blocker_full
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate::CURRENT_SCHEMA;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_repo() -> (tempfile::TempDir, RepoPaths) {
        let dir = tempdir().unwrap();
        let paths = RepoPaths {
            worktree_root: dir.path().to_path_buf(),
            git_common_dir: dir.path().join(".git"),
            brd_common_dir: dir.path().join(".git/brd"),
        };
        // Create .git/brd for lock file
        fs::create_dir_all(&paths.brd_common_dir).unwrap();
        (dir, paths)
    }

    fn create_braid_dir(paths: &RepoPaths) {
        fs::create_dir_all(paths.braid_dir().join("issues")).unwrap();
    }

    fn create_valid_config(paths: &RepoPaths) {
        let config = Config::default();
        config.save(&paths.config_path()).unwrap();
    }

    fn create_issue(paths: &RepoPaths, id: &str, deps: &[&str]) {
        let config = Config::default();
        let deps_yaml = if deps.is_empty() {
            "deps: []".to_string()
        } else {
            format!(
                "deps:\n{}",
                deps.iter()
                    .map(|d| format!("  - {}", d))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };
        let content = format!(
            r#"---
schema_version: {}
id: {}
title: Test Issue {}
priority: P2
status: todo
{}
tags: []
owner: ~
created_at: 2024-01-01T00:00:00Z
updated_at: 2024-01-01T00:00:00Z
acceptance: []
---
Test body."#,
            CURRENT_SCHEMA, id, id, deps_yaml
        );
        let issue_path = paths.issues_dir(&config).join(format!("{}.md", id));
        fs::write(issue_path, content).unwrap();
    }

    fn make_cli() -> Cli {
        Cli {
            json: false,
            repo: None,
            no_color: true,
            verbose: false,
            command: crate::cli::Command::Doctor, // placeholder, not used
        }
    }

    // =========================================================================
    // cmd_dep_add tests
    // =========================================================================

    #[test]
    fn test_dep_add_self_dependency_rejected() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "issue-a", &[]);

        let cli = make_cli();
        let result = cmd_dep_add(&cli, &paths, "issue-a", "issue-a");

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("self-dependency"));
    }

    #[test]
    fn test_dep_add_cycle_rejected_with_message() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        // a -> b exists, adding b -> a would create cycle
        create_issue(&paths, "issue-a", &["issue-b"]);
        create_issue(&paths, "issue-b", &[]);

        let cli = make_cli();
        let result = cmd_dep_add(&cli, &paths, "issue-b", "issue-a");

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cycle"));
        assert!(err.contains("->"));
    }

    #[test]
    fn test_dep_add_indirect_cycle_rejected() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        // a -> b -> c exists, adding c -> a would create cycle
        create_issue(&paths, "issue-a", &["issue-b"]);
        create_issue(&paths, "issue-b", &["issue-c"]);
        create_issue(&paths, "issue-c", &[]);

        let cli = make_cli();
        let result = cmd_dep_add(&cli, &paths, "issue-c", "issue-a");

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cycle"));
    }

    #[test]
    fn test_dep_add_success() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "issue-a", &[]);
        create_issue(&paths, "issue-b", &[]);

        let cli = make_cli();
        let result = cmd_dep_add(&cli, &paths, "issue-a", "issue-b");

        assert!(result.is_ok());

        // Verify the dep was added by reloading
        let config = Config::default();
        let issues = load_all_issues(&paths, &config).unwrap();
        assert!(issues["issue-a"].deps().contains(&"issue-b".to_string()));
    }

    #[test]
    fn test_dep_add_idempotent() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "issue-a", &["issue-b"]);
        create_issue(&paths, "issue-b", &[]);

        let cli = make_cli();
        // Add same dep again
        let result = cmd_dep_add(&cli, &paths, "issue-a", "issue-b");

        assert!(result.is_ok());

        // Verify no duplicate
        let config = Config::default();
        let issues = load_all_issues(&paths, &config).unwrap();
        let deps: Vec<_> = issues["issue-a"]
            .deps()
            .iter()
            .filter(|d| *d == "issue-b")
            .collect();
        assert_eq!(deps.len(), 1);
    }

    #[test]
    fn test_dep_add_partial_id_resolution() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "brd-abc1", &[]);
        create_issue(&paths, "brd-xyz9", &[]);

        let cli = make_cli();
        // Use partial IDs
        let result = cmd_dep_add(&cli, &paths, "abc1", "xyz9");

        assert!(result.is_ok());

        // Verify dep was added with full IDs
        let config = Config::default();
        let issues = load_all_issues(&paths, &config).unwrap();
        assert!(issues["brd-abc1"].deps().contains(&"brd-xyz9".to_string()));
    }

    #[test]
    fn test_dep_add_child_not_found() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "issue-b", &[]);

        let cli = make_cli();
        let result = cmd_dep_add(&cli, &paths, "nonexistent", "issue-b");

        assert!(result.is_err());
    }

    #[test]
    fn test_dep_add_parent_not_found() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "issue-a", &[]);

        let cli = make_cli();
        let result = cmd_dep_add(&cli, &paths, "issue-a", "nonexistent");

        assert!(result.is_err());
    }

    // =========================================================================
    // cmd_dep_rm tests
    // =========================================================================

    #[test]
    fn test_dep_rm_existing() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "issue-a", &["issue-b"]);
        create_issue(&paths, "issue-b", &[]);

        let cli = make_cli();
        let result = cmd_dep_rm(&cli, &paths, "issue-a", "issue-b");

        assert!(result.is_ok());

        // Verify dep was removed
        let config = Config::default();
        let issues = load_all_issues(&paths, &config).unwrap();
        assert!(!issues["issue-a"].deps().contains(&"issue-b".to_string()));
    }

    #[test]
    fn test_dep_rm_nonexistent_noop() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "issue-a", &[]);
        create_issue(&paths, "issue-b", &[]);

        let cli = make_cli();
        // Remove dep that doesn't exist - should succeed (no-op)
        let result = cmd_dep_rm(&cli, &paths, "issue-a", "issue-b");

        assert!(result.is_ok());
    }

    #[test]
    fn test_dep_rm_partial_id_resolution() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "brd-abc1", &["brd-xyz9"]);
        create_issue(&paths, "brd-xyz9", &[]);

        let cli = make_cli();
        // Use partial IDs
        let result = cmd_dep_rm(&cli, &paths, "abc1", "xyz9");

        assert!(result.is_ok());

        // Verify dep was removed
        let config = Config::default();
        let issues = load_all_issues(&paths, &config).unwrap();
        assert!(!issues["brd-abc1"].deps().contains(&"brd-xyz9".to_string()));
    }

    #[test]
    fn test_dep_rm_child_not_found() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "issue-b", &[]);

        let cli = make_cli();
        let result = cmd_dep_rm(&cli, &paths, "nonexistent", "issue-b");

        assert!(result.is_err());
    }

    #[test]
    fn test_dep_rm_preserves_other_deps() {
        let (_dir, paths) = create_test_repo();
        create_braid_dir(&paths);
        create_valid_config(&paths);
        create_issue(&paths, "issue-a", &["issue-b", "issue-c"]);
        create_issue(&paths, "issue-b", &[]);
        create_issue(&paths, "issue-c", &[]);

        let cli = make_cli();
        let result = cmd_dep_rm(&cli, &paths, "issue-a", "issue-b");

        assert!(result.is_ok());

        // Verify only issue-b was removed, issue-c remains
        let config = Config::default();
        let issues = load_all_issues(&paths, &config).unwrap();
        assert!(!issues["issue-a"].deps().contains(&"issue-b".to_string()));
        assert!(issues["issue-a"].deps().contains(&"issue-c".to_string()));
    }
}
