//! brd done command.

use crate::cli::Cli;
use crate::config::Config;
use crate::error::{BrdError, Result};
use crate::graph::would_create_cycle;
use crate::issue::{Issue, IssueType, Status};
use crate::lock::LockGuard;
use crate::repo::RepoPaths;

use super::start::{commit_and_push_issues_branch_with_action, commit_and_push_main_with_action};
use super::{issue_to_json, load_all_issues, resolve_issue_id};
use std::collections::{HashMap, HashSet};

pub fn cmd_done(
    cli: &Cli,
    paths: &RepoPaths,
    id: &str,
    force: bool,
    result_ids: &[String],
    no_push: bool,
) -> Result<()> {
    let config = Config::load(&paths.config_path())?;
    let _lock = LockGuard::acquire(&paths.lock_path())?;

    let mut issues = load_all_issues(paths, &config)?;
    let full_id = resolve_issue_id(id, &issues)?;
    let mut changed_ids = HashSet::new();

    // check if this is a design issue
    let is_design = issues
        .get(&full_id)
        .map(|i| i.issue_type() == Some(IssueType::Design))
        .unwrap_or(false);

    // design issues require --result or --force
    if is_design && result_ids.is_empty() && !force {
        return Err(BrdError::Other(
            "design issues require --result <issue-id> to specify resulting issues\n\
             use --force to close without results"
                .to_string(),
        ));
    }

    // resolve and validate result issue IDs
    let mut resolved_results = Vec::new();
    let mut seen_results = HashSet::new();
    for result_id in result_ids {
        let resolved = resolve_issue_id(result_id, &issues)?;
        if resolved == full_id {
            return Err(BrdError::Other(
                "design issue cannot list itself as a result".to_string(),
            ));
        }
        if seen_results.insert(resolved.clone()) {
            resolved_results.push(resolved);
        }
    }

    // transfer deps: results inherit this issue's deps, and dependents swap to the results
    if !resolved_results.is_empty() {
        let design_deps = issues
            .get(&full_id)
            .map(|issue| issue.deps().to_vec())
            .unwrap_or_default();

        for result_id in &resolved_results {
            for dep_id in &design_deps {
                if dep_id == result_id {
                    continue;
                }
                let changed = add_dep_checked(&mut issues, result_id, dep_id)?;
                if changed {
                    changed_ids.insert(result_id.clone());
                }
            }
        }

        let dependents: Vec<String> = issues
            .iter()
            .filter(|(_, issue)| issue.deps().contains(&full_id))
            .map(|(id, _)| id.clone())
            .collect();

        // Build a set of result IDs for quick lookup
        let result_set: HashSet<&String> = resolved_results.iter().collect();

        for dependent_id in &dependents {
            {
                let dependent = issues
                    .get_mut(dependent_id)
                    .ok_or_else(|| BrdError::IssueNotFound(dependent_id.clone()))?;
                let before_len = dependent.frontmatter.deps.len();
                dependent.frontmatter.deps.retain(|d| d != &full_id);
                if dependent.frontmatter.deps.len() != before_len {
                    changed_ids.insert(dependent_id.clone());
                }
            }

            // Skip adding result deps if this dependent is itself a result issue.
            // Result issues are parallel outputs, not dependent on each other.
            if result_set.contains(dependent_id) {
                continue;
            }

            for result_id in &resolved_results {
                if dependent_id == result_id {
                    continue;
                }
                let changed = add_dep_checked(&mut issues, dependent_id, result_id)?;
                if changed {
                    changed_ids.insert(dependent_id.clone());
                }
            }
        }

        if !cli.json && !dependents.is_empty() {
            eprintln!(
                "updated deps for {} issue(s): {}",
                dependents.len(),
                dependents.join(", ")
            );
        }
    }

    // mark the issue as done
    {
        let issue = issues
            .get_mut(&full_id)
            .ok_or_else(|| BrdError::IssueNotFound(id.to_string()))?;

        issue.frontmatter.status = Status::Done;
        issue.frontmatter.owner = None;
        issue.mark_completed();
        changed_ids.insert(full_id.clone());
    }

    for issue_id in changed_ids {
        let issue = issues
            .get(&issue_id)
            .ok_or_else(|| BrdError::IssueNotFound(issue_id.clone()))?;
        let issue_path = paths.issues_dir(&config).join(format!("{}.md", issue_id));
        issue.save(&issue_path)?;
    }

    // Commit and push if auto_push is enabled (unless --no-push)
    if !no_push && config.auto_push {
        if config.is_issues_branch_mode() {
            commit_and_push_issues_branch_with_action(paths, &config, &full_id, "done", cli)?;
        } else {
            commit_and_push_main_with_action(paths, &full_id, "done", cli)?;
        }
    }

    if cli.json {
        let issue = issues.get(&full_id).unwrap();
        let json = issue_to_json(issue, &issues);
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        println!("Done: {}", full_id);
    }

    Ok(())
}

