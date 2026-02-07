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
        let issue_path = paths
            .issues_dir(&config)
            .join(format!("{}.md", blocked_full));
        blocked.save(&issue_path)?;
    }

    if cli.json {
        println!(r#"{{"ok": true}}"#);
    } else {
        println!("{} is now blocked by {}", blocked_full, blocker_full);
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
    let issue_path = paths
        .issues_dir(&config)
        .join(format!("{}.md", blocked_full));
    blocked.save(&issue_path)?;

    if cli.json {
        println!(r#"{{"ok": true}}"#);
    } else {
        println!("{} is no longer blocked by {}", blocked_full, blocker_full);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{TestRepo, test_cli};

    // =========================================================================
    // cmd_dep_add tests
    // =========================================================================

    #[test]
    fn test_dep_add_self_dependency_rejected() {
        let repo = TestRepo::builder().build();
        repo.issue("issue-a").create();

        let result = cmd_dep_add(&test_cli(), &repo.paths, "issue-a", "issue-a");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("self-dependency"));
    }

    #[test]
    fn test_dep_add_cycle_rejected_with_message() {
        let repo = TestRepo::builder().build();
        // a -> b exists, adding b -> a would create cycle
        repo.issue("issue-a").deps(&["issue-b"]).create();
        repo.issue("issue-b").create();

        let result = cmd_dep_add(&test_cli(), &repo.paths, "issue-b", "issue-a");

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cycle"));
        assert!(err.contains("->"));
    }

    #[test]
    fn test_dep_add_indirect_cycle_rejected() {
        let repo = TestRepo::builder().build();
        // a -> b -> c exists, adding c -> a would create cycle
        repo.issue("issue-a").deps(&["issue-b"]).create();
        repo.issue("issue-b").deps(&["issue-c"]).create();
        repo.issue("issue-c").create();

        let result = cmd_dep_add(&test_cli(), &repo.paths, "issue-c", "issue-a");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cycle"));
    }

    #[test]
    fn test_dep_add_success() {
        let repo = TestRepo::builder().build();
        repo.issue("issue-a").create();
        repo.issue("issue-b").create();

        let result = cmd_dep_add(&test_cli(), &repo.paths, "issue-a", "issue-b");

        assert!(result.is_ok());
        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert!(issues["issue-a"].deps().contains(&"issue-b".to_string()));
    }

    #[test]
    fn test_dep_add_idempotent() {
        let repo = TestRepo::builder().build();
        repo.issue("issue-a").deps(&["issue-b"]).create();
        repo.issue("issue-b").create();

        // Add same dep again
        let result = cmd_dep_add(&test_cli(), &repo.paths, "issue-a", "issue-b");

        assert!(result.is_ok());
        // Verify no duplicate
        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let deps: Vec<_> = issues["issue-a"]
            .deps()
            .iter()
            .filter(|d| *d == "issue-b")
            .collect();
        assert_eq!(deps.len(), 1);
    }

    #[test]
    fn test_dep_add_partial_id_resolution() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-abc1").create();
        repo.issue("brd-xyz9").create();

        // Use partial IDs
        let result = cmd_dep_add(&test_cli(), &repo.paths, "abc1", "xyz9");

        assert!(result.is_ok());
        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert!(issues["brd-abc1"].deps().contains(&"brd-xyz9".to_string()));
    }

    #[test]
    fn test_dep_add_child_not_found() {
        let repo = TestRepo::builder().build();
        repo.issue("issue-b").create();

        let result = cmd_dep_add(&test_cli(), &repo.paths, "nonexistent", "issue-b");

        assert!(result.is_err());
    }

    #[test]
    fn test_dep_add_parent_not_found() {
        let repo = TestRepo::builder().build();
        repo.issue("issue-a").create();

        let result = cmd_dep_add(&test_cli(), &repo.paths, "issue-a", "nonexistent");

        assert!(result.is_err());
    }

    // =========================================================================
    // cmd_dep_rm tests
    // =========================================================================

    #[test]
    fn test_dep_rm_existing() {
        let repo = TestRepo::builder().build();
        repo.issue("issue-a").deps(&["issue-b"]).create();
        repo.issue("issue-b").create();

        let result = cmd_dep_rm(&test_cli(), &repo.paths, "issue-a", "issue-b");

        assert!(result.is_ok());
        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert!(!issues["issue-a"].deps().contains(&"issue-b".to_string()));
    }

    #[test]
    fn test_dep_rm_nonexistent_noop() {
        let repo = TestRepo::builder().build();
        repo.issue("issue-a").create();
        repo.issue("issue-b").create();

        // Remove dep that doesn't exist - should succeed (no-op)
        let result = cmd_dep_rm(&test_cli(), &repo.paths, "issue-a", "issue-b");

        assert!(result.is_ok());
    }

    #[test]
    fn test_dep_rm_partial_id_resolution() {
        let repo = TestRepo::builder().build();
        repo.issue("brd-abc1").deps(&["brd-xyz9"]).create();
        repo.issue("brd-xyz9").create();

        // Use partial IDs
        let result = cmd_dep_rm(&test_cli(), &repo.paths, "abc1", "xyz9");

        assert!(result.is_ok());
        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert!(!issues["brd-abc1"].deps().contains(&"brd-xyz9".to_string()));
    }

    #[test]
    fn test_dep_rm_child_not_found() {
        let repo = TestRepo::builder().build();
        repo.issue("issue-b").create();

        let result = cmd_dep_rm(&test_cli(), &repo.paths, "nonexistent", "issue-b");

        assert!(result.is_err());
    }

    #[test]
    fn test_dep_rm_preserves_other_deps() {
        let repo = TestRepo::builder().build();
        repo.issue("issue-a").deps(&["issue-b", "issue-c"]).create();
        repo.issue("issue-b").create();
        repo.issue("issue-c").create();

        let result = cmd_dep_rm(&test_cli(), &repo.paths, "issue-a", "issue-b");

        assert!(result.is_ok());
        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert!(!issues["issue-a"].deps().contains(&"issue-b".to_string()));
        assert!(issues["issue-a"].deps().contains(&"issue-c".to_string()));
    }
}