fn add_dep_checked(
    issues: &mut HashMap<String, Issue>,
    child_id: &str,
    parent_id: &str,
) -> Result<bool> {
    if child_id == parent_id {
        return Err(BrdError::Other("cannot add self-dependency".to_string()));
    }

    let parent = parent_id.to_string();
    let child = issues
        .get(child_id)
        .ok_or_else(|| BrdError::IssueNotFound(child_id.to_string()))?;

    if child.frontmatter.deps.contains(&parent) {
        return Ok(false);
    }

    if let Some(cycle_path) = would_create_cycle(child_id, &parent, issues) {
        let cycle_str = cycle_path.join(" -> ");
        return Err(BrdError::Other(format!(
            "cannot add dependency: would create cycle: {}",
            cycle_str
        )));
    }

    let child = issues
        .get_mut(child_id)
        .ok_or_else(|| BrdError::IssueNotFound(child_id.to_string()))?;
    child.frontmatter.deps.push(parent);

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{TestRepo, test_cli};

    #[test]
    fn test_done_sets_status_and_clears_owner() {
        let repo = TestRepo::new().build();
        repo.issue("brd-aaaa")
            .status(Status::Doing)
            .owner("tester")
            .create();

        cmd_done(&test_cli(), &repo.paths, "brd-aaaa", false, &[], true).unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let issue = issues.get("brd-aaaa").unwrap();
        assert_eq!(issue.status(), Status::Done);
        assert!(issue.frontmatter.owner.is_none());
    }

    #[test]
    fn test_done_issue_not_found() {
        let repo = TestRepo::new().build();
        let err = cmd_done(&test_cli(), &repo.paths, "brd-missing", false, &[], true).unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }

    #[test]
    fn test_done_ambiguous_id() {
        let repo = TestRepo::new().build();
        repo.issue("brd-aaaa").create();
        repo.issue("brd-aaab").create();

        let err = cmd_done(&test_cli(), &repo.paths, "aaa", false, &[], true).unwrap_err();
        assert!(matches!(err, BrdError::AmbiguousId(_, _)));
    }

    #[test]
    fn test_done_design_requires_result() {
        let repo = TestRepo::new().build();
        repo.issue("brd-design")
            .issue_type(IssueType::Design)
            .create();

        let err = cmd_done(&test_cli(), &repo.paths, "brd-design", false, &[], true).unwrap_err();
        assert!(err.to_string().contains("design issues require --result"));
    }

    #[test]
    fn test_done_design_with_force() {
        let repo = TestRepo::new().build();
        repo.issue("brd-design")
            .issue_type(IssueType::Design)
            .create();

        cmd_done(&test_cli(), &repo.paths, "brd-design", true, &[], true).unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.get("brd-design").unwrap().status(), Status::Done);
    }

    #[test]
    fn test_done_design_with_result() {
        let repo = TestRepo::new().build();
        repo.issue("brd-design")
            .issue_type(IssueType::Design)
            .create();
        repo.issue("brd-impl").create();

        cmd_done(
            &test_cli(),
            &repo.paths,
            "brd-design",
            false,
            &["brd-impl".to_string()],
            true,
        )
        .unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        assert_eq!(issues.get("brd-design").unwrap().status(), Status::Done);
    }

    #[test]
    fn test_done_design_result_not_found() {
        let repo = TestRepo::new().build();
        repo.issue("brd-design")
            .issue_type(IssueType::Design)
            .create();

        let err = cmd_done(
            &test_cli(),
            &repo.paths,
            "brd-design",
            false,
            &["brd-missing".to_string()],
            true,
        )
        .unwrap_err();
        assert!(matches!(err, BrdError::IssueNotFound(_)));
    }

    #[test]
    fn test_done_design_replaces_dependents() {
        let repo = TestRepo::new().build();
        repo.issue("brd-design")
            .issue_type(IssueType::Design)
            .create();
        repo.issue("brd-impl").create();
        repo.issue("brd-dependent").deps(&["brd-design"]).create();

        cmd_done(
            &test_cli(),
            &repo.paths,
            "brd-design",
            false,
            &["brd-impl".to_string()],
            true,
        )
        .unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let dependent = issues.get("brd-dependent").unwrap();
        assert!(dependent.deps().contains(&"brd-impl".to_string()));
        assert!(!dependent.deps().contains(&"brd-design".to_string()));
    }

    #[test]
    fn test_done_design_transfers_deps_to_results() {
        let repo = TestRepo::new().build();
        repo.issue("brd-upstream").create();
        repo.issue("brd-existing").create();
        repo.issue("brd-design")
            .issue_type(IssueType::Design)
            .deps(&["brd-upstream"])
            .create();
        repo.issue("brd-impl").deps(&["brd-existing"]).create();

        cmd_done(
            &test_cli(),
            &repo.paths,
            "brd-design",
            false,
            &["brd-impl".to_string()],
            true,
        )
        .unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let impl_issue = issues.get("brd-impl").unwrap();
        assert!(impl_issue.deps().contains(&"brd-existing".to_string()));
        assert!(impl_issue.deps().contains(&"brd-upstream".to_string()));
    }

    #[test]
    fn test_done_design_multiple_results_that_are_also_dependents() {
        // Scenario: design issue has two impl issues that both depend on it
        // Closing with --result for both should NOT create cross-deps between them
        let repo = TestRepo::new().build();
        repo.issue("brd-design")
            .issue_type(IssueType::Design)
            .create();
        repo.issue("brd-impl1").deps(&["brd-design"]).create();
        repo.issue("brd-impl2").deps(&["brd-design"]).create();

        // This should succeed - impl1 and impl2 are parallel, not dependent on each other
        cmd_done(
            &test_cli(),
            &repo.paths,
            "brd-design",
            false,
            &["brd-impl1".to_string(), "brd-impl2".to_string()],
            true,
        )
        .unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let impl1 = issues.get("brd-impl1").unwrap();
        let impl2 = issues.get("brd-impl2").unwrap();

        // Neither should depend on the other
        assert!(
            !impl1.deps().contains(&"brd-impl2".to_string()),
            "impl1 should not depend on impl2"
        );
        assert!(
            !impl2.deps().contains(&"brd-impl1".to_string()),
            "impl2 should not depend on impl1"
        );
        // Design issue should not be in deps anymore
        assert!(!impl1.deps().contains(&"brd-design".to_string()));
        assert!(!impl2.deps().contains(&"brd-design".to_string()));
        // Design issue should be done
        assert_eq!(issues.get("brd-design").unwrap().status(), Status::Done);
    }

    #[test]
    fn test_done_design_results_and_external_dependent() {
        // Scenario: design has two impl issues that depend on it, PLUS an external issue
        // that also depends on the design. The external should depend on both results.
        let repo = TestRepo::new().build();
        repo.issue("brd-design")
            .issue_type(IssueType::Design)
            .create();
        repo.issue("brd-impl1").deps(&["brd-design"]).create();
        repo.issue("brd-impl2").deps(&["brd-design"]).create();
        repo.issue("brd-external").deps(&["brd-design"]).create();

        cmd_done(
            &test_cli(),
            &repo.paths,
            "brd-design",
            false,
            &["brd-impl1".to_string(), "brd-impl2".to_string()],
            true,
        )
        .unwrap();

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();

        // impl1 and impl2 should have no deps (design dep removed, no cross-deps)
        let impl1 = issues.get("brd-impl1").unwrap();
        let impl2 = issues.get("brd-impl2").unwrap();
        assert!(impl1.deps().is_empty(), "impl1 should have no deps");
        assert!(impl2.deps().is_empty(), "impl2 should have no deps");

        // external should now depend on BOTH impl1 and impl2
        let external = issues.get("brd-external").unwrap();
        assert!(
            external.deps().contains(&"brd-impl1".to_string()),
            "external should depend on impl1"
        );
        assert!(
            external.deps().contains(&"brd-impl2".to_string()),
            "external should depend on impl2"
        );
        assert!(
            !external.deps().contains(&"brd-design".to_string()),
            "external should not depend on design anymore"
        );
    }

    #[test]
    fn test_done_design_rejects_cycles() {
        let repo = TestRepo::new().build();
        repo.issue("brd-design")
            .issue_type(IssueType::Design)
            .create();
        repo.issue("brd-dependent").deps(&["brd-design"]).create();
        repo.issue("brd-impl").deps(&["brd-dependent"]).create();

        let err = cmd_done(
            &test_cli(),
            &repo.paths,
            "brd-design",
            false,
            &["brd-impl".to_string()],
            true,
        )
        .unwrap_err();
        assert!(err.to_string().contains("cycle"));

        let issues = load_all_issues(&repo.paths, &repo.config).unwrap();
        let dependent = issues.get("brd-dependent").unwrap();
        assert!(dependent.deps().contains(&"brd-design".to_string()));
        assert_eq!(issues.get("brd-design").unwrap().status(), Status::Open);
    }
}
